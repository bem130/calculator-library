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

    pub(crate) fn nodes(&self) -> &[ExpressionNode] {
        &self.nodes
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

pub(crate) fn evaluate_real_algebraic_dag(
    dag: &ExactExpressionDag,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    evaluate_real_algebraic_node(dag, dag.root(), limits)
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
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Exp
            | Function::Log => Ok(None),
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
        ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => Ok(None),
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
) -> Result<Option<RealAlgebraic>, EvaluationError> {
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
        ExpressionNode::Function {
            function: function @ (Function::Sin | Function::Cos | Function::Tan),
            argument,
        } => evaluate_real_algebraic_trigonometric_function(dag, *function, *argument, limits),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Function { .. } => Ok(None),
    }
}

fn evaluate_real_algebraic_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let exponent = match evaluate_node(dag, exponent) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match evaluate_node(dag, base) {
        Ok(base) => rational_prime_root_algebraic(base.value(), exponent.value(), limits),
        Err(error) if is_unsupported_exact_expression(&error) => {
            if !exponent.value().is_integer() {
                return Ok(None);
            }
            let Some(exponent) = exponent.value().as_i64_if_integer() else {
                return Err(exponent_too_large_error());
            };
            let Some(base) = evaluate_real_algebraic_node(dag, base, limits)? else {
                return Ok(None);
            };
            real_algebraic_integer_power(base, exponent, limits)
        }
        Err(error) => Err(error),
    }
}

fn evaluate_real_algebraic_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let mut rational = Rational::zero();
    let mut algebraic = None;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.add(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                algebraic = match algebraic {
                    Some(current) => {
                        let Some(sum) = add_real_algebraics(current, value, limits)? else {
                            return Ok(None);
                        };
                        Some(sum)
                    }
                    None => Some(value),
                };
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(None);
    };
    if rational.is_zero() {
        return Ok(Some(algebraic));
    }
    match algebraic.add_rational_bounded(
        &rational,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value),
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
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    match lhs.add_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
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

fn evaluate_real_algebraic_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let mut rational = Rational::one();
    let mut algebraic = None;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.multiply(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                algebraic = match algebraic {
                    Some(current) => {
                        let Some(product) = multiply_real_algebraics(current, value, limits)?
                        else {
                            return Ok(None);
                        };
                        Some(product)
                    }
                    None => Some(value),
                };
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(None);
    };
    scale_real_algebraic_by_rational(algebraic, &rational, limits)
}

