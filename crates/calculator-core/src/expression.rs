use alloc::{vec, vec::Vec};
use core::cmp::Ordering;

use num_traits::ToPrimitive;

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
    fn special_angle(value: SimpleRadical) -> Self {
        Self {
            value,
            used_special_angle: true,
        }
    }

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
            let numerator = evaluate_node(dag, *numerator)?;
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
        ExpressionNode::Function { function, argument } => match function {
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
            Function::Log => evaluate_log_function(dag, *argument),
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin | Function::Acos | Function::Atan => {
                evaluate_inverse_trigonometric_function(dag, *function, *argument)
            }
        },
    }
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
            _ => Ok(None),
        },
        ExpressionNode::Power { .. } => Ok(None),
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
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_radical_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin | Function::Acos | Function::Atan | Function::Exp | Function::Log => {
                Ok(None)
            }
        },
        ExpressionNode::Power { base, exponent } => {
            let exponent = match evaluate_node(dag, *exponent) {
                Ok(value) => value,
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            if exponent.value() != &rational(1, 2) {
                return Ok(None);
            }
            let Some(base) = evaluate_radical_reduction(dag, *base)? else {
                return Ok(None);
            };
            reduce_square_root(&base, DomainErrorKind::NonRealPower)
        }
        ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => Ok(None),
    }
}

fn evaluate_radical_reduction(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(Some(RadicalReduction::Rational(value))),
        Err(error) if is_unsupported_exact_expression(&error) => evaluate_radical_node(dag, id),
        Err(error) => Err(error),
    }
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
        let Some(value) = evaluate_radical_reduction(dag, *child)? else {
            return Ok(None);
        };
        match value {
            RadicalReduction::Rational(value) => {
                used_special_angle |= value.used_special_angle();
                rational = rational.add(value.value());
            }
            RadicalReduction::Radical(radical) => {
                saw_radical = true;
                used_special_angle |= radical.used_special_angle();
                add_radical_term(&mut radicals, radical.value());
            }
            RadicalReduction::LinearCombination(value) => {
                saw_radical = true;
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
    let mut coefficient = Rational::one();
    let mut radicand = Rational::one();
    let mut linear_combination = None;
    let mut saw_radical = false;
    let mut used_special_angle = false;

    for child in dag.list(list_id) {
        let Some(value) = evaluate_radical_reduction(dag, *child)? else {
            return Ok(None);
        };
        match value {
            RadicalReduction::Rational(value) => {
                used_special_angle |= value.used_special_angle();
                coefficient = coefficient.multiply(value.value());
            }
            RadicalReduction::Radical(value) => {
                saw_radical = true;
                used_special_angle |= value.used_special_angle();
                coefficient = coefficient.multiply(&value.value().coefficient);
                radicand =
                    radicand.multiply(&rational_from_positive_integer(&value.value().radicand));
            }
            RadicalReduction::LinearCombination(value) => {
                if linear_combination.is_some() {
                    return Ok(None);
                }
                used_special_angle |= value.used_special_angle();
                linear_combination = Some(value);
            }
        }
    }

    if let Some(linear_combination) = linear_combination {
        if saw_radical {
            return Ok(None);
        }
        return Ok(Some(scale_radical_linear_combination(
            linear_combination.value(),
            &coefficient,
            used_special_angle,
        )));
    }
    if !saw_radical {
        return Ok(None);
    }
    reduce_radical_product(coefficient, radicand, used_special_angle)
}

fn evaluate_radical_trigonometric_function(
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
        _ => unreachable!("only trigonometric functions are dispatched here"),
    };
    Ok(value)
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
    let coefficient = match evaluate_pi_coefficient(dag, argument)? {
        Some(coefficient) => coefficient,
        None => PiCoefficientEvaluation::direct(rational_zero_as_pi_coefficient(dag, argument)?),
    };
    let value = match function {
        Function::Sin => sine_special_angle(coefficient.coefficient()),
        Function::Cos => cosine_special_angle(coefficient.coefficient()),
        Function::Tan => tangent_special_angle(coefficient.coefficient())?,
        _ => unreachable!("only trigonometric functions are dispatched here"),
    }
    .ok_or_else(unsupported_function_evaluation)?;
    Ok(RationalEvaluation::special_angle(value))
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

fn sine_special_angle(coefficient: &Rational) -> Option<Rational> {
    let phase = coefficient.modulo_integer(2);
    if phase == rational(0, 1) || phase == rational(1, 1) {
        Some(Rational::zero())
    } else if phase == rational(1, 2) {
        Some(Rational::one())
    } else if phase == rational(3, 2) {
        Some(rational_integer(-1))
    } else if phase == rational(1, 6) || phase == rational(5, 6) {
        Some(rational(1, 2))
    } else if phase == rational(7, 6) || phase == rational(11, 6) {
        Some(rational(-1, 2))
    } else {
        None
    }
}

fn cosine_special_angle(coefficient: &Rational) -> Option<Rational> {
    let phase = coefficient.modulo_integer(2);
    if phase == rational(0, 1) {
        Some(Rational::one())
    } else if phase == rational(1, 1) {
        Some(rational_integer(-1))
    } else if phase == rational(1, 2) || phase == rational(3, 2) {
        Some(Rational::zero())
    } else if phase == rational(1, 3) || phase == rational(5, 3) {
        Some(rational(1, 2))
    } else if phase == rational(2, 3) || phase == rational(4, 3) {
        Some(rational(-1, 2))
    } else {
        None
    }
}

fn tangent_special_angle(coefficient: &Rational) -> Result<Option<Rational>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase == rational(1, 2) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else if phase == rational(0, 1) {
        Ok(Some(Rational::zero()))
    } else if phase == rational(1, 4) {
        Ok(Some(Rational::one()))
    } else if phase == rational(3, 4) {
        Ok(Some(rational_integer(-1)))
    } else {
        Ok(None)
    }
}

