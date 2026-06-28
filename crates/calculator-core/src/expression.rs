use alloc::{vec, vec::Vec};
use core::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::{
    interval::{self, IntervalError},
    syntax::{SourceExpr, UnaryOperator},
    types::*,
};

const EVALUATION_INTERVAL_PRECISION_BITS: u32 = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactExpressionDag {
    root: ExprId,
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
    semantics: SemanticSettings,
}

impl ExactExpressionDag {
    pub(crate) fn root(&self) -> ExprId {
        self.root
    }

    pub(crate) fn semantics(&self) -> SemanticSettings {
        self.semantics
    }

    pub(crate) fn node(&self, id: ExprId) -> &ExpressionNode {
        &self.nodes[id.0 as usize]
    }

    pub(crate) fn nodes(&self) -> &[ExpressionNode] {
        &self.nodes
    }

    pub(crate) fn list(&self, id: ExprListId) -> &[ExprId] {
        &self.lists[id.0 as usize]
    }

    pub(crate) fn rational(&self, id: RationalId) -> &Rational {
        &self.rationals[id.0 as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RationalEvaluation {
    value: Rational,
    used_special_angle: bool,
}

impl RationalEvaluation {
    pub(crate) fn direct(value: Rational) -> Self {
        Self {
            value,
            used_special_angle: false,
        }
    }

    fn special_angle(value: Rational) -> Self {
        Self {
            value,
            used_special_angle: true,
        }
    }

    fn with_origin(value: Rational, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &Rational {
        &self.value
    }

    pub(crate) fn into_value(self) -> Rational {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PiCoefficientEvaluation {
    coefficient: Rational,
    used_special_angle: bool,
}

impl PiCoefficientEvaluation {
    fn direct(coefficient: Rational) -> Self {
        Self {
            coefficient,
            used_special_angle: false,
        }
    }

    fn special_angle(coefficient: Rational) -> Self {
        Self {
            coefficient,
            used_special_angle: true,
        }
    }

    fn with_origin(coefficient: Rational, used_special_angle: bool) -> Self {
        Self {
            coefficient,
            used_special_angle,
        }
    }

    pub(crate) fn coefficient(&self) -> &Rational {
        &self.coefficient
    }

    pub(crate) fn into_coefficient(self) -> Rational {
        self.coefficient
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RadicalEvaluation {
    value: SimpleRadical,
    used_special_angle: bool,
}

impl RadicalEvaluation {
    fn with_origin(value: SimpleRadical, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &SimpleRadical {
        &self.value
    }

    pub(crate) fn into_value(self) -> SimpleRadical {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RadicalLinearCombinationEvaluation {
    value: RadicalLinearCombination,
    used_special_angle: bool,
}

impl RadicalLinearCombinationEvaluation {
    fn with_origin(value: RadicalLinearCombination, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &RadicalLinearCombination {
        &self.value
    }

    pub(crate) fn into_value(self) -> RadicalLinearCombination {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RadicalReduction {
    Rational(RationalEvaluation),
    Radical(RadicalEvaluation),
    LinearCombination(RadicalLinearCombinationEvaluation),
}

impl RadicalReduction {
    fn rational(value: Rational, used_special_angle: bool) -> Self {
        Self::Rational(RationalEvaluation::with_origin(value, used_special_angle))
    }

    fn radical(value: SimpleRadical, used_special_angle: bool) -> Self {
        Self::Radical(RadicalEvaluation::with_origin(value, used_special_angle))
    }

    fn linear_combination(value: RadicalLinearCombination, used_special_angle: bool) -> Self {
        Self::LinearCombination(RadicalLinearCombinationEvaluation::with_origin(
            value,
            used_special_angle,
        ))
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        match self {
            Self::Rational(value) => value.used_special_angle(),
            Self::Radical(value) => value.used_special_angle(),
            Self::LinearCombination(value) => value.used_special_angle(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RealAlgebraicEvaluation {
    Rational(RationalEvaluation),
    Algebraic(RealAlgebraic),
}

impl RealAlgebraicEvaluation {
    fn rational(value: Rational) -> Self {
        Self::Rational(RationalEvaluation::direct(value))
    }

    fn from_algebraic(value: RealAlgebraic) -> Self {
        if let Some(rational) = value.rational_value() {
            Self::rational(rational)
        } else {
            Self::Algebraic(value)
        }
    }

    #[cfg(test)]
    fn into_algebraic(self) -> Option<RealAlgebraic> {
        match self {
            Self::Rational(_) => None,
            Self::Algebraic(value) => Some(value),
        }
    }
}

#[derive(Default)]
struct DagBuilder {
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
    semantics: SemanticSettings,
}

pub(crate) fn lower_source_expression(
    expression: &SourceExpr,
    semantics: SemanticSettings,
) -> Result<ExactExpressionDag, EvaluationError> {
    let mut builder = DagBuilder {
        semantics,
        ..DagBuilder::default()
    };
    let root = builder.lower(expression)?;
    Ok(ExactExpressionDag {
        root,
        nodes: builder.nodes,
        lists: builder.lists,
        rationals: builder.rationals,
        semantics,
    })
}

#[cfg(test)]
pub(crate) fn evaluate_rational_dag(dag: &ExactExpressionDag) -> Result<Rational, EvaluationError> {
    Ok(evaluate_rational_evaluation_dag(dag)?.into_value())
}

pub(crate) fn evaluate_rational_evaluation_dag(
    dag: &ExactExpressionDag,
) -> Result<RationalEvaluation, EvaluationError> {
    evaluate_node(dag, dag.root())
}

pub(crate) fn evaluate_interval_dag(
    dag: &ExactExpressionDag,
) -> Result<CertifiedInterval, IntervalError> {
    evaluate_interval_node(dag, dag.root(), EVALUATION_INTERVAL_PRECISION_BITS)
}

pub(crate) fn evaluate_rational_pi_multiple_dag(
    dag: &ExactExpressionDag,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    evaluate_pi_coefficient(dag, dag.root())
}

pub(crate) fn evaluate_radical_dag(
    dag: &ExactExpressionDag,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    evaluate_radical_node(dag, dag.root())
}

pub(crate) fn evaluate_real_algebraic_dag(
    dag: &ExactExpressionDag,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    evaluate_real_algebraic_node(dag, dag.root(), limits)
}

pub(crate) fn structurally_equal_expressions(
    dag: &ExactExpressionDag,
    left: ExprId,
    right: ExprId,
) -> bool {
    if left == right {
        return true;
    }

    match (dag.node(left), dag.node(right)) {
        (ExpressionNode::Rational(left), ExpressionNode::Rational(right)) => {
            dag.rational(*left) == dag.rational(*right)
        }
        (ExpressionNode::Constant(left), ExpressionNode::Constant(right)) => left == right,
        (ExpressionNode::Add(left), ExpressionNode::Add(right))
        | (ExpressionNode::Multiply(left), ExpressionNode::Multiply(right)) => {
            structurally_equal_expression_lists(dag, *left, *right)
        }
        (
            ExpressionNode::Divide {
                numerator: left_numerator,
                denominator: left_denominator,
            },
            ExpressionNode::Divide {
                numerator: right_numerator,
                denominator: right_denominator,
            },
        ) => {
            structurally_equal_expressions(dag, *left_numerator, *right_numerator)
                && structurally_equal_expressions(dag, *left_denominator, *right_denominator)
        }
        (
            ExpressionNode::Power {
                base: left_base,
                exponent: left_exponent,
            },
            ExpressionNode::Power {
                base: right_base,
                exponent: right_exponent,
            },
        ) => {
            structurally_equal_expressions(dag, *left_base, *right_base)
                && structurally_equal_expressions(dag, *left_exponent, *right_exponent)
        }
        (
            ExpressionNode::LogBase {
                argument: left_argument,
                base: left_base,
            },
            ExpressionNode::LogBase {
                argument: right_argument,
                base: right_base,
            },
        ) => {
            structurally_equal_expressions(dag, *left_argument, *right_argument)
                && structurally_equal_expressions(dag, *left_base, *right_base)
        }
        (
            ExpressionNode::Function {
                function: left_function,
                argument: left_argument,
            },
            ExpressionNode::Function {
                function: right_function,
                argument: right_argument,
            },
        ) => {
            left_function == right_function
                && structurally_equal_expressions(dag, *left_argument, *right_argument)
        }
        (
            ExpressionNode::BinaryFunction {
                function: left_function,
                left: left_left,
                right: left_right,
            },
            ExpressionNode::BinaryFunction {
                function: right_function,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_function == right_function
                && structurally_equal_expressions(dag, *left_left, *right_left)
                && structurally_equal_expressions(dag, *left_right, *right_right)
        }
        _ => false,
    }
}

fn structurally_equal_expression_lists(
    dag: &ExactExpressionDag,
    left: ExprListId,
    right: ExprListId,
) -> bool {
    let left = dag.list(left);
    let right = dag.list(right);
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| structurally_equal_expressions(dag, *left, *right))
}

fn prove_expression_nonzero(dag: &ExactExpressionDag, id: ExprId) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_zero()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.node(id) {
        ExpressionNode::Rational(_) => Ok(false),
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Add(_) => Ok(false),
        ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !prove_expression_nonzero(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(prove_expression_nonzero(dag, *numerator)?
            && prove_expression_nonzero(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_nonzero(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { argument, base } => {
            prove_log_base_domain(dag, *argument, *base)?;
            Ok(prove_log_argument_not_one(dag, *argument)?)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_positive(dag, *argument),
            Function::Exp => expression_domain_known_well_defined(dag, *argument),
            Function::Log | Function::Ln => {
                prove_log_argument_positive(dag, *argument)?;
                prove_log_argument_not_one(dag, *argument)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(false),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn expression_domain_known_well_defined(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(_) => return Ok(true),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.node(id) {
        ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => Ok(true),
        ExpressionNode::Add(list_id) | ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !expression_domain_known_well_defined(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(expression_domain_known_well_defined(dag, *numerator)?
            && prove_expression_nonzero(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_positive(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { argument, base } => {
            prove_log_base_domain(dag, *argument, *base)?;
            Ok(true)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_nonnegative(dag, *argument),
            Function::Exp | Function::Sin | Function::Cos | Function::Atan => {
                expression_domain_known_well_defined(dag, *argument)
            }
            Function::Log | Function::Ln => {
                prove_log_argument_positive(dag, *argument)?;
                Ok(true)
            }
            Function::Tan | Function::Asin | Function::Acos => Ok(false),
            Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => expression_domain_known_well_defined(dag, *argument),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn prove_expression_positive(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_negative() && !value.value().is_zero()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.node(id) {
        ExpressionNode::Rational(_) => Ok(false),
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Add(list_id) => {
            let mut saw_positive = false;
            for child in dag.list(*list_id) {
                if prove_expression_positive(dag, *child)? {
                    saw_positive = true;
                } else if !prove_expression_nonnegative(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(saw_positive)
        }
        ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !prove_expression_positive(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(prove_expression_positive(dag, *numerator)?
            && prove_expression_positive(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_positive(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { .. } => Ok(false),
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_positive(dag, *argument),
            Function::Exp => expression_domain_known_well_defined(dag, *argument),
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh
            | Function::Log
            | Function::Ln => Ok(false),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn prove_expression_nonnegative(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_negative()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.node(id) {
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => prove_expression_nonnegative(dag, *argument),
        ExpressionNode::Function {
            function: Function::Exp,
            argument,
        } => expression_domain_known_well_defined(dag, *argument),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => prove_expression_positive(dag, id),
    }
}

fn prove_log_base_domain(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<(), EvaluationError> {
    prove_log_argument_positive(dag, argument)?;
    let base = match evaluate_node(dag, base) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Err(error),
        Err(error) => return Err(error),
    };
    ensure_log_base_domain(base.value())
}

fn prove_log_argument_positive(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<(), EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => {
            if value.value().is_negative() || value.value().is_zero() {
                Err(logarithm_of_non_positive_error())
            } else {
                Ok(())
            }
        }
        Err(error) if is_unsupported_exact_expression(&error) => Err(error),
        Err(error) => Err(error),
    }
}

fn prove_log_argument_not_one(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(value.value() != &Rational::one()),
        Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn evaluate_node(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => Ok(RationalEvaluation::direct(dag.rational(*id).clone())),
        ExpressionNode::Constant(_) => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::ConstantEvaluation,
            },
        )),
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let child = evaluate_node(dag, *child)?;
                used_special_angle |= child.used_special_angle();
                total = total.add(child.value());
            }
            Ok(RationalEvaluation::with_origin(total, used_special_angle))
        }
        ExpressionNode::Multiply(list_id) => {
            let mut product = Rational::one();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let child = evaluate_node(dag, *child)?;
                used_special_angle |= child.used_special_angle();
                product = product.multiply(child.value());
            }
            Ok(RationalEvaluation::with_origin(product, used_special_angle))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let numerator = match evaluate_node(dag, *numerator) {
                Ok(value) => value,
                Err(error)
                    if is_unsupported_exact_expression(&error)
                        && structurally_equal_expressions(dag, *numerator, *denominator)
                        && prove_expression_nonzero(dag, *numerator)? =>
                {
                    return Ok(RationalEvaluation::direct(Rational::one()));
                }
                Err(error) => return Err(error),
            };
            let denominator = evaluate_node(dag, *denominator)?;
            let used_special_angle =
                numerator.used_special_angle() || denominator.used_special_angle();
            let value = numerator
                .value()
                .divide(denominator.value())
                .map_err(arithmetic_error)?;
            Ok(RationalEvaluation::with_origin(value, used_special_angle))
        }
        ExpressionNode::Power { base, exponent } => evaluate_power(dag, *base, *exponent),
        ExpressionNode::LogBase { argument, base } => {
            evaluate_log_base_function(dag, *argument, *base)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Abs => {
                let argument = evaluate_node(dag, *argument)?;
                let value = if argument.value().is_negative() {
                    argument.value().negate()
                } else {
                    argument.value().clone()
                };
                Ok(RationalEvaluation::with_origin(
                    value,
                    argument.used_special_angle(),
                ))
            }
            Function::Floor => {
                let argument = evaluate_node(dag, *argument)?;
                Ok(RationalEvaluation::with_origin(
                    rational_floor(argument.value()),
                    argument.used_special_angle(),
                ))
            }
            Function::Factorial => {
                let argument = evaluate_node(dag, *argument)?;
                evaluate_factorial(argument.value()).map(|value| {
                    RationalEvaluation::with_origin(value, argument.used_special_angle())
                })
            }
            Function::Sqrt => {
                let argument = evaluate_node(dag, *argument)?;
                if argument.value().is_negative() {
                    return Err(EvaluationError::Domain(DomainError {
                        kind: DomainErrorKind::EvenRootOfNegative,
                        span: None,
                    }));
                }
                argument
                    .value()
                    .sqrt_if_rational()
                    .map(|value| {
                        RationalEvaluation::with_origin(value, argument.used_special_angle())
                    })
                    .ok_or_else(unsupported_function_evaluation)
            }
            Function::Exp => evaluate_exp_function(dag, *argument),
            Function::Log | Function::Ln => evaluate_log_function(dag, *argument),
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin | Function::Acos | Function::Atan => {
                evaluate_inverse_trigonometric_function(dag, *function, *argument)
            }
            Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Err(unsupported_function_evaluation()),
        },
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => evaluate_binary_function(dag, *function, *left, *right),
    }
}

const MAX_EXACT_INTEGER_FUNCTION_ORDER: u32 = 4096;

fn evaluate_binary_function(
    dag: &ExactExpressionDag,
    function: Function,
    left: ExprId,
    right: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let left = evaluate_node(dag, left)?;
    let right = evaluate_node(dag, right)?;
    let used_special_angle = left.used_special_angle() || right.used_special_angle();
    let value = match function {
        Function::Gcd => evaluate_gcd(left.value(), right.value())?,
        Function::Lcm => evaluate_lcm(left.value(), right.value())?,
        Function::Modulo => evaluate_modulo(left.value(), right.value())?,
        Function::Permutation => evaluate_permutation(left.value(), right.value())?,
        Function::Combination => evaluate_combination(left.value(), right.value())?,
        Function::Root
        | Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => return Err(unsupported_function_evaluation()),
    };
    Ok(RationalEvaluation::with_origin(value, used_special_angle))
}

fn rational_floor(value: &Rational) -> Rational {
    Rational::from_integer(Integer::from_bigint(
        value
            .numerator
            .inner
            .div_floor(&value.denominator.inner.inner),
    ))
}

fn evaluate_factorial(value: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(value)?;
    ensure_integer_function_order(n)?;
    Ok(Rational::from_integer(Integer::from_bigint(
        factorial_bigint(n),
    )))
}

fn evaluate_gcd(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?.abs();
    let right = integer_bigint(right)?.abs();
    Ok(Rational::from_integer(Integer::from_bigint(
        left.gcd(&right),
    )))
}

fn evaluate_lcm(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?;
    let right = integer_bigint(right)?;
    if left.is_zero() || right.is_zero() {
        return Ok(Rational::zero());
    }
    let gcd = left.gcd(&right);
    Ok(Rational::from_integer(Integer::from_bigint(
        (left / gcd * right).abs(),
    )))
}

fn evaluate_modulo(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?;
    let right = integer_bigint(right)?;
    if right.is_zero() {
        return Err(domain_error(DomainErrorKind::DivisionByZero));
    }
    Ok(Rational::from_integer(Integer::from_bigint(
        left.mod_floor(&right),
    )))
}

fn evaluate_permutation(n: &Rational, r: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(n)?;
    let r = nonnegative_integer_u32(r)?;
    ensure_integer_function_order(n)?;
    if r > n {
        return Ok(Rational::zero());
    }
    let mut value = BigInt::one();
    for factor in (n - r + 1)..=n {
        value *= BigInt::from(factor);
    }
    Ok(Rational::from_integer(Integer::from_bigint(value)))
}

fn evaluate_combination(n: &Rational, r: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(n)?;
    let mut r = nonnegative_integer_u32(r)?;
    ensure_integer_function_order(n)?;
    if r > n {
        return Ok(Rational::zero());
    }
    r = r.min(n - r);
    let mut value = BigInt::one();
    for i in 1..=r {
        value *= BigInt::from(n - r + i);
        value /= BigInt::from(i);
    }
    Ok(Rational::from_integer(Integer::from_bigint(value)))
}

fn integer_bigint(value: &Rational) -> Result<BigInt, EvaluationError> {
    if value.is_integer() {
        Ok(value.numerator.inner.clone())
    } else {
        Err(domain_error(
            DomainErrorKind::IntegerFunctionRequiresInteger,
        ))
    }
}

fn nonnegative_integer_u32(value: &Rational) -> Result<u32, EvaluationError> {
    let integer = integer_bigint(value)?;
    if integer.is_negative() {
        return Err(domain_error(
            DomainErrorKind::IntegerFunctionRequiresNonNegative,
        ));
    }
    integer.to_u32().ok_or_else(logical_work_limit_error)
}

fn ensure_integer_function_order(value: u32) -> Result<(), EvaluationError> {
    if value > MAX_EXACT_INTEGER_FUNCTION_ORDER {
        Err(logical_work_limit_error())
    } else {
        Ok(())
    }
}

fn factorial_bigint(value: u32) -> BigInt {
    let mut product = BigInt::one();
    for factor in 2..=value {
        product *= BigInt::from(factor);
    }
    product
}

fn evaluate_pi_coefficient(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => {
            let value = dag.rational(*id);
            Ok(value
                .is_zero()
                .then(|| PiCoefficientEvaluation::direct(Rational::zero())))
        }
        ExpressionNode::Constant(Constant::Pi) => {
            Ok(Some(PiCoefficientEvaluation::direct(Rational::one())))
        }
        ExpressionNode::Constant(Constant::Euler) => Ok(None),
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
                    return Ok(None);
                };
                used_special_angle |= coefficient.used_special_angle();
                total = total.add(coefficient.coefficient());
            }
            Ok(Some(PiCoefficientEvaluation::with_origin(
                total,
                used_special_angle,
            )))
        }
        ExpressionNode::Multiply(list_id) => {
            let mut scalar = Rational::one();
            let mut pi_coefficient = None;
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                match evaluate_node(dag, *child) {
                    Ok(value) => {
                        if value.value().is_zero() {
                            return Ok(Some(PiCoefficientEvaluation::with_origin(
                                Rational::zero(),
                                value.used_special_angle(),
                            )));
                        }
                        used_special_angle |= value.used_special_angle();
                        scalar = scalar.multiply(value.value());
                    }
                    Err(error) if is_unsupported_exact_expression(&error) => {
                        let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
                            return Ok(None);
                        };
                        if pi_coefficient.is_some() {
                            return Ok(None);
                        }
                        used_special_angle |= coefficient.used_special_angle();
                        pi_coefficient = Some(coefficient);
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(pi_coefficient.map(|coefficient| {
                PiCoefficientEvaluation::with_origin(
                    scalar.multiply(coefficient.coefficient()),
                    used_special_angle,
                )
            }))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let Some(numerator) = evaluate_pi_coefficient(dag, *numerator)? else {
                return Ok(None);
            };
            let denominator = match evaluate_node(dag, *denominator) {
                Ok(value) => value.into_value(),
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            let used_special_angle = numerator.used_special_angle();
            numerator
                .coefficient()
                .divide(&denominator)
                .map(|coefficient| {
                    Some(PiCoefficientEvaluation::with_origin(
                        coefficient,
                        used_special_angle,
                    ))
                })
                .map_err(arithmetic_error)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Asin | Function::Acos | Function::Atan
                if dag.semantics().angle_unit == AngleUnit::Radian =>
            {
                evaluate_inverse_trig_pi_coefficient(dag, *function, *argument)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Power { .. } => Ok(None),
        ExpressionNode::LogBase { .. } => Ok(None),
        ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_radical_node(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Add(list_id) => evaluate_radical_sum(dag, *list_id),
        ExpressionNode::Multiply(list_id) => evaluate_radical_product(dag, *list_id),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let Some(numerator) = evaluate_radical_reduction(dag, *numerator)? else {
                return Ok(None);
            };
            let Some(denominator) = evaluate_radical_reduction(dag, *denominator)? else {
                return Ok(None);
            };
            reduce_radical_quotient(
                &numerator,
                &denominator,
                numerator.used_special_angle() || denominator.used_special_angle(),
            )
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => {
                let Some(argument) = evaluate_radical_reduction(dag, *argument)? else {
                    return Ok(None);
                };
                reduce_square_root(&argument, DomainErrorKind::EvenRootOfNegative)
            }
            Function::Exp => evaluate_radical_exp_function(dag, *argument),
            Function::Log | Function::Ln => evaluate_radical_log_function(dag, *argument),
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_radical_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Power { base, exponent } => {
            let exponent = match evaluate_node(dag, *exponent) {
                Ok(value) => value,
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            if exponent.value() != &rational(1, 2) {
                if !exponent.value().is_integer() {
                    return Ok(None);
                }
                let Some(exponent_value) = exponent.value().as_i64_if_integer() else {
                    return Err(exponent_too_large_error());
                };
                let Some(base) = evaluate_radical_reduction(dag, *base)? else {
                    return Ok(None);
                };
                return reduce_radical_integer_power(
                    &base,
                    exponent_value,
                    base.used_special_angle() || exponent.used_special_angle(),
                );
            }
            let Some(base) = evaluate_radical_reduction(dag, *base)? else {
                return Ok(None);
            };
            let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
            reduce_square_root(&base, DomainErrorKind::NonRealPower).map(|value| {
                value.map(|value| radical_reduction_with_origin(value, used_special_angle))
            })
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_radical_reduction(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    Ok(evaluate_radical_reduction_with_origin(dag, id)?.map(|(value, _)| value))
}

fn evaluate_radical_reduction_with_origin(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<(RadicalReduction, bool)>, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(Some((RadicalReduction::Rational(value), true))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            Ok(evaluate_radical_node(dag, id)?.map(|value| (value, false)))
        }
        Err(error) => Err(error),
    }
}

fn evaluate_real_algebraic_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Power { base, exponent } => {
            evaluate_real_algebraic_power(dag, *base, *exponent, limits)
        }
        ExpressionNode::Add(list_id) => evaluate_real_algebraic_sum(dag, *list_id, limits),
        ExpressionNode::Multiply(list_id) => evaluate_real_algebraic_product(dag, *list_id, limits),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => evaluate_real_algebraic_quotient(dag, *numerator, *denominator, limits),
        ExpressionNode::Function { function, argument } => match function {
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_real_algebraic_trigonometric_function(dag, *function, *argument, limits)
            }
            Function::Exp => evaluate_real_algebraic_exp_function(dag, *argument, limits),
            Function::Log | Function::Ln => {
                evaluate_real_algebraic_log_function(dag, *argument, limits)
            }
            Function::Sqrt => evaluate_real_algebraic_sqrt_function(dag, *argument, limits),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_real_algebraic_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let exponent = match evaluate_node(dag, exponent) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match evaluate_node(dag, base) {
        Ok(base) => rational_power_algebraic(base.value(), exponent.value(), limits),
        Err(error) if is_unsupported_exact_expression(&error) => {
            let Some(base) = evaluate_real_algebraic_node(dag, base, limits)? else {
                return Ok(None);
            };
            match base {
                RealAlgebraicEvaluation::Rational(base) => {
                    evaluate_collapsed_rational_power(base, exponent, limits)
                }
                RealAlgebraicEvaluation::Algebraic(base) => {
                    if !exponent.value().is_integer() {
                        return real_algebraic_rational_power(
                            base,
                            exponent.value(),
                            DomainErrorKind::NonRealPower,
                            limits,
                        );
                    }
                    let Some(exponent) = exponent.value().as_i64_if_integer() else {
                        return Err(exponent_too_large_error());
                    };
                    real_algebraic_integer_power(base, exponent, limits)
                }
            }
        }
        Err(error) => Err(error),
    }
}

fn evaluate_collapsed_rational_power(
    base: RationalEvaluation,
    exponent: RationalEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
    match evaluate_rational_power(base.value(), exponent.value()) {
        Ok(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(value, used_special_angle),
        ))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            rational_power_algebraic(base.value(), exponent.value(), limits)
        }
        Err(error) => Err(error),
    }
}

fn evaluate_real_algebraic_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let mut rational = Rational::zero();
    let mut algebraic = None;
    let mut used_algebraic_path = false;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.add(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                used_algebraic_path = true;
                match value {
                    RealAlgebraicEvaluation::Rational(value) => {
                        rational = rational.add(value.value());
                    }
                    RealAlgebraicEvaluation::Algebraic(value) => {
                        algebraic = match algebraic {
                            Some(current) => {
                                let Some(sum) = add_real_algebraics(current, value, limits)? else {
                                    return Ok(None);
                                };
                                match sum {
                                    RealAlgebraicEvaluation::Rational(value) => {
                                        rational = rational.add(value.value());
                                        None
                                    }
                                    RealAlgebraicEvaluation::Algebraic(value) => Some(value),
                                }
                            }
                            None => Some(value),
                        };
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(used_algebraic_path.then(|| RealAlgebraicEvaluation::rational(rational)));
    };
    if rational.is_zero() {
        return Ok(Some(RealAlgebraicEvaluation::from_algebraic(algebraic)));
    }
    match algebraic.add_rational_bounded(
        &rational,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn add_real_algebraics(
    lhs: RealAlgebraic,
    rhs: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match lhs.add_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn evaluate_real_algebraic_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let mut rational = Rational::one();
    let mut algebraic = None;
    let mut used_algebraic_path = false;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.multiply(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                used_algebraic_path = true;
                match value {
                    RealAlgebraicEvaluation::Rational(value) => {
                        rational = rational.multiply(value.value());
                    }
                    RealAlgebraicEvaluation::Algebraic(value) => {
                        algebraic = match algebraic {
                            Some(current) => {
                                let Some(product) =
                                    multiply_real_algebraics(current, value, limits)?
                                else {
                                    return Ok(None);
                                };
                                match product {
                                    RealAlgebraicEvaluation::Rational(value) => {
                                        rational = rational.multiply(value.value());
                                        None
                                    }
                                    RealAlgebraicEvaluation::Algebraic(value) => Some(value),
                                }
                            }
                            None => Some(value),
                        };
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(used_algebraic_path.then(|| RealAlgebraicEvaluation::rational(rational)));
    };
    scale_real_algebraic_by_rational(algebraic, &rational, limits)
}

fn multiply_real_algebraics(
    lhs: RealAlgebraic,
    rhs: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match lhs.multiply_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn evaluate_real_algebraic_quotient(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(denominator) = evaluate_rational_or_real_algebraic_node(dag, denominator, limits)?
    else {
        return Ok(None);
    };
    let Some(numerator) = evaluate_rational_or_real_algebraic_node(dag, numerator, limits)? else {
        return Ok(None);
    };
    divide_real_algebraic_evaluations(numerator, denominator, limits)
}

fn evaluate_rational_or_real_algebraic_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(Some(RealAlgebraicEvaluation::Rational(value))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            evaluate_real_algebraic_node(dag, id, limits)
        }
        Err(error) => Err(error),
    }
}

fn divide_real_algebraic_evaluations(
    numerator: RealAlgebraicEvaluation,
    denominator: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match denominator {
        RealAlgebraicEvaluation::Rational(denominator) => {
            let reciprocal = Rational::one()
                .divide(denominator.value())
                .map_err(arithmetic_error)?;
            multiply_real_algebraic_evaluation_by_rational(numerator, &reciprocal, limits)
        }
        RealAlgebraicEvaluation::Algebraic(denominator) => {
            let Some(reciprocal) = reciprocal_real_algebraic(denominator, limits)? else {
                return Ok(None);
            };
            multiply_real_algebraic_evaluations(numerator, reciprocal, limits)
        }
    }
}

fn multiply_real_algebraic_evaluations(
    lhs: RealAlgebraicEvaluation,
    rhs: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match (lhs, rhs) {
        (RealAlgebraicEvaluation::Rational(lhs), RealAlgebraicEvaluation::Rational(rhs)) => {
            let used_special_angle = lhs.used_special_angle() || rhs.used_special_angle();
            Ok(Some(RealAlgebraicEvaluation::Rational(
                RationalEvaluation::with_origin(
                    lhs.value().multiply(rhs.value()),
                    used_special_angle,
                ),
            )))
        }
        (RealAlgebraicEvaluation::Algebraic(lhs), RealAlgebraicEvaluation::Rational(rhs))
        | (RealAlgebraicEvaluation::Rational(rhs), RealAlgebraicEvaluation::Algebraic(lhs)) => {
            multiply_real_algebraic_evaluation_by_rational(
                RealAlgebraicEvaluation::Algebraic(lhs),
                rhs.value(),
                limits,
            )
        }
        (RealAlgebraicEvaluation::Algebraic(lhs), RealAlgebraicEvaluation::Algebraic(rhs)) => {
            multiply_real_algebraics(lhs, rhs, limits)
        }
    }
}

fn multiply_real_algebraic_evaluation_by_rational(
    value: RealAlgebraicEvaluation,
    scalar: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(
                value.value().multiply(scalar),
                value.used_special_angle(),
            ),
        ))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            scale_real_algebraic_by_rational(value, scalar, limits)
        }
    }
}

fn scale_real_algebraic_by_rational(
    algebraic: RealAlgebraic,
    scalar: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if scalar.is_zero() {
        return Ok(Some(RealAlgebraicEvaluation::rational(Rational::zero())));
    }
    if scalar == &Rational::one() {
        return Ok(Some(RealAlgebraicEvaluation::from_algebraic(algebraic)));
    }
    match algebraic.scale_rational_bounded(
        scalar,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn reciprocal_real_algebraic(
    algebraic: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match algebraic.reciprocal_bounded(
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn real_algebraic_integer_power(
    base: RealAlgebraic,
    exponent: i64,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if exponent == 0 {
        return Ok(Some(RealAlgebraicEvaluation::rational(Rational::one())));
    }
    let magnitude = exponent
        .checked_abs()
        .ok_or_else(exponent_too_large_error)?;
    let magnitude = u32::try_from(magnitude).map_err(|_| exponent_too_large_error())?;
    let base = if exponent < 0 {
        let Some(reciprocal) = reciprocal_real_algebraic(base, limits)? else {
            return Ok(None);
        };
        reciprocal
    } else {
        RealAlgebraicEvaluation::Algebraic(base)
    };
    real_algebraic_positive_integer_power(base, magnitude, limits)
}

fn real_algebraic_positive_integer_power(
    base: RealAlgebraicEvaluation,
    mut exponent: u32,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    debug_assert!(exponent > 0);
    let mut result = None;
    let mut factor = base;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(current) => {
                    let Some(product) =
                        multiply_real_algebraic_evaluations(current, factor.clone(), limits)?
                    else {
                        return Ok(None);
                    };
                    product
                }
                None => factor.clone(),
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            let Some(product) =
                multiply_real_algebraic_evaluations(factor.clone(), factor, limits)?
            else {
                return Ok(None);
            };
            factor = product;
        }
    }
    Ok(result)
}

fn evaluate_real_algebraic_sqrt_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(value) = evaluate_rational_or_real_algebraic_node(dag, argument, limits)? else {
        return Ok(None);
    };
    match value {
        RealAlgebraicEvaluation::Rational(value) => evaluate_collapsed_rational_power(
            value,
            RationalEvaluation::direct(rational(1, 2)),
            limits,
        ),
        RealAlgebraicEvaluation::Algebraic(value) => {
            real_algebraic_nth_root(value, 2, DomainErrorKind::EvenRootOfNegative, limits)
        }
    }
}

fn real_algebraic_rational_power(
    value: RealAlgebraic,
    exponent: &Rational,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if exponent.is_integer() {
        let Some(exponent) = exponent.as_i64_if_integer() else {
            return Err(exponent_too_large_error());
        };
        return real_algebraic_integer_power(value, exponent, limits);
    }

    let Some(root_index) = exponent.denominator.inner.inner.to_u32() else {
        return Ok(None);
    };
    if root_index <= 1 || root_index > limits.max_resultant_degree {
        return Ok(None);
    }
    let Some(exponent_numerator) = exponent.numerator.inner.to_i64() else {
        return Ok(None);
    };

    match value.sign_bounded(limits.max_root_isolation_steps) {
        Ok(Some(Ordering::Equal)) => {
            return evaluate_zero_power(exponent)
                .map(RealAlgebraicEvaluation::rational)
                .map(Some);
        }
        Ok(Some(Ordering::Less)) if root_index.is_multiple_of(2) => {
            return Err(domain_error(negative_even_root_error));
        }
        Ok(Some(Ordering::Greater | Ordering::Less)) => {}
        Ok(None)
        | Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    }

    let Some(powered) = real_algebraic_integer_power(value, exponent_numerator, limits)? else {
        return Ok(None);
    };
    nth_root_real_algebraic_evaluation(powered, root_index, negative_even_root_error, limits)
}

fn real_algebraic_nth_root(
    value: RealAlgebraic,
    root_index: u32,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if root_index <= 1 {
        return Ok(Some(RealAlgebraicEvaluation::Algebraic(value)));
    }
    match value.sign_bounded(limits.max_root_isolation_steps) {
        Ok(Some(Ordering::Greater)) => {}
        Ok(Some(Ordering::Equal)) => {
            return Ok(Some(RealAlgebraicEvaluation::rational(Rational::zero())));
        }
        Ok(Some(Ordering::Less)) if root_index.is_multiple_of(2) => {
            return Err(domain_error(negative_even_root_error));
        }
        Ok(Some(Ordering::Less)) => {}
        Ok(None)
        | Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    }

    match value.principal_nth_root_bounded(
        root_index,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn nth_root_real_algebraic_evaluation(
    value: RealAlgebraicEvaluation,
    root_index: u32,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => evaluate_collapsed_rational_power(
            value,
            RationalEvaluation::direct(rational(1, i64::from(root_index))),
            limits,
        ),
        RealAlgebraicEvaluation::Algebraic(value) => {
            real_algebraic_nth_root(value, root_index, negative_even_root_error, limits)
        }
    }
}

fn evaluate_real_algebraic_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Log,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    let Some(value) = evaluate_rational_or_real_algebraic_node(dag, *argument, limits)? else {
        return Ok(None);
    };
    match real_algebraic_log_domain_proof(&value, limits)? {
        LogDomainProof::Positive => Ok(Some(value)),
        LogDomainProof::NonPositive => Err(logarithm_of_non_positive_error()),
        LogDomainProof::Unknown => Ok(None),
    }
}

fn evaluate_real_algebraic_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    evaluate_rational_or_real_algebraic_node(dag, *argument, limits)
}

fn evaluate_real_algebraic_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if let Some(value) =
        evaluate_real_algebraic_trigonometric_special_angle(dag, function, argument, limits)?
    {
        return Ok(Some(value));
    }
    evaluate_real_algebraic_inverse_trig_composition(dag, function, argument, limits)
}

fn evaluate_real_algebraic_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(coefficient) = evaluate_pi_coefficient(dag, argument)? else {
        return Ok(None);
    };
    match function {
        Function::Sin => Ok(
            cyclotomic_sine_algebraic(coefficient.coefficient(), limits)?
                .map(RealAlgebraicEvaluation::from_algebraic),
        ),
        Function::Cos => Ok(
            cyclotomic_cosine_algebraic(coefficient.coefficient(), limits)?
                .map(RealAlgebraicEvaluation::from_algebraic),
        ),
        Function::Tan => {
            let phase = coefficient.coefficient().modulo_integer(1);
            if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
                return Err(domain_error(DomainErrorKind::TangentPole));
            }
            let Some(sine) = cyclotomic_sine_algebraic(coefficient.coefficient(), limits)? else {
                return Ok(None);
            };
            let Some(cosine) = cyclotomic_cosine_algebraic(coefficient.coefficient(), limits)?
            else {
                return Ok(None);
            };
            let Some(reciprocal_cosine) = reciprocal_real_algebraic(cosine, limits)? else {
                return Ok(None);
            };
            multiply_real_algebraic_evaluations(
                RealAlgebraicEvaluation::from_algebraic(sine),
                reciprocal_cosine,
                limits,
            )
        }
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => Ok(None),
    }
}

fn cyclotomic_sine_algebraic(
    coefficient: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let cosine_coefficient = rational(1, 2).subtract(coefficient);
    cyclotomic_cosine_algebraic(&cosine_coefficient, limits)
}

fn cyclotomic_cosine_algebraic(
    coefficient: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let phase = coefficient.modulo_integer(2);
    let Some((numerator, denominator)) = cyclotomic_phase_parts(&phase) else {
        return Ok(None);
    };
    if denominator > limits.max_cyclotomic_order {
        return Ok(None);
    }

    let candidate_polynomial = chebyshev_cosine_candidate_polynomial(numerator, denominator)?;
    let Some(root_index) = cosine_candidate_root_index(numerator, denominator) else {
        return Ok(None);
    };
    let intervals = match candidate_polynomial.isolate_real_roots(limits.max_root_isolation_steps) {
        Ok(intervals) => intervals,
        Err(PrimitivePolynomialRootIsolationError::StepLimitExceeded) => return Ok(None),
        Err(error) => {
            return Err(real_algebraic_construction_error(
                RealAlgebraicConstructionError::RootIsolation(error),
            ));
        }
    };
    let Some(isolating_interval) = intervals.get(root_index).cloned() else {
        return Err(invalid_algebraic_isolation_error());
    };

    match RealAlgebraic::from_candidate_polynomial_bounded(
        candidate_polynomial,
        isolating_interval,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn cyclotomic_phase_parts(phase: &Rational) -> Option<(u32, u32)> {
    let numerator = phase.numerator.inner.to_u32()?;
    let denominator = phase.denominator.inner.inner.to_u32()?;
    Some((numerator, denominator))
}

fn cosine_candidate_root_index(numerator: u32, denominator: u32) -> Option<usize> {
    let denominator = u64::from(denominator);
    let period = denominator.checked_mul(2)?;
    let phase_numerator = u64::from(numerator) % period;
    let target_angle_numerator = if phase_numerator > denominator {
        period - phase_numerator
    } else {
        phase_numerator
    };
    let parity = u64::from(numerator % 2);
    let index = ((target_angle_numerator + 1)..=denominator)
        .filter(|candidate| candidate % 2 == parity)
        .count();
    Some(index)
}

fn chebyshev_cosine_candidate_polynomial(
    numerator: u32,
    denominator: u32,
) -> Result<PrimitivePolynomial, EvaluationError> {
    let mut polynomial = chebyshev_t_polynomial(denominator)?;
    let target = if numerator.is_multiple_of(2) { 1 } else { -1 };
    polynomial.coefficients_low_to_high[0].inner -= target;
    PrimitivePolynomial::new(polynomial.coefficients_low_to_high)
        .map_err(|_| invalid_algebraic_isolation_error())
}

fn chebyshev_t_polynomial(order: u32) -> Result<PrimitivePolynomial, EvaluationError> {
    let mut previous = vec![Integer::one()];
    if order == 0 {
        return PrimitivePolynomial::new(previous).map_err(|_| invalid_algebraic_isolation_error());
    }

    let mut current = vec![Integer::zero(), Integer::one()];
    for _ in 1..order {
        let mut next = vec![Integer::zero(); current.len() + 1];
        for (index, coefficient) in current.iter().enumerate() {
            next[index + 1].inner += &coefficient.inner * 2;
        }
        for (index, coefficient) in previous.iter().enumerate() {
            next[index].inner -= &coefficient.inner;
        }
        previous = current;
        current = next;
    }
    PrimitivePolynomial::new(current).map_err(|_| invalid_algebraic_isolation_error())
}

fn rational_power_algebraic(
    base: &Rational,
    exponent: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if base.is_zero() || exponent.is_integer() {
        return Ok(None);
    }
    let Some(root_index) = exponent.denominator.inner.inner.to_u32() else {
        return Ok(None);
    };
    if root_index <= 1 || root_index > limits.max_resultant_degree {
        return Ok(None);
    }
    if base.is_negative() && root_index.is_multiple_of(2) {
        return Err(domain_error(DomainErrorKind::NonRealPower));
    }
    let Some(exponent_numerator) = exponent.numerator.inner.to_i64() else {
        return Ok(None);
    };
    let powered_base = match base.pow_i64(exponent_numerator) {
        Ok(value) => value,
        Err(RationalArithmeticError::ExponentTooLarge) => return Ok(None),
        Err(error) => return Err(arithmetic_error(error)),
    };

    let root = match RealAlgebraic::nth_root_of_rational_bounded(
        &powered_base,
        root_index,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(Some(root)) => RealAlgebraicEvaluation::from_algebraic(root),
        Ok(None) => return Ok(None),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    };
    Ok(Some(root))
}

fn evaluate_radical_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let mut rational = Rational::zero();
    let mut radicals = Vec::new();
    let mut used_special_angle = false;
    let mut saw_radical = false;

    for child in dag.list(list_id) {
        let Some((value, direct_rational)) = evaluate_radical_reduction_with_origin(dag, *child)?
        else {
            return Ok(None);
        };
        saw_radical |= !direct_rational;
        match value {
            RadicalReduction::Rational(value) => {
                used_special_angle |= value.used_special_angle();
                rational = rational.add(value.value());
            }
            RadicalReduction::Radical(radical) => {
                used_special_angle |= radical.used_special_angle();
                add_radical_term(&mut radicals, radical.value());
            }
            RadicalReduction::LinearCombination(value) => {
                used_special_angle |= value.used_special_angle();
                rational = rational.add(&value.value().rational);
                for radical in &value.value().radicals {
                    add_radical_term(&mut radicals, radical);
                }
            }
        }
    }

    if !saw_radical {
        return Ok(None);
    }
    Ok(Some(radical_linear_combination_reduction(
        rational,
        radicals,
        used_special_angle,
    )))
}

fn evaluate_radical_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let mut product = RadicalReduction::rational(Rational::one(), false);
    let mut saw_non_rational = false;

    for child in dag.list(list_id) {
        let Some((value, direct_rational)) = evaluate_radical_reduction_with_origin(dag, *child)?
        else {
            return Ok(None);
        };
        saw_non_rational |= !direct_rational;
        let used_special_angle = product.used_special_angle() || value.used_special_angle();
        let Some(next_product) = multiply_radical_reductions(&product, &value, used_special_angle)?
        else {
            return Ok(None);
        };
        product = next_product;
    }

    if !saw_non_rational {
        return Ok(None);
    }
    Ok(Some(product))
}

fn evaluate_radical_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if let Some(value) = evaluate_radical_trigonometric_special_angle(dag, function, argument)? {
        return Ok(Some(value));
    }
    evaluate_radical_inverse_trig_composition(dag, function, argument)
}

fn evaluate_radical_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(coefficient) = evaluate_pi_coefficient(dag, argument)? else {
        return Ok(None);
    };
    let value = match function {
        Function::Sin => sine_radical_special_angle(coefficient.coefficient()),
        Function::Cos => cosine_radical_special_angle(coefficient.coefficient()),
        Function::Tan => tangent_radical_special_angle(coefficient.coefficient())?,
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    };
    Ok(value)
}

fn evaluate_radical_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Log,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    let Some(value) = evaluate_radical_reduction(dag, *argument)? else {
        return Ok(None);
    };
    match radical_log_domain_proof(&value) {
        LogDomainProof::Positive => Ok(Some(value)),
        LogDomainProof::NonPositive => Err(logarithm_of_non_positive_error()),
        LogDomainProof::Unknown => Ok(None),
    }
}

fn evaluate_radical_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    evaluate_radical_reduction(dag, *argument)
}

fn reduce_square_root(
    value: &RadicalReduction,
    negative_kind: DomainErrorKind,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    match value {
        RadicalReduction::Rational(value) => {
            if value.value().is_negative() {
                return Err(domain_error(negative_kind));
            }
            Ok(reduce_rational_square_root(
                value.value(),
                value.used_special_angle(),
            ))
        }
        RadicalReduction::Radical(value) => {
            if value.value().coefficient.is_negative() {
                return Err(domain_error(negative_kind));
            }
            Ok(None)
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn reduce_radical_integer_power(
    base: &RadicalReduction,
    exponent: i64,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if exponent == 0 {
        return Ok(Some(RadicalReduction::rational(
            Rational::one(),
            used_special_angle,
        )));
    }
    let magnitude = exponent
        .checked_abs()
        .ok_or_else(exponent_too_large_error)?;
    let magnitude = u32::try_from(magnitude).map_err(|_| exponent_too_large_error())?;
    if exponent < 0 {
        let Some(denominator) =
            reduce_radical_positive_integer_power(base, magnitude, used_special_angle)?
        else {
            return Ok(None);
        };
        let numerator = RadicalReduction::rational(Rational::one(), used_special_angle);
        return reduce_radical_quotient(
            &numerator,
            &denominator,
            used_special_angle || denominator.used_special_angle(),
        );
    }
    reduce_radical_positive_integer_power(base, magnitude, used_special_angle)
}

fn reduce_radical_positive_integer_power(
    base: &RadicalReduction,
    mut exponent: u32,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    debug_assert!(exponent > 0);
    let mut result = None;
    let mut factor = radical_reduction_with_origin(base.clone(), used_special_angle);
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(current) => {
                    let Some(product) =
                        multiply_radical_reductions(&current, &factor, used_special_angle)?
                    else {
                        return Ok(None);
                    };
                    product
                }
                None => factor.clone(),
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            let Some(product) = multiply_radical_reductions(&factor, &factor, used_special_angle)?
            else {
                return Ok(None);
            };
            factor = product;
        }
    }
    Ok(result)
}

fn multiply_radical_reductions(
    lhs: &RadicalReduction,
    rhs: &RadicalReduction,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let used_special_angle =
        used_special_angle || lhs.used_special_angle() || rhs.used_special_angle();
    let lhs = radical_reduction_as_linear_combination(lhs);
    let rhs = radical_reduction_as_linear_combination(rhs);
    let mut rational = lhs.rational.multiply(&rhs.rational);
    let mut radicals = Vec::new();

    for radical in &lhs.radicals {
        add_scaled_radical_term(&mut radicals, radical, &rhs.rational);
    }
    for radical in &rhs.radicals {
        add_scaled_radical_term(&mut radicals, radical, &lhs.rational);
    }
    for lhs_radical in &lhs.radicals {
        for rhs_radical in &rhs.radicals {
            let coefficient = lhs_radical.coefficient.multiply(&rhs_radical.coefficient);
            let radicand = rational_from_positive_integer(&lhs_radical.radicand)
                .multiply(&rational_from_positive_integer(&rhs_radical.radicand));
            let Some(product) = reduce_radical_product(coefficient, radicand, used_special_angle)?
            else {
                return Ok(None);
            };
            add_radical_reduction_terms(&mut rational, &mut radicals, &product);
        }
    }

    Ok(Some(radical_linear_combination_reduction(
        rational,
        radicals,
        used_special_angle,
    )))
}

fn radical_reduction_as_linear_combination(value: &RadicalReduction) -> RadicalLinearCombination {
    match value {
        RadicalReduction::Rational(value) => RadicalLinearCombination {
            rational: value.value().clone(),
            radicals: Vec::new(),
        },
        RadicalReduction::Radical(value) => RadicalLinearCombination {
            rational: Rational::zero(),
            radicals: vec![value.value().clone()],
        },
        RadicalReduction::LinearCombination(value) => value.value().clone(),
    }
}

fn radical_reduction_with_origin(
    value: RadicalReduction,
    used_special_angle: bool,
) -> RadicalReduction {
    let used_special_angle = used_special_angle || value.used_special_angle();
    match value {
        RadicalReduction::Rational(value) => {
            RadicalReduction::rational(value.into_value(), used_special_angle)
        }
        RadicalReduction::Radical(value) => {
            RadicalReduction::radical(value.into_value(), used_special_angle)
        }
        RadicalReduction::LinearCombination(value) => {
            RadicalReduction::linear_combination(value.into_value(), used_special_angle)
        }
    }
}

fn add_scaled_radical_term(
    radicals: &mut Vec<SimpleRadical>,
    radical: &SimpleRadical,
    scalar: &Rational,
) {
    if scalar.is_zero() {
        return;
    }
    add_radical_term(
        radicals,
        &SimpleRadical {
            coefficient: radical.coefficient.multiply(scalar),
            radicand: radical.radicand.clone(),
        },
    );
}

fn add_radical_reduction_terms(
    rational: &mut Rational,
    radicals: &mut Vec<SimpleRadical>,
    value: &RadicalReduction,
) {
    match value {
        RadicalReduction::Rational(value) => {
            *rational = rational.add(value.value());
        }
        RadicalReduction::Radical(value) => add_radical_term(radicals, value.value()),
        RadicalReduction::LinearCombination(value) => {
            *rational = rational.add(&value.value().rational);
            for radical in &value.value().radicals {
                add_radical_term(radicals, radical);
            }
        }
    }
}

fn reduce_rational_square_root(
    value: &Rational,
    used_special_angle: bool,
) -> Option<RadicalReduction> {
    if let Some(root) = value.sqrt_if_rational() {
        return Some(RadicalReduction::rational(root, used_special_angle));
    }
    value
        .sqrt_as_simple_radical()
        .map(|value| RadicalReduction::radical(value, used_special_angle))
}

fn reduce_radical_quotient(
    numerator: &RadicalReduction,
    denominator: &RadicalReduction,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if let RadicalReduction::LinearCombination(numerator) = numerator {
        let Some(denominator) = radical_components(denominator) else {
            return Ok(None);
        };
        if denominator.radicand != Rational::one() {
            return Ok(None);
        }
        let scalar = Rational::one()
            .divide(&denominator.coefficient)
            .map_err(arithmetic_error)?;
        return Ok(Some(scale_radical_linear_combination(
            numerator.value(),
            &scalar,
            used_special_angle,
        )));
    }
    let Some(numerator) = radical_components(numerator) else {
        return Ok(None);
    };
    let Some(denominator) = radical_components(denominator) else {
        return Ok(None);
    };
    let coefficient = numerator
        .coefficient
        .divide(&denominator.coefficient)
        .map_err(arithmetic_error)?;
    let radicand = numerator
        .radicand
        .divide(&denominator.radicand)
        .map_err(arithmetic_error)?;
    reduce_radical_product(coefficient, radicand, used_special_angle)
}

fn reduce_radical_product(
    coefficient: Rational,
    radicand: Rational,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if coefficient.is_zero() {
        return Ok(Some(RadicalReduction::rational(
            Rational::zero(),
            used_special_angle,
        )));
    }
    if radicand.is_negative() || radicand.is_zero() {
        return Err(domain_error(DomainErrorKind::EvenRootOfNegative));
    }
    if let Some(root) = radicand.sqrt_if_rational() {
        return Ok(Some(RadicalReduction::rational(
            coefficient.multiply(&root),
            used_special_angle,
        )));
    }
    let Some(radical) = radicand.sqrt_as_simple_radical() else {
        return Ok(None);
    };
    Ok(Some(RadicalReduction::radical(
        SimpleRadical {
            coefficient: coefficient.multiply(&radical.coefficient),
            radicand: radical.radicand,
        },
        used_special_angle,
    )))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RadicalComponents {
    coefficient: Rational,
    radicand: Rational,
}

fn radical_components(value: &RadicalReduction) -> Option<RadicalComponents> {
    match value {
        RadicalReduction::Rational(value) => Some(RadicalComponents {
            coefficient: value.value().clone(),
            radicand: Rational::one(),
        }),
        RadicalReduction::Radical(value) => Some(RadicalComponents {
            coefficient: value.value().coefficient.clone(),
            radicand: rational_from_positive_integer(&value.value().radicand),
        }),
        RadicalReduction::LinearCombination(_) => None,
    }
}

fn rational_from_positive_integer(value: &PositiveInteger) -> Rational {
    Rational::from_integer(value.inner.clone())
}

fn add_radical_term(radicals: &mut Vec<SimpleRadical>, radical: &SimpleRadical) {
    if radical.coefficient.is_zero() {
        return;
    }
    if let Some(existing) = radicals
        .iter_mut()
        .find(|existing| existing.radicand == radical.radicand)
    {
        existing.coefficient = existing.coefficient.add(&radical.coefficient);
        return;
    }
    radicals.push(radical.clone());
}

fn radical_linear_combination_reduction(
    rational: Rational,
    mut radicals: Vec<SimpleRadical>,
    used_special_angle: bool,
) -> RadicalReduction {
    radicals.retain(|radical| !radical.coefficient.is_zero());
    radicals.sort_by(|left, right| {
        left.coefficient
            .is_negative()
            .cmp(&right.coefficient.is_negative())
            .then_with(|| left.radicand.inner.inner.cmp(&right.radicand.inner.inner))
    });

    match (rational.is_zero(), radicals.len()) {
        (_, 0) => RadicalReduction::rational(rational, used_special_angle),
        (true, 1) => RadicalReduction::radical(
            radicals
                .pop()
                .expect("radical length was checked before popping"),
            used_special_angle,
        ),
        _ => RadicalReduction::linear_combination(
            RadicalLinearCombination { rational, radicals },
            used_special_angle,
        ),
    }
}

fn scale_radical_linear_combination(
    value: &RadicalLinearCombination,
    scalar: &Rational,
    used_special_angle: bool,
) -> RadicalReduction {
    let rational = value.rational.multiply(scalar);
    let radicals = value
        .radicals
        .iter()
        .map(|radical| SimpleRadical {
            coefficient: radical.coefficient.multiply(scalar),
            radicand: radical.radicand.clone(),
        })
        .collect();
    radical_linear_combination_reduction(rational, radicals, used_special_angle)
}

const MAX_RADICAL_SIGN_PROOF_DEPTH: usize = 8;
const MAX_RADICAL_SIGN_PROOF_TERMS: usize = 8;

fn radical_reduction_sign(value: &RadicalReduction) -> Option<Ordering> {
    radical_reduction_sign_with_depth(value, 0)
}

fn radical_reduction_sign_with_depth(value: &RadicalReduction, depth: usize) -> Option<Ordering> {
    match value {
        RadicalReduction::Rational(value) => Some(rational_sign(value.value())),
        RadicalReduction::Radical(value) => Some(rational_sign(&value.value().coefficient)),
        RadicalReduction::LinearCombination(value) => {
            radical_linear_combination_sign(value.value(), depth)
        }
    }
}

fn radical_linear_combination_sign(
    value: &RadicalLinearCombination,
    depth: usize,
) -> Option<Ordering> {
    if let Some(sign) = radical_linear_combination_trivial_sign(value) {
        return Some(sign);
    }
    if depth >= MAX_RADICAL_SIGN_PROOF_DEPTH
        || radical_linear_combination_term_count(value) > MAX_RADICAL_SIGN_PROOF_TERMS
    {
        return None;
    }

    let (positive, negative) = split_radical_linear_combination_by_sign(value);
    compare_nonnegative_radical_reductions(&positive, &negative, depth + 1)
}

fn radical_linear_combination_trivial_sign(value: &RadicalLinearCombination) -> Option<Ordering> {
    let mut saw_positive = false;
    let mut saw_negative = false;
    update_component_sign(
        rational_sign(&value.rational),
        &mut saw_positive,
        &mut saw_negative,
    );
    for radical in &value.radicals {
        update_component_sign(
            rational_sign(&radical.coefficient),
            &mut saw_positive,
            &mut saw_negative,
        );
    }

    match (saw_positive, saw_negative) {
        (false, false) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Greater),
        (false, true) => Some(Ordering::Less),
        (true, true) => None,
    }
}

fn update_component_sign(sign: Ordering, saw_positive: &mut bool, saw_negative: &mut bool) {
    match sign {
        Ordering::Less => *saw_negative = true,
        Ordering::Equal => {}
        Ordering::Greater => *saw_positive = true,
    }
}

fn radical_linear_combination_term_count(value: &RadicalLinearCombination) -> usize {
    usize::from(!value.rational.is_zero()) + value.radicals.len()
}

fn split_radical_linear_combination_by_sign(
    value: &RadicalLinearCombination,
) -> (RadicalReduction, RadicalReduction) {
    let mut positive_rational = Rational::zero();
    let mut negative_rational = Rational::zero();
    if value.rational.is_negative() {
        negative_rational = value.rational.negate();
    } else {
        positive_rational = value.rational.clone();
    }

    let mut positive_radicals = Vec::new();
    let mut negative_radicals = Vec::new();
    for radical in &value.radicals {
        if radical.coefficient.is_negative() {
            negative_radicals.push(SimpleRadical {
                coefficient: radical.coefficient.negate(),
                radicand: radical.radicand.clone(),
            });
        } else {
            positive_radicals.push(radical.clone());
        }
    }

    (
        radical_linear_combination_reduction(positive_rational, positive_radicals, false),
        radical_linear_combination_reduction(negative_rational, negative_radicals, false),
    )
}

fn compare_nonnegative_radical_reductions(
    left: &RadicalReduction,
    right: &RadicalReduction,
    depth: usize,
) -> Option<Ordering> {
    if depth >= MAX_RADICAL_SIGN_PROOF_DEPTH {
        return None;
    }
    let left_terms =
        radical_linear_combination_term_count(&radical_reduction_as_linear_combination(left));
    let right_terms =
        radical_linear_combination_term_count(&radical_reduction_as_linear_combination(right));
    if left_terms + right_terms > MAX_RADICAL_SIGN_PROOF_TERMS {
        return None;
    }

    let left_squared = multiply_radical_reductions(left, left, false).ok()??;
    let right_squared = multiply_radical_reductions(right, right, false).ok()??;
    let difference = subtract_radical_reductions(&left_squared, &right_squared);
    radical_reduction_sign_with_depth(&difference, depth + 1)
}

fn subtract_radical_reductions(lhs: &RadicalReduction, rhs: &RadicalReduction) -> RadicalReduction {
    let lhs = radical_reduction_as_linear_combination(lhs);
    let rhs = radical_reduction_as_linear_combination(rhs);
    let mut radicals = lhs.radicals;
    for radical in rhs.radicals {
        add_radical_term(
            &mut radicals,
            &SimpleRadical {
                coefficient: radical.coefficient.negate(),
                radicand: radical.radicand,
            },
        );
    }
    radical_linear_combination_reduction(lhs.rational.subtract(&rhs.rational), radicals, false)
}

fn rational_sign(value: &Rational) -> Ordering {
    if value.is_negative() {
        Ordering::Less
    } else if value.is_zero() {
        Ordering::Equal
    } else {
        Ordering::Greater
    }
}

fn evaluate_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let base = evaluate_node(dag, base)?;
    let exponent = match evaluate_node(dag, exponent) {
        Ok(exponent) => exponent,
        Err(error) if base.value().is_negative() && is_unsupported_exact_expression(&error) => {
            return Err(domain_error(DomainErrorKind::NonRealPower));
        }
        Err(error) => return Err(error),
    };
    let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
    evaluate_rational_power(base.value(), exponent.value())
        .map(|value| RationalEvaluation::with_origin(value, used_special_angle))
}

fn is_unsupported_exact_expression(error: &EvaluationError) -> bool {
    matches!(error, EvaluationError::UnsupportedFeature(_))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogDomainProof {
    Positive,
    NonPositive,
    Unknown,
}

fn rational_log_domain_proof(value: &Rational) -> LogDomainProof {
    if value.is_negative() || value.is_zero() {
        LogDomainProof::NonPositive
    } else {
        LogDomainProof::Positive
    }
}

fn radical_log_domain_proof(value: &RadicalReduction) -> LogDomainProof {
    match radical_reduction_sign(value) {
        Some(Ordering::Greater) => LogDomainProof::Positive,
        Some(Ordering::Equal | Ordering::Less) => LogDomainProof::NonPositive,
        None => LogDomainProof::Unknown,
    }
}

fn real_algebraic_log_domain_proof(
    value: &RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<LogDomainProof, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(rational_log_domain_proof(value.value())),
        RealAlgebraicEvaluation::Algebraic(value) => {
            match value.sign_bounded(limits.max_root_isolation_steps) {
                Ok(Some(Ordering::Greater)) => Ok(LogDomainProof::Positive),
                Ok(Some(Ordering::Equal | Ordering::Less)) => Ok(LogDomainProof::NonPositive),
                Ok(None)
                | Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(LogDomainProof::Unknown),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitIntervalDomainProof {
    InRange,
    OutOfRange,
    Unknown,
}

fn rational_unit_interval_domain_proof(value: &Rational) -> UnitIntervalDomainProof {
    if value.compare(&rational_integer(-1)) == Ordering::Less
        || value.compare(&Rational::one()) == Ordering::Greater
    {
        UnitIntervalDomainProof::OutOfRange
    } else {
        UnitIntervalDomainProof::InRange
    }
}

fn radical_unit_interval_domain_proof(value: &RadicalReduction) -> UnitIntervalDomainProof {
    let minus_one = RadicalReduction::rational(rational_integer(-1), false);
    let one = RadicalReduction::rational(Rational::one(), false);
    let lower_margin = subtract_radical_reductions(value, &minus_one);
    let upper_margin = subtract_radical_reductions(&one, value);
    let lower_sign = radical_reduction_sign(&lower_margin);
    let upper_sign = radical_reduction_sign(&upper_margin);

    if matches!(lower_sign, Some(Ordering::Less)) || matches!(upper_sign, Some(Ordering::Less)) {
        UnitIntervalDomainProof::OutOfRange
    } else if lower_sign.is_some() && upper_sign.is_some() {
        UnitIntervalDomainProof::InRange
    } else {
        UnitIntervalDomainProof::Unknown
    }
}

fn real_algebraic_unit_interval_domain_proof(
    value: &RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<UnitIntervalDomainProof, EvaluationError> {
    let lower_comparison =
        compare_real_algebraic_evaluation_to_rational(value, &rational_integer(-1), limits)?;
    let upper_comparison =
        compare_real_algebraic_evaluation_to_rational(value, &Rational::one(), limits)?;

    if matches!(lower_comparison, Some(Ordering::Less))
        || matches!(upper_comparison, Some(Ordering::Greater))
    {
        Ok(UnitIntervalDomainProof::OutOfRange)
    } else if lower_comparison.is_some() && upper_comparison.is_some() {
        Ok(UnitIntervalDomainProof::InRange)
    } else {
        Ok(UnitIntervalDomainProof::Unknown)
    }
}

fn compare_real_algebraic_evaluation_to_rational(
    value: &RealAlgebraicEvaluation,
    rhs: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<Ordering>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(value.value().compare(rhs))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            let difference = match value.add_rational_bounded(
                &rhs.negate(),
                limits.max_polynomial_coefficient_bits,
                limits.max_root_isolation_steps,
            ) {
                Ok(Some(value)) => value,
                Ok(None)
                | Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => return Ok(None),
                Err(error) => return Err(real_algebraic_construction_error(error)),
            };
            match difference.sign_bounded(limits.max_root_isolation_steps) {
                Ok(sign) => Ok(sign),
                Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(None),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

fn evaluate_rational_power(
    base: &Rational,
    exponent: &Rational,
) -> Result<Rational, EvaluationError> {
    if base.is_zero() {
        return evaluate_zero_power(exponent);
    }

    let exponent_numerator = exponent
        .numerator
        .inner
        .to_i64()
        .ok_or_else(exponent_too_large_error)?;
    if exponent.is_integer() {
        return base.pow_i64(exponent_numerator).map_err(arithmetic_error);
    }

    let root_index = exponent
        .denominator
        .inner
        .inner
        .to_u32()
        .ok_or_else(exponent_too_large_error)?;
    if base.is_negative() && root_index.is_multiple_of(2) {
        return Err(domain_error(DomainErrorKind::NonRealPower));
    }

    let root = base
        .nth_root_if_rational(root_index)
        .ok_or_else(non_integer_power_error)?;
    root.pow_i64(exponent_numerator).map_err(arithmetic_error)
}

fn evaluate_zero_power(exponent: &Rational) -> Result<Rational, EvaluationError> {
    if exponent.is_zero() {
        Err(domain_error(DomainErrorKind::IndeterminateZeroToZero))
    } else if exponent.is_negative() {
        Err(domain_error(DomainErrorKind::ZeroToNegativePower))
    } else {
        Ok(Rational::zero())
    }
}

fn evaluate_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_exp_log_identity(dag, argument)? {
        return Ok(value);
    }
    let argument = evaluate_node(dag, argument)?;
    if argument.value().is_zero() {
        Ok(RationalEvaluation::with_origin(
            Rational::one(),
            argument.used_special_angle(),
        ))
    } else {
        Err(unsupported_function_evaluation())
    }
}

fn evaluate_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_log_exp_identity(dag, argument)? {
        return Ok(value);
    }
    if matches!(
        dag.node(argument),
        ExpressionNode::Constant(Constant::Euler)
    ) {
        return Ok(RationalEvaluation::direct(Rational::one()));
    }
    let argument = evaluate_node(dag, argument)?;
    if argument.value().is_negative() || argument.value().is_zero() {
        return Err(logarithm_of_non_positive_error());
    }
    if argument.value() == &Rational::one() {
        Ok(RationalEvaluation::with_origin(
            Rational::zero(),
            argument.used_special_angle(),
        ))
    } else {
        Err(unsupported_function_evaluation())
    }
}

const MAX_EXACT_LOG_BASE_ABS_EXPONENT: i64 = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalPowerPattern {
    basis: RationalEvaluation,
    exponent: RationalEvaluation,
}

impl RationalPowerPattern {
    fn used_special_angle(&self) -> bool {
        self.basis.used_special_angle() || self.exponent.used_special_angle()
    }
}

fn evaluate_log_base_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_log_base_common_power_identity(dag, argument, base)? {
        return Ok(value);
    }

    let base = evaluate_node(dag, base)?;
    ensure_log_base_domain(base.value())?;

    let argument = evaluate_node(dag, argument)?;
    if argument.value().is_negative() || argument.value().is_zero() {
        return Err(logarithm_of_non_positive_error());
    }

    let used_special_angle = argument.used_special_angle() || base.used_special_angle();
    if argument.value() == &Rational::one() {
        return Ok(RationalEvaluation::with_origin(
            Rational::zero(),
            used_special_angle,
        ));
    }
    if argument.value() == base.value() {
        return Ok(RationalEvaluation::with_origin(
            Rational::one(),
            used_special_angle,
        ));
    }

    if let Some(exponent) = integer_log_exponent(argument.value(), base.value())? {
        return Ok(RationalEvaluation::with_origin(
            Rational::from_integer(Integer::from(exponent)),
            used_special_angle,
        ));
    }

    Err(unsupported_function_evaluation())
}

fn evaluate_log_base_common_power_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let Some(base_pattern) = evaluate_positive_rational_power_pattern(dag, base)? else {
        return Ok(None);
    };
    ensure_log_base_pattern_domain(&base_pattern)?;

    match evaluate_node(dag, argument) {
        Ok(value) => {
            if value.value().is_negative() || value.value().is_zero() {
                return Err(logarithm_of_non_positive_error());
            }
            if value.value() == &Rational::one() {
                return Ok(Some(RationalEvaluation::with_origin(
                    Rational::zero(),
                    value.used_special_angle() || base_pattern.used_special_angle(),
                )));
            }
        }
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    let Some(argument_pattern) = evaluate_positive_rational_power_pattern(dag, argument)? else {
        return Ok(None);
    };
    finish_common_power_log_identity(argument_pattern, base_pattern)
}

fn finish_common_power_log_identity(
    argument: RationalPowerPattern,
    base: RationalPowerPattern,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    if argument.basis.value() == base.basis.value() {
        return log_power_exponent_quotient(argument, 1, base, 1);
    }

    if let Some(argument_basis_exponent) =
        integer_log_exponent(argument.basis.value(), base.basis.value())?
    {
        return log_power_exponent_quotient(argument, argument_basis_exponent, base, 1);
    }

    if let Some(base_basis_exponent) =
        integer_log_exponent(base.basis.value(), argument.basis.value())?
    {
        return log_power_exponent_quotient(argument, 1, base, base_basis_exponent);
    }

    Ok(None)
}

fn log_power_exponent_quotient(
    argument: RationalPowerPattern,
    argument_basis_exponent: i64,
    base: RationalPowerPattern,
    base_basis_exponent: i64,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let numerator = argument
        .exponent
        .value()
        .multiply(&Rational::from_integer(Integer::from(
            argument_basis_exponent,
        )));
    let denominator = base
        .exponent
        .value()
        .multiply(&Rational::from_integer(Integer::from(base_basis_exponent)));
    let value = numerator.divide(&denominator).map_err(arithmetic_error)?;
    Ok(Some(RationalEvaluation::with_origin(
        value,
        argument.used_special_angle() || base.used_special_angle(),
    )))
}

fn evaluate_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Power {
            base: power_base,
            exponent,
        } => evaluate_explicit_positive_rational_power_pattern(dag, *power_base, *exponent),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => evaluate_square_root_positive_rational_power_pattern(dag, *argument),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Exp
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => {
            evaluate_direct_positive_rational_power_pattern(dag, id)
        }
    }
}

fn evaluate_explicit_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    power_base: ExprId,
    exponent: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let power_base = match evaluate_node(dag, power_base) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if power_base.value().is_negative() || power_base.value().is_zero() {
        return Ok(None);
    }

    let exponent = match evaluate_node(dag, exponent) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Some(RationalPowerPattern {
        basis: power_base,
        exponent,
    }))
}

fn evaluate_square_root_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    radicand: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let radicand = match evaluate_node(dag, radicand) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if radicand.value().is_negative() || radicand.value().is_zero() {
        return Ok(None);
    }
    Ok(Some(RationalPowerPattern {
        basis: radicand,
        exponent: RationalEvaluation::direct(rational(1, 2)),
    }))
}

fn evaluate_direct_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let value = match evaluate_node(dag, id) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if value.value().is_negative() || value.value().is_zero() {
        return Ok(None);
    }
    Ok(Some(RationalPowerPattern {
        basis: value,
        exponent: RationalEvaluation::direct(Rational::one()),
    }))
}

fn ensure_log_base_pattern_domain(base: &RationalPowerPattern) -> Result<(), EvaluationError> {
    if base.basis.value() == &Rational::one() || base.exponent.value().is_zero() {
        return Err(domain_error(DomainErrorKind::LogarithmBaseOne));
    }
    Ok(())
}

fn integer_log_exponent(
    argument: &Rational,
    base: &Rational,
) -> Result<Option<i64>, EvaluationError> {
    if argument.is_negative()
        || argument.is_zero()
        || base.is_negative()
        || base.is_zero()
        || base == &Rational::one()
    {
        return Ok(None);
    }
    if argument == &Rational::one() {
        return Ok(Some(0));
    }
    if argument == base {
        return Ok(Some(1));
    }
    for exponent in 2..=MAX_EXACT_LOG_BASE_ABS_EXPONENT {
        if base.pow_i64(exponent).map_err(arithmetic_error)? == *argument {
            return Ok(Some(exponent));
        }
        if base.pow_i64(-exponent).map_err(arithmetic_error)? == *argument {
            return Ok(Some(-exponent));
        }
    }
    Ok(None)
}

fn ensure_log_base_domain(base: &Rational) -> Result<(), EvaluationError> {
    if base.is_negative() || base.is_zero() {
        return Err(logarithm_of_non_positive_error());
    }
    if base == &Rational::one() {
        return Err(domain_error(DomainErrorKind::LogarithmBaseOne));
    }
    Ok(())
}

fn evaluate_exp_log_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Log,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    let value = evaluate_node(dag, *argument)?;
    if value.value().is_negative() || value.value().is_zero() {
        Err(logarithm_of_non_positive_error())
    } else {
        Ok(Some(value))
    }
}

fn evaluate_log_exp_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.node(argument)
    else {
        return Ok(None);
    };
    Ok(Some(evaluate_node(dag, *argument)?))
}

fn evaluate_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_rational_trigonometric_special_angle(dag, function, argument)? {
        return Ok(value);
    }
    if let Some(value) = evaluate_rational_inverse_trig_composition(dag, function, argument)? {
        return Ok(value);
    }
    Err(unsupported_function_evaluation())
}

fn evaluate_rational_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let coefficient = match evaluate_pi_coefficient(dag, argument)? {
        Some(coefficient) => coefficient,
        None => match rational_zero_as_pi_coefficient(dag, argument) {
            Ok(coefficient) => PiCoefficientEvaluation::direct(coefficient),
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
            Err(error) => return Err(error),
        },
    };
    let value = match function {
        Function::Sin => sine_special_angle(coefficient.coefficient()),
        Function::Cos => cosine_special_angle(coefficient.coefficient()),
        Function::Tan => tangent_special_angle(coefficient.coefficient())?,
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    }
    .map(RationalEvaluation::special_angle);
    Ok(value)
}

fn evaluate_rational_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    if composition.kind != InverseTrigCompositionKind::Identity {
        return Ok(None);
    }
    let value = match evaluate_node(dag, composition.inner_argument) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match rational_unit_interval_domain_proof(value.value()) {
                UnitIntervalDomainProof::InRange => Ok(Some(value)),
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => Ok(Some(value)),
    }
}

fn evaluate_radical_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    let Some(value) = evaluate_radical_reduction(dag, composition.inner_argument)? else {
        return Ok(None);
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match radical_unit_interval_domain_proof(&value) {
                UnitIntervalDomainProof::InRange => match composition.kind {
                    InverseTrigCompositionKind::Identity => Ok(Some(value)),
                    InverseTrigCompositionKind::UnitCofunction => {
                        radical_inverse_trig_cofunction_value(&value)
                    }
                },
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => match composition.kind {
            InverseTrigCompositionKind::Identity => Ok(Some(value)),
            InverseTrigCompositionKind::UnitCofunction => Ok(None),
        },
    }
}

fn radical_inverse_trig_cofunction_value(
    value: &RadicalReduction,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(square) = multiply_radical_reductions(value, value, value.used_special_angle())?
    else {
        return Ok(None);
    };
    let one = RadicalReduction::rational(Rational::one(), value.used_special_angle());
    let complement = subtract_radical_reductions(&one, &square);
    reduce_square_root(&complement, DomainErrorKind::EvenRootOfNegative)
}

fn real_algebraic_inverse_trig_cofunction_value(
    value: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(square) = multiply_real_algebraic_evaluations(value.clone(), value, limits)? else {
        return Ok(None);
    };
    let Some(negative_square) =
        multiply_real_algebraic_evaluation_by_rational(square, &rational_integer(-1), limits)?
    else {
        return Ok(None);
    };
    let Some(complement) =
        add_rational_to_real_algebraic_evaluation(negative_square, &Rational::one(), limits)?
    else {
        return Ok(None);
    };
    nth_root_real_algebraic_evaluation(complement, 2, DomainErrorKind::EvenRootOfNegative, limits)
}

fn add_rational_to_real_algebraic_evaluation(
    value: RealAlgebraicEvaluation,
    rhs: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(value.value().add(rhs), value.used_special_angle()),
        ))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            match value.add_rational_bounded(
                rhs,
                limits.max_polynomial_coefficient_bits,
                limits.max_root_isolation_steps,
            ) {
                Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
                Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(None),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

fn evaluate_real_algebraic_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    let Some(value) =
        evaluate_rational_or_real_algebraic_node(dag, composition.inner_argument, limits)?
    else {
        return Ok(None);
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match real_algebraic_unit_interval_domain_proof(&value, limits)? {
                UnitIntervalDomainProof::InRange => match composition.kind {
                    InverseTrigCompositionKind::Identity => Ok(Some(value)),
                    InverseTrigCompositionKind::UnitCofunction => {
                        real_algebraic_inverse_trig_cofunction_value(value, limits)
                    }
                },
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => match composition.kind {
            InverseTrigCompositionKind::Identity => Ok(Some(value)),
            InverseTrigCompositionKind::UnitCofunction => Ok(None),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InverseTrigComposition {
    domain: InverseTrigCompositionDomain,
    kind: InverseTrigCompositionKind,
    inner_argument: ExprId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InverseTrigCompositionKind {
    Identity,
    UnitCofunction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InverseTrigCompositionDomain {
    UnitInterval,
    RealLine,
}

fn inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
) -> Result<Option<InverseTrigComposition>, EvaluationError> {
    match dag.semantics().angle_unit {
        AngleUnit::Radian => Ok(direct_inverse_trig_composition(
            dag,
            outer_function,
            argument,
        )),
        AngleUnit::Degree => scaled_inverse_trig_composition(dag, outer_function, argument, 180),
        AngleUnit::Gradian => scaled_inverse_trig_composition(dag, outer_function, argument, 200),
    }
}

fn direct_inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
) -> Option<InverseTrigComposition> {
    let ExpressionNode::Function {
        function: inner_function,
        argument: inner_argument,
    } = dag.node(argument)
    else {
        return None;
    };
    inverse_trig_composition_rule(outer_function, *inner_function).map(|(domain, kind)| {
        InverseTrigComposition {
            domain,
            kind,
            inner_argument: *inner_argument,
        }
    })
}

fn scaled_inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
    unit_denominator: i64,
) -> Result<Option<InverseTrigComposition>, EvaluationError> {
    let ExpressionNode::Multiply(list_id) = dag.node(argument) else {
        return Ok(None);
    };
    let mut composition = None;
    let mut scale = Rational::one();
    for child in dag.list(*list_id) {
        if let ExpressionNode::Function {
            function: inner_function,
            argument: inner_argument,
        } = dag.node(*child)
        {
            if let Some((domain, kind)) =
                inverse_trig_composition_rule(outer_function, *inner_function)
            {
                if composition
                    .replace(InverseTrigComposition {
                        domain,
                        kind,
                        inner_argument: *inner_argument,
                    })
                    .is_some()
                {
                    return Ok(None);
                }
                continue;
            }
        }

        let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
            return Ok(None);
        };
        scale = scale.multiply(coefficient.coefficient());
    }

    if scale == rational(1, unit_denominator) {
        Ok(composition)
    } else {
        Ok(None)
    }
}

fn inverse_trig_composition_rule(
    outer_function: Function,
    inner_function: Function,
) -> Option<(InverseTrigCompositionDomain, InverseTrigCompositionKind)> {
    match (outer_function, inner_function) {
        (Function::Sin, Function::Asin) | (Function::Cos, Function::Acos) => Some((
            InverseTrigCompositionDomain::UnitInterval,
            InverseTrigCompositionKind::Identity,
        )),
        (Function::Cos, Function::Asin) | (Function::Sin, Function::Acos) => Some((
            InverseTrigCompositionDomain::UnitInterval,
            InverseTrigCompositionKind::UnitCofunction,
        )),
        (Function::Tan, Function::Atan) => Some((
            InverseTrigCompositionDomain::RealLine,
            InverseTrigCompositionKind::Identity,
        )),
        _ => None,
    }
}

fn rational_zero_as_pi_coefficient(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Rational, EvaluationError> {
    match evaluate_node(dag, argument) {
        Ok(value) if value.value().is_zero() => Ok(Rational::zero()),
        Ok(_) => Err(unsupported_function_evaluation()),
        Err(error) if is_unsupported_exact_expression(&error) => {
            Err(unsupported_function_evaluation())
        }
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy)]
struct RationalParts {
    numerator: i64,
    denominator: i64,
}

#[derive(Clone, Copy)]
struct RationalSpecialAngleEntry {
    phase: RationalParts,
    value: RationalParts,
}

#[derive(Clone, Copy)]
struct RadicalTermParts {
    coefficient: RationalParts,
    radicand: i64,
}

#[derive(Clone, Copy)]
struct RadicalSpecialAngleEntry {
    phase: RationalParts,
    rational: RationalParts,
    radicals: &'static [RadicalTermParts],
}

const SQRT_2_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(1, 2, 2)];
const NEG_SQRT_2_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(-1, 2, 2)];
const SQRT_3_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(1, 2, 3)];
const NEG_SQRT_3_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(-1, 2, 3)];
const SQRT_3: [RadicalTermParts; 1] = [radical_term_parts(1, 1, 3)];
const NEG_SQRT_3: [RadicalTermParts; 1] = [radical_term_parts(-1, 1, 3)];
const SQRT_3_OVER_3: [RadicalTermParts; 1] = [radical_term_parts(1, 3, 3)];
const NEG_SQRT_3_OVER_3: [RadicalTermParts; 1] = [radical_term_parts(-1, 3, 3)];
const SQRT_6_MINUS_SQRT_2_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(-1, 4, 2), radical_term_parts(1, 4, 6)];
const SQRT_2_PLUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(1, 4, 2), radical_term_parts(1, 4, 6)];
const SQRT_2_MINUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(1, 4, 2), radical_term_parts(-1, 4, 6)];
const NEG_SQRT_2_MINUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(-1, 4, 2), radical_term_parts(-1, 4, 6)];

const SINE_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 0, 1),
    rational_angle_entry(1, 1, 0, 1),
    rational_angle_entry(1, 2, 1, 1),
    rational_angle_entry(3, 2, -1, 1),
    rational_angle_entry(1, 6, 1, 2),
    rational_angle_entry(5, 6, 1, 2),
    rational_angle_entry(7, 6, -1, 2),
    rational_angle_entry(11, 6, -1, 2),
];

const COSINE_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 1, 1),
    rational_angle_entry(1, 1, -1, 1),
    rational_angle_entry(1, 2, 0, 1),
    rational_angle_entry(3, 2, 0, 1),
    rational_angle_entry(1, 3, 1, 2),
    rational_angle_entry(5, 3, 1, 2),
    rational_angle_entry(2, 3, -1, 2),
    rational_angle_entry(4, 3, -1, 2),
];

const TANGENT_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 0, 1),
    rational_angle_entry(1, 4, 1, 1),
    rational_angle_entry(3, 4, -1, 1),
];

const TANGENT_POLE_PHASES: &[RationalParts] = &[rational_parts(1, 2)];

const SINE_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(3, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(5, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(7, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(1, 3, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(2, 3, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(4, 3, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(5, 3, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(1, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(11, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(5, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(7, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(13, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(23, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(17, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(19, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
];

const COSINE_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(7, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(3, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(5, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(1, 6, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(11, 6, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(5, 6, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(7, 6, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(1, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(23, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(5, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(19, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(7, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(17, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(11, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(13, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
];

const TANGENT_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 3, 0, 1, &SQRT_3),
    radical_angle_entry(2, 3, 0, 1, &NEG_SQRT_3),
    radical_angle_entry(1, 6, 0, 1, &SQRT_3_OVER_3),
    radical_angle_entry(5, 6, 0, 1, &NEG_SQRT_3_OVER_3),
    radical_angle_entry(1, 12, 2, 1, &NEG_SQRT_3),
    radical_angle_entry(5, 12, 2, 1, &SQRT_3),
    radical_angle_entry(7, 12, -2, 1, &NEG_SQRT_3),
    radical_angle_entry(11, 12, -2, 1, &SQRT_3),
];

const fn rational_parts(numerator: i64, denominator: i64) -> RationalParts {
    RationalParts {
        numerator,
        denominator,
    }
}

const fn rational_angle_entry(
    phase_numerator: i64,
    phase_denominator: i64,
    value_numerator: i64,
    value_denominator: i64,
) -> RationalSpecialAngleEntry {
    RationalSpecialAngleEntry {
        phase: rational_parts(phase_numerator, phase_denominator),
        value: rational_parts(value_numerator, value_denominator),
    }
}

const fn radical_term_parts(
    coefficient_numerator: i64,
    coefficient_denominator: i64,
    radicand: i64,
) -> RadicalTermParts {
    RadicalTermParts {
        coefficient: rational_parts(coefficient_numerator, coefficient_denominator),
        radicand,
    }
}

const fn radical_angle_entry(
    phase_numerator: i64,
    phase_denominator: i64,
    rational_numerator: i64,
    rational_denominator: i64,
    radicals: &'static [RadicalTermParts],
) -> RadicalSpecialAngleEntry {
    RadicalSpecialAngleEntry {
        phase: rational_parts(phase_numerator, phase_denominator),
        rational: rational_parts(rational_numerator, rational_denominator),
        radicals,
    }
}

fn sine_special_angle(coefficient: &Rational) -> Option<Rational> {
    lookup_rational_special_angle(coefficient, 2, SINE_RATIONAL_SPECIAL_ANGLES)
}

fn cosine_special_angle(coefficient: &Rational) -> Option<Rational> {
    lookup_rational_special_angle(coefficient, 2, COSINE_RATIONAL_SPECIAL_ANGLES)
}

fn tangent_special_angle(coefficient: &Rational) -> Result<Option<Rational>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else {
        Ok(lookup_rational_phase(
            &phase,
            TANGENT_RATIONAL_SPECIAL_ANGLES,
        ))
    }
}

fn sine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    lookup_radical_special_angle(coefficient, 2, SINE_RADICAL_SPECIAL_ANGLES)
}

fn cosine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    lookup_radical_special_angle(coefficient, 2, COSINE_RADICAL_SPECIAL_ANGLES)
}

fn tangent_radical_special_angle(
    coefficient: &Rational,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else {
        Ok(lookup_radical_phase(&phase, TANGENT_RADICAL_SPECIAL_ANGLES))
    }
}

fn lookup_rational_special_angle(
    coefficient: &Rational,
    period: u32,
    entries: &[RationalSpecialAngleEntry],
) -> Option<Rational> {
    let phase = coefficient.modulo_integer(period);
    lookup_rational_phase(&phase, entries)
}

fn lookup_rational_phase(
    phase: &Rational,
    entries: &[RationalSpecialAngleEntry],
) -> Option<Rational> {
    entries
        .iter()
        .find(|entry| phase_matches(phase, entry.phase))
        .map(|entry| rational_from_parts(entry.value))
}

fn lookup_radical_special_angle(
    coefficient: &Rational,
    period: u32,
    entries: &[RadicalSpecialAngleEntry],
) -> Option<RadicalReduction> {
    let phase = coefficient.modulo_integer(period);
    lookup_radical_phase(&phase, entries)
}

fn lookup_radical_phase(
    phase: &Rational,
    entries: &[RadicalSpecialAngleEntry],
) -> Option<RadicalReduction> {
    entries
        .iter()
        .find(|entry| phase_matches(phase, entry.phase))
        .map(radical_reduction_from_parts)
}

fn phase_matches_any(phase: &Rational, entries: &[RationalParts]) -> bool {
    entries.iter().any(|entry| phase_matches(phase, *entry))
}

fn phase_matches(phase: &Rational, expected: RationalParts) -> bool {
    phase == &rational_from_parts(expected)
}

fn rational_from_parts(parts: RationalParts) -> Rational {
    rational(parts.numerator, parts.denominator)
}

fn radical_reduction_from_parts(entry: &RadicalSpecialAngleEntry) -> RadicalReduction {
    let radicals = entry
        .radicals
        .iter()
        .map(|radical| simple_radical(rational_from_parts(radical.coefficient), radical.radicand))
        .collect();
    special_angle_linear_combination(rational_from_parts(entry.rational), radicals)
}

fn special_angle_linear_combination(
    rational: Rational,
    radicals: Vec<SimpleRadical>,
) -> RadicalReduction {
    radical_linear_combination_reduction(rational, radicals, true)
}

fn evaluate_inverse_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let Some(coefficient) = evaluate_inverse_trig_pi_coefficient(dag, function, argument)? else {
        return Err(unsupported_function_evaluation());
    };
    match dag.semantics().angle_unit {
        AngleUnit::Radian if coefficient.coefficient().is_zero() => {
            Ok(RationalEvaluation::special_angle(Rational::zero()))
        }
        AngleUnit::Radian => Err(unsupported_function_evaluation()),
        AngleUnit::Degree => Ok(RationalEvaluation::special_angle(
            coefficient.coefficient().multiply(&rational_integer(180)),
        )),
        AngleUnit::Gradian => Ok(RationalEvaluation::special_angle(
            coefficient.coefficient().multiply(&rational_integer(200)),
        )),
    }
}

fn evaluate_inverse_trig_pi_coefficient(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    let reduction = match evaluate_node(dag, argument) {
        Ok(value) => RadicalReduction::Rational(value),
        Err(error) if is_unsupported_exact_expression(&error) => {
            let Some(reduction) = evaluate_radical_node(dag, argument)? else {
                return Ok(None);
            };
            reduction
        }
        Err(error) => return Err(error),
    };
    let coefficient = match function {
        Function::Asin => asin_known_pi_coefficient(&reduction)?,
        Function::Acos => acos_known_pi_coefficient(&reduction)?,
        Function::Atan => atan_known_pi_coefficient(&reduction),
        Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    };
    Ok(coefficient.map(PiCoefficientEvaluation::special_angle))
}

fn asin_known_pi_coefficient(
    argument: &RadicalReduction,
) -> Result<Option<Rational>, EvaluationError> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            ensure_inverse_sine_cosine_rational_domain(argument)?;
            if *argument == rational_integer(-1) {
                Ok(Some(rational(-1, 2)))
            } else if *argument == rational(-1, 2) {
                Ok(Some(rational(-1, 6)))
            } else if argument.is_zero() {
                Ok(Some(Rational::zero()))
            } else if *argument == rational(1, 2) {
                Ok(Some(rational(1, 6)))
            } else if *argument == Rational::one() {
                Ok(Some(rational(1, 2)))
            } else {
                Ok(None)
            }
        }
        RadicalReduction::Radical(argument) => {
            ensure_inverse_sine_cosine_radical_domain(argument.value())?;
            Ok(asin_radical_pi_coefficient(argument.value()))
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn acos_known_pi_coefficient(
    argument: &RadicalReduction,
) -> Result<Option<Rational>, EvaluationError> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            ensure_inverse_sine_cosine_rational_domain(argument)?;
            if *argument == rational_integer(-1) {
                Ok(Some(Rational::one()))
            } else if *argument == rational(-1, 2) {
                Ok(Some(rational(2, 3)))
            } else if argument.is_zero() {
                Ok(Some(rational(1, 2)))
            } else if *argument == rational(1, 2) {
                Ok(Some(rational(1, 3)))
            } else if *argument == Rational::one() {
                Ok(Some(Rational::zero()))
            } else {
                Ok(None)
            }
        }
        RadicalReduction::Radical(argument) => {
            ensure_inverse_sine_cosine_radical_domain(argument.value())?;
            Ok(acos_radical_pi_coefficient(argument.value()))
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn atan_known_pi_coefficient(argument: &RadicalReduction) -> Option<Rational> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            if *argument == rational_integer(-1) {
                Some(rational(-1, 4))
            } else if argument.is_zero() {
                Some(Rational::zero())
            } else if *argument == Rational::one() {
                Some(rational(1, 4))
            } else {
                None
            }
        }
        RadicalReduction::Radical(argument) => atan_radical_pi_coefficient(argument.value()),
        RadicalReduction::LinearCombination(_) => None,
    }
}

fn asin_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if is_simple_radical(argument, rational(1, 2), 2) {
        Some(rational(1, 4))
    } else if is_simple_radical(argument, rational(-1, 2), 2) {
        Some(rational(-1, 4))
    } else if is_simple_radical(argument, rational(1, 2), 3) {
        Some(rational(1, 3))
    } else if is_simple_radical(argument, rational(-1, 2), 3) {
        Some(rational(-1, 3))
    } else {
        None
    }
}

fn acos_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if is_simple_radical(argument, rational(1, 2), 3) {
        Some(rational(1, 6))
    } else if is_simple_radical(argument, rational(-1, 2), 3) {
        Some(rational(5, 6))
    } else if is_simple_radical(argument, rational(1, 2), 2) {
        Some(rational(1, 4))
    } else if is_simple_radical(argument, rational(-1, 2), 2) {
        Some(rational(3, 4))
    } else {
        None
    }
}

fn atan_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if !has_radicand(argument, 3) {
        return None;
    }
    if argument.coefficient == Rational::one() {
        Some(rational(1, 3))
    } else if argument.coefficient == rational_integer(-1) {
        Some(rational(-1, 3))
    } else if argument.coefficient == rational(1, 3) {
        Some(rational(1, 6))
    } else if argument.coefficient == rational(-1, 3) {
        Some(rational(-1, 6))
    } else {
        None
    }
}

fn is_simple_radical(argument: &SimpleRadical, coefficient: Rational, radicand: i64) -> bool {
    argument.coefficient == coefficient && has_radicand(argument, radicand)
}

fn has_radicand(argument: &SimpleRadical, radicand: i64) -> bool {
    argument.radicand.inner == Integer::from(radicand)
}

fn ensure_inverse_sine_cosine_rational_domain(argument: &Rational) -> Result<(), EvaluationError> {
    match rational_unit_interval_domain_proof(argument) {
        UnitIntervalDomainProof::InRange => Ok(()),
        UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
        UnitIntervalDomainProof::Unknown => Ok(()),
    }
}

fn ensure_inverse_sine_cosine_radical_domain(
    argument: &SimpleRadical,
) -> Result<(), EvaluationError> {
    let squared = argument
        .coefficient
        .multiply(&argument.coefficient)
        .multiply(&rational_from_positive_integer(&argument.radicand));
    if squared.compare(&Rational::one()) == Ordering::Greater {
        Err(domain_error(
            DomainErrorKind::InverseTrigonometricOutOfRange,
        ))
    } else {
        Ok(())
    }
}

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("hard-coded rational constants have non-zero denominators")
}

fn rational_integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn simple_radical(coefficient: Rational, radicand: i64) -> SimpleRadical {
    SimpleRadical {
        coefficient,
        radicand: PositiveInteger::new(Integer::from(radicand))
            .expect("hard-coded radical radicands are positive"),
    }
}

fn unsupported_function_evaluation() -> EvaluationError {
    EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
        feature: UnsupportedFeature::FunctionEvaluation,
    })
}

fn non_integer_power_error() -> EvaluationError {
    EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
        feature: UnsupportedFeature::NonIntegerPower,
    })
}

fn exponent_too_large_error() -> EvaluationError {
    logical_work_limit_error()
}

fn logical_work_limit_error() -> EvaluationError {
    EvaluationError::ComputationLimit(ComputationLimitError {
        kind: ComputationLimitKind::LogicalWorkUnits,
    })
}

fn logarithm_of_non_positive_error() -> EvaluationError {
    domain_error(DomainErrorKind::LogarithmOfNonPositive)
}

fn inverse_trig_out_of_range_error() -> EvaluationError {
    domain_error(DomainErrorKind::InverseTrigonometricOutOfRange)
}

fn domain_error(kind: DomainErrorKind) -> EvaluationError {
    EvaluationError::Domain(DomainError { kind, span: None })
}

fn invalid_algebraic_isolation_error() -> EvaluationError {
    EvaluationError::InternalInvariant(InternalInvariantError {
        code: InternalInvariantCode::InvalidAlgebraicIsolation,
    })
}

fn real_algebraic_construction_error(error: RealAlgebraicConstructionError) -> EvaluationError {
    match error {
        RealAlgebraicConstructionError::RootCounting(
            PrimitivePolynomialRootCountingError::CountOverflow,
        )
        | RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::CountOverflow,
        ) => EvaluationError::ComputationLimit(ComputationLimitError {
            kind: ComputationLimitKind::RootIsolationSteps,
        }),
        RealAlgebraicConstructionError::ConstantPolynomial
        | RealAlgebraicConstructionError::InvalidInterval
        | RealAlgebraicConstructionError::EndpointRoot
        | RealAlgebraicConstructionError::NonIsolatingInterval
        | RealAlgebraicConstructionError::PolynomialConstruction(_)
        | RealAlgebraicConstructionError::PolynomialResultant(_)
        | RealAlgebraicConstructionError::NoMatchingPolynomialFactor
        | RealAlgebraicConstructionError::RootIndexOverflow
        | RealAlgebraicConstructionError::RootIndexNotFound
        | RealAlgebraicConstructionError::RootCounting(
            PrimitivePolynomialRootCountingError::ZeroPolynomial
            | PrimitivePolynomialRootCountingError::InvalidInterval,
        )
        | RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::ZeroPolynomial
            | PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        ) => invalid_algebraic_isolation_error(),
    }
}

fn evaluate_interval_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => {
            Ok(interval::from_rational(dag.rational(*id), precision_bits))
        }
        ExpressionNode::Constant(value) => interval::constant(*value, precision_bits),
        ExpressionNode::Add(list_id) => {
            let mut total = interval::from_rational(&Rational::zero(), precision_bits);
            for child in dag.list(*list_id) {
                total = interval::add(
                    &total,
                    &evaluate_interval_node(dag, *child, precision_bits)?,
                )?;
            }
            Ok(total)
        }
        ExpressionNode::Multiply(list_id) => {
            let mut product = interval::from_rational(&Rational::one(), precision_bits);
            for child in dag.list(*list_id) {
                product = interval::multiply(
                    &product,
                    &evaluate_interval_node(dag, *child, precision_bits)?,
                )?;
            }
            Ok(product)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => interval::divide(
            &evaluate_interval_node(dag, *numerator, precision_bits)?,
            &evaluate_interval_node(dag, *denominator, precision_bits)?,
            precision_bits,
        ),
        ExpressionNode::Power { base, exponent } => {
            evaluate_interval_power(dag, *base, *exponent, precision_bits)
        }
        ExpressionNode::LogBase { argument, base } => {
            evaluate_interval_log_base(dag, *argument, *base, precision_bits)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => interval::sqrt(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Exp => interval::exp(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Log | Function::Ln => interval::log(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Sin => sine_interval(dag, *argument, precision_bits),
            Function::Cos => cosine_interval(dag, *argument, precision_bits),
            Function::Tan => tangent_interval(dag, *argument, precision_bits),
            Function::Atan => interval::atan(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Asin => interval::asin(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Acos => interval::acos(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => evaluate_node(dag, id)
                .map(|value| interval::from_rational(value.value(), precision_bits))
                .map_err(evaluation_error_to_interval_error),
        },
        ExpressionNode::BinaryFunction { .. } => evaluate_node(dag, id)
            .map(|value| interval::from_rational(value.value(), precision_bits))
            .map_err(evaluation_error_to_interval_error),
    }
}

fn evaluate_interval_log_base(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if let Ok(base) = evaluate_node(dag, base) {
        ensure_log_base_domain(base.value()).map_err(|error| match error {
            EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
            EvaluationError::InputLimit(_)
            | EvaluationError::ComputationLimit(_)
            | EvaluationError::UnsupportedFeature(_)
            | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
        })?;
    }
    interval::divide(
        &interval::log(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        )?,
        &interval::log(
            &evaluate_interval_node(dag, base, precision_bits)?,
            precision_bits,
        )?,
        precision_bits,
    )
}

fn evaluate_interval_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let exponent = match evaluate_node(dag, exponent) {
        Ok(exponent) => exponent,
        Err(error) if is_unsupported_exact_expression(&error) => {
            return interval::pow_positive_base(
                &evaluate_interval_node(dag, base, precision_bits)?,
                &evaluate_interval_node(dag, exponent, precision_bits)?,
                precision_bits,
            );
        }
        Err(error) => return Err(evaluation_error_to_interval_error(error)),
    };

    if let Some(exponent_integer) = exponent.value().as_i64_if_integer() {
        return interval::pow_i64(
            &evaluate_interval_node(dag, base, precision_bits)?,
            exponent_integer,
            precision_bits,
        );
    }

    match evaluate_node(dag, base) {
        Ok(base) => interval::pow_rational(base.value(), exponent.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::pow_positive_base(
            &evaluate_interval_node(dag, base, precision_bits)?,
            &interval::from_rational(exponent.value(), precision_bits),
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn tangent_rational_pi_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let coefficient = evaluate_pi_coefficient(dag, argument).map_err(|error| match error {
        EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
        EvaluationError::InputLimit(_)
        | EvaluationError::ComputationLimit(_)
        | EvaluationError::UnsupportedFeature(_)
        | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
    })?;
    let Some(coefficient) = coefficient else {
        return Err(IntervalError::UnsupportedExpression);
    };

    let phase = coefficient.coefficient().modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        return Err(IntervalError::Domain(DomainErrorKind::TangentPole));
    }

    let mut distance_to_pole = phase.subtract(&rational(1, 2));
    if distance_to_pole.is_negative() {
        distance_to_pole = distance_to_pole.negate();
    }
    if distance_to_pole.is_zero() {
        return Err(IntervalError::Domain(DomainErrorKind::TangentPole));
    }
    let denominator = distance_to_pole.multiply(&rational_integer(2));
    let bound = Rational::one()
        .divide(&denominator)
        .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    interval::from_rational_bounds(&bound.negate(), &bound, precision_bits)
}

fn tangent_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match tangent_rational_pi_interval(dag, argument, precision_bits) {
        Ok(interval) => Ok(interval),
        Err(IntervalError::UnsupportedExpression) => match evaluate_node(dag, argument) {
            Ok(argument) => interval::tan_rational(argument.value(), precision_bits),
            Err(error) if is_unsupported_exact_expression(&error) => interval::tan(
                &evaluate_interval_node(dag, argument, precision_bits)?,
                precision_bits,
            ),
            Err(error) => Err(evaluation_error_to_interval_error(error)),
        },
        Err(error) => Err(error),
    }
}

fn sine_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match evaluate_node(dag, argument) {
        Ok(argument) => interval::sin_rational(argument.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::sin(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn cosine_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match evaluate_node(dag, argument) {
        Ok(argument) => interval::cos_rational(argument.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::cos(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn evaluation_error_to_interval_error(error: EvaluationError) -> IntervalError {
    match error {
        EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
        EvaluationError::ComputationLimit(_)
        | EvaluationError::InputLimit(_)
        | EvaluationError::UnsupportedFeature(_)
        | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
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
                let rational = Rational::from_decimal_literal(literal)
                    .map_err(decimal_literal_error_to_evaluation_error)?;
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
                function,
                argument,
                base,
                ..
            } => {
                let argument = self.lower(argument)?;
                let argument = self.lower_function_argument(*function, argument);
                if let Some(base) = base {
                    let base = self.lower(base)?;
                    return self.lower_binary_function(*function, argument, base);
                }
                let function = match function {
                    Function::Ln => Function::Log,
                    function @ (Function::Sin
                    | Function::Cos
                    | Function::Tan
                    | Function::Asin
                    | Function::Acos
                    | Function::Atan
                    | Function::Sqrt
                    | Function::Abs
                    | Function::Floor
                    | Function::Factorial
                    | Function::Exp
                    | Function::Log) => *function,
                    Function::Sinh
                    | Function::Cosh
                    | Function::Tanh
                    | Function::Asinh
                    | Function::Acosh
                    | Function::Atanh => {
                        return Ok(self.lower_hyperbolic_function(*function, argument));
                    }
                    Function::Root
                    | Function::Permutation
                    | Function::Combination
                    | Function::Modulo
                    | Function::Gcd
                    | Function::Lcm => *function,
                };
                Ok(self.push_node(ExpressionNode::Function { function, argument }))
            }
        }
    }

    fn lower_binary_function(
        &mut self,
        function: Function,
        argument: ExprId,
        base: ExprId,
    ) -> Result<ExprId, EvaluationError> {
        match function {
            Function::Exp if self.is_euler_constant(base) => {
                Ok(self.push_node(ExpressionNode::Function {
                    function: Function::Exp,
                    argument,
                }))
            }
            Function::Exp => Ok(self.push_node(ExpressionNode::Power {
                base,
                exponent: argument,
            })),
            Function::Log if self.is_euler_constant(base) => {
                Ok(self.push_node(ExpressionNode::Function {
                    function: Function::Log,
                    argument,
                }))
            }
            Function::Log => Ok(self.push_node(ExpressionNode::LogBase { argument, base })),
            Function::Root => {
                let one = self.push_rational(Rational::one());
                let exponent = self.push_node(ExpressionNode::Divide {
                    numerator: one,
                    denominator: base,
                });
                Ok(self.push_node(ExpressionNode::Power {
                    base: argument,
                    exponent,
                }))
            }
            Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm => Ok(self.push_node(ExpressionNode::BinaryFunction {
                function,
                left: argument,
                right: base,
            })),
            Function::Ln
            | Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => {
                Ok(self.push_node(ExpressionNode::Function { function, argument }))
            }
        }
    }

    fn lower_hyperbolic_function(&mut self, function: Function, argument: ExprId) -> ExprId {
        match function {
            Function::Sinh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.subtract(exp_positive, exp_negative);
                let two = self.integer(2);
                self.divide(numerator, two)
            }
            Function::Cosh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.add(exp_positive, exp_negative);
                let two = self.integer(2);
                self.divide(numerator, two)
            }
            Function::Tanh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.subtract(exp_positive, exp_negative);
                let denominator = self.add(exp_positive, exp_negative);
                self.divide(numerator, denominator)
            }
            Function::Asinh => {
                let two = self.integer(2);
                let square = self.power(argument, two);
                let one = self.integer(1);
                let under_root = self.add(square, one);
                let root = self.sqrt(under_root);
                let logarithm_argument = self.add(argument, root);
                self.log(logarithm_argument)
            }
            Function::Acosh => {
                let two = self.integer(2);
                let square = self.power(argument, two);
                let one = self.integer(1);
                let under_root = self.subtract(square, one);
                let root = self.sqrt(under_root);
                let logarithm_argument = self.add(argument, root);
                self.log(logarithm_argument)
            }
            Function::Atanh => {
                let numerator_one = self.integer(1);
                let numerator = self.add(numerator_one, argument);
                let denominator_one = self.integer(1);
                let denominator = self.subtract(denominator_one, argument);
                let quotient = self.divide(numerator, denominator);
                let logarithm = self.log(quotient);
                let two = self.integer(2);
                self.divide(logarithm, two)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm => self.push_node(ExpressionNode::Function { function, argument }),
        }
    }

    fn is_euler_constant(&self, id: ExprId) -> bool {
        matches!(
            self.nodes[id.0 as usize],
            ExpressionNode::Constant(Constant::Euler)
        )
    }

    fn lower_function_argument(&mut self, function: Function, argument: ExprId) -> ExprId {
        match function {
            Function::Sin | Function::Cos | Function::Tan => self.lower_angle_to_radians(argument),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => argument,
        }
    }

    fn lower_angle_to_radians(&mut self, argument: ExprId) -> ExprId {
        match self.semantics.angle_unit {
            AngleUnit::Radian => argument,
            AngleUnit::Degree => self.multiply_by_pi_over_integer(argument, 180),
            AngleUnit::Gradian => self.multiply_by_pi_over_integer(argument, 200),
        }
    }

    fn multiply_by_pi_over_integer(&mut self, argument: ExprId, denominator: i64) -> ExprId {
        let pi = self.push_node(ExpressionNode::Constant(Constant::Pi));
        let denominator = self.push_rational(Rational::from_integer(Integer::from(denominator)));
        let scale = self.push_node(ExpressionNode::Divide {
            numerator: pi,
            denominator,
        });
        let list = self.push_list(vec![argument, scale]);
        self.push_node(ExpressionNode::Multiply(list))
    }

    fn negate(&mut self, value: ExprId) -> ExprId {
        let minus_one = self.push_rational(Rational::from_integer(Integer::from(-1)));
        let list = self.push_list(vec![minus_one, value]);
        self.push_node(ExpressionNode::Multiply(list))
    }

    fn integer(&mut self, value: i64) -> ExprId {
        self.push_rational(Rational::from_integer(Integer::from(value)))
    }

    fn add(&mut self, left: ExprId, right: ExprId) -> ExprId {
        let list = self.push_list(vec![left, right]);
        self.push_node(ExpressionNode::Add(list))
    }

    fn subtract(&mut self, left: ExprId, right: ExprId) -> ExprId {
        let negative_right = self.negate(right);
        self.add(left, negative_right)
    }

    fn divide(&mut self, numerator: ExprId, denominator: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Divide {
            numerator,
            denominator,
        })
    }

    fn power(&mut self, base: ExprId, exponent: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Power { base, exponent })
    }

    fn exp(&mut self, argument: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Function {
            function: Function::Exp,
            argument,
        })
    }

    fn log(&mut self, argument: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Function {
            function: Function::Log,
            argument,
        })
    }

    fn sqrt(&mut self, argument: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        })
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

fn decimal_literal_error_to_evaluation_error(error: DecimalLiteralError) -> EvaluationError {
    match error {
        DecimalLiteralError::InvalidExponent | DecimalLiteralError::ExponentTooLarge => {
            EvaluationError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::IntegerTooLarge,
            })
        }
        DecimalLiteralError::Empty | DecimalLiteralError::InvalidDigit => {
            EvaluationError::InternalInvariant(InternalInvariantError {
                code: InternalInvariantCode::InvalidParsedNumberLiteral,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;
    use crate::{syntax::parse_source, types::ParseSettings};

    fn lower(source: &str) -> ExactExpressionDag {
        let parsed = parse_source(source, &ParseSettings::default()).expect(source);
        lower_source_expression(&parsed, SemanticSettings::default()).expect(source)
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

    #[test]
    fn rational_over_real_algebraic_is_recognized_as_real_algebraic() {
        let dag = lower("1/2^(1/3)");
        let ExpressionNode::Divide {
            numerator,
            denominator,
        } = dag.node(dag.root())
        else {
            panic!("expected divide");
        };
        let denominator =
            evaluate_real_algebraic_node(&dag, *denominator, &ResourceLimits::default())
                .unwrap()
                .expect("denominator should be algebraic")
                .into_algebraic()
                .expect("denominator should remain irrational algebraic");
        assert!(evaluate_node(&dag, *numerator).is_ok());
        assert!(
            reciprocal_real_algebraic(denominator, &ResourceLimits::default())
                .unwrap()
                .is_some()
        );
        assert!(
            evaluate_real_algebraic_dag(&dag, &ResourceLimits::default())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn real_algebraic_over_real_algebraic_is_recognized_as_real_algebraic() {
        let dag = lower("2^(1/3)/4^(1/3)");
        let quotient = evaluate_real_algebraic_dag(&dag, &ResourceLimits::default())
            .unwrap()
            .expect("algebraic quotient should be recognized")
            .into_algebraic()
            .expect("quotient should remain irrational algebraic");

        assert_eq!(
            quotient.minimal_polynomial(),
            &PrimitivePolynomial::new(vec![
                Integer::from(-1),
                Integer::zero(),
                Integer::zero(),
                Integer::from(2),
            ])
            .expect("minimal polynomial normalizes")
        );
    }
}