fn multiply_real_algebraics(
    lhs: RealAlgebraic,
    rhs: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    match lhs.multiply_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
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

fn evaluate_real_algebraic_quotient(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let denominator_rational = match evaluate_node(dag, denominator) {
        Ok(value) => Some(value),
        Err(error) if is_unsupported_exact_expression(&error) => None,
        Err(error) => return Err(error),
    };
    if let Some(denominator_rational) = denominator_rational {
        let Some(numerator) = evaluate_real_algebraic_node(dag, numerator, limits)? else {
            return Ok(None);
        };
        let scalar = Rational::one()
            .divide(denominator_rational.value())
            .map_err(arithmetic_error)?;
        return scale_real_algebraic_by_rational(numerator, &scalar, limits);
    }

    let Some(denominator) = evaluate_real_algebraic_node(dag, denominator, limits)? else {
        return Ok(None);
    };
    let Some(reciprocal_denominator) = reciprocal_real_algebraic(denominator, limits)? else {
        return Ok(None);
    };
    match evaluate_node(dag, numerator) {
        Ok(numerator) => {
            scale_real_algebraic_by_rational(reciprocal_denominator, numerator.value(), limits)
        }
        Err(error) if is_unsupported_exact_expression(&error) => {
            let Some(numerator) = evaluate_real_algebraic_node(dag, numerator, limits)? else {
                return Ok(None);
            };
            multiply_real_algebraics(numerator, reciprocal_denominator, limits)
        }
        Err(error) => Err(error),
    }
}

fn scale_real_algebraic_by_rational(
    algebraic: RealAlgebraic,
    scalar: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    if scalar.is_zero() {
        return Ok(None);
    }
    if scalar == &Rational::one() {
        return Ok(Some(algebraic));
    }
    match algebraic.scale_rational_bounded(
        scalar,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn reciprocal_real_algebraic(
    algebraic: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    match algebraic.reciprocal_bounded(
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value),
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
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    if exponent == 0 {
        return rational_as_real_algebraic(Rational::one(), limits);
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
        base
    };
    real_algebraic_positive_integer_power(base, magnitude, limits)
}

fn real_algebraic_positive_integer_power(
    base: RealAlgebraic,
    mut exponent: u32,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    debug_assert!(exponent > 0);
    let mut result = None;
    let mut factor = base;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(current) => {
                    let Some(product) = multiply_real_algebraics(current, factor.clone(), limits)?
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
            let Some(product) = multiply_real_algebraics(factor.clone(), factor, limits)? else {
                return Ok(None);
            };
            factor = product;
        }
    }
    Ok(result)
}

fn rational_as_real_algebraic(
    value: Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    if limits.max_algebraic_degree < 1 {
        return Ok(None);
    }
    let polynomial = PrimitivePolynomial::new(vec![
        Integer::from_bigint(-value.numerator.inner.clone()),
        value.denominator.inner.clone(),
    ])
    .map_err(|_| invalid_algebraic_isolation_error())?;
    if polynomial.max_coefficient_bits() > u64::from(limits.max_polynomial_coefficient_bits) {
        return Ok(None);
    }
    let isolating_interval = RationalInterval {
        lower: value.subtract(&Rational::one()),
        upper: value.add(&Rational::one()),
    };
    match RealAlgebraic::from_irreducible_polynomial(
        polynomial,
        isolating_interval,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(Some(value)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn evaluate_real_algebraic_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let Some(coefficient) = evaluate_pi_coefficient(dag, argument)? else {
        return Ok(None);
    };
    match function {
        Function::Sin => cyclotomic_sine_algebraic(coefficient.coefficient(), limits),
        Function::Cos => cyclotomic_cosine_algebraic(coefficient.coefficient(), limits),
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
            multiply_real_algebraics(sine, reciprocal_cosine, limits)
        }
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Exp
        | Function::Log => Ok(None),
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

fn rational_prime_root_algebraic(
    base: &Rational,
    exponent: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    if base.is_zero() || exponent.is_integer() {
        return Ok(None);
    }
    let Some(root_index) = exponent.denominator.inner.inner.to_u32() else {
        return Ok(None);
    };
    if !is_supported_minimal_prime_root_index(root_index) {
        return Ok(None);
    }
    if root_index > limits.max_algebraic_degree {
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
    let polynomial = prime_root_polynomial(&powered_base, root_index)?;
    if polynomial.max_coefficient_bits() > u64::from(limits.max_polynomial_coefficient_bits) {
        return Ok(None);
    }
    let intervals = match polynomial.isolate_real_roots(limits.max_root_isolation_steps) {
        Ok(intervals) => intervals,
        Err(PrimitivePolynomialRootIsolationError::StepLimitExceeded) => return Ok(None),
        Err(PrimitivePolynomialRootIsolationError::ZeroPolynomial) => {
            return Err(invalid_algebraic_isolation_error());
        }
        Err(PrimitivePolynomialRootIsolationError::CountOverflow) => {
            return Err(EvaluationError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::RootIsolationSteps,
            }));
        }
    };
    let [isolating_interval] = intervals.as_slice() else {
        return Err(invalid_algebraic_isolation_error());
    };
    match RealAlgebraic::from_irreducible_polynomial(
        polynomial,
        isolating_interval.clone(),
        limits.max_root_isolation_steps,
    ) {
        Ok(algebraic) => Ok(Some(algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn is_supported_minimal_prime_root_index(value: u32) -> bool {
    if value < 3 || value.is_multiple_of(2) {
        return false;
    }
    let mut divisor = 3;
    while divisor <= value / divisor {
        if value.is_multiple_of(divisor) {
            return false;
        }
        divisor += 2;
    }
    true
}

fn prime_root_polynomial(
    powered_base: &Rational,
    root_index: u32,
) -> Result<PrimitivePolynomial, EvaluationError> {
    let mut coefficients = vec![Integer::from_bigint(-powered_base.numerator.inner.clone())];
    coefficients.resize(root_index as usize, Integer::zero());
    coefficients.push(powered_base.denominator.inner.clone());
    PrimitivePolynomial::new(coefficients).map_err(|_| invalid_algebraic_isolation_error())
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
        | Function::Exp
        | Function::Log => {
            return Ok(None);
        }
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
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Exp
        | Function::Log => {
            return Err(unsupported_function_evaluation());
        }
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
        | Function::Exp
        | Function::Log => {
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
            Function::Exp => interval::exp(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Log => interval::log(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Sin | Function::Cos => {
                evaluate_interval_node(dag, *argument, precision_bits)?;
                interval::from_rational_bounds(&rational(-1, 1), &rational(1, 1), precision_bits)
            }
            Function::Tan => tangent_rational_pi_interval(dag, *argument, precision_bits),
            Function::Atan => interval::atan(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Asin | Function::Acos => Err(IntervalError::UnsupportedExpression),
        },
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
                .expect("denominator should be algebraic");
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
            .expect("algebraic quotient should be recognized");

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