fn sine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    let phase = coefficient.modulo_integer(2);
    if phase == rational(1, 4) || phase == rational(3, 4) {
        Some(special_angle_radical(simple_radical(rational(1, 2), 2)))
    } else if phase == rational(5, 4) || phase == rational(7, 4) {
        Some(special_angle_radical(simple_radical(rational(-1, 2), 2)))
    } else if phase == rational(1, 3) || phase == rational(2, 3) {
        Some(special_angle_radical(simple_radical(rational(1, 2), 3)))
    } else if phase == rational(4, 3) || phase == rational(5, 3) {
        Some(special_angle_radical(simple_radical(rational(-1, 2), 3)))
    } else if phase == rational(1, 12) || phase == rational(11, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(-1, 4),
            rational(1, 4),
        ))
    } else if phase == rational(5, 12) || phase == rational(7, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(1, 4),
            rational(1, 4),
        ))
    } else if phase == rational(13, 12) || phase == rational(23, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(1, 4),
            rational(-1, 4),
        ))
    } else if phase == rational(17, 12) || phase == rational(19, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(-1, 4),
            rational(-1, 4),
        ))
    } else {
        None
    }
}

fn cosine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    let phase = coefficient.modulo_integer(2);
    if phase == rational(1, 4) || phase == rational(7, 4) {
        Some(special_angle_radical(simple_radical(rational(1, 2), 2)))
    } else if phase == rational(3, 4) || phase == rational(5, 4) {
        Some(special_angle_radical(simple_radical(rational(-1, 2), 2)))
    } else if phase == rational(1, 6) || phase == rational(11, 6) {
        Some(special_angle_radical(simple_radical(rational(1, 2), 3)))
    } else if phase == rational(5, 6) || phase == rational(7, 6) {
        Some(special_angle_radical(simple_radical(rational(-1, 2), 3)))
    } else if phase == rational(1, 12) || phase == rational(23, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(1, 4),
            rational(1, 4),
        ))
    } else if phase == rational(5, 12) || phase == rational(19, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(-1, 4),
            rational(1, 4),
        ))
    } else if phase == rational(7, 12) || phase == rational(17, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(1, 4),
            rational(-1, 4),
        ))
    } else if phase == rational(11, 12) || phase == rational(13, 12) {
        Some(twelfth_angle_radical_combination(
            Rational::zero(),
            rational(-1, 4),
            rational(-1, 4),
        ))
    } else {
        None
    }
}

