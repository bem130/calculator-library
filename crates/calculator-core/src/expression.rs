use alloc::{vec, vec::Vec};

use crate::{
    syntax::{SourceExpr, UnaryOperator},
    types::*,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactExpressionDag {
    root: ExprId,
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
}

impl ExactExpressionDag {
    pub(crate) fn root(&self) -> ExprId {
        self.root
    }

    fn node(&self, id: ExprId) -> &ExpressionNode {
        &self.nodes[id.0 as usize]
    }

    fn list(&self, id: ExprListId) -> &[ExprId] {
        &self.lists[id.0 as usize]
    }

    fn rational(&self, id: RationalId) -> &Rational {
        &self.rationals[id.0 as usize]
    }
}

#[derive(Default)]
struct DagBuilder {
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
}

pub(crate) fn lower_source_expression(
    expression: &SourceExpr,
) -> Result<ExactExpressionDag, EvaluationError> {
    let mut builder = DagBuilder::default();
    let root = builder.lower(expression)?;
    Ok(ExactExpressionDag {
        root,
        nodes: builder.nodes,
        lists: builder.lists,
        rationals: builder.rationals,
    })
}

pub(crate) fn evaluate_rational_dag(dag: &ExactExpressionDag) -> Result<Rational, EvaluationError> {
    evaluate_node(dag, dag.root())
}

fn evaluate_node(dag: &ExactExpressionDag, id: ExprId) -> Result<Rational, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => Ok(dag.rational(*id).clone()),
        ExpressionNode::Constant(_) => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::ConstantEvaluation,
            },
        )),
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            for child in dag.list(*list_id) {
                total = total.add(&evaluate_node(dag, *child)?);
            }
            Ok(total)
        }
        ExpressionNode::Multiply(list_id) => {
            let mut product = Rational::one();
            for child in dag.list(*list_id) {
                product = product.multiply(&evaluate_node(dag, *child)?);
            }
            Ok(product)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => evaluate_node(dag, *numerator)?
            .divide(&evaluate_node(dag, *denominator)?)
            .map_err(arithmetic_error),
        ExpressionNode::Power { base, exponent } => {
            let exponent = evaluate_node(dag, *exponent)?.as_i64_if_integer().ok_or(
                EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
                    feature: UnsupportedFeature::NonIntegerPower,
                }),
            )?;
            evaluate_node(dag, *base)?
                .pow_i64(exponent)
                .map_err(arithmetic_error)
        }
        ExpressionNode::Function { .. } => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::FunctionEvaluation,
            },
        )),
    }
}

fn arithmetic_error(error: RationalArithmeticError) -> EvaluationError {
    match error {
        RationalArithmeticError::DivisionByZero => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::DivisionByZero,
            span: None,
        }),
        RationalArithmeticError::ZeroToNegativePower => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::ZeroToNegativePower,
            span: None,
        }),
        RationalArithmeticError::ExponentTooLarge => {
            EvaluationError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::LogicalWorkUnits,
            })
        }
    }
}

impl DagBuilder {
    fn lower(&mut self, expression: &SourceExpr) -> Result<ExprId, EvaluationError> {
        match expression {
            SourceExpr::Number { literal, .. } => {
                let rational = Rational::from_decimal_literal(literal).map_err(|_| {
                    EvaluationError::InternalInvariant(InternalInvariantError {
                        code: InternalInvariantCode::InvalidParsedNumberLiteral,
                    })
                })?;
                Ok(self.push_rational(rational))
            }
            SourceExpr::Constant { constant, .. } => {
                Ok(self.push_node(ExpressionNode::Constant(*constant)))
            }
            SourceExpr::Unary { op, expr, .. } => match op {
                UnaryOperator::Plus => self.lower(expr),
                UnaryOperator::Negate => {
                    let minus_one = self.push_rational(Rational::from_integer(Integer::from(-1)));
                    let value = self.lower(expr)?;
                    let list = self.push_list(vec![minus_one, value]);
                    Ok(self.push_node(ExpressionNode::Multiply(list)))
                }
            },
            SourceExpr::Binary {
                op, left, right, ..
            } => {
                let left = self.lower(left)?;
                let right = self.lower(right)?;
                Ok(match op {
                    BinaryOperator::Add => {
                        let list = self.push_list(vec![left, right]);
                        self.push_node(ExpressionNode::Add(list))
                    }
                    BinaryOperator::Subtract => {
                        let minus_right = self.negate(right);
                        let list = self.push_list(vec![left, minus_right]);
                        self.push_node(ExpressionNode::Add(list))
                    }
                    BinaryOperator::Multiply => {
                        let list = self.push_list(vec![left, right]);
                        self.push_node(ExpressionNode::Multiply(list))
                    }
                    BinaryOperator::Divide => self.push_node(ExpressionNode::Divide {
                        numerator: left,
                        denominator: right,
                    }),
                    BinaryOperator::Power => self.push_node(ExpressionNode::Power {
                        base: left,
                        exponent: right,
                    }),
                })
            }
            SourceExpr::Percent { expr, .. } => {
                let numerator = self.lower(expr)?;
                let denominator = self.push_rational(Rational::from_integer(Integer::from(100)));
                Ok(self.push_node(ExpressionNode::Divide {
                    numerator,
                    denominator,
                }))
            }
            SourceExpr::Function {
                function, argument, ..
            } => {
                let argument = self.lower(argument)?;
                Ok(self.push_node(ExpressionNode::Function {
                    function: *function,
                    argument,
                }))
            }
        }
    }

    fn negate(&mut self, value: ExprId) -> ExprId {
        let minus_one = self.push_rational(Rational::from_integer(Integer::from(-1)));
        let list = self.push_list(vec![minus_one, value]);
        self.push_node(ExpressionNode::Multiply(list))
    }

    fn push_rational(&mut self, rational: Rational) -> ExprId {
        let id = RationalId(self.rationals.len() as u32);
        self.rationals.push(rational);
        self.push_node(ExpressionNode::Rational(id))
    }

    fn push_list(&mut self, values: Vec<ExprId>) -> ExprListId {
        let id = ExprListId(self.lists.len() as u32);
        self.lists.push(values);
        id
    }

    fn push_node(&mut self, node: ExpressionNode) -> ExprId {
        let id = ExprId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;
    use crate::{syntax::parse_source, types::ParseSettings};

    fn lower(source: &str) -> ExactExpressionDag {
        let parsed = parse_source(source, &ParseSettings::default()).expect(source);
        lower_source_expression(&parsed).expect(source)
    }

    #[test]
    fn decimal_addition_lowers_to_rational_addition() {
        let dag = lower("0.1 + 0.2");
        assert_eq!(dag.rationals[0].to_string(), "1/10");
        assert_eq!(dag.rationals[1].to_string(), "1/5");
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Add(_)));
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "3/10");
    }

    #[test]
    fn percent_lowers_to_division_by_one_hundred() {
        let dag = lower("50%");
        let ExpressionNode::Divide { denominator, .. } = dag.node(dag.root()) else {
            panic!("expected percent to lower to division");
        };
        let ExpressionNode::Rational(id) = dag.node(*denominator) else {
            panic!("expected rational denominator");
        };
        assert_eq!(dag.rational(*id).to_string(), "100");
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "1/2");
    }

    #[test]
    fn subtraction_lowers_to_addition_with_negated_rhs() {
        let dag = lower("7 - 2");
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Add(_)));
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "5");
    }
}