fn tangent_radical_special_angle(
    coefficient: &Rational,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase == rational(1, 2) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else if phase == rational(1, 3) {
        Ok(Some(special_angle_radical(simple_radical(
            Rational::one(),
            3,
        ))))
    } else if phase == rational(2, 3) {
        Ok(Some(special_angle_radical(simple_radical(
            rational_integer(-1),
            3,
        ))))
    } else if phase == rational(1, 6) {
        Ok(Some(special_angle_radical(simple_radical(
            rational(1, 3),
            3,
        ))))
    } else if phase == rational(5, 6) {
        Ok(Some(special_angle_radical(simple_radical(
            rational(-1, 3),
            3,
        ))))
    } else if phase == rational(1, 12) {
        Ok(Some(special_angle_linear_combination(
            rational_integer(2),
            vec![simple_radical(rational_integer(-1), 3)],
        )))
    } else if phase == rational(5, 12) {
        Ok(Some(special_angle_linear_combination(
            rational_integer(2),
            vec![simple_radical(Rational::one(), 3)],
        )))
    } else if phase == rational(7, 12) {
        Ok(Some(special_angle_linear_combination(
            rational_integer(-2),
            vec![simple_radical(rational_integer(-1), 3)],
        )))
    } else if phase == rational(11, 12) {
        Ok(Some(special_angle_linear_combination(
            rational_integer(-2),
            vec![simple_radical(Rational::one(), 3)],
        )))
    } else {
        Ok(None)
    }
}

fn special_angle_radical(value: SimpleRadical) -> RadicalReduction {
    RadicalReduction::Radical(RadicalEvaluation::special_angle(value))
}

fn twelfth_angle_radical_combination(
    rational: Rational,
    sqrt_2_coefficient: Rational,
    sqrt_6_coefficient: Rational,
) -> RadicalReduction {
    let radicals = vec![
        simple_radical(sqrt_2_coefficient, 2),
        simple_radical(sqrt_6_coefficient, 6),
    ];
    special_angle_linear_combination(rational, radicals)
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
        _ => unreachable!("only inverse trigonometric functions are dispatched here"),
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
    if argument.compare(&rational_integer(-1)) == Ordering::Less
        || argument.compare(&Rational::one()) == Ordering::Greater
    {
        Err(domain_error(
            DomainErrorKind::InverseTrigonometricOutOfRange,
        ))
    } else {
        Ok(())
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
    EvaluationError::ComputationLimit(ComputationLimitError {
        kind: ComputationLimitKind::LogicalWorkUnits,
    })
}

fn logarithm_of_non_positive_error() -> EvaluationError {
    domain_error(DomainErrorKind::LogarithmOfNonPositive)
}

fn domain_error(kind: DomainErrorKind) -> EvaluationError {
    EvaluationError::Domain(DomainError { kind, span: None })
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
            let exponent =
                evaluate_node(dag, *exponent).map_err(|_| IntervalError::UnsupportedExpression)?;
            if let Some(exponent) = exponent.value().as_i64_if_integer() {
                interval::pow_i64(
                    &evaluate_interval_node(dag, *base, precision_bits)?,
                    exponent,
                    precision_bits,
                )
            } else {
                let base =
                    evaluate_node(dag, *base).map_err(|_| IntervalError::UnsupportedExpression)?;
                interval::pow_rational(base.value(), exponent.value(), precision_bits)
            }
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => interval::sqrt(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Exp => {
                let argument = evaluate_node(dag, *argument)
                    .map_err(|_| IntervalError::UnsupportedExpression)?;
                if argument.value().is_zero() {
                    Ok(interval::from_rational(&Rational::one(), precision_bits))
                } else if argument.value() == &Rational::one() {
                    interval::constant(Constant::Euler, precision_bits)
                } else {
                    Err(IntervalError::UnsupportedExpression)
                }
            }
            Function::Log => {
                let argument = evaluate_node(dag, *argument).map_err(|error| match error {
                    EvaluationError::Domain(DomainError {
                        kind: DomainErrorKind::LogarithmOfNonPositive,
                        ..
                    }) => IntervalError::Domain(DomainErrorKind::LogarithmOfNonPositive),
                    _ => IntervalError::UnsupportedExpression,
                })?;
                if argument.value().is_negative() || argument.value().is_zero() {
                    return Err(IntervalError::Domain(
                        DomainErrorKind::LogarithmOfNonPositive,
                    ));
                }
                if argument.value() == &Rational::one() {
                    Ok(interval::from_rational(&Rational::zero(), precision_bits))
                } else {
                    Err(IntervalError::UnsupportedExpression)
                }
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan => Err(IntervalError::UnsupportedExpression),
        },
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
                let argument = self.lower_function_argument(*function, argument);
                Ok(self.push_node(ExpressionNode::Function {
                    function: *function,
                    argument,
                }))
            }
        }
    }

    fn lower_function_argument(&mut self, function: Function, argument: ExprId) -> ExprId {
        match function {
            Function::Sin | Function::Cos | Function::Tan => self.lower_angle_to_radians(argument),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Exp
            | Function::Log => argument,
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
}
