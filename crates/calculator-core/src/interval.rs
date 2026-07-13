use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::types::{
    ceil_nth_root_nonnegative, ceil_sqrt_nonnegative, floor_nth_root_nonnegative,
    floor_sqrt_nonnegative, CertifiedInterval, Constant, DomainErrorKind, ExactDyadic, Integer,
    PositiveInteger, Rational,
};

const MAX_EXP_RANGE_REDUCTION_STEPS: u32 = 4096;
const MAX_DIRECT_EXP_REDUCTION: u64 = 64;
const MAX_BINARY_EXPONENT_MAGNITUDE: u64 = 1_000_000;
const MAX_LOG_RANGE_REDUCTION_STEPS: u32 = 4096;
const LOG_BINARY_SPLIT_LEAF_TERMS: u32 = 32;
const LOG_BINARY_SPLIT_THRESHOLD: u32 = LOG_BINARY_SPLIT_LEAF_TERMS + 1;
const ATAN_BINARY_SPLIT_LEAF_TERMS: u32 = 32;
const ATAN_BINARY_SPLIT_THRESHOLD: u32 = ATAN_BINARY_SPLIT_LEAF_TERMS + 1;
const MAX_TRIG_RANGE_REDUCTION_STEPS: u32 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntervalError {
    Domain(DomainErrorKind),
    InvalidBounds,
    UnsupportedExpression,
    ExponentTooLarge,
    DivisionByIntervalContainingZero,
}

pub(crate) fn from_rational(value: &Rational, precision_bits: u32) -> CertifiedInterval {
    let exponent_two = -BigInt::from(precision_bits);
    CertifiedInterval {
        lower: rational_to_dyadic_lower(value, precision_bits, exponent_two.clone()),
        upper: rational_to_dyadic_upper(value, precision_bits, exponent_two),
    }
}

pub(crate) fn from_rational_bounds(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if compare_rationals(lower, upper) == Ordering::Greater {
        return Err(IntervalError::InvalidBounds);
    }
    Ok(CertifiedInterval {
        lower: rational_to_dyadic_lower(lower, precision_bits, -BigInt::from(precision_bits)),
        upper: rational_to_dyadic_upper(upper, precision_bits, -BigInt::from(precision_bits)),
    })
}

pub(crate) fn intersect(
    left: &CertifiedInterval,
    right: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let left_lower = dyadic_to_rational(&left.lower)?;
    let left_upper = dyadic_to_rational(&left.upper)?;
    let right_lower = dyadic_to_rational(&right.lower)?;
    let right_upper = dyadic_to_rational(&right.upper)?;
    let lower = if compare_rationals(&left_lower, &right_lower) == Ordering::Less {
        right_lower
    } else {
        left_lower
    };
    let upper = if compare_rationals(&left_upper, &right_upper) == Ordering::Greater {
        right_upper
    } else {
        left_upper
    };
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn absolute(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let zero = Rational::zero();
    if compare_rationals(&lower, &zero) != Ordering::Less {
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    if compare_rationals(&upper, &zero) != Ordering::Greater {
        return from_rational_bounds(&upper.negate(), &lower.negate(), precision_bits);
    }
    let magnitude = if compare_rationals(&lower.negate(), &upper) == Ordering::Greater {
        lower.negate()
    } else {
        upper
    };
    from_rational_bounds(&zero, &magnitude, precision_bits)
}

pub(crate) fn unique_floor(value: &CertifiedInterval) -> Result<Option<Rational>, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let lower_floor = lower
        .numerator
        .inner
        .div_floor(&lower.denominator.inner.inner);
    let upper_floor = upper
        .numerator
        .inner
        .div_floor(&upper.denominator.inner.inner);
    if lower_floor == upper_floor {
        Ok(Some(Rational::from_integer(Integer::from_bigint(
            lower_floor,
        ))))
    } else {
        Ok(None)
    }
}

pub(crate) fn constant(
    value: Constant,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let (lower, upper) = match value {
        Constant::Euler => e_bounds(precision_bits)?,
        Constant::Pi => pi_bounds(precision_bits)?,
    };
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn add(
    left: &CertifiedInterval,
    right: &CertifiedInterval,
) -> Result<CertifiedInterval, IntervalError> {
    Ok(CertifiedInterval {
        lower: add_dyadic(&left.lower, &right.lower)?,
        upper: add_dyadic(&left.upper, &right.upper)?,
    })
}

pub(crate) fn multiply(
    left: &CertifiedInterval,
    right: &CertifiedInterval,
) -> Result<CertifiedInterval, IntervalError> {
    let left_nonnegative = left.lower.coefficient.sign() != Sign::Minus;
    let left_nonpositive = left.upper.coefficient.sign() != Sign::Plus;
    let right_nonnegative = right.lower.coefficient.sign() != Sign::Minus;
    let right_nonpositive = right.upper.coefficient.sign() != Sign::Plus;
    if left_nonnegative && right_nonnegative {
        return Ok(CertifiedInterval {
            lower: multiply_dyadic(&left.lower, &right.lower),
            upper: multiply_dyadic(&left.upper, &right.upper),
        });
    }
    if left_nonpositive && right_nonpositive {
        return Ok(CertifiedInterval {
            lower: multiply_dyadic(&left.upper, &right.upper),
            upper: multiply_dyadic(&left.lower, &right.lower),
        });
    }
    if left_nonnegative && right_nonpositive {
        return Ok(CertifiedInterval {
            lower: multiply_dyadic(&left.upper, &right.lower),
            upper: multiply_dyadic(&left.lower, &right.upper),
        });
    }
    if left_nonpositive && right_nonnegative {
        return Ok(CertifiedInterval {
            lower: multiply_dyadic(&left.lower, &right.upper),
            upper: multiply_dyadic(&left.upper, &right.lower),
        });
    }
    let candidates = [
        multiply_dyadic(&left.lower, &right.lower),
        multiply_dyadic(&left.lower, &right.upper),
        multiply_dyadic(&left.upper, &right.lower),
        multiply_dyadic(&left.upper, &right.upper),
    ];
    let mut lower = candidates[0].clone();
    let mut upper = candidates[0].clone();
    for candidate in candidates.iter().skip(1) {
        if compare_dyadic(candidate, &lower)? == Ordering::Less {
            lower = candidate.clone();
        }
        if compare_dyadic(candidate, &upper)? == Ordering::Greater {
            upper = candidate.clone();
        }
    }
    Ok(CertifiedInterval { lower, upper })
}

pub(crate) fn divide(
    numerator: &CertifiedInterval,
    denominator: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if contains_zero(denominator) {
        return Err(IntervalError::DivisionByIntervalContainingZero);
    }
    multiply(
        numerator,
        &reciprocal_interval(denominator, precision_bits)?,
    )
}

pub(crate) fn sqrt(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if value.upper.coefficient.inner.sign() == Sign::Minus {
        return Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative));
    }
    if value.lower.coefficient.inner.sign() == Sign::Minus {
        return Err(IntervalError::UnsupportedExpression);
    }

    if value.lower == value.upper {
        let (lower, upper) = sqrt_dyadic_bounds(&value.lower, precision_bits)?;
        Ok(CertifiedInterval { lower, upper })
    } else {
        Ok(CertifiedInterval {
            lower: sqrt_dyadic_lower(&value.lower, precision_bits)?,
            upper: sqrt_dyadic_upper(&value.upper, precision_bits)?,
        })
    }
}

pub(crate) fn exp(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if exp_uses_binary_scaling(&lower) || exp_uses_binary_scaling(&upper) {
        if lower == upper {
            let plan = exp_binary_scaling_plan(&lower, precision_bits)?;
            return Ok(CertifiedInterval {
                lower: exp_binary_scaled_bound_with_plan(&lower, BoundDirection::Lower, &plan)?,
                upper: exp_binary_scaled_bound_with_plan(&upper, BoundDirection::Upper, &plan)?,
            });
        }
        return Ok(CertifiedInterval {
            lower: exp_binary_scaled_bound(&lower, precision_bits, BoundDirection::Lower)?,
            upper: exp_binary_scaled_bound(&upper, precision_bits, BoundDirection::Upper)?,
        });
    }
    if lower == upper && exp_can_round_series_directly(&lower) {
        let series_plan = exp_series_plan(precision_bits)?;
        return exp_series_dyadic_bounds_with_plan(&lower, &series_plan, precision_bits);
    }
    if lower == upper {
        let (lower, upper) = exp_rational_bounds(&lower, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let series_plan = exp_series_plan(precision_bits)?;
    let term_count = series_plan.term_count;
    let (lower, upper) = if lower.denominator == upper.denominator
        && exp_can_round_series_directly(&lower)
        && exp_can_round_series_directly(&upper)
    {
        let common_denominator = exp_series_common_denominator_with_factorial(
            &lower.denominator.inner.inner,
            term_count,
            &series_plan.factorial,
        )?;
        (
            exp_series_dyadic_bound_with_common_denominator(
                &lower,
                term_count,
                BoundDirection::Lower,
                common_denominator.clone(),
                precision_bits,
            )?,
            exp_series_dyadic_bound_with_common_denominator(
                &upper,
                term_count,
                BoundDirection::Upper,
                common_denominator,
                precision_bits,
            )?,
        )
    } else {
        (
            exp_dyadic_bound_with_plan(
                &lower,
                &series_plan,
                BoundDirection::Lower,
                precision_bits,
            )?,
            exp_dyadic_bound_with_plan(
                &upper,
                &series_plan,
                BoundDirection::Upper,
                precision_bits,
            )?,
        )
    };
    Ok(CertifiedInterval { lower, upper })
}

fn exp_can_round_series_directly(value: &Rational) -> bool {
    !value.is_negative()
        && !value.is_zero()
        && !value.denominator.inner.inner.is_one()
        && value.numerator.inner.magnitude() <= value.denominator.inner.inner.magnitude()
}

fn exp_uses_binary_scaling(value: &Rational) -> bool {
    let numerator = value.numerator.inner.magnitude();
    let denominator = value.denominator.inner.inner.magnitude();
    let threshold_bits = denominator
        .bits()
        .saturating_add(MAX_DIRECT_EXP_REDUCTION.ilog2().into());
    match numerator.bits().cmp(&threshold_bits) {
        Ordering::Less => false,
        Ordering::Greater => true,
        Ordering::Equal => numerator > &(denominator << MAX_DIRECT_EXP_REDUCTION.ilog2()),
    }
}

fn exp_binary_scaled_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<ExactDyadic, IntervalError> {
    let plan = exp_binary_scaling_plan(value, precision_bits)?;
    exp_binary_scaled_bound_with_plan(value, direction, &plan)
}

struct ExpBinaryScalingPlan {
    working_precision: u32,
    series_plan: ExpSeriesPlan,
    log_two_lower: Rational,
    log_two_upper: Rational,
    binary_exponent: i64,
}

fn exp_binary_scaling_plan(
    value: &Rational,
    precision_bits: u32,
) -> Result<ExpBinaryScalingPlan, IntervalError> {
    let magnitude = abs_rational(value);
    // ln(2) < 1, so this magnitude already implies an exponent beyond the
    // public cap. Reject it before magnitude-dependent guard precision and
    // certified ln(2) construction. Endpoints below this check need at most
    // 20 magnitude guard bits, including endpoints of non-rational syntax.
    if magnitude.numerator.inner.magnitude()
        > &(magnitude.denominator.inner.inner.magnitude()
            * BigUint::from(MAX_BINARY_EXPONENT_MAGNITUDE))
    {
        return Err(IntervalError::ExponentTooLarge);
    }
    let magnitude_integer = magnitude
        .numerator
        .inner
        .div_ceil(&magnitude.denominator.inner.inner);
    let guard_bits = magnitude_integer
        .to_biguint()
        .map_or(0_u64, |value| value.bits())
        .try_into()
        .map_err(|_| IntervalError::ExponentTooLarge)?;
    let working_precision = precision_bits
        .checked_add(guard_bits)
        .and_then(|value| value.checked_add(2))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let series_plan = exp_series_plan(working_precision)?;
    let (log_two_lower, log_two_upper) =
        log_reduced_rational_bounds(&rational_integer(2), working_precision)?;
    let log_two_midpoint = halve_rational(&log_two_lower.add(&log_two_upper))?;
    let quotient = divide_rational(value, &log_two_midpoint)?;
    let binary_exponent = quotient
        .numerator
        .inner
        .div_floor(&quotient.denominator.inner.inner)
        .to_i64()
        .ok_or(IntervalError::ExponentTooLarge)?;
    if binary_exponent.unsigned_abs() > MAX_BINARY_EXPONENT_MAGNITUDE {
        return Err(IntervalError::ExponentTooLarge);
    }
    Ok(ExpBinaryScalingPlan {
        working_precision,
        series_plan,
        log_two_lower,
        log_two_upper,
        binary_exponent,
    })
}

fn exp_binary_scaled_bound_with_plan(
    value: &Rational,
    direction: BoundDirection,
    plan: &ExpBinaryScalingPlan,
) -> Result<ExactDyadic, IntervalError> {
    let residual = match (plan.binary_exponent.is_negative(), direction) {
        (false, BoundDirection::Lower) | (true, BoundDirection::Upper) => value.subtract(
            &scale_rational_by_i64(&plan.log_two_upper, plan.binary_exponent)?,
        ),
        (false, BoundDirection::Upper) | (true, BoundDirection::Lower) => value.subtract(
            &scale_rational_by_i64(&plan.log_two_lower, plan.binary_exponent)?,
        ),
    };
    let mut dyadic = exp_dyadic_bound_with_plan(
        &residual,
        &plan.series_plan,
        direction,
        plan.working_precision,
    )?;
    dyadic.exponent_two.inner += BigInt::from(plan.binary_exponent);
    Ok(normalize_dyadic(
        dyadic.coefficient.inner,
        dyadic.exponent_two.inner,
    ))
}

fn exp_dyadic_bound_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    direction: BoundDirection,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    if exp_can_round_series_directly(value) {
        return exp_series_dyadic_bound_with_plan(value, plan, direction, precision_bits);
    }
    let bound = exp_rational_bound_with_plan(value, plan, direction)?;
    Ok(rational_to_dyadic_bound(&bound, precision_bits, direction))
}

pub(crate) fn log(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if upper.is_negative() || upper.is_zero() {
        return Err(IntervalError::Domain(
            DomainErrorKind::LogarithmOfNonPositive,
        ));
    }
    if lower.is_negative() || lower.is_zero() {
        return Err(IntervalError::UnsupportedExpression);
    }
    let (lower, upper) = if lower == upper {
        log_rational_bounds(&lower, precision_bits)?
    } else {
        log_rational_directed_endpoint_bounds(&lower, &upper, precision_bits)?
    };
    from_rational_bounds(&lower, &upper, precision_bits)
}

fn log_rational_directed_endpoint_bounds(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let (lower_reduced, lower_exponent_two) = reduce_log_argument_to_unit_range(lower)?;
    let (upper_reduced, upper_exponent_two) = reduce_log_argument_to_unit_range(upper)?;
    let needs_series = !is_positive_one_rational(&lower_reduced)
        || !is_positive_one_rational(&upper_reduced)
        || lower_exponent_two != 0
        || upper_exponent_two != 0;
    if !needs_series {
        return Ok((Rational::zero(), Rational::zero()));
    }
    let term_count = log_series_terms(precision_bits)?;
    let lower_bound =
        log_reduced_rational_bound_with_terms(&lower_reduced, term_count, BoundDirection::Lower)?;
    let upper_bound =
        log_reduced_rational_bound_with_terms(&upper_reduced, term_count, BoundDirection::Upper)?;
    if lower_exponent_two == 0 && upper_exponent_two == 0 {
        return Ok((lower_bound, upper_bound));
    }
    let shared_log_two = if lower_exponent_two != 0 && upper_exponent_two != 0 {
        Some(log_reduced_rational_bounds_with_terms(
            &rational_integer(2),
            term_count,
        )?)
    } else {
        None
    };
    let lower = if lower_exponent_two == 0 {
        lower_bound
    } else {
        let direction = if lower_exponent_two > 0 {
            BoundDirection::Lower
        } else {
            BoundDirection::Upper
        };
        let owned_log_two;
        let log_two = if let Some((log_two_lower, log_two_upper)) = shared_log_two.as_ref() {
            match direction {
                BoundDirection::Lower => log_two_lower,
                BoundDirection::Upper => log_two_upper,
            }
        } else {
            owned_log_two =
                log_reduced_rational_bound_with_terms(&rational_integer(2), term_count, direction)?;
            &owned_log_two
        };
        lower_bound.add(&scale_rational_by_i64(log_two, lower_exponent_two)?)
    };
    let upper = if upper_exponent_two == 0 {
        upper_bound
    } else {
        let direction = if upper_exponent_two > 0 {
            BoundDirection::Upper
        } else {
            BoundDirection::Lower
        };
        let owned_log_two;
        let log_two = if let Some((log_two_lower, log_two_upper)) = shared_log_two.as_ref() {
            match direction {
                BoundDirection::Lower => log_two_lower,
                BoundDirection::Upper => log_two_upper,
            }
        } else {
            owned_log_two =
                log_reduced_rational_bound_with_terms(&rational_integer(2), term_count, direction)?;
            &owned_log_two
        };
        upper_bound.add(&scale_rational_by_i64(log_two, upper_exponent_two)?)
    };
    Ok((lower, upper))
}

pub(crate) fn atan(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if lower == upper {
        let (lower, upper) = atan_rational_bounds(&lower, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let shared_pi = if !is_unit_rational(&lower) && !is_unit_rational(&upper) {
        Some(pi_bounds(precision_bits)?)
    } else {
        None
    };
    let lower = atan_rational_bound_with_pi(
        &lower,
        precision_bits,
        BoundDirection::Lower,
        shared_pi.as_ref(),
    )?;
    let upper = atan_rational_bound_with_pi(
        &upper,
        precision_bits,
        BoundDirection::Upper,
        shared_pi.as_ref(),
    )?;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn asin(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let (lower, upper) = inverse_sine_cosine_domain_bounds(value)?;
    if lower == upper {
        let (lower, upper) = asin_rational_bounds(&lower, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let shared_pi = if compare_absolute_rational_to_half(&lower) == Ordering::Greater
        && compare_absolute_rational_to_half(&upper) == Ordering::Greater
    {
        Some(pi_bounds(precision_bits)?)
    } else {
        None
    };
    let lower = asin_rational_bound_with_pi(
        &lower,
        precision_bits,
        BoundDirection::Lower,
        shared_pi.as_ref(),
    )?;
    let upper = asin_rational_bound_with_pi(
        &upper,
        precision_bits,
        BoundDirection::Upper,
        shared_pi.as_ref(),
    )?;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn acos(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let (lower_endpoint, upper_endpoint) = inverse_sine_cosine_domain_bounds(value)?;
    if lower_endpoint == upper_endpoint {
        let (lower, upper) = acos_rational_bounds(&lower_endpoint, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let pi = pi_bounds(precision_bits)?;
    let lower =
        acos_rational_bound_with_pi(&upper_endpoint, precision_bits, BoundDirection::Lower, &pi)?;
    let upper =
        acos_rational_bound_with_pi(&lower_endpoint, precision_bits, BoundDirection::Upper, &pi)?;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn tan(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if contains_possible_tangent_pole(&lower, &upper, precision_bits)? {
        return Err(IntervalError::UnsupportedExpression);
    }

    let (lower, upper) =
        trigonometric_endpoint_bounds(&lower, &upper, precision_bits, tan_rational)?;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn sin_rational(
    value: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if is_unit_rational(value) {
        let (lower, upper) = sin_unit_rational_bounds(value, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let (sine, _) = sin_cos_rational(value, precision_bits)?;
    Ok(sine)
}

pub(crate) fn cos_rational(
    value: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if is_unit_rational(value) {
        let (lower, upper) = cos_unit_rational_bounds(value, precision_bits)?;
        return from_rational_bounds(&lower, &upper, precision_bits);
    }
    let (_, cosine) = sin_cos_rational(value, precision_bits)?;
    Ok(cosine)
}

fn is_unit_rational(value: &Rational) -> bool {
    value.numerator.inner.magnitude() <= value.denominator.inner.inner.magnitude()
}

fn is_positive_one_rational(value: &Rational) -> bool {
    value.numerator.inner.sign() == Sign::Plus
        && value.numerator.inner.magnitude() == value.denominator.inner.inner.magnitude()
}

fn is_negative_one_rational(value: &Rational) -> bool {
    value.numerator.inner.sign() == Sign::Minus
        && value.numerator.inner.magnitude() == value.denominator.inner.inner.magnitude()
}

fn compare_nonnegative_rational_to_half(value: &Rational) -> Ordering {
    debug_assert!(!value.is_negative());
    compare_absolute_rational_to_half(value)
}

fn compare_absolute_rational_to_half(value: &Rational) -> Ordering {
    (value.numerator.inner.magnitude() * 2_u8).cmp(value.denominator.inner.inner.magnitude())
}

pub(crate) fn tan_rational(
    value: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let (sine, cosine) = sin_cos_rational(value, precision_bits)?;
    if contains_zero(&cosine) {
        return Err(IntervalError::UnsupportedExpression);
    }
    divide(&sine, &cosine, precision_bits)
}

pub(crate) fn sin(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if covers_full_trigonometric_period(&lower, &upper, precision_bits)? {
        return full_trigonometric_range(precision_bits);
    }

    let Some((mut lower_bound, mut upper_bound)) =
        bounded_trigonometric_endpoint_bounds(&lower, &upper, precision_bits, sin_rational)?
    else {
        return full_trigonometric_range(precision_bits);
    };
    if !include_sine_extrema(
        &lower,
        &upper,
        &mut lower_bound,
        &mut upper_bound,
        precision_bits,
    )? {
        return full_trigonometric_range(precision_bits);
    }

    let lower = lower_bound;
    let upper = upper_bound;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn cos(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    if covers_full_trigonometric_period(&lower, &upper, precision_bits)? {
        return full_trigonometric_range(precision_bits);
    }

    let Some((mut lower_bound, mut upper_bound)) =
        bounded_trigonometric_endpoint_bounds(&lower, &upper, precision_bits, cos_rational)?
    else {
        return full_trigonometric_range(precision_bits);
    };
    if !include_cosine_extrema(
        &lower,
        &upper,
        &mut lower_bound,
        &mut upper_bound,
        precision_bits,
    )? {
        return full_trigonometric_range(precision_bits);
    }

    let lower = lower_bound;
    let upper = upper_bound;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn pow_i64(
    base: &CertifiedInterval,
    exponent: i64,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if exponent == 0 {
        return Ok(from_rational(&Rational::one(), precision_bits));
    }
    let magnitude = exponent
        .checked_abs()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(IntervalError::ExponentTooLarge)?;
    let mut result = from_rational(&Rational::one(), precision_bits);
    let mut factor = base.clone();
    let mut remaining = magnitude;
    while remaining > 0 {
        if remaining % 2 == 1 {
            result = multiply(&result, &factor)?;
        }
        remaining /= 2;
        if remaining > 0 {
            factor = multiply(&factor, &factor)?;
        }
    }
    if exponent > 0 {
        Ok(result)
    } else {
        divide(
            &from_rational(&Rational::one(), precision_bits),
            &result,
            precision_bits,
        )
    }
}

pub(crate) fn pow_rational(
    base: &Rational,
    exponent: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if base.is_zero() {
        return zero_power(exponent, precision_bits);
    }

    let exponent_numerator = exponent
        .numerator
        .inner
        .to_i64()
        .ok_or(IntervalError::ExponentTooLarge)?;
    if exponent.is_integer() {
        return pow_i64(
            &from_rational(base, precision_bits),
            exponent_numerator,
            precision_bits,
        );
    }

    let root_index = exponent
        .denominator
        .inner
        .inner
        .to_u32()
        .ok_or(IntervalError::ExponentTooLarge)?;
    if base.is_negative() && root_index.is_multiple_of(2) {
        return Err(IntervalError::Domain(DomainErrorKind::NonRealPower));
    }
    let root = nth_root_rational(base, root_index, precision_bits)?;
    pow_i64(&root, exponent_numerator, precision_bits)
}

pub(crate) fn pow_positive_base(
    base: &CertifiedInterval,
    exponent: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if base.lower.coefficient.sign() != Sign::Plus {
        return Err(IntervalError::UnsupportedExpression);
    }
    exp(
        &multiply(&log(base, precision_bits)?, exponent)?,
        precision_bits,
    )
}

pub(crate) fn contains_rational(
    interval: &CertifiedInterval,
    value: &Rational,
) -> Result<bool, IntervalError> {
    Ok(
        compare_dyadic_to_rational(&interval.lower, value)? != Ordering::Greater
            && compare_dyadic_to_rational(&interval.upper, value)? != Ordering::Less,
    )
}

fn zero_power(
    exponent: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if exponent.is_zero() {
        Err(IntervalError::Domain(
            DomainErrorKind::IndeterminateZeroToZero,
        ))
    } else if exponent.is_negative() {
        Err(IntervalError::Domain(DomainErrorKind::ZeroToNegativePower))
    } else {
        Ok(from_rational(&Rational::zero(), precision_bits))
    }
}

fn nth_root_rational(
    value: &Rational,
    index: u32,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if index == 0 {
        return Err(IntervalError::ExponentTooLarge);
    }
    if value.is_negative() {
        if index.is_multiple_of(2) {
            return Err(IntervalError::Domain(DomainErrorKind::NonRealPower));
        }
        let positive = nth_root_nonnegative_rational(&value.negate(), index, precision_bits)?;
        return Ok(CertifiedInterval {
            lower: negate_dyadic(&positive.upper),
            upper: negate_dyadic(&positive.lower),
        });
    }
    nth_root_nonnegative_rational(value, index, precision_bits)
}

fn nth_root_nonnegative_rational(
    value: &Rational,
    index: u32,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    debug_assert!(!value.is_negative());
    if index == 1 {
        return Ok(from_rational(value, precision_bits));
    }
    let scale_bits = precision_bits
        .checked_mul(index)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let scaled_numerator = &value.numerator.inner << scale_bits;
    let denominator = &value.denominator.inner.inner;
    let scaled_lower = scaled_numerator.div_floor(denominator);
    let scaled_upper = scaled_numerator.div_ceil(denominator);
    Ok(CertifiedInterval {
        lower: normalize_dyadic(
            floor_nth_root_nonnegative(&scaled_lower, index),
            -BigInt::from(precision_bits),
        ),
        upper: normalize_dyadic(
            ceil_nth_root_nonnegative(&scaled_upper, index),
            -BigInt::from(precision_bits),
        ),
    })
}

fn exp_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    if value.is_zero() {
        return Ok((Rational::one(), Rational::one()));
    }
    if value.is_negative() {
        let (positive_lower, positive_upper) =
            exp_nonnegative_rational_bounds(&value.negate(), precision_bits)?;
        let lower = reciprocal_nonzero_rational(&positive_upper)?;
        let upper = reciprocal_nonzero_rational(&positive_lower)?;
        return Ok((lower, upper));
    }
    exp_nonnegative_rational_bounds(value, precision_bits)
}

#[derive(Clone, Copy)]
enum BoundDirection {
    Lower,
    Upper,
}

#[cfg(test)]
fn exp_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    if value.is_zero() {
        return Ok(Rational::one());
    }
    exp_rational_bound_with_terms(value, exp_series_terms(precision_bits)?, direction)
}

#[cfg(test)]
fn exp_rational_bound_with_terms(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    if value.is_zero() {
        return Ok(Rational::one());
    }
    term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let plan = ExpSeriesPlan {
        term_count,
        factorial: exp_series_factorial(term_count),
    };
    exp_rational_bound_with_plan(value, &plan, direction)
}

fn exp_rational_bound_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    if value.is_zero() {
        return Ok(Rational::one());
    }
    if value.is_negative() {
        let reciprocal_direction = match direction {
            BoundDirection::Lower => BoundDirection::Upper,
            BoundDirection::Upper => BoundDirection::Lower,
        };
        let positive =
            exp_nonnegative_rational_bound_with_plan(&value.negate(), plan, reciprocal_direction)?;
        return reciprocal_nonzero_rational(&positive);
    }
    exp_nonnegative_rational_bound_with_plan(value, plan, direction)
}

fn reciprocal_nonzero_rational(value: &Rational) -> Result<Rational, IntervalError> {
    if value.is_zero() {
        return Err(IntervalError::DivisionByIntervalContainingZero);
    }
    let (numerator, denominator) = if value.is_negative() {
        (
            Integer::from_bigint(-value.denominator.inner.inner.clone()),
            Integer::from_bigint(-value.numerator.inner.clone()),
        )
    } else {
        (value.denominator.inner.clone(), value.numerator.clone())
    };
    Ok(Rational {
        numerator,
        denominator: PositiveInteger { inner: denominator },
    })
}

fn exp_nonnegative_rational_bound_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    debug_assert!(!value.is_negative());
    let reduction = ceil_nonnegative_rational_to_u32(value)?;
    let reduced_storage;
    let reduced = if reduction == 1 {
        value
    } else {
        reduced_storage = divide_rational_by_positive_u32(value, reduction)?;
        &reduced_storage
    };
    let bound = exp_series_rational_bound_with_plan(reduced, plan, direction)?;
    if reduction == 1 {
        Ok(bound)
    } else {
        pow_positive_rational(&bound, reduction)
    }
}

fn exp_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    exp_nonnegative_rational_bounds_with(
        value,
        precision_bits,
        exp_small_nonnegative_rational_bounds,
        pow_positive_rational,
    )
}

fn exp_nonnegative_rational_bounds_with<S, P>(
    value: &Rational,
    precision_bits: u32,
    mut small_bounds: S,
    mut positive_power: P,
) -> Result<(Rational, Rational), IntervalError>
where
    S: FnMut(&Rational, u32) -> Result<(Rational, Rational), IntervalError>,
    P: FnMut(&Rational, u32) -> Result<Rational, IntervalError>,
{
    debug_assert!(!value.is_negative());
    let reduction = ceil_nonnegative_rational_to_u32(value)?;
    if reduction == 1 {
        return small_bounds(value, precision_bits);
    }
    let reduced = divide_rational_by_positive_u32(value, reduction)?;
    let (lower, upper) = small_bounds(&reduced, precision_bits)?;
    Ok((
        positive_power(&lower, reduction)?,
        positive_power(&upper, reduction)?,
    ))
}

fn exp_small_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Greater);
    let plan = exp_series_plan(precision_bits)?;
    exp_series_rational_bounds_with_plan(value, &plan)
}

#[cfg(test)]
fn exp_series_rational_bounds(
    value: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let plan = ExpSeriesPlan {
        term_count,
        factorial: exp_series_factorial(term_count),
    };
    exp_series_rational_bounds_with_plan(value, &plan)
}

fn exp_series_rational_bounds_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
) -> Result<(Rational, Rational), IntervalError> {
    let term_count = plan.term_count;
    debug_assert!(!value.is_negative());
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Greater);
    let tail_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    if value.is_zero() {
        return Ok((Rational::one(), Rational::one()));
    }
    let state = exp_series_state_with_plan(value, plan, tail_index)?;
    state.into_bounds()
}

fn exp_series_dyadic_bounds_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let tail_index = plan
        .term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    if value.is_zero() {
        let one = ExactDyadic {
            coefficient: Integer::one(),
            exponent_two: Integer::zero(),
        };
        return Ok(CertifiedInterval {
            lower: one.clone(),
            upper: one,
        });
    }
    let state = exp_series_state_with_plan(value, plan, tail_index)?;
    let (upper_numerator, upper_denominator) = state.upper_parts();
    let lower = fraction_to_dyadic_bound(
        &state.sum_numerator,
        &state.common_denominator,
        precision_bits,
        BoundDirection::Lower,
    );
    let upper = fraction_to_dyadic_bound(
        &upper_numerator,
        &upper_denominator,
        precision_bits,
        BoundDirection::Upper,
    );
    Ok(CertifiedInterval { lower, upper })
}

#[cfg(test)]
fn exp_series_rational_bound(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let plan = ExpSeriesPlan {
        term_count,
        factorial: exp_series_factorial(term_count),
    };
    exp_series_rational_bound_with_plan(value, &plan, direction)
}

fn exp_series_rational_bound_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    let term_count = plan.term_count;
    let tail_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    if value.is_zero() {
        return Ok(Rational::one());
    }
    let state = exp_series_state_with_plan(value, plan, tail_index)?;
    match direction {
        BoundDirection::Lower => state.into_lower(),
        BoundDirection::Upper => state.into_upper(),
    }
}

fn exp_series_dyadic_bound_with_plan(
    value: &Rational,
    plan: &ExpSeriesPlan,
    direction: BoundDirection,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    let tail_index = plan
        .term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    if value.is_zero() {
        return Ok(ExactDyadic {
            coefficient: Integer::one(),
            exponent_two: Integer::zero(),
        });
    }
    let state = exp_series_state_with_plan(value, plan, tail_index)?;
    let (numerator, denominator) = state.into_parts(direction);
    Ok(fraction_to_dyadic_bound(
        &numerator,
        &denominator,
        precision_bits,
        direction,
    ))
}

#[cfg(test)]
fn exp_series_rational_bound_with_common_denominator(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
    common_denominator: BigInt,
) -> Result<Rational, IntervalError> {
    let tail_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let state = exp_series_state_with_common_denominator(
        value,
        term_count,
        tail_index,
        common_denominator,
    )?;
    match direction {
        BoundDirection::Lower => state.into_lower(),
        BoundDirection::Upper => state.into_upper(),
    }
}

fn exp_series_dyadic_bound_with_common_denominator(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
    common_denominator: BigInt,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    let tail_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let state = exp_series_state_with_common_denominator(
        value,
        term_count,
        tail_index,
        common_denominator,
    )?;
    let (numerator, denominator) = state.into_parts(direction);
    Ok(fraction_to_dyadic_bound(
        &numerator,
        &denominator,
        precision_bits,
        direction,
    ))
}

struct ExpSeriesState<'a> {
    sum_numerator: BigInt,
    term_numerator: BigInt,
    common_denominator: BigInt,
    value_numerator: &'a BigInt,
    value_denominator: &'a BigInt,
    tail_index: u32,
}

impl ExpSeriesState<'_> {
    fn into_lower(self) -> Result<Rational, IntervalError> {
        rational_from_parts(self.sum_numerator, self.common_denominator)
    }

    fn into_upper(self) -> Result<Rational, IntervalError> {
        let mut state = self;
        let next_denominator_factor = state.value_denominator * state.tail_index;
        state.sum_numerator *= &next_denominator_factor;
        state.term_numerator *= state.value_numerator;
        state.term_numerator *= 2_u8;
        state.sum_numerator += state.term_numerator;
        state.common_denominator *= next_denominator_factor;
        rational_from_parts(state.sum_numerator, state.common_denominator)
    }

    fn into_bounds(self) -> Result<(Rational, Rational), IntervalError> {
        let upper = self.upper()?;
        let lower = rational_from_parts(self.sum_numerator, self.common_denominator)?;
        Ok((lower, upper))
    }

    fn upper(&self) -> Result<Rational, IntervalError> {
        let (upper_numerator, upper_denominator) = self.upper_parts();
        rational_from_parts(upper_numerator, upper_denominator)
    }

    fn into_parts(mut self, direction: BoundDirection) -> (BigInt, BigInt) {
        if matches!(direction, BoundDirection::Upper) {
            let next_denominator_factor = self.value_denominator * self.tail_index;
            self.sum_numerator *= &next_denominator_factor;
            self.term_numerator *= self.value_numerator;
            self.term_numerator *= 2_u8;
            self.sum_numerator += self.term_numerator;
            self.common_denominator *= next_denominator_factor;
        }
        (self.sum_numerator, self.common_denominator)
    }

    fn upper_parts(&self) -> (BigInt, BigInt) {
        let next_denominator_factor = self.value_denominator * self.tail_index;
        let mut upper_numerator = &self.sum_numerator * &next_denominator_factor;
        upper_numerator += &self.term_numerator * self.value_numerator * 2_u8;
        let upper_denominator = &self.common_denominator * next_denominator_factor;
        (upper_numerator, upper_denominator)
    }
}

fn multiply_by_exp_denominator_factor(
    value: &mut BigInt,
    denominator: &BigInt,
    denominator_shift: Option<usize>,
    index: u32,
) {
    if let Some(shift) = denominator_shift {
        *value *= index;
        *value <<= shift;
    } else {
        *value *= denominator * index;
    }
}

fn exp_series_state_with_plan<'a>(
    value: &'a Rational,
    plan: &ExpSeriesPlan,
    tail_index: u32,
) -> Result<ExpSeriesState<'a>, IntervalError> {
    let term_count = plan.term_count;
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let common_denominator = exp_series_common_denominator_with_factorial(
        value_denominator,
        term_count,
        &plan.factorial,
    )?;
    let value_denominator_shift = recurrence_denominator_shift(value_denominator)?;
    let mut sum_numerator = BigInt::one();
    let mut term_numerator = BigInt::one();
    for next_n in 1..=term_count {
        term_numerator *= value_numerator;
        multiply_by_exp_denominator_factor(
            &mut sum_numerator,
            value_denominator,
            value_denominator_shift,
            next_n,
        );
        sum_numerator += &term_numerator;
    }
    Ok(ExpSeriesState {
        sum_numerator,
        term_numerator,
        common_denominator,
        value_numerator,
        value_denominator,
        tail_index,
    })
}

fn exp_series_state_with_common_denominator<'a>(
    value: &'a Rational,
    term_count: u32,
    tail_index: u32,
    common_denominator: BigInt,
) -> Result<ExpSeriesState<'a>, IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let value_denominator_shift = recurrence_denominator_shift(value_denominator)?;
    let mut sum_numerator = BigInt::one();
    let mut term_numerator = BigInt::one();
    for next_n in 1..=term_count {
        term_numerator *= value_numerator;
        multiply_by_exp_denominator_factor(
            &mut sum_numerator,
            value_denominator,
            value_denominator_shift,
            next_n,
        );
        sum_numerator += &term_numerator;
    }
    Ok(ExpSeriesState {
        sum_numerator,
        term_numerator,
        common_denominator,
        value_numerator,
        value_denominator,
        tail_index,
    })
}

#[cfg(test)]
fn exp_series_common_denominator(
    value_denominator: &BigInt,
    term_count: u32,
) -> Result<BigInt, IntervalError> {
    let factorial = exp_series_factorial(term_count);
    exp_series_common_denominator_with_factorial(value_denominator, term_count, &factorial)
}

fn exp_series_common_denominator_with_factorial(
    value_denominator: &BigInt,
    term_count: u32,
    factorial: &BigInt,
) -> Result<BigInt, IntervalError> {
    if let Some(denominator_shift) = positive_power_of_two_shift(value_denominator) {
        let shift = checked_exp_denominator_total_shift(denominator_shift, term_count)?;
        Ok(factorial << shift)
    } else {
        Ok(value_denominator.pow(term_count) * factorial)
    }
}

fn positive_power_of_two_shift(value: &BigInt) -> Option<u64> {
    if value.sign() != Sign::Plus {
        return None;
    }
    let magnitude = value.magnitude();
    let bits = magnitude.bits();
    if bits > 0 && magnitude.trailing_zeros() == Some(bits - 1) {
        Some(bits - 1)
    } else {
        None
    }
}

fn recurrence_denominator_shift(value: &BigInt) -> Result<Option<usize>, IntervalError> {
    positive_power_of_two_shift(value)
        .map(checked_exp_denominator_shift)
        .transpose()
}

fn checked_exp_denominator_shift(shift: u64) -> Result<usize, IntervalError> {
    shift
        .try_into()
        .map_err(|_| IntervalError::ExponentTooLarge)
}

fn checked_exp_denominator_total_shift(
    denominator_shift: u64,
    term_count: u32,
) -> Result<usize, IntervalError> {
    denominator_shift
        .checked_mul(u64::from(term_count))
        .and_then(|shift| shift.try_into().ok())
        .ok_or(IntervalError::ExponentTooLarge)
}

fn log_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    if value.is_negative() || value.is_zero() {
        return Err(IntervalError::Domain(
            DomainErrorKind::LogarithmOfNonPositive,
        ));
    }
    let (reduced, exponent_two) = reduce_log_argument_to_unit_range(value)?;
    let (lower, upper) = log_reduced_rational_bounds(&reduced, precision_bits)?;
    if exponent_two == 0 {
        return Ok((lower, upper));
    }

    let (log_two_lower, log_two_upper) =
        log_reduced_rational_bounds(&rational_integer(2), precision_bits)?;
    if exponent_two > 0 {
        Ok((
            lower.add(&scale_rational_by_i64(&log_two_lower, exponent_two)?),
            upper.add(&scale_rational_by_i64(&log_two_upper, exponent_two)?),
        ))
    } else {
        Ok((
            lower.add(&scale_rational_by_i64(&log_two_upper, exponent_two)?),
            upper.add(&scale_rational_by_i64(&log_two_lower, exponent_two)?),
        ))
    }
}

#[cfg(test)]
fn log_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    if value.is_negative() || value.is_zero() {
        return Err(IntervalError::Domain(
            DomainErrorKind::LogarithmOfNonPositive,
        ));
    }
    let (reduced, exponent_two) = reduce_log_argument_to_unit_range(value)?;
    let reduced_bound = log_reduced_rational_bound(&reduced, precision_bits, direction)?;
    if exponent_two == 0 {
        return Ok(reduced_bound);
    }
    let log_two_direction = if exponent_two > 0 {
        direction
    } else {
        match direction {
            BoundDirection::Lower => BoundDirection::Upper,
            BoundDirection::Upper => BoundDirection::Lower,
        }
    };
    let log_two_bound =
        log_reduced_rational_bound(&rational_integer(2), precision_bits, log_two_direction)?;
    Ok(reduced_bound.add(&scale_rational_by_i64(&log_two_bound, exponent_two)?))
}

fn reduce_log_argument_to_unit_range(value: &Rational) -> Result<(Rational, i64), IntervalError> {
    let mut reduced = value.clone();
    let mut exponent_two = 0_i64;
    let mut steps = 0_u32;
    while compare_positive_rational_to_two(&reduced) != Ordering::Less {
        guard_log_range_reduction_step(&mut steps)?;
        reduced = halve_log_range_rational(&reduced);
        exponent_two = exponent_two
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
    }
    while compare_positive_rational_to_one(&reduced) == Ordering::Less {
        guard_log_range_reduction_step(&mut steps)?;
        reduced = double_log_range_rational(&reduced);
        exponent_two = exponent_two
            .checked_sub(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
    }
    Ok((reduced, exponent_two))
}

fn halve_log_range_rational(value: &Rational) -> Rational {
    debug_assert!(!value.is_negative() && !value.is_zero());
    let (numerator, denominator) = if value.numerator.inner.is_even() {
        (
            &value.numerator.inner >> 1_u8,
            value.denominator.inner.inner.clone(),
        )
    } else {
        (
            value.numerator.inner.clone(),
            &value.denominator.inner.inner << 1_u8,
        )
    };
    Rational {
        numerator: Integer::from_bigint(numerator),
        denominator: PositiveInteger {
            inner: Integer::from_bigint(denominator),
        },
    }
}

fn double_log_range_rational(value: &Rational) -> Rational {
    debug_assert!(!value.is_negative() && !value.is_zero());
    let (numerator, denominator) = if value.denominator.inner.inner.is_even() {
        (
            value.numerator.inner.clone(),
            &value.denominator.inner.inner >> 1_u8,
        )
    } else {
        (
            &value.numerator.inner << 1_u8,
            value.denominator.inner.inner.clone(),
        )
    };
    Rational {
        numerator: Integer::from_bigint(numerator),
        denominator: PositiveInteger {
            inner: Integer::from_bigint(denominator),
        },
    }
}

fn compare_positive_rational_to_one(value: &Rational) -> Ordering {
    debug_assert!(!value.is_negative() && !value.is_zero());
    value
        .numerator
        .inner
        .magnitude()
        .cmp(value.denominator.inner.inner.magnitude())
}

fn compare_positive_rational_to_two(value: &Rational) -> Ordering {
    debug_assert!(!value.is_negative() && !value.is_zero());
    value
        .numerator
        .inner
        .cmp(&(&value.denominator.inner.inner * 2_u8))
}

fn log_reduced_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Less);
    debug_assert!(compare_rationals(value, &rational_integer(2)) != Ordering::Greater);
    if is_positive_one_rational(value) {
        return Ok((Rational::zero(), Rational::zero()));
    }
    let z = log_series_argument(value)?;
    let term_count = log_series_terms(precision_bits)?;
    log_reduced_rational_bounds_with_argument_and_terms(&z, term_count)
}

fn log_reduced_rational_bounds_with_terms(
    value: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(compare_positive_rational_to_one(value) != Ordering::Less);
    debug_assert!(compare_positive_rational_to_two(value) != Ordering::Greater);
    if is_positive_one_rational(value) {
        return Ok((Rational::zero(), Rational::zero()));
    }
    let z = log_series_argument(value)?;
    log_reduced_rational_bounds_with_argument_and_terms(&z, term_count)
}

fn log_reduced_rational_bounds_with_argument_and_terms(
    z: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    log_series_common_denominator_bounds(z, term_count)
}

#[cfg(test)]
fn log_reduced_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    debug_assert!(compare_positive_rational_to_one(value) != Ordering::Less);
    debug_assert!(compare_positive_rational_to_two(value) != Ordering::Greater);
    if is_positive_one_rational(value) {
        return Ok(Rational::zero());
    }
    let z = log_series_argument(value)?;
    let term_count = log_series_terms(precision_bits)?;
    log_reduced_rational_bound_with_argument_and_terms(&z, term_count, direction)
}

fn log_reduced_rational_bound_with_terms(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    debug_assert!(compare_positive_rational_to_one(value) != Ordering::Less);
    debug_assert!(compare_positive_rational_to_two(value) != Ordering::Greater);
    if is_positive_one_rational(value) {
        return Ok(Rational::zero());
    }
    let z = log_series_argument(value)?;
    log_reduced_rational_bound_with_argument_and_terms(&z, term_count, direction)
}

fn log_reduced_rational_bound_with_argument_and_terms(
    z: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    log_series_common_denominator_bound(z, term_count, direction)
}

fn log_series_argument(value: &Rational) -> Result<Rational, IntervalError> {
    let numerator = &value.numerator.inner - &value.denominator.inner.inner;
    let denominator = &value.numerator.inner + &value.denominator.inner.inner;
    rational_from_parts(numerator, denominator)
}

fn log_series_common_denominator_bounds(
    z: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    log_series_evaluation(z, term_count, true)?.into_bounds()
}

fn log_series_common_denominator_bound(
    z: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    let state = log_series_evaluation(z, term_count, matches!(direction, BoundDirection::Upper))?;
    match direction {
        BoundDirection::Lower => state.into_lower(),
        BoundDirection::Upper => state.into_upper(),
    }
}

enum LogSeriesEvaluation {
    Recurrence(LogSeriesState),
    BinarySplit(LogBinarySplitState),
}

impl LogSeriesEvaluation {
    fn into_lower(self) -> Result<Rational, IntervalError> {
        match self {
            Self::Recurrence(state) => state.into_lower(),
            Self::BinarySplit(state) => state.into_lower(),
        }
    }

    fn into_upper(self) -> Result<Rational, IntervalError> {
        match self {
            Self::Recurrence(state) => state.into_upper(),
            Self::BinarySplit(state) => state.into_upper(),
        }
    }

    fn into_bounds(self) -> Result<(Rational, Rational), IntervalError> {
        match self {
            Self::Recurrence(state) => state.into_bounds(),
            Self::BinarySplit(state) => state.into_bounds(),
        }
    }
}

struct LogSeriesState {
    sum_numerator: BigInt,
    term_numerator: BigInt,
    odd_product: BigInt,
    common_denominator: BigInt,
    numerator_squared: BigInt,
    denominator_squared: BigInt,
    term_count: u32,
}

impl LogSeriesState {
    fn into_lower(self) -> Result<Rational, IntervalError> {
        rational_from_parts(self.sum_numerator * 2_u8, self.common_denominator)
    }

    fn into_upper(self) -> Result<Rational, IntervalError> {
        let next_odd_denominator = self
            .term_count
            .checked_add(1)
            .and_then(|value| value.checked_mul(2))
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let next_term_numerator = self.term_numerator * self.numerator_squared;
        let next_denominator_factor = self.denominator_squared * next_odd_denominator;
        let upper_numerator = self.sum_numerator * &next_denominator_factor * 2_u8
            + next_term_numerator * self.odd_product * 4_u8;
        rational_from_parts(
            upper_numerator,
            self.common_denominator * next_denominator_factor,
        )
    }

    fn into_bounds(self) -> Result<(Rational, Rational), IntervalError> {
        let lower =
            rational_from_parts(&self.sum_numerator * 2_u8, self.common_denominator.clone())?;
        let upper = self.into_upper()?;
        Ok((lower, upper))
    }
}

fn log_series_evaluation(
    z: &Rational,
    term_count: u32,
    include_upper: bool,
) -> Result<LogSeriesEvaluation, IntervalError> {
    if !z.is_zero() && !z.numerator.inner.is_one() && term_count >= LOG_BINARY_SPLIT_THRESHOLD {
        return log_binary_split_state(z, term_count, include_upper)
            .map(LogSeriesEvaluation::BinarySplit);
    }
    log_series_recurrence_state(z, term_count).map(LogSeriesEvaluation::Recurrence)
}

fn log_series_recurrence_state(
    z: &Rational,
    term_count: u32,
) -> Result<LogSeriesState, IntervalError> {
    let value_numerator = &z.numerator.inner;
    let value_denominator = &z.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let mut sum_numerator = value_numerator.clone();
    let mut term_numerator = value_numerator.clone();
    let mut odd_product = BigInt::one();
    let mut common_denominator = value_denominator.clone();

    if value_numerator.is_one() {
        for k in 1..=term_count {
            let odd_denominator = k
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .ok_or(IntervalError::ExponentTooLarge)?;
            let denominator_factor = &denominator_squared * odd_denominator;
            sum_numerator *= &denominator_factor;
            sum_numerator += &odd_product;
            common_denominator *= denominator_factor;
            odd_product *= odd_denominator;
        }
    } else {
        for k in 1..=term_count {
            let odd_denominator = k
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .ok_or(IntervalError::ExponentTooLarge)?;
            term_numerator *= &numerator_squared;
            let denominator_factor = &denominator_squared * odd_denominator;
            sum_numerator *= &denominator_factor;
            sum_numerator += &term_numerator * &odd_product;
            common_denominator *= denominator_factor;
            odd_product *= odd_denominator;
        }
    }

    Ok(LogSeriesState {
        sum_numerator,
        term_numerator,
        odd_product,
        common_denominator,
        numerator_squared,
        denominator_squared,
        term_count,
    })
}

struct LogBinarySplitState {
    sum_numerator: BigInt,
    common_denominator: BigInt,
    last_product_numerator: Option<BigInt>,
    value_numerator: BigInt,
    numerator_squared: BigInt,
    denominator_squared: BigInt,
    term_count: u32,
}

impl LogBinarySplitState {
    fn into_lower(self) -> Result<Rational, IntervalError> {
        rational_from_parts(self.sum_numerator * 2_u8, self.common_denominator)
    }

    fn into_upper(self) -> Result<Rational, IntervalError> {
        let last_product_numerator = self
            .last_product_numerator
            .expect("upper log split retains the final term product");
        let final_odd = self
            .term_count
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let next_odd = final_odd
            .checked_add(2)
            .ok_or(IntervalError::ExponentTooLarge)?;
        let next_denominator_factor = self.denominator_squared * next_odd;
        let next_term_scaled_numerator =
            self.value_numerator * last_product_numerator * self.numerator_squared * final_odd;
        let upper_numerator = self.sum_numerator * &next_denominator_factor * 2_u8
            + next_term_scaled_numerator * 4_u8;
        rational_from_parts(
            upper_numerator,
            self.common_denominator * next_denominator_factor,
        )
    }

    fn into_bounds(self) -> Result<(Rational, Rational), IntervalError> {
        let lower =
            rational_from_parts(&self.sum_numerator * 2_u8, self.common_denominator.clone())?;
        let upper = self.into_upper()?;
        Ok((lower, upper))
    }
}

struct LogBinarySplit {
    // For a segment of recurrence indices, P/Q is the product of all term
    // ratios and T/Q is the sum of the segment's cumulative products.
    product_numerator: Option<BigInt>,
    product_denominator: BigInt,
    sum_numerator: BigInt,
}

fn log_binary_split_state(
    z: &Rational,
    term_count: u32,
    include_upper: bool,
) -> Result<LogBinarySplitState, IntervalError> {
    debug_assert!(!z.numerator.inner.is_one());
    debug_assert!(term_count > 0);
    let numerator = &z.numerator.inner;
    let denominator = &z.denominator.inner.inner;
    let numerator_squared = numerator * numerator;
    let denominator_squared = denominator * denominator;
    let split = log_binary_split(
        &numerator_squared,
        &denominator_squared,
        1,
        term_count
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?,
        include_upper,
    )?;
    let sum_numerator = numerator * (&split.product_denominator + &split.sum_numerator);
    let common_denominator = denominator * &split.product_denominator;
    Ok(LogBinarySplitState {
        sum_numerator,
        common_denominator,
        last_product_numerator: split.product_numerator,
        value_numerator: numerator.clone(),
        numerator_squared,
        denominator_squared,
        term_count,
    })
}

fn log_binary_split(
    numerator_squared: &BigInt,
    denominator_squared: &BigInt,
    start: u32,
    end: u32,
    retain_product: bool,
) -> Result<LogBinarySplit, IntervalError> {
    debug_assert!(start < end);
    if end - start <= LOG_BINARY_SPLIT_LEAF_TERMS {
        let mut split =
            log_binary_split_leaf_block(numerator_squared, denominator_squared, start, end)?;
        if !retain_product {
            split.product_numerator = None;
        }
        return Ok(split);
    }
    let middle = start + (end - start) / 2;
    let mut left = log_binary_split(numerator_squared, denominator_squared, start, middle, true)?;
    let mut right = log_binary_split(numerator_squared, denominator_squared, middle, end, true)?;
    let left_product = left
        .product_numerator
        .take()
        .expect("internal log split retains its product");
    let right_product = right
        .product_numerator
        .take()
        .expect("internal log split retains its product");
    left.sum_numerator *= &right.product_denominator;
    right.sum_numerator *= &left_product;
    left.sum_numerator += right.sum_numerator;
    left.product_numerator = retain_product.then(|| left_product * right_product);
    left.product_denominator *= right.product_denominator;
    Ok(left)
}

fn log_binary_split_leaf_block(
    numerator_squared: &BigInt,
    denominator_squared: &BigInt,
    start: u32,
    end: u32,
) -> Result<LogBinarySplit, IntervalError> {
    let mut product_numerator = BigInt::one();
    let mut product_denominator = BigInt::one();
    let mut sum_numerator = BigInt::zero();
    for index in start..end {
        let odd_before = index
            .checked_mul(2)
            .and_then(|value| value.checked_sub(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let odd_after = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let step_numerator = numerator_squared * odd_before;
        let step_denominator = denominator_squared * odd_after;
        sum_numerator *= &step_denominator;
        sum_numerator += &product_numerator * &step_numerator;
        product_numerator *= step_numerator;
        product_denominator *= step_denominator;
    }
    Ok(LogBinarySplit {
        product_numerator: Some(product_numerator),
        product_denominator,
        sum_numerator,
    })
}

fn atan_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    if value.is_negative() {
        let (lower, upper) = atan_nonnegative_rational_bounds(&value.negate(), precision_bits)?;
        return Ok((upper.negate(), lower.negate()));
    }
    atan_nonnegative_rational_bounds(value, precision_bits)
}

fn atan_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    atan_rational_bound_with_pi(value, precision_bits, direction, None)
}

fn atan_rational_bound_with_pi(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<Rational, IntervalError> {
    if value.is_negative() {
        let positive_direction = match direction {
            BoundDirection::Lower => BoundDirection::Upper,
            BoundDirection::Upper => BoundDirection::Lower,
        };
        return Ok(atan_nonnegative_rational_bound_with_pi(
            &value.negate(),
            precision_bits,
            positive_direction,
            shared_pi,
        )?
        .negate());
    }
    atan_nonnegative_rational_bound_with_pi(value, precision_bits, direction, shared_pi)
}

fn atan_nonnegative_rational_bound_with_pi(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<Rational, IntervalError> {
    debug_assert!(!value.is_negative());
    if value.is_zero() {
        return Ok(Rational::zero());
    }
    if !is_unit_rational(value) {
        let reciprocal = reciprocal_nonzero_rational(value)?;
        let reciprocal_direction = match direction {
            BoundDirection::Lower => BoundDirection::Upper,
            BoundDirection::Upper => BoundDirection::Lower,
        };
        let reciprocal_bound =
            atan_unit_rational_bound(&reciprocal, precision_bits, reciprocal_direction)?;
        let owned_pi_bound;
        let selected_pi = match (shared_pi, direction) {
            (Some((pi_lower, _)), BoundDirection::Lower) => pi_lower,
            (Some((_, pi_upper)), BoundDirection::Upper) => pi_upper,
            (None, _) => {
                owned_pi_bound = pi_bound(precision_bits, direction)?;
                &owned_pi_bound
            }
        };
        return Ok(halve_rational(selected_pi)?.subtract(&reciprocal_bound));
    }
    atan_unit_rational_bound(value, precision_bits, direction)
}

fn atan_unit_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(is_unit_rational(value));
    atan_series_common_denominator_bound(value, series_terms(precision_bits)?, direction)
}

fn atan_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    if value.is_zero() {
        return Ok((Rational::zero(), Rational::zero()));
    }
    if !is_unit_rational(value) {
        let reciprocal = reciprocal_nonzero_rational(value)?;
        let (reciprocal_lower, reciprocal_upper) =
            atan_unit_rational_bounds(&reciprocal, precision_bits)?;
        let (pi_lower, pi_upper) = pi_bounds(precision_bits)?;
        return Ok((
            halve_rational(&pi_lower)?.subtract(&reciprocal_upper),
            halve_rational(&pi_upper)?.subtract(&reciprocal_lower),
        ));
    }
    atan_unit_rational_bounds(value, precision_bits)
}

fn atan_unit_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(is_unit_rational(value));
    let term_count = series_terms(precision_bits)?;
    atan_series_common_denominator_bounds(value, term_count)
}

fn atan_series_common_denominator_bounds(
    value: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    if value.is_zero() {
        return Ok((Rational::zero(), Rational::zero()));
    }
    if atan_series_uses_binary_split(value, term_count) {
        return atan_binary_split_state(value, term_count, true)?.into_bounds();
    }
    atan_series_recurrence_bounds(value, term_count)
}

fn atan_series_recurrence_bounds(
    value: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let mut sum_numerator = value_numerator.clone();
    let mut term_numerator = value_numerator.clone();
    let mut odd_product = BigInt::one();
    let mut common_denominator = value_denominator.clone();
    for k in 1..=term_count {
        let odd_denominator = k
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        term_numerator *= &numerator_squared;
        let denominator_factor = &denominator_squared * odd_denominator;
        sum_numerator *= &denominator_factor;
        let correction = &term_numerator * &odd_product;
        if k.is_multiple_of(2) {
            sum_numerator += correction;
        } else {
            sum_numerator -= correction;
        }
        common_denominator *= denominator_factor;
        odd_product *= odd_denominator;
    }
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next_odd_denominator = next_index
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next_term_numerator = term_numerator * numerator_squared;
    let next_denominator_factor = denominator_squared * next_odd_denominator;
    let sum = rational_from_parts(sum_numerator.clone(), common_denominator.clone())?;
    sum_numerator *= &next_denominator_factor;
    let next_correction = next_term_numerator * odd_product;
    if next_index.is_multiple_of(2) {
        sum_numerator += next_correction;
    } else {
        sum_numerator -= next_correction;
    }
    let adjacent =
        rational_from_parts(sum_numerator, common_denominator * next_denominator_factor)?;
    if compare_rationals(&sum, &adjacent) == Ordering::Less {
        Ok((sum, adjacent))
    } else {
        Ok((adjacent, sum))
    }
}

fn atan_series_common_denominator_bound(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    if value.is_zero() {
        return Ok(Rational::zero());
    }
    let use_sum = atan_series_sum_is_bound(term_count, direction)?;
    if atan_series_uses_binary_split(value, term_count) {
        return atan_binary_split_state(value, term_count, !use_sum)?.into_bound(direction);
    }
    atan_series_recurrence_bound(value, term_count, direction)
}

fn atan_series_recurrence_bound(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let mut sum_numerator = value_numerator.clone();
    let mut term_numerator = value_numerator.clone();
    let mut odd_product = BigInt::one();
    let mut common_denominator = value_denominator.clone();
    for k in 1..=term_count {
        let odd_denominator = k
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        term_numerator *= &numerator_squared;
        let denominator_factor = &denominator_squared * odd_denominator;
        sum_numerator *= &denominator_factor;
        let correction = &term_numerator * &odd_product;
        if k.is_multiple_of(2) {
            sum_numerator += correction;
        } else {
            sum_numerator -= correction;
        }
        common_denominator *= denominator_factor;
        odd_product *= odd_denominator;
    }
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let use_sum = atan_series_sum_is_bound(term_count, direction)?;
    if use_sum {
        return rational_from_parts(sum_numerator, common_denominator);
    }
    let next_odd_denominator = next_index
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next_term_numerator = term_numerator * numerator_squared;
    let next_denominator_factor = denominator_squared * next_odd_denominator;
    sum_numerator *= &next_denominator_factor;
    let next_correction = next_term_numerator * odd_product;
    if next_index.is_multiple_of(2) {
        sum_numerator += next_correction;
    } else {
        sum_numerator -= next_correction;
    }
    rational_from_parts(sum_numerator, common_denominator * next_denominator_factor)
}

fn atan_series_uses_binary_split(value: &Rational, term_count: u32) -> bool {
    !value.is_zero() && !value.numerator.inner.is_one() && term_count >= ATAN_BINARY_SPLIT_THRESHOLD
}

fn atan_series_sum_is_bound(
    term_count: u32,
    direction: BoundDirection,
) -> Result<bool, IntervalError> {
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let sum_is_lower = next_index.is_multiple_of(2);
    Ok(match direction {
        BoundDirection::Lower => sum_is_lower,
        BoundDirection::Upper => !sum_is_lower,
    })
}

struct AtanBinarySplitState {
    sum_numerator: BigInt,
    common_denominator: BigInt,
    last_product_numerator: Option<BigInt>,
    value_numerator: BigInt,
    numerator_squared: BigInt,
    denominator_squared: BigInt,
    term_count: u32,
}

impl AtanBinarySplitState {
    fn into_bound(self, direction: BoundDirection) -> Result<Rational, IntervalError> {
        if atan_series_sum_is_bound(self.term_count, direction)? {
            rational_from_parts(self.sum_numerator, self.common_denominator)
        } else {
            self.into_adjacent()
        }
    }

    fn into_bounds(self) -> Result<(Rational, Rational), IntervalError> {
        let sum = rational_from_parts(self.sum_numerator.clone(), self.common_denominator.clone())?;
        let adjacent = self.into_adjacent()?;
        if compare_rationals(&sum, &adjacent) == Ordering::Less {
            Ok((sum, adjacent))
        } else {
            Ok((adjacent, sum))
        }
    }

    fn into_adjacent(self) -> Result<Rational, IntervalError> {
        let last_product_numerator = self
            .last_product_numerator
            .expect("adjacent atan split retains the final term product");
        let final_odd = self
            .term_count
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let next_odd = final_odd
            .checked_add(2)
            .ok_or(IntervalError::ExponentTooLarge)?;
        let next_denominator_factor = self.denominator_squared * next_odd;
        let next_term_scaled_numerator =
            -(self.value_numerator * last_product_numerator * self.numerator_squared * final_odd);
        let adjacent_numerator =
            self.sum_numerator * &next_denominator_factor + next_term_scaled_numerator;
        rational_from_parts(
            adjacent_numerator,
            self.common_denominator * next_denominator_factor,
        )
    }
}

struct AtanBinarySplit {
    product_numerator: Option<BigInt>,
    product_denominator: BigInt,
    sum_numerator: BigInt,
}

fn atan_binary_split_state(
    value: &Rational,
    term_count: u32,
    include_adjacent: bool,
) -> Result<AtanBinarySplitState, IntervalError> {
    debug_assert!(!value.numerator.inner.is_one());
    debug_assert!(term_count > 0);
    let numerator = &value.numerator.inner;
    let denominator = &value.denominator.inner.inner;
    let numerator_squared = numerator * numerator;
    let denominator_squared = denominator * denominator;
    let split = atan_binary_split(
        &numerator_squared,
        &denominator_squared,
        1,
        term_count
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?,
        include_adjacent,
    )?;
    let sum_numerator = numerator * (&split.product_denominator + &split.sum_numerator);
    let common_denominator = denominator * &split.product_denominator;
    Ok(AtanBinarySplitState {
        sum_numerator,
        common_denominator,
        last_product_numerator: split.product_numerator,
        value_numerator: numerator.clone(),
        numerator_squared,
        denominator_squared,
        term_count,
    })
}

fn atan_binary_split(
    numerator_squared: &BigInt,
    denominator_squared: &BigInt,
    start: u32,
    end: u32,
    retain_product: bool,
) -> Result<AtanBinarySplit, IntervalError> {
    debug_assert!(start < end);
    if end - start <= ATAN_BINARY_SPLIT_LEAF_TERMS {
        let mut split =
            atan_binary_split_leaf_block(numerator_squared, denominator_squared, start, end)?;
        if !retain_product {
            split.product_numerator = None;
        }
        return Ok(split);
    }
    let middle = start + (end - start) / 2;
    let mut left = atan_binary_split(numerator_squared, denominator_squared, start, middle, true)?;
    let mut right = atan_binary_split(numerator_squared, denominator_squared, middle, end, true)?;
    let left_product = left
        .product_numerator
        .take()
        .expect("internal atan split retains its product");
    let right_product = right
        .product_numerator
        .take()
        .expect("internal atan split retains its product");
    left.sum_numerator *= &right.product_denominator;
    right.sum_numerator *= &left_product;
    left.sum_numerator += right.sum_numerator;
    left.product_numerator = retain_product.then(|| left_product * right_product);
    left.product_denominator *= right.product_denominator;
    Ok(left)
}

fn atan_binary_split_leaf_block(
    numerator_squared: &BigInt,
    denominator_squared: &BigInt,
    start: u32,
    end: u32,
) -> Result<AtanBinarySplit, IntervalError> {
    let mut product_numerator = BigInt::one();
    let mut product_denominator = BigInt::one();
    let mut sum_numerator = BigInt::zero();
    for index in start..end {
        let odd_before = index
            .checked_mul(2)
            .and_then(|value| value.checked_sub(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let odd_after = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let step_numerator = -(numerator_squared * odd_before);
        let step_denominator = denominator_squared * odd_after;
        sum_numerator *= &step_denominator;
        sum_numerator += &product_numerator * &step_numerator;
        product_numerator *= step_numerator;
        product_denominator *= step_denominator;
    }
    Ok(AtanBinarySplit {
        product_numerator: Some(product_numerator),
        product_denominator,
        sum_numerator,
    })
}

fn inverse_sine_cosine_domain_bounds(
    value: &CertifiedInterval,
) -> Result<(Rational, Rational), IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let lower_outside_unit = !is_unit_rational(&lower);
    let upper_outside_unit = !is_unit_rational(&upper);
    if (upper_outside_unit && upper.is_negative()) || (lower_outside_unit && !lower.is_negative()) {
        return Err(IntervalError::Domain(
            DomainErrorKind::InverseTrigonometricOutOfRange,
        ));
    }
    if lower_outside_unit || upper_outside_unit {
        return Err(IntervalError::UnsupportedExpression);
    }
    Ok((lower, upper))
}

fn asin_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    asin_rational_bounds_with_pi(value, precision_bits, None)
}

fn asin_rational_bounds_with_pi(
    value: &Rational,
    precision_bits: u32,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<(Rational, Rational), IntervalError> {
    if is_negative_one_rational(value) {
        let owned_pi;
        let (pi_lower, pi_upper) = match shared_pi {
            Some(pi) => pi,
            None => {
                owned_pi = pi_bounds(precision_bits)?;
                &owned_pi
            }
        };
        return Ok((
            halve_rational(pi_upper)?.negate(),
            halve_rational(pi_lower)?.negate(),
        ));
    }
    if is_positive_one_rational(value) {
        let owned_pi;
        let (pi_lower, pi_upper) = match shared_pi {
            Some(pi) => pi,
            None => {
                owned_pi = pi_bounds(precision_bits)?;
                &owned_pi
            }
        };
        return Ok((halve_rational(pi_lower)?, halve_rational(pi_upper)?));
    }
    if value.is_zero() {
        return Ok((Rational::zero(), Rational::zero()));
    }
    if value.is_negative() {
        let (lower, upper) =
            asin_positive_rational_bounds_with_pi(&value.negate(), precision_bits, shared_pi)?;
        return Ok((upper.negate(), lower.negate()));
    }
    asin_positive_rational_bounds_with_pi(value, precision_bits, shared_pi)
}

#[cfg(test)]
fn asin_rational_bound(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    asin_rational_bound_with_pi(value, precision_bits, direction, None)
}

fn asin_rational_bound_with_pi(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<Rational, IntervalError> {
    if value.is_negative() {
        let positive_direction = match direction {
            BoundDirection::Lower => BoundDirection::Upper,
            BoundDirection::Upper => BoundDirection::Lower,
        };
        return Ok(asin_rational_bound_with_pi(
            &value.negate(),
            precision_bits,
            positive_direction,
            shared_pi,
        )?
        .negate());
    }
    if value.is_zero() {
        return Ok(Rational::zero());
    }
    if compare_nonnegative_rational_to_half(value) != Ordering::Greater {
        return asin_common_denominator_bound(value, series_terms(precision_bits)?, direction);
    }
    let one_minus_square = one_minus_rational_square(value)?;
    let numerator = match direction {
        BoundDirection::Lower => sqrt_rational_upper(&one_minus_square, precision_bits)?,
        BoundDirection::Upper => sqrt_rational_lower(&one_minus_square, precision_bits)?,
    };
    if rational_square_is_below_half(value) {
        let ratio = divide_rational(value, &dyadic_to_rational(&numerator)?)?;
        return atan_rational_bound(&ratio, precision_bits, direction);
    }
    let ratio = divide_rational(&dyadic_to_rational(&numerator)?, value)?;
    let atan_direction = match direction {
        BoundDirection::Lower => BoundDirection::Upper,
        BoundDirection::Upper => BoundDirection::Lower,
    };
    let atan_bound = atan_rational_bound(&ratio, precision_bits, atan_direction)?;
    let owned_pi_bound;
    let selected_pi = match (shared_pi, direction) {
        (Some((pi_lower, _)), BoundDirection::Lower) => pi_lower,
        (Some((_, pi_upper)), BoundDirection::Upper) => pi_upper,
        (None, _) => {
            owned_pi_bound = pi_bound(precision_bits, direction)?;
            &owned_pi_bound
        }
    };
    Ok(halve_rational(selected_pi)?.subtract(&atan_bound))
}

fn asin_positive_rational_bounds_with_pi(
    value: &Rational,
    precision_bits: u32,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(!value.is_zero());
    debug_assert!(compare_rationals(value, &Rational::one()) == Ordering::Less);

    if compare_nonnegative_rational_to_half(value) != Ordering::Greater {
        return asin_unit_rational_bounds(value, precision_bits);
    }

    let one_minus_square = one_minus_rational_square(value)?;
    let numerator = nth_root_nonnegative_rational(&one_minus_square, 2, precision_bits)?;
    if rational_square_is_below_half(value) {
        let denominator_lower = dyadic_to_rational(&numerator.lower)?;
        let denominator_upper = dyadic_to_rational(&numerator.upper)?;
        let ratio_lower = divide_rational(value, &denominator_upper)?;
        let ratio_upper = divide_rational(value, &denominator_lower)?;
        return Ok((
            atan_rational_bound(&ratio_lower, precision_bits, BoundDirection::Lower)?,
            atan_rational_bound(&ratio_upper, precision_bits, BoundDirection::Upper)?,
        ));
    }
    let ratio_lower = divide_rational(&dyadic_to_rational(&numerator.lower)?, value)?;
    let ratio_upper = divide_rational(&dyadic_to_rational(&numerator.upper)?, value)?;
    let (atan_lower, atan_upper) = (
        atan_rational_bound(&ratio_lower, precision_bits, BoundDirection::Lower)?,
        atan_rational_bound(&ratio_upper, precision_bits, BoundDirection::Upper)?,
    );
    let owned_pi;
    let pi = match shared_pi {
        Some(pi) => pi,
        None => {
            owned_pi = pi_bounds(precision_bits)?;
            &owned_pi
        }
    };
    Ok((
        halve_rational(&pi.0)?.subtract(&atan_upper),
        halve_rational(&pi.1)?.subtract(&atan_lower),
    ))
}

fn rational_square_is_below_half(value: &Rational) -> bool {
    debug_assert!(!value.is_negative());
    let numerator_squared = &value.numerator.inner * &value.numerator.inner;
    let denominator_squared = &value.denominator.inner.inner * &value.denominator.inner.inner;
    numerator_squared * 2_u8 < denominator_squared
}

fn one_minus_rational_square(value: &Rational) -> Result<Rational, IntervalError> {
    let numerator_squared = &value.numerator.inner * &value.numerator.inner;
    let denominator_squared = &value.denominator.inner.inner * &value.denominator.inner.inner;
    let numerator = &denominator_squared - numerator_squared;
    if numerator.is_zero() {
        return Ok(Rational::zero());
    }
    Ok(Rational {
        numerator: Integer::from_bigint(numerator),
        denominator: PositiveInteger {
            inner: Integer::from_bigint(denominator_squared),
        },
    })
}

fn asin_unit_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(compare_nonnegative_rational_to_half(value) != Ordering::Greater);
    let term_count = series_terms(precision_bits)?;
    asin_common_denominator_bounds(value, term_count)
}

fn asin_common_denominator_bounds(
    value: &Rational,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let mut sum_numerator = value_numerator.clone();
    let mut term_numerator = value_numerator.clone();
    let mut common_denominator = value_denominator.clone();
    for index in 1..=term_count {
        let (numerator_factor, denominator_factor) =
            asin_term_factors(index, &numerator_squared, &denominator_squared)?;
        term_numerator *= numerator_factor;
        sum_numerator *= &denominator_factor;
        sum_numerator += &term_numerator;
        common_denominator *= denominator_factor;
    }
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let (next_numerator_factor, next_denominator_factor) =
        asin_term_factors(next_index, &numerator_squared, &denominator_squared)?;
    term_numerator *= next_numerator_factor;
    let lower = rational_from_parts(sum_numerator.clone(), common_denominator.clone())?;
    sum_numerator *= &next_denominator_factor;
    sum_numerator += term_numerator * 2_u8;
    let upper = rational_from_parts(sum_numerator, common_denominator * next_denominator_factor)?;
    Ok((lower, upper))
}

fn asin_common_denominator_bound(
    value: &Rational,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let mut sum_numerator = value_numerator.clone();
    let mut term_numerator = value_numerator.clone();
    let mut common_denominator = value_denominator.clone();
    for index in 1..=term_count {
        let (numerator_factor, denominator_factor) =
            asin_term_factors(index, &numerator_squared, &denominator_squared)?;
        term_numerator *= numerator_factor;
        sum_numerator *= &denominator_factor;
        sum_numerator += &term_numerator;
        common_denominator *= denominator_factor;
    }
    if matches!(direction, BoundDirection::Lower) {
        return rational_from_parts(sum_numerator, common_denominator);
    }
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let (next_numerator_factor, next_denominator_factor) =
        asin_term_factors(next_index, &numerator_squared, &denominator_squared)?;
    term_numerator *= next_numerator_factor;
    sum_numerator *= &next_denominator_factor;
    sum_numerator += term_numerator * 2_u8;
    rational_from_parts(sum_numerator, common_denominator * next_denominator_factor)
}

fn asin_term_factors(
    index: u32,
    numerator_squared: &BigInt,
    denominator_squared: &BigInt,
) -> Result<(BigInt, BigInt), IntervalError> {
    let doubled = index
        .checked_mul(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let odd = doubled
        .checked_sub(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next_odd = doubled
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    Ok((
        (numerator_squared * odd) * odd,
        (denominator_squared * doubled) * next_odd,
    ))
}

fn acos_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    acos_rational_bounds_with_pi(value, precision_bits, None)
}

fn acos_rational_bounds_with_pi(
    value: &Rational,
    precision_bits: u32,
    shared_pi: Option<&(Rational, Rational)>,
) -> Result<(Rational, Rational), IntervalError> {
    if is_negative_one_rational(value) {
        return match shared_pi {
            Some((pi_lower, pi_upper)) => Ok((pi_lower.clone(), pi_upper.clone())),
            None => pi_bounds(precision_bits),
        };
    }
    if is_positive_one_rational(value) {
        return Ok((Rational::zero(), Rational::zero()));
    }
    if value.is_zero() {
        let owned_pi;
        let (pi_lower, pi_upper) = match shared_pi {
            Some(pi) => pi,
            None => {
                owned_pi = pi_bounds(precision_bits)?;
                &owned_pi
            }
        };
        return Ok((halve_rational(pi_lower)?, halve_rational(pi_upper)?));
    }

    let owned_pi;
    let pi = match shared_pi {
        Some(pi) => pi,
        None => {
            owned_pi = pi_bounds(precision_bits)?;
            &owned_pi
        }
    };
    let (asin_lower, asin_upper) = asin_rational_bounds_with_pi(value, precision_bits, Some(pi))?;
    Ok((
        halve_rational(&pi.0)?.subtract(&asin_upper),
        halve_rational(&pi.1)?.subtract(&asin_lower),
    ))
}

fn acos_rational_bound_with_pi(
    value: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
    pi: &(Rational, Rational),
) -> Result<Rational, IntervalError> {
    if is_negative_one_rational(value) {
        return Ok(match direction {
            BoundDirection::Lower => pi.0.clone(),
            BoundDirection::Upper => pi.1.clone(),
        });
    }
    if is_positive_one_rational(value) {
        return Ok(Rational::zero());
    }
    if value.is_zero() {
        return match direction {
            BoundDirection::Lower => halve_rational(&pi.0),
            BoundDirection::Upper => halve_rational(&pi.1),
        };
    }
    let asin_direction = match direction {
        BoundDirection::Lower => BoundDirection::Upper,
        BoundDirection::Upper => BoundDirection::Lower,
    };
    let asin_bound = asin_rational_bound_with_pi(value, precision_bits, asin_direction, Some(pi))?;
    let pi_bound = match direction {
        BoundDirection::Lower => &pi.0,
        BoundDirection::Upper => &pi.1,
    };
    Ok(halve_rational(pi_bound)?.subtract(&asin_bound))
}

fn sin_cos_rational(
    value: &Rational,
    precision_bits: u32,
) -> Result<(CertifiedInterval, CertifiedInterval), IntervalError> {
    let divisor = ceil_absolute_rational_to_u32(value)?;
    let reduced_storage;
    let reduced = if divisor == 1 {
        value
    } else {
        reduced_storage = divide_rational_by_positive_u32(value, divisor)?;
        &reduced_storage
    };
    let (sin_lower, sin_upper) = sin_unit_rational_bounds(reduced, precision_bits)?;
    let (cos_lower, cos_upper) = cos_unit_rational_bounds(reduced, precision_bits)?;
    let mut factor = TrigPair {
        cosine: from_rational_bounds(&cos_lower, &cos_upper, precision_bits)?,
        sine: from_rational_bounds(&sin_lower, &sin_upper, precision_bits)?,
    };
    if divisor == 1 {
        return Ok((factor.sine, factor.cosine));
    }
    let mut result = None;
    let mut remaining = divisor;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result = Some(match result {
                Some(result) => multiply_trig_pairs(&result, &factor, precision_bits)?,
                None => factor.clone(),
            });
        }
        remaining >>= 1;
        if remaining > 0 {
            factor = multiply_trig_pairs(&factor, &factor, precision_bits)?;
        }
    }
    let result = result.ok_or(IntervalError::InvalidBounds)?;
    Ok((result.sine, result.cosine))
}

#[derive(Clone)]
struct TrigPair {
    cosine: CertifiedInterval,
    sine: CertifiedInterval,
}

fn multiply_trig_pairs(
    left: &TrigPair,
    right: &TrigPair,
    precision_bits: u32,
) -> Result<TrigPair, IntervalError> {
    let cos_cos = multiply(&left.cosine, &right.cosine)?;
    let sin_sin = multiply(&left.sine, &right.sine)?;
    let cos_sin = multiply(&left.cosine, &right.sine)?;
    let sin_cos = multiply(&left.sine, &right.cosine)?;
    Ok(TrigPair {
        cosine: clamp_trigonometric_interval(&subtract(&cos_cos, &sin_sin)?, precision_bits)?,
        sine: clamp_trigonometric_interval(&add(&cos_sin, &sin_cos)?, precision_bits)?,
    })
}

fn sin_unit_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    if value.is_negative() {
        let (lower, upper) = sin_unit_rational_bounds(&value.negate(), precision_bits)?;
        return Ok((upper.negate(), lower.negate()));
    }
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Greater);
    let term_count = trigonometric_series_terms(precision_bits)?;
    trigonometric_series_common_denominator_bounds(value, term_count, TrigonometricSeries::Sine)
}

fn cos_unit_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let positive_storage;
    let positive = if value.is_negative() {
        positive_storage = value.negate();
        &positive_storage
    } else {
        value
    };
    debug_assert!(compare_rationals(positive, &Rational::one()) != Ordering::Greater);
    let term_count = trigonometric_series_terms(precision_bits)?;
    trigonometric_series_common_denominator_bounds(
        positive,
        term_count,
        TrigonometricSeries::Cosine,
    )
}

#[derive(Clone, Copy)]
enum TrigonometricSeries {
    Sine,
    Cosine,
}

fn trigonometric_series_common_denominator_bounds(
    value: &Rational,
    term_count: u32,
    series: TrigonometricSeries,
) -> Result<(Rational, Rational), IntervalError> {
    let value_numerator = &value.numerator.inner;
    let value_denominator = &value.denominator.inner.inner;
    let numerator_squared = value_numerator * value_numerator;
    let denominator_squared = value_denominator * value_denominator;
    let (mut sum_numerator, mut term_numerator, mut common_denominator) = match series {
        TrigonometricSeries::Sine => (
            value_numerator.clone(),
            value_numerator.clone(),
            value_denominator.clone(),
        ),
        TrigonometricSeries::Cosine => (BigInt::one(), BigInt::one(), BigInt::one()),
    };
    for index in 1..=term_count {
        let denominator_factor =
            trigonometric_term_denominator_factor(index, series, &denominator_squared)?;
        term_numerator *= &numerator_squared;
        sum_numerator *= &denominator_factor;
        if index.is_multiple_of(2) {
            sum_numerator += &term_numerator;
        } else {
            sum_numerator -= &term_numerator;
        }
        common_denominator *= denominator_factor;
    }
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next_denominator_factor =
        trigonometric_term_denominator_factor(next_index, series, &denominator_squared)?;
    term_numerator *= numerator_squared;
    let sum = rational_from_parts(sum_numerator.clone(), common_denominator.clone())?;
    sum_numerator *= &next_denominator_factor;
    if next_index.is_multiple_of(2) {
        sum_numerator += term_numerator;
    } else {
        sum_numerator -= term_numerator;
    }
    let adjacent =
        rational_from_parts(sum_numerator, common_denominator * next_denominator_factor)?;
    ordered_rational_bounds(sum, adjacent)
}

fn trigonometric_term_denominator_factor(
    index: u32,
    series: TrigonometricSeries,
    denominator_squared: &BigInt,
) -> Result<BigInt, IntervalError> {
    let doubled = index
        .checked_mul(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let first = match series {
        TrigonometricSeries::Sine => doubled,
        TrigonometricSeries::Cosine => doubled
            .checked_sub(1)
            .ok_or(IntervalError::ExponentTooLarge)?,
    };
    let second = match series {
        TrigonometricSeries::Sine => doubled
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?,
        TrigonometricSeries::Cosine => doubled,
    };
    Ok((denominator_squared * first) * second)
}

fn ordered_rational_bounds(
    left: Rational,
    right: Rational,
) -> Result<(Rational, Rational), IntervalError> {
    if compare_rationals(&left, &right) == Ordering::Greater {
        Ok((right, left))
    } else {
        Ok((left, right))
    }
}

fn covers_full_trigonometric_period(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<bool, IntervalError> {
    if compare_rationals(lower, upper) == Ordering::Greater {
        return Err(IntervalError::InvalidBounds);
    }
    let width = upper.subtract(lower);
    let (_, pi_upper) = pi_bounds(precision_bits)?;
    Ok(compare_rationals(&width, &scale_rational(&pi_upper, 2)) != Ordering::Less)
}

fn bounded_trigonometric_endpoint_bounds(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
    evaluator: fn(&Rational, u32) -> Result<CertifiedInterval, IntervalError>,
) -> Result<Option<(Rational, Rational)>, IntervalError> {
    match trigonometric_endpoint_bounds(lower, upper, precision_bits, evaluator) {
        Ok(bounds) => Ok(Some(bounds)),
        Err(IntervalError::UnsupportedExpression | IntervalError::ExponentTooLarge) => Ok(None),
        Err(error) => Err(error),
    }
}

fn trigonometric_endpoint_bounds(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
    evaluator: fn(&Rational, u32) -> Result<CertifiedInterval, IntervalError>,
) -> Result<(Rational, Rational), IntervalError> {
    if compare_rationals(lower, upper) == Ordering::Greater {
        return Err(IntervalError::InvalidBounds);
    }
    let lower_value = evaluator(lower, precision_bits)?;
    let upper_value = evaluator(upper, precision_bits)?;
    let mut result_lower = dyadic_to_rational(&lower_value.lower)?;
    let mut result_upper = dyadic_to_rational(&lower_value.upper)?;
    include_interval_bounds(&mut result_lower, &mut result_upper, &upper_value)?;
    Ok((result_lower, result_upper))
}

fn include_interval_bounds(
    lower: &mut Rational,
    upper: &mut Rational,
    value: &CertifiedInterval,
) -> Result<(), IntervalError> {
    include_rational_candidate(lower, upper, &dyadic_to_rational(&value.lower)?);
    include_rational_candidate(lower, upper, &dyadic_to_rational(&value.upper)?);
    Ok(())
}

fn include_rational_candidate(lower: &mut Rational, upper: &mut Rational, candidate: &Rational) {
    if compare_rationals(candidate, lower) == Ordering::Less {
        *lower = candidate.clone();
    }
    if compare_rationals(candidate, upper) == Ordering::Greater {
        *upper = candidate.clone();
    }
}

fn include_sine_extrema(
    lower: &Rational,
    upper: &Rational,
    result_lower: &mut Rational,
    result_upper: &mut Rational,
    precision_bits: u32,
) -> Result<bool, IntervalError> {
    let Some(limit) = half_pi_scan_limit(lower, upper, precision_bits)? else {
        return Ok(false);
    };
    for index in -limit..=limit {
        if index % 2 == 0 {
            continue;
        }
        match half_pi_multiple_containment(index, lower, upper, precision_bits)? {
            HalfPiContainment::ProvenInside => {
                if index.rem_euclid(4) == 1 {
                    include_rational_candidate(result_lower, result_upper, &Rational::one());
                } else {
                    include_rational_candidate(result_lower, result_upper, &rational_integer(-1));
                }
            }
            HalfPiContainment::ProvenOutside => {}
            HalfPiContainment::Uncertain => return Ok(false),
        }
    }
    Ok(true)
}

fn include_cosine_extrema(
    lower: &Rational,
    upper: &Rational,
    result_lower: &mut Rational,
    result_upper: &mut Rational,
    precision_bits: u32,
) -> Result<bool, IntervalError> {
    let Some(limit) = half_pi_scan_limit(lower, upper, precision_bits)? else {
        return Ok(false);
    };
    for index in -limit..=limit {
        if index % 2 != 0 {
            continue;
        }
        match half_pi_multiple_containment(index, lower, upper, precision_bits)? {
            HalfPiContainment::ProvenInside => {
                if index.rem_euclid(4) == 0 {
                    include_rational_candidate(result_lower, result_upper, &Rational::one());
                } else {
                    include_rational_candidate(result_lower, result_upper, &rational_integer(-1));
                }
            }
            HalfPiContainment::ProvenOutside => {}
            HalfPiContainment::Uncertain => return Ok(false),
        }
    }
    Ok(true)
}

fn contains_possible_tangent_pole(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<bool, IntervalError> {
    let Some(limit) = half_pi_scan_limit(lower, upper, precision_bits)? else {
        return Ok(true);
    };
    for index in -limit..=limit {
        if index % 2 == 0 {
            continue;
        }
        match half_pi_multiple_containment(index, lower, upper, precision_bits)? {
            HalfPiContainment::ProvenOutside => {}
            HalfPiContainment::ProvenInside | HalfPiContainment::Uncertain => return Ok(true),
        }
    }
    Ok(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HalfPiContainment {
    ProvenInside,
    ProvenOutside,
    Uncertain,
}

fn half_pi_multiple_containment(
    index: i64,
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<HalfPiContainment, IntervalError> {
    let (point_lower, point_upper) = half_pi_multiple_bounds(index, precision_bits)?;
    if compare_rationals(&point_upper, lower) == Ordering::Less
        || compare_rationals(&point_lower, upper) == Ordering::Greater
    {
        return Ok(HalfPiContainment::ProvenOutside);
    }
    if compare_rationals(&point_lower, lower) != Ordering::Less
        && compare_rationals(&point_upper, upper) != Ordering::Greater
    {
        return Ok(HalfPiContainment::ProvenInside);
    }
    Ok(HalfPiContainment::Uncertain)
}

fn half_pi_multiple_bounds(
    index: i64,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let (pi_lower, pi_upper) = pi_bounds(precision_bits)?;
    let multiplier = rational_integer(index);
    ordered_rational_bounds(
        halve_rational(&multiplier.multiply(&pi_lower))?,
        halve_rational(&multiplier.multiply(&pi_upper))?,
    )
}

fn half_pi_scan_limit(
    lower: &Rational,
    upper: &Rational,
    precision_bits: u32,
) -> Result<Option<i64>, IntervalError> {
    if compare_rationals(lower, upper) == Ordering::Greater {
        return Err(IntervalError::InvalidBounds);
    }
    let lower_abs = abs_rational(lower);
    let upper_abs = abs_rational(upper);
    let max_abs = if compare_rationals(&lower_abs, &upper_abs) == Ordering::Greater {
        lower_abs
    } else {
        upper_abs
    };
    let (pi_lower, _) = pi_bounds(precision_bits)?;
    let half_pi_lower = halve_rational(&pi_lower)?;
    let ratio = divide_rational(&max_abs, &half_pi_lower)?;
    let index = match ceil_nonnegative_rational_to_u32(&ratio) {
        Ok(index) => index,
        Err(IntervalError::ExponentTooLarge) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(limit) = index.checked_add(2) else {
        return Ok(None);
    };
    if limit > MAX_TRIG_RANGE_REDUCTION_STEPS {
        return Ok(None);
    }
    Ok(Some(i64::from(limit)))
}

fn subtract(
    left: &CertifiedInterval,
    right: &CertifiedInterval,
) -> Result<CertifiedInterval, IntervalError> {
    add(left, &negate_interval(right))
}

fn negate_interval(value: &CertifiedInterval) -> CertifiedInterval {
    CertifiedInterval {
        lower: negate_dyadic(&value.upper),
        upper: negate_dyadic(&value.lower),
    }
}

fn clamp_trigonometric_interval(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    intersect_with_rational_bounds(
        value,
        &rational_integer(-1),
        &Rational::one(),
        precision_bits,
    )
}

fn intersect_with_rational_bounds(
    value: &CertifiedInterval,
    lower_bound: &Rational,
    upper_bound: &Rational,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let lower = if compare_rationals(&lower, lower_bound) == Ordering::Less {
        lower_bound.clone()
    } else {
        lower
    };
    let upper = if compare_rationals(&upper, upper_bound) == Ordering::Greater {
        upper_bound.clone()
    } else {
        upper
    };
    from_rational_bounds(&lower, &upper, precision_bits)
}

fn full_trigonometric_range(precision_bits: u32) -> Result<CertifiedInterval, IntervalError> {
    interval_from_integer_bounds(-1, 1, precision_bits)
}

fn interval_from_integer_bounds(
    lower: i64,
    upper: i64,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    from_rational_bounds(
        &rational_integer(lower),
        &rational_integer(upper),
        precision_bits,
    )
}

fn abs_rational(value: &Rational) -> Rational {
    if value.is_negative() {
        value.negate()
    } else {
        value.clone()
    }
}

fn ceil_nonnegative_rational_to_u32(value: &Rational) -> Result<u32, IntervalError> {
    debug_assert!(!value.is_negative());
    let quotient = value
        .numerator
        .inner
        .div_ceil(&value.denominator.inner.inner);
    let value = if quotient.is_zero() {
        1
    } else {
        quotient.to_u32().ok_or(IntervalError::ExponentTooLarge)?
    };
    if value > MAX_EXP_RANGE_REDUCTION_STEPS {
        return Err(IntervalError::ExponentTooLarge);
    }
    Ok(value)
}

fn ceil_absolute_rational_to_u32(value: &Rational) -> Result<u32, IntervalError> {
    let value = abs_rational(value);
    let quotient = value
        .numerator
        .inner
        .div_ceil(&value.denominator.inner.inner);
    let value = if quotient.is_zero() {
        1
    } else {
        quotient.to_u32().ok_or(IntervalError::ExponentTooLarge)?
    };
    if value > MAX_TRIG_RANGE_REDUCTION_STEPS {
        return Err(IntervalError::ExponentTooLarge);
    }
    Ok(value)
}

fn guard_log_range_reduction_step(steps: &mut u32) -> Result<(), IntervalError> {
    *steps = steps
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    if *steps > MAX_LOG_RANGE_REDUCTION_STEPS {
        return Err(IntervalError::ExponentTooLarge);
    }
    Ok(())
}

fn pow_positive_rational(value: &Rational, exponent: u32) -> Result<Rational, IntervalError> {
    value
        .pow_i64(i64::from(exponent))
        .map_err(|_| IntervalError::ExponentTooLarge)
}

fn divide_rational(left: &Rational, right: &Rational) -> Result<Rational, IntervalError> {
    left.divide(right)
        .map_err(|_| IntervalError::DivisionByIntervalContainingZero)
}

fn divide_rational_by_positive_u32(
    value: &Rational,
    divisor: u32,
) -> Result<Rational, IntervalError> {
    if divisor == 0 {
        return Err(IntervalError::DivisionByIntervalContainingZero);
    }
    rational_from_parts(
        value.numerator.inner.clone(),
        &value.denominator.inner.inner * divisor,
    )
}

fn halve_rational(value: &Rational) -> Result<Rational, IntervalError> {
    divide_rational_by_positive_u32(value, 2)
}

fn negate_dyadic(value: &ExactDyadic) -> ExactDyadic {
    ExactDyadic {
        coefficient: Integer::from_bigint(-value.coefficient.inner.clone()),
        exponent_two: value.exponent_two.clone(),
    }
}

fn rational_to_dyadic_lower(
    rational: &Rational,
    precision_bits: u32,
    exponent_two: BigInt,
) -> ExactDyadic {
    let scale = BigInt::one() << precision_bits;
    let scaled_numerator = &rational.numerator.inner * scale;
    let denominator = &rational.denominator.inner.inner;
    normalize_dyadic(scaled_numerator.div_floor(denominator), exponent_two)
}

fn rational_to_dyadic_upper(
    rational: &Rational,
    precision_bits: u32,
    exponent_two: BigInt,
) -> ExactDyadic {
    let scale = BigInt::one() << precision_bits;
    let scaled_numerator = &rational.numerator.inner * scale;
    let denominator = &rational.denominator.inner.inner;
    normalize_dyadic(scaled_numerator.div_ceil(denominator), exponent_two)
}

fn rational_to_dyadic_bound(
    rational: &Rational,
    precision_bits: u32,
    direction: BoundDirection,
) -> ExactDyadic {
    let exponent_two = -BigInt::from(precision_bits);
    match direction {
        BoundDirection::Lower => rational_to_dyadic_lower(rational, precision_bits, exponent_two),
        BoundDirection::Upper => rational_to_dyadic_upper(rational, precision_bits, exponent_two),
    }
}

fn fraction_to_dyadic_bound(
    numerator: &BigInt,
    denominator: &BigInt,
    precision_bits: u32,
    direction: BoundDirection,
) -> ExactDyadic {
    debug_assert!(denominator.is_positive());
    let scaled_numerator = numerator << precision_bits;
    let coefficient = match direction {
        BoundDirection::Lower => scaled_numerator.div_floor(denominator),
        BoundDirection::Upper => scaled_numerator.div_ceil(denominator),
    };
    normalize_dyadic(coefficient, -BigInt::from(precision_bits))
}

fn reciprocal_interval(
    interval: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&interval.lower)?;
    let upper = dyadic_to_rational(&interval.upper)?;
    let reciprocal_lower = reciprocal_nonzero_rational(&upper)?;
    let reciprocal_upper = reciprocal_nonzero_rational(&lower)?;
    from_rational_bounds(&reciprocal_lower, &reciprocal_upper, precision_bits)
}

fn sqrt_dyadic_lower(
    value: &ExactDyadic,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    let value = dyadic_to_rational(value)?;
    sqrt_rational_lower(&value, precision_bits)
}

fn sqrt_dyadic_bounds(
    value: &ExactDyadic,
    precision_bits: u32,
) -> Result<(ExactDyadic, ExactDyadic), IntervalError> {
    let value = dyadic_to_rational(value)?;
    sqrt_rational_bounds(&value, precision_bits)
}

fn sqrt_dyadic_upper(
    value: &ExactDyadic,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    let value = dyadic_to_rational(value)?;
    sqrt_rational_upper(&value, precision_bits)
}

fn sqrt_rational_lower(
    value: &Rational,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    if value.is_negative() {
        return Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative));
    }
    let scale_bits = precision_bits
        .checked_mul(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let scaled_numerator = &value.numerator.inner << scale_bits;
    let scaled = scaled_numerator.div_floor(&value.denominator.inner.inner);
    Ok(normalize_dyadic(
        floor_sqrt_nonnegative(&scaled),
        -BigInt::from(precision_bits),
    ))
}

fn sqrt_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(ExactDyadic, ExactDyadic), IntervalError> {
    if value.is_negative() {
        return Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative));
    }
    let scale_bits = precision_bits
        .checked_mul(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let scaled_numerator = &value.numerator.inner << scale_bits;
    let denominator = &value.denominator.inner.inner;
    let scaled_lower = scaled_numerator.div_floor(denominator);
    let scaled_upper = if scaled_numerator.is_multiple_of(denominator) {
        scaled_lower.clone()
    } else {
        &scaled_lower + BigInt::one()
    };
    let lower_root = floor_sqrt_nonnegative(&scaled_lower);
    let lower_root_squared = &lower_root * &lower_root;
    let upper_root = if lower_root_squared == scaled_upper {
        lower_root.clone()
    } else {
        &lower_root + 1_u8
    };
    let exponent = -BigInt::from(precision_bits);
    Ok((
        normalize_dyadic(lower_root, exponent.clone()),
        normalize_dyadic(upper_root, exponent),
    ))
}

fn sqrt_rational_upper(
    value: &Rational,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    if value.is_negative() {
        return Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative));
    }
    let scale_bits = precision_bits
        .checked_mul(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let scaled_numerator = &value.numerator.inner << scale_bits;
    let scaled = scaled_numerator.div_ceil(&value.denominator.inner.inner);
    Ok(normalize_dyadic(
        ceil_sqrt_nonnegative(&scaled),
        -BigInt::from(precision_bits),
    ))
}

fn e_bounds(precision_bits: u32) -> Result<(Rational, Rational), IntervalError> {
    let term_count = series_terms(precision_bits)?;
    e_common_denominator_bounds(term_count)
}

fn e_common_denominator_bounds(term_count: u32) -> Result<(Rational, Rational), IntervalError> {
    let mut sum_numerator = BigInt::one();
    let mut factorial = BigInt::one();
    for index in 1..=term_count {
        sum_numerator *= index;
        sum_numerator += 1_u8;
        factorial *= index;
    }
    let lower = rational_from_parts(sum_numerator.clone(), factorial.clone())?;
    let next_index = term_count
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    sum_numerator *= next_index;
    sum_numerator += 2_u8;
    factorial *= next_index;
    let upper = rational_from_parts(sum_numerator, factorial)?;
    Ok((lower, upper))
}

fn pi_bounds(precision_bits: u32) -> Result<(Rational, Rational), IntervalError> {
    let term_count = series_terms(precision_bits)?;
    let (atan_1_5_lower, atan_1_5_upper) = arctan_reciprocal_bounds(5, term_count)?;
    let (atan_1_239_lower, atan_1_239_upper) = arctan_reciprocal_bounds(239, term_count)?;

    let lower = scale_rational_by_positive_u32(&atan_1_5_lower, 16)?
        .subtract(&scale_rational_by_positive_u32(&atan_1_239_upper, 4)?);
    let upper = scale_rational_by_positive_u32(&atan_1_5_upper, 16)?
        .subtract(&scale_rational_by_positive_u32(&atan_1_239_lower, 4)?);
    Ok((lower, upper))
}

fn pi_bound(precision_bits: u32, direction: BoundDirection) -> Result<Rational, IntervalError> {
    let term_count = series_terms(precision_bits)?;
    let opposite = match direction {
        BoundDirection::Lower => BoundDirection::Upper,
        BoundDirection::Upper => BoundDirection::Lower,
    };
    let atan_1_5 = arctan_reciprocal_bound(5, term_count, direction)?;
    let atan_1_239 = arctan_reciprocal_bound(239, term_count, opposite)?;
    Ok(scale_rational_by_positive_u32(&atan_1_5, 16)?
        .subtract(&scale_rational_by_positive_u32(&atan_1_239, 4)?))
}

fn arctan_reciprocal_bounds(
    reciprocal_denominator: u32,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let value = Rational::new(
        Integer::one(),
        Integer::from(i64::from(reciprocal_denominator)),
    )
    .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    atan_series_common_denominator_bounds(&value, term_count)
}

fn arctan_reciprocal_bound(
    reciprocal_denominator: u32,
    term_count: u32,
    direction: BoundDirection,
) -> Result<Rational, IntervalError> {
    let value = Rational::new(
        Integer::one(),
        Integer::from(i64::from(reciprocal_denominator)),
    )
    .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    atan_series_common_denominator_bound(&value, term_count, direction)
}

fn series_terms(precision_bits: u32) -> Result<u32, IntervalError> {
    precision_bits
        .checked_div(3)
        .and_then(|value| value.checked_add(16))
        .ok_or(IntervalError::ExponentTooLarge)
}

fn log_series_terms(precision_bits: u32) -> Result<u32, IntervalError> {
    let target_bits = precision_bits
        .checked_add(2)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let target = BigInt::one() << target_bits;
    let mut odd_power_of_three = BigInt::from(27_u8);
    let mut term_count = 0_u32;
    loop {
        let next_denominator = term_count
            .checked_add(1)
            .and_then(|value| value.checked_mul(2))
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        if &odd_power_of_three * next_denominator >= target {
            return Ok(term_count);
        }
        term_count = term_count
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
        odd_power_of_three *= 9_u8;
    }
}

struct ExpSeriesPlan {
    term_count: u32,
    factorial: BigInt,
}

fn exp_series_plan(precision_bits: u32) -> Result<ExpSeriesPlan, IntervalError> {
    let target_bits = precision_bits
        .checked_add(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    let target = BigInt::one() << target_bits;
    let mut factorial = BigInt::one();
    let mut next_factor = 1_u32;
    while factorial < target {
        next_factor = next_factor
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
        factorial *= next_factor;
    }
    let term_count = next_factor
        .checked_sub(1)
        .ok_or(IntervalError::ExponentTooLarge)?;
    factorial /= next_factor;
    Ok(ExpSeriesPlan {
        term_count,
        factorial,
    })
}

#[cfg(test)]
fn exp_series_factorial(term_count: u32) -> BigInt {
    let mut factorial = BigInt::one();
    for factor in 1..=term_count {
        factorial *= factor;
    }
    factorial
}

#[cfg(test)]
fn exp_series_terms(precision_bits: u32) -> Result<u32, IntervalError> {
    Ok(exp_series_plan(precision_bits)?.term_count)
}

fn trigonometric_series_terms(precision_bits: u32) -> Result<u32, IntervalError> {
    let target = BigInt::one() << precision_bits;
    let mut factorial = BigInt::one();
    let mut term_count = 0_u32;
    loop {
        let first_factor = term_count
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let second_factor = first_factor
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
        factorial *= first_factor;
        factorial *= second_factor;
        if factorial >= target {
            return Ok(term_count);
        }
        term_count = term_count
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
    }
}

fn rational_integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn scale_rational(value: &Rational, factor: i64) -> Rational {
    value.multiply(&rational_integer(factor))
}

fn scale_rational_by_positive_u32(
    value: &Rational,
    factor: u32,
) -> Result<Rational, IntervalError> {
    rational_from_parts(
        &value.numerator.inner * factor,
        value.denominator.inner.inner.clone(),
    )
}

fn scale_rational_by_i64(value: &Rational, factor: i64) -> Result<Rational, IntervalError> {
    rational_from_parts(
        &value.numerator.inner * factor,
        value.denominator.inner.inner.clone(),
    )
}

fn rational_from_parts(numerator: BigInt, denominator: BigInt) -> Result<Rational, IntervalError> {
    Rational::new(
        Integer::from_bigint(numerator),
        Integer::from_bigint(denominator),
    )
    .map_err(|_| IntervalError::DivisionByIntervalContainingZero)
}

fn contains_zero(interval: &CertifiedInterval) -> bool {
    interval.lower.coefficient.inner.sign() != Sign::Plus
        && interval.upper.coefficient.inner.sign() != Sign::Minus
}

fn add_dyadic(left: &ExactDyadic, right: &ExactDyadic) -> Result<ExactDyadic, IntervalError> {
    let exponent_two = left
        .exponent_two
        .inner
        .clone()
        .min(right.exponent_two.inner.clone());
    let left_shift = (&left.exponent_two.inner - &exponent_two)
        .to_u32()
        .ok_or(IntervalError::ExponentTooLarge)?;
    let right_shift = (&right.exponent_two.inner - &exponent_two)
        .to_u32()
        .ok_or(IntervalError::ExponentTooLarge)?;
    let coefficient =
        (&left.coefficient.inner << left_shift) + (&right.coefficient.inner << right_shift);
    Ok(normalize_dyadic(coefficient, exponent_two))
}

fn multiply_dyadic(left: &ExactDyadic, right: &ExactDyadic) -> ExactDyadic {
    normalize_dyadic(
        &left.coefficient.inner * &right.coefficient.inner,
        &left.exponent_two.inner + &right.exponent_two.inner,
    )
}

fn compare_dyadic(left: &ExactDyadic, right: &ExactDyadic) -> Result<Ordering, IntervalError> {
    if left.exponent_two == right.exponent_two {
        return Ok(left.coefficient.inner.cmp(&right.coefficient.inner));
    }
    if left.exponent_two.inner > right.exponent_two.inner {
        let shift = (&left.exponent_two.inner - &right.exponent_two.inner)
            .to_u32()
            .ok_or(IntervalError::ExponentTooLarge)?;
        Ok((&left.coefficient.inner << shift).cmp(&right.coefficient.inner))
    } else {
        let shift = (&right.exponent_two.inner - &left.exponent_two.inner)
            .to_u32()
            .ok_or(IntervalError::ExponentTooLarge)?;
        Ok(left
            .coefficient
            .inner
            .cmp(&(&right.coefficient.inner << shift)))
    }
}

fn compare_dyadic_to_rational(
    dyadic: &ExactDyadic,
    rational: &Rational,
) -> Result<Ordering, IntervalError> {
    let exponent = &dyadic.exponent_two.inner;
    if exponent.sign() == Sign::Minus {
        let scale = BigInt::one()
            << exponent
                .abs()
                .to_u32()
                .ok_or(IntervalError::ExponentTooLarge)?;
        Ok(
            (&dyadic.coefficient.inner * &rational.denominator.inner.inner)
                .cmp(&(&rational.numerator.inner * scale)),
        )
    } else {
        let scale = BigInt::one() << exponent.to_u32().ok_or(IntervalError::ExponentTooLarge)?;
        Ok(
            (&dyadic.coefficient.inner * scale * &rational.denominator.inner.inner)
                .cmp(&rational.numerator.inner),
        )
    }
}

fn compare_rationals(left: &Rational, right: &Rational) -> Ordering {
    (&left.numerator.inner * &right.denominator.inner.inner)
        .cmp(&(&right.numerator.inner * &left.denominator.inner.inner))
}

pub(crate) fn dyadic_to_rational(value: &ExactDyadic) -> Result<Rational, IntervalError> {
    let exponent = &value.exponent_two.inner;
    let exponent_shift = if exponent.sign() == Sign::Minus {
        exponent.abs().to_u32()
    } else {
        exponent.to_u32()
    }
    .ok_or(IntervalError::ExponentTooLarge)?;
    if value.coefficient.is_zero() {
        return Ok(Rational::zero());
    }
    if exponent.sign() == Sign::Minus && !value.coefficient.inner.is_even() {
        let denominator = BigInt::one() << exponent_shift;
        Ok(Rational {
            numerator: value.coefficient.clone(),
            denominator: PositiveInteger {
                inner: Integer::from_bigint(denominator),
            },
        })
    } else if exponent.sign() == Sign::Minus {
        let mut coefficient = value.coefficient.inner.clone();
        let trailing_zeros = coefficient.trailing_zeros().unwrap_or(0);
        let removed = u64::from(exponent_shift).min(trailing_zeros);
        coefficient >>= removed;
        let remaining_exponent = exponent_shift - removed as u32;
        if remaining_exponent > 0 {
            let denominator = BigInt::one() << remaining_exponent;
            Ok(Rational {
                numerator: Integer::from_bigint(coefficient),
                denominator: PositiveInteger {
                    inner: Integer::from_bigint(denominator),
                },
            })
        } else {
            Ok(Rational::from_integer(Integer::from_bigint(coefficient)))
        }
    } else {
        let numerator = &value.coefficient.inner << exponent_shift;
        Ok(Rational::from_integer(Integer::from_bigint(numerator)))
    }
}

fn normalize_dyadic(mut coefficient: BigInt, mut exponent_two: BigInt) -> ExactDyadic {
    if coefficient.is_zero() {
        return ExactDyadic {
            coefficient: Integer::zero(),
            exponent_two: Integer::zero(),
        };
    }
    while coefficient.is_even() {
        coefficient >>= 1_u8;
        exponent_two += 1_u8;
    }
    ExactDyadic {
        coefficient: Integer::from_bigint(coefficient),
        exponent_two: Integer::from_bigint(exponent_two),
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::cell::{Cell, RefCell};

    use super::*;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
    }

    #[test]
    fn dyadic_comparison_aligns_exponents_without_rational_conversion() {
        let values = [
            ExactDyadic {
                coefficient: Integer::from(-7),
                exponent_two: Integer::from(-3),
            },
            ExactDyadic {
                coefficient: Integer::from(-1),
                exponent_two: Integer::from(4),
            },
            ExactDyadic {
                coefficient: Integer::zero(),
                exponent_two: Integer::from(-100),
            },
            ExactDyadic {
                coefficient: Integer::from(3),
                exponent_two: Integer::from(-5),
            },
            ExactDyadic {
                coefficient: Integer::from(6),
                exponent_two: Integer::from(-6),
            },
            ExactDyadic {
                coefficient: Integer::from(5),
                exponent_two: Integer::from(7),
            },
        ];
        for left in &values {
            for right in &values {
                let rational_order = compare_rationals(
                    &dyadic_to_rational(left).unwrap(),
                    &dyadic_to_rational(right).unwrap(),
                );
                assert_eq!(compare_dyadic(left, right).unwrap(), rational_order);
            }
        }
    }

    #[test]
    fn dyadic_conversion_matches_general_rational_canonicalization() {
        for value in [
            exact_dyadic(0, -100),
            exact_dyadic(3, -5),
            exact_dyadic(-3, -5),
            exact_dyadic(12, -5),
            exact_dyadic(-12, -5),
            exact_dyadic(12, -2),
            exact_dyadic(5, 0),
            exact_dyadic(-5, 0),
            exact_dyadic(12, 3),
        ] {
            let expected = if value.exponent_two.inner.sign() == Sign::Minus {
                Rational::new(
                    value.coefficient.clone(),
                    Integer::from_bigint(
                        BigInt::one() << value.exponent_two.inner.abs().to_u32().unwrap(),
                    ),
                )
                .unwrap()
            } else {
                Rational::new(
                    Integer::from_bigint(
                        &value.coefficient.inner << value.exponent_two.inner.to_u32().unwrap(),
                    ),
                    Integer::one(),
                )
                .unwrap()
            };
            assert_eq!(dyadic_to_rational(&value).unwrap(), expected);
        }

        let oversized = BigInt::from(u32::MAX) + BigInt::one();
        for (coefficient, exponent) in [
            (Integer::one(), oversized.clone()),
            (Integer::one(), -oversized.clone()),
            (Integer::from(12), -oversized.clone()),
            (Integer::zero(), oversized.clone()),
            (Integer::zero(), -oversized),
        ] {
            assert_eq!(
                dyadic_to_rational(&ExactDyadic {
                    coefficient,
                    exponent_two: Integer::from_bigint(exponent),
                }),
                Err(IntervalError::ExponentTooLarge)
            );
        }
    }

    #[test]
    fn rational_conversion_contains_exact_value() {
        for value in [rational(3, 10), rational(-3, 10), rational(0, 1)] {
            let interval = from_rational(&value, 16);
            assert!(contains_rational(&interval, &value).unwrap());
        }
    }

    #[test]
    fn rational_bounds_conversion_contains_bounds() {
        let lower = rational(-1, 10);
        let upper = rational(3, 10);
        let interval = from_rational_bounds(&lower, &upper, 12).unwrap();
        assert!(contains_rational(&interval, &lower).unwrap());
        assert!(contains_rational(&interval, &upper).unwrap());
    }

    #[test]
    fn signed_interval_multiplication_matches_endpoint_extrema() {
        for (left, right, expected) in [
            ((2, 3), (4, 5), (8, 15)),
            ((-3, -2), (-5, -4), (8, 15)),
            ((2, 3), (-5, -4), (-15, -8)),
            ((-3, -2), (4, 5), (-15, -8)),
            ((0, 3), (-5, 0), (-15, 0)),
            ((-2, 3), (4, 5), (-10, 15)),
            ((-2, 3), (-5, 4), (-15, 12)),
        ] {
            let left =
                from_rational_bounds(&rational(left.0, 1), &rational(left.1, 1), 32).unwrap();
            let right =
                from_rational_bounds(&rational(right.0, 1), &rational(right.1, 1), 32).unwrap();
            let expected =
                from_rational_bounds(&rational(expected.0, 1), &rational(expected.1, 1), 32)
                    .unwrap();
            assert_eq!(multiply(&left, &right).unwrap(), expected);
        }

        let intervals = [
            CertifiedInterval {
                lower: exact_dyadic(-3, -1),
                upper: exact_dyadic(-1, -2),
            },
            CertifiedInterval {
                lower: exact_dyadic(-1, 0),
                upper: exact_dyadic(0, 0),
            },
            CertifiedInterval {
                lower: exact_dyadic(0, 0),
                upper: exact_dyadic(0, 0),
            },
            CertifiedInterval {
                lower: exact_dyadic(0, 0),
                upper: exact_dyadic(3, -3),
            },
            CertifiedInterval {
                lower: exact_dyadic(1, -2),
                upper: exact_dyadic(5, -1),
            },
            CertifiedInterval {
                lower: exact_dyadic(-3, -2),
                upper: exact_dyadic(7, -3),
            },
        ];
        for left in &intervals {
            for right in &intervals {
                assert_eq!(
                    multiply(left, right).unwrap(),
                    multiply_all_endpoint_candidates(left, right),
                );
            }
        }
    }

    fn exact_dyadic(coefficient: i64, exponent_two: i64) -> ExactDyadic {
        ExactDyadic {
            coefficient: Integer::from(coefficient),
            exponent_two: Integer::from(exponent_two),
        }
    }

    fn multiply_all_endpoint_candidates(
        left: &CertifiedInterval,
        right: &CertifiedInterval,
    ) -> CertifiedInterval {
        let mut candidates = [
            multiply_dyadic(&left.lower, &right.lower),
            multiply_dyadic(&left.lower, &right.upper),
            multiply_dyadic(&left.upper, &right.lower),
            multiply_dyadic(&left.upper, &right.upper),
        ];
        candidates.sort_by(|left, right| compare_dyadic(left, right).unwrap());
        CertifiedInterval {
            lower: candidates[0].clone(),
            upper: candidates[3].clone(),
        }
    }

    #[test]
    fn exp_log_exact_points_use_their_single_directed_bound_pair() {
        let precision_bits = 96;
        let one = rational(1, 1);
        let (exp_lower, exp_upper) = exp_rational_bounds(&one, precision_bits).unwrap();
        assert_eq!(
            exp(&from_rational(&one, precision_bits), precision_bits).unwrap(),
            from_rational_bounds(&exp_lower, &exp_upper, precision_bits).unwrap()
        );

        let two = rational(2, 1);
        let (log_lower, log_upper) = log_rational_bounds(&two, precision_bits).unwrap();
        assert_eq!(
            log(&from_rational(&two, precision_bits), precision_bits).unwrap(),
            from_rational_bounds(&log_lower, &log_upper, precision_bits).unwrap()
        );
    }

    #[test]
    fn directed_logarithm_bounds_match_paired_bounds() {
        for precision_bits in [1, 64, 128] {
            for value in [
                rational(1, 3),
                rational(3, 4),
                Rational::one(),
                rational(3, 2),
                rational(2, 1),
                rational(3, 1),
                rational(8, 1),
            ] {
                let (lower, upper) = log_rational_bounds(&value, precision_bits).unwrap();
                assert_eq!(
                    log_rational_bound(&value, precision_bits, BoundDirection::Lower).unwrap(),
                    lower,
                );
                assert_eq!(
                    log_rational_bound(&value, precision_bits, BoundDirection::Upper).unwrap(),
                    upper,
                );
            }
        }
    }

    #[test]
    fn shared_log_term_plan_matches_independent_directed_endpoints() {
        for precision_bits in [1, 64, 128] {
            for (lower, upper) in [
                (rational(1, 4), rational(1, 2)),
                (rational(1, 4), rational(3, 4)),
                (rational(1, 2), rational(2, 1)),
                (rational(3, 4), rational(3, 2)),
                (rational(3, 4), rational(3, 1)),
                (rational(3, 2), rational(3, 1)),
                (rational(3, 2), rational(7, 4)),
                (rational(2, 1), rational(3, 1)),
                (rational(3, 1), rational(9, 1)),
            ] {
                assert_eq!(
                    log_rational_directed_endpoint_bounds(&lower, &upper, precision_bits).unwrap(),
                    (
                        log_rational_bound(&lower, precision_bits, BoundDirection::Lower).unwrap(),
                        log_rational_bound(&upper, precision_bits, BoundDirection::Upper).unwrap(),
                    ),
                );
            }
        }

        assert_eq!(
            log_rational_directed_endpoint_bounds(&Rational::one(), &Rational::one(), u32::MAX,),
            Ok((Rational::zero(), Rational::zero())),
        );
        assert_eq!(
            log_rational_directed_endpoint_bounds(&Rational::one(), &rational(3, 2), u32::MAX,),
            Err(IntervalError::ExponentTooLarge),
        );
    }

    #[test]
    fn directed_arctangent_bounds_match_paired_bounds() {
        for precision_bits in [1, 64, 128] {
            for value in [
                rational(-3, 1),
                rational(-1, 1),
                rational(-1, 3),
                Rational::zero(),
                rational(1, 3),
                Rational::one(),
                rational(3, 1),
            ] {
                let (lower, upper) = atan_rational_bounds(&value, precision_bits).unwrap();
                assert_eq!(
                    atan_rational_bound(&value, precision_bits, BoundDirection::Lower).unwrap(),
                    lower,
                );
                assert_eq!(
                    atan_rational_bound(&value, precision_bits, BoundDirection::Upper).unwrap(),
                    upper,
                );
            }
            let (pi_lower, pi_upper) = pi_bounds(precision_bits).unwrap();
            assert_eq!(
                pi_bound(precision_bits, BoundDirection::Lower).unwrap(),
                pi_lower,
            );
            assert_eq!(
                pi_bound(precision_bits, BoundDirection::Upper).unwrap(),
                pi_upper,
            );
        }
    }

    #[test]
    fn shared_arctangent_pi_matches_independent_directed_bounds() {
        let pi = pi_bounds(128).unwrap();
        for value in [
            rational(-3, 1),
            rational(-1, 1),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            Rational::one(),
            rational(3, 1),
        ] {
            for direction in [BoundDirection::Lower, BoundDirection::Upper] {
                assert_eq!(
                    atan_rational_bound_with_pi(&value, 128, direction, Some(&pi)).unwrap(),
                    atan_rational_bound(&value, 128, direction).unwrap(),
                );
            }
        }

        for (lower, upper) in [
            (rational(2, 1), rational(3, 1)),
            (rational(-3, 1), rational(-2, 1)),
            (rational(-3, 1), rational(3, 1)),
            (rational(-1, 2), rational(2, 1)),
        ] {
            let input = from_rational_bounds(&lower, &upper, 128).unwrap();
            let input_lower = dyadic_to_rational(&input.lower).unwrap();
            let input_upper = dyadic_to_rational(&input.upper).unwrap();
            assert_eq!(
                atan(&input, 128).unwrap(),
                from_rational_bounds(
                    &atan_rational_bound(&input_lower, 128, BoundDirection::Lower).unwrap(),
                    &atan_rational_bound(&input_upper, 128, BoundDirection::Upper).unwrap(),
                    128,
                )
                .unwrap(),
            );
        }
    }

    #[test]
    fn directed_inverse_sine_unit_bounds_match_paired_bounds() {
        for precision_bits in [1, 64, 128] {
            for value in [
                rational(-1, 1),
                rational(-999, 1_000),
                rational(-2, 3),
                rational(-1, 2),
                rational(-1, 3),
                Rational::zero(),
                rational(1, 3),
                rational(1, 2),
                rational(501, 1_000),
                rational(2, 3),
                rational(999, 1_000),
                Rational::one(),
            ] {
                let (lower, upper) = asin_rational_bounds(&value, precision_bits).unwrap();
                assert_eq!(
                    asin_rational_bound(&value, precision_bits, BoundDirection::Lower).unwrap(),
                    lower,
                );
                assert_eq!(
                    asin_rational_bound(&value, precision_bits, BoundDirection::Upper).unwrap(),
                    upper,
                );
            }
        }
    }

    #[test]
    fn shared_inverse_sine_pi_matches_independent_directed_bounds() {
        let pi = pi_bounds(128).unwrap();
        for value in [
            rational(-1, 1),
            rational(-2, 3),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            rational(2, 3),
            Rational::one(),
        ] {
            for direction in [BoundDirection::Lower, BoundDirection::Upper] {
                assert_eq!(
                    asin_rational_bound_with_pi(&value, 128, direction, Some(&pi)).unwrap(),
                    asin_rational_bound(&value, 128, direction).unwrap(),
                );
            }
        }

        for (lower, upper) in [
            (rational(1, 3), rational(2, 3)),
            (rational(-2, 3), rational(-1, 3)),
            (rational(-2, 3), rational(2, 3)),
        ] {
            let input = from_rational_bounds(&lower, &upper, 128).unwrap();
            let input_lower = dyadic_to_rational(&input.lower).unwrap();
            let input_upper = dyadic_to_rational(&input.upper).unwrap();
            assert_eq!(
                asin(&input, 128).unwrap(),
                from_rational_bounds(
                    &asin_rational_bound(&input_lower, 128, BoundDirection::Lower).unwrap(),
                    &asin_rational_bound(&input_upper, 128, BoundDirection::Upper).unwrap(),
                    128,
                )
                .unwrap(),
            );
        }
    }

    #[test]
    fn directed_exponential_bounds_match_paired_bounds() {
        for precision_bits in [1, 64, 128] {
            for value in [
                rational(-3, 2),
                rational(-1, 3),
                Rational::zero(),
                rational(1, 3),
                Rational::one(),
                rational(3, 2),
            ] {
                let (lower, upper) = exp_rational_bounds(&value, precision_bits).unwrap();
                let term_count = exp_series_terms(precision_bits).unwrap();
                assert_eq!(
                    exp_rational_bound(&value, precision_bits, BoundDirection::Lower).unwrap(),
                    lower
                );
                assert_eq!(
                    exp_rational_bound(&value, precision_bits, BoundDirection::Upper).unwrap(),
                    upper
                );
                assert_eq!(
                    exp_rational_bound_with_terms(&value, term_count, BoundDirection::Lower)
                        .unwrap(),
                    lower
                );
                assert_eq!(
                    exp_rational_bound_with_terms(&value, term_count, BoundDirection::Upper)
                        .unwrap(),
                    upper
                );
            }
        }

        for (lower, upper) in [
            (rational(1, 3), rational(2, 3)),
            (rational(-2, 3), rational(-1, 3)),
            (rational(-1, 3), rational(1, 3)),
            (rational(3, 2), rational(5, 2)),
        ] {
            let input = from_rational_bounds(&lower, &upper, 128).unwrap();
            let input_lower = dyadic_to_rational(&input.lower).unwrap();
            let input_upper = dyadic_to_rational(&input.upper).unwrap();
            assert_eq!(
                exp(&input, 128).unwrap(),
                from_rational_bounds(
                    &exp_rational_bounds(&input_lower, 128).unwrap().0,
                    &exp_rational_bounds(&input_upper, 128).unwrap().1,
                    128
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn raw_exponential_dyadic_rounding_matches_canonical_rational_route() {
        fn canonical_binary_bound(
            value: &Rational,
            direction: BoundDirection,
            plan: &ExpBinaryScalingPlan,
        ) -> ExactDyadic {
            let residual = match (plan.binary_exponent.is_negative(), direction) {
                (false, BoundDirection::Lower) | (true, BoundDirection::Upper) => value.subtract(
                    &scale_rational_by_i64(&plan.log_two_upper, plan.binary_exponent).unwrap(),
                ),
                (false, BoundDirection::Upper) | (true, BoundDirection::Lower) => value.subtract(
                    &scale_rational_by_i64(&plan.log_two_lower, plan.binary_exponent).unwrap(),
                ),
            };
            let bound =
                exp_rational_bound_with_plan(&residual, &plan.series_plan, direction).unwrap();
            let mut dyadic = rational_to_dyadic_bound(&bound, plan.working_precision, direction);
            dyadic.exponent_two.inner += BigInt::from(plan.binary_exponent);
            normalize_dyadic(dyadic.coefficient.inner, dyadic.exponent_two.inner)
        }

        for precision_bits in [64, 128] {
            for value in [
                rational(-2, 1),
                rational(-1, 2),
                Rational::zero(),
                rational(1, 2),
                Rational::one(),
                rational(2, 1),
            ] {
                let input = from_rational(&value, precision_bits);
                let input_value = dyadic_to_rational(&input.lower).unwrap();
                let (lower, upper) = exp_rational_bounds(&input_value, precision_bits).unwrap();
                assert_eq!(
                    exp(&input, precision_bits).unwrap(),
                    from_rational_bounds(&lower, &upper, precision_bits).unwrap(),
                );
            }

            for (lower, upper) in [
                (rational(1, 8), rational(3, 8)),
                (rational(1, 3), rational(2, 3)),
                (rational(-2, 3), rational(-1, 3)),
                (rational(-1, 3), rational(1, 3)),
                (rational(3, 2), rational(5, 2)),
            ] {
                let input = from_rational_bounds(&lower, &upper, precision_bits).unwrap();
                let input_lower = dyadic_to_rational(&input.lower).unwrap();
                let input_upper = dyadic_to_rational(&input.upper).unwrap();
                let canonical_lower =
                    exp_rational_bound(&input_lower, precision_bits, BoundDirection::Lower)
                        .unwrap();
                let canonical_upper =
                    exp_rational_bound(&input_upper, precision_bits, BoundDirection::Upper)
                        .unwrap();
                assert_eq!(
                    exp(&input, precision_bits).unwrap(),
                    from_rational_bounds(&canonical_lower, &canonical_upper, precision_bits,)
                        .unwrap(),
                );
            }

            for value in [
                rational(-10_000, 1),
                rational(-65, 1),
                rational(65, 1),
                rational(10_000, 1),
            ] {
                let input = from_rational(&value, precision_bits);
                let input_value = dyadic_to_rational(&input.lower).unwrap();
                let plan = exp_binary_scaling_plan(&input_value, precision_bits).unwrap();
                let expected = CertifiedInterval {
                    lower: canonical_binary_bound(&input_value, BoundDirection::Lower, &plan),
                    upper: canonical_binary_bound(&input_value, BoundDirection::Upper, &plan),
                };
                assert_eq!(exp(&input, precision_bits).unwrap(), expected);
            }
        }
    }

    #[test]
    fn shared_exponential_denominator_matches_independent_endpoint_states() {
        let term_count = exp_series_terms(128).unwrap();
        for (lower, upper) in [
            (rational(1, 8), rational(3, 8)),
            (rational(5, 16), rational(15, 16)),
        ] {
            let common_denominator =
                exp_series_common_denominator(&lower.denominator.inner.inner, term_count).unwrap();
            assert_eq!(
                exp_series_rational_bound_with_common_denominator(
                    &lower,
                    term_count,
                    BoundDirection::Lower,
                    common_denominator.clone(),
                )
                .unwrap(),
                exp_series_rational_bound(&lower, term_count, BoundDirection::Lower).unwrap(),
            );
            assert_eq!(
                exp_series_rational_bound_with_common_denominator(
                    &upper,
                    term_count,
                    BoundDirection::Upper,
                    common_denominator,
                )
                .unwrap(),
                exp_series_rational_bound(&upper, term_count, BoundDirection::Upper).unwrap(),
            );
        }
    }

    #[test]
    fn exponential_common_denominator_matches_product_definition() {
        for denominator in [1_i64, 2, 3, 6, 8] {
            for term_count in [0_u32, 1, 5, 17] {
                let denominator = BigInt::from(denominator);
                let mut expected = BigInt::one();
                for factor in 1..=term_count {
                    expected *= &denominator * factor;
                }
                assert_eq!(
                    exp_series_common_denominator(&denominator, term_count).unwrap(),
                    expected,
                );
            }
        }
    }

    #[test]
    fn exponential_denominator_shift_classification_preserves_checked_boundaries() {
        assert_eq!(positive_power_of_two_shift(&BigInt::from(1_u8)), Some(0));
        assert_eq!(positive_power_of_two_shift(&BigInt::from(8_u8)), Some(3));
        assert_eq!(positive_power_of_two_shift(&BigInt::from(6_u8)), None);
        assert_eq!(positive_power_of_two_shift(&BigInt::zero()), None);
        assert_eq!(positive_power_of_two_shift(&BigInt::from(-8_i8)), None);
        assert_eq!(
            checked_exp_denominator_total_shift(u64::MAX, 2),
            Err(IntervalError::ExponentTooLarge),
        );
        #[cfg(target_pointer_width = "32")]
        assert_eq!(
            checked_exp_denominator_shift(u64::from(u32::MAX) + 1),
            Err(IntervalError::ExponentTooLarge),
        );
    }

    #[test]
    fn dyadic_exponential_recurrence_matches_general_denominator_products() {
        fn legacy_state<'a>(
            value: &'a Rational,
            plan: &ExpSeriesPlan,
            tail_index: u32,
        ) -> ExpSeriesState<'a> {
            let value_numerator = &value.numerator.inner;
            let value_denominator = &value.denominator.inner.inner;
            let common_denominator = exp_series_common_denominator_with_factorial(
                value_denominator,
                plan.term_count,
                &plan.factorial,
            )
            .unwrap();
            let mut sum_numerator = BigInt::one();
            let mut term_numerator = BigInt::one();
            for next_n in 1..=plan.term_count {
                let denominator_factor = value_denominator * next_n;
                term_numerator *= value_numerator;
                sum_numerator *= denominator_factor;
                sum_numerator += &term_numerator;
            }
            ExpSeriesState {
                sum_numerator,
                term_numerator,
                common_denominator,
                value_numerator,
                value_denominator,
                tail_index,
            }
        }

        let plan = ExpSeriesPlan {
            term_count: 17,
            factorial: exp_series_factorial(17),
        };
        for value in [
            Rational::one(),
            rational(1, 2),
            rational(3, 8),
            rational(1, 3),
        ] {
            let tail_index = plan.term_count + 1;
            assert_eq!(
                exp_series_state_with_plan(&value, &plan, tail_index)
                    .unwrap()
                    .into_lower()
                    .unwrap(),
                legacy_state(&value, &plan, tail_index)
                    .into_lower()
                    .unwrap(),
            );
            assert_eq!(
                exp_series_state_with_plan(&value, &plan, tail_index)
                    .unwrap()
                    .into_upper()
                    .unwrap(),
                legacy_state(&value, &plan, tail_index)
                    .into_upper()
                    .unwrap(),
            );
        }
    }

    #[test]
    fn inverse_trig_exact_points_share_paired_bounds() {
        let value = rational(1, 2);
        let point = from_rational(&value, 128);
        for (actual, expected) in [
            (
                atan(&point, 128).unwrap(),
                atan_rational_bounds(&value, 128).unwrap(),
            ),
            (
                asin(&point, 128).unwrap(),
                asin_rational_bounds(&value, 128).unwrap(),
            ),
            (
                acos(&point, 128).unwrap(),
                acos_rational_bounds(&value, 128).unwrap(),
            ),
        ] {
            assert_eq!(
                actual,
                from_rational_bounds(&expected.0, &expected.1, 128).unwrap()
            );
        }
    }

    #[test]
    fn shared_acos_pi_matches_independent_endpoint_bounds() {
        let pi = pi_bounds(128).unwrap();
        for value in [
            rational(-1, 1),
            rational(-2, 3),
            Rational::zero(),
            rational(1, 3),
            rational(2, 3),
            Rational::one(),
        ] {
            assert_eq!(
                asin_rational_bounds_with_pi(&value, 128, Some(&pi)).unwrap(),
                asin_rational_bounds(&value, 128).unwrap(),
            );
            assert_eq!(
                acos_rational_bounds_with_pi(&value, 128, Some(&pi)).unwrap(),
                acos_rational_bounds(&value, 128).unwrap(),
            );
        }
    }

    #[test]
    fn directed_exact_asin_atan_matches_former_paired_composition() {
        fn former_positive_bounds(value: &Rational, precision_bits: u32) -> (Rational, Rational) {
            let complement = one_minus_rational_square(value).unwrap();
            let numerator = nth_root_nonnegative_rational(&complement, 2, precision_bits).unwrap();
            let ratio_lower =
                divide_rational(&dyadic_to_rational(&numerator.lower).unwrap(), value).unwrap();
            let ratio_upper =
                divide_rational(&dyadic_to_rational(&numerator.upper).unwrap(), value).unwrap();
            let atan_lower = atan_rational_bounds(&ratio_lower, precision_bits)
                .unwrap()
                .0;
            let atan_upper = atan_rational_bounds(&ratio_upper, precision_bits)
                .unwrap()
                .1;
            let pi = pi_bounds(precision_bits).unwrap();
            (
                halve_rational(&pi.0).unwrap().subtract(&atan_upper),
                halve_rational(&pi.1).unwrap().subtract(&atan_lower),
            )
        }

        for precision_bits in [1, 64, 128] {
            let pi = pi_bounds(precision_bits).unwrap();
            for (value, expected) in [
                (rational(1, 2), true),
                (rational(501, 1_000), true),
                (rational(707, 1_000), true),
                (rational(708, 1_000), false),
                (Rational::one(), false),
            ] {
                assert_eq!(rational_square_is_below_half(&value), expected);
            }
            for positive in [rational(708, 1_000), rational(3, 4), rational(999, 1_000)] {
                let former = former_positive_bounds(&positive, precision_bits);
                assert_eq!(
                    asin_rational_bounds_with_pi(&positive, precision_bits, Some(&pi)).unwrap(),
                    former,
                );
                let negative = positive.negate();
                let former_negative = (former.1.negate(), former.0.negate());
                assert_eq!(
                    asin_rational_bounds_with_pi(&negative, precision_bits, Some(&pi)).unwrap(),
                    former_negative,
                );
                assert_eq!(
                    acos_rational_bounds_with_pi(&positive, precision_bits, Some(&pi)).unwrap(),
                    (
                        halve_rational(&pi.0).unwrap().subtract(&former.1),
                        halve_rational(&pi.1).unwrap().subtract(&former.0),
                    ),
                );
                assert_eq!(
                    acos_rational_bounds_with_pi(&negative, precision_bits, Some(&pi)).unwrap(),
                    (
                        halve_rational(&pi.0).unwrap().subtract(&former_negative.1),
                        halve_rational(&pi.1).unwrap().subtract(&former_negative.0),
                    ),
                );
            }

            for positive in [rational(501, 1_000), rational(5, 8), rational(707, 1_000)] {
                let former = former_positive_bounds(&positive, precision_bits);
                let actual =
                    asin_rational_bounds_with_pi(&positive, precision_bits, Some(&pi)).unwrap();
                assert!(compare_rationals(&actual.0, &former.0) != Ordering::Less);
                assert!(compare_rationals(&actual.1, &former.1) != Ordering::Greater);
                let negative = positive.negate();
                let negative_actual =
                    asin_rational_bounds_with_pi(&negative, precision_bits, Some(&pi)).unwrap();
                assert!(
                    compare_rationals(&negative_actual.0, &former.1.negate()) != Ordering::Less
                );
                assert!(
                    compare_rationals(&negative_actual.1, &former.0.negate()) != Ordering::Greater
                );
            }
        }
    }

    #[test]
    fn directed_acos_bounds_match_shared_paired_bounds() {
        let pi = pi_bounds(128).unwrap();
        for value in [
            rational(-1, 1),
            rational(-2, 3),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            rational(2, 3),
            Rational::one(),
        ] {
            let (lower, upper) = acos_rational_bounds_with_pi(&value, 128, Some(&pi)).unwrap();
            assert_eq!(
                acos_rational_bound_with_pi(&value, 128, BoundDirection::Lower, &pi).unwrap(),
                lower,
            );
            assert_eq!(
                acos_rational_bound_with_pi(&value, 128, BoundDirection::Upper, &pi).unwrap(),
                upper,
            );
        }
    }

    #[test]
    fn canonical_rational_reciprocal_matches_general_division() {
        for value in [rational(1, 3), rational(7, 5), rational(12345, 6789)] {
            assert_eq!(
                reciprocal_nonzero_rational(&value).unwrap(),
                divide_rational(&Rational::one(), &value).unwrap(),
            );
        }
        assert_eq!(
            reciprocal_nonzero_rational(&rational(-1, 3)).unwrap(),
            rational(-3, 1),
        );
        assert_eq!(
            reciprocal_nonzero_rational(&rational(-7, 5)).unwrap(),
            divide_rational(&Rational::one(), &rational(-7, 5)).unwrap(),
        );
        assert_eq!(
            reciprocal_nonzero_rational(&Rational::zero()),
            Err(IntervalError::DivisionByIntervalContainingZero),
        );
    }

    #[test]
    fn signed_interval_and_atan_reciprocals_preserve_direction() {
        let negative = from_rational_bounds(&rational(-2, 1), &rational(-1, 1), 64).unwrap();
        let reciprocal = reciprocal_interval(&negative, 64).unwrap();
        assert!(contains_rational(&reciprocal, &rational(-1, 1)).unwrap());
        assert!(contains_rational(&reciprocal, &rational(-1, 2)).unwrap());

        let two = rational(2, 1);
        let half = rational(1, 2);
        let actual = atan_rational_bounds(&two, 96).unwrap();
        let half_bounds = atan_unit_rational_bounds(&half, 96).unwrap();
        let pi = pi_bounds(96).unwrap();
        assert_eq!(
            actual.0,
            halve_rational(&pi.0).unwrap().subtract(&half_bounds.1)
        );
        assert_eq!(
            actual.1,
            halve_rational(&pi.1).unwrap().subtract(&half_bounds.0)
        );
    }

    #[test]
    fn common_denominator_atan_series_matches_rational_recurrence() {
        for value in [
            Rational::zero(),
            rational(1, 7),
            rational(1, 2),
            Rational::one(),
        ] {
            for term_count in [0_u32, 1, 5, 20] {
                let value_squared = value.multiply(&value);
                let mut sum = Rational::zero();
                let mut term_power = value.clone();
                for k in 0..=term_count {
                    let odd_denominator = k * 2 + 1;
                    let term = divide_rational(
                        &term_power,
                        &Rational::from_integer(Integer::from(i64::from(odd_denominator))),
                    )
                    .unwrap();
                    if k.is_multiple_of(2) {
                        sum = sum.add(&term);
                    } else {
                        sum = sum.subtract(&term);
                    }
                    term_power = term_power.multiply(&value_squared);
                }
                let next_index = term_count + 1;
                let next_term = divide_rational(
                    &term_power,
                    &Rational::from_integer(Integer::from(i64::from(next_index * 2 + 1))),
                )
                .unwrap();
                let adjacent = if next_index.is_multiple_of(2) {
                    sum.add(&next_term)
                } else {
                    sum.subtract(&next_term)
                };
                let expected = if compare_rationals(&sum, &adjacent) == Ordering::Less {
                    (sum, adjacent)
                } else {
                    (adjacent, sum)
                };
                assert_eq!(
                    atan_series_common_denominator_bounds(&value, term_count).unwrap(),
                    expected,
                    "value={value:?}, term_count={term_count}",
                );
            }
        }
    }

    #[test]
    fn atan_series_strategies_match_incremental_recurrence() {
        for value in [
            Rational::zero(),
            rational(1, 7),
            rational(1, 2),
            rational(3, 10),
            rational(7, 10),
            Rational::one(),
        ] {
            for term_count in [
                0,
                1,
                5,
                20,
                ATAN_BINARY_SPLIT_THRESHOLD - 1,
                ATAN_BINARY_SPLIT_THRESHOLD,
                ATAN_BINARY_SPLIT_THRESHOLD + 1,
                series_terms(64).unwrap(),
                series_terms(128).unwrap(),
                series_terms(256).unwrap(),
            ] {
                let expected = if value.is_zero() {
                    (Rational::zero(), Rational::zero())
                } else {
                    atan_series_recurrence_bounds(&value, term_count).unwrap()
                };
                assert_eq!(
                    atan_series_common_denominator_bounds(&value, term_count).unwrap(),
                    expected,
                    "value={value:?}, term_count={term_count}",
                );
                assert_eq!(
                    atan_series_common_denominator_bound(
                        &value,
                        term_count,
                        BoundDirection::Lower,
                    )
                    .unwrap(),
                    expected.0,
                    "lower value={value:?}, term_count={term_count}",
                );
                assert_eq!(
                    atan_series_common_denominator_bound(
                        &value,
                        term_count,
                        BoundDirection::Upper,
                    )
                    .unwrap(),
                    expected.1,
                    "upper value={value:?}, term_count={term_count}",
                );
            }
        }
    }

    #[test]
    fn atan_binary_split_dispatch_preserves_small_unit_and_directed_paths() {
        let nonunit = rational(3, 10);
        assert!(!atan_series_uses_binary_split(
            &nonunit,
            ATAN_BINARY_SPLIT_THRESHOLD - 1,
        ));
        assert!(atan_series_uses_binary_split(
            &nonunit,
            ATAN_BINARY_SPLIT_THRESHOLD,
        ));
        assert!(!atan_series_uses_binary_split(
            &rational(1, 3),
            series_terms(256).unwrap(),
        ));
        assert!(!atan_series_uses_binary_split(
            &Rational::zero(),
            series_terms(256).unwrap(),
        ));

        let sum_direction = if (ATAN_BINARY_SPLIT_THRESHOLD + 1).is_multiple_of(2) {
            BoundDirection::Lower
        } else {
            BoundDirection::Upper
        };
        assert!(atan_series_sum_is_bound(ATAN_BINARY_SPLIT_THRESHOLD, sum_direction).unwrap());
        let sum_only =
            atan_binary_split_state(&nonunit, ATAN_BINARY_SPLIT_THRESHOLD, false).unwrap();
        assert!(sum_only.last_product_numerator.is_none());
        assert_eq!(
            sum_only.into_bound(sum_direction).unwrap(),
            atan_series_recurrence_bound(&nonunit, ATAN_BINARY_SPLIT_THRESHOLD, sum_direction,)
                .unwrap(),
        );
        assert!(matches!(
            atan_binary_split_state(&nonunit, u32::MAX, true),
            Err(IntervalError::ExponentTooLarge),
        ));
    }

    #[test]
    fn common_denominator_trigonometric_series_match_rational_recurrences() {
        for value in [
            Rational::zero(),
            rational(1, 7),
            rational(1, 2),
            Rational::one(),
        ] {
            for term_count in [0_u32, 1, 5, 20] {
                for series in [TrigonometricSeries::Sine, TrigonometricSeries::Cosine] {
                    let value_squared = value.multiply(&value);
                    let mut sum = Rational::zero();
                    let mut term = match series {
                        TrigonometricSeries::Sine => value.clone(),
                        TrigonometricSeries::Cosine => Rational::one(),
                    };
                    for index in 0..=term_count {
                        if index.is_multiple_of(2) {
                            sum = sum.add(&term);
                        } else {
                            sum = sum.subtract(&term);
                        }
                        let next_index = index + 1;
                        let doubled = next_index * 2;
                        let factor = match series {
                            TrigonometricSeries::Sine => doubled * (doubled + 1),
                            TrigonometricSeries::Cosine => (doubled - 1) * doubled,
                        };
                        term = divide_rational(
                            &term.multiply(&value_squared),
                            &rational_integer(i64::from(factor)),
                        )
                        .unwrap();
                    }
                    let next_index = term_count + 1;
                    let adjacent = if next_index.is_multiple_of(2) {
                        sum.add(&term)
                    } else {
                        sum.subtract(&term)
                    };
                    assert_eq!(
                        trigonometric_series_common_denominator_bounds(&value, term_count, series,)
                            .unwrap(),
                        ordered_rational_bounds(sum, adjacent).unwrap(),
                        "value={value:?}, term_count={term_count}",
                    );
                }
            }
        }
    }

    #[test]
    fn borrowed_cosine_inputs_match_owned_normalization() {
        for value in [
            rational(-1, 1),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            Rational::one(),
        ] {
            let owned = if value.is_negative() {
                value.negate()
            } else {
                value.clone()
            };
            let term_count = trigonometric_series_terms(128).unwrap();
            let expected = trigonometric_series_common_denominator_bounds(
                &owned,
                term_count,
                TrigonometricSeries::Cosine,
            )
            .unwrap();
            assert_eq!(cos_unit_rational_bounds(&value, 128).unwrap(), expected);
        }
    }

    #[test]
    fn unit_trigonometric_projections_match_paired_evaluation() {
        for value in [
            rational(-1, 1),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            Rational::one(),
        ] {
            let (paired_sine, paired_cosine) = sin_cos_rational(&value, 128).unwrap();
            assert_eq!(sin_rational(&value, 128).unwrap(), paired_sine);
            assert_eq!(cos_rational(&value, 128).unwrap(), paired_cosine);
        }
        assert!(!is_unit_rational(&rational(-4, 3)));
        assert!(!is_unit_rational(&rational(4, 3)));
    }

    #[test]
    fn unit_trigonometric_pair_matches_identity_composition() {
        for value in [
            rational(-1, 1),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            Rational::one(),
        ] {
            let precision_bits = 128;
            let direct = sin_cos_rational(&value, precision_bits).unwrap();
            let (sin_lower, sin_upper) = sin_unit_rational_bounds(&value, precision_bits).unwrap();
            let (cos_lower, cos_upper) = cos_unit_rational_bounds(&value, precision_bits).unwrap();
            let factor = TrigPair {
                cosine: from_rational_bounds(&cos_lower, &cos_upper, precision_bits).unwrap(),
                sine: from_rational_bounds(&sin_lower, &sin_upper, precision_bits).unwrap(),
            };
            let identity = TrigPair {
                cosine: from_rational(&Rational::one(), precision_bits),
                sine: from_rational(&Rational::zero(), precision_bits),
            };
            let composed = multiply_trig_pairs(&identity, &factor, precision_bits).unwrap();
            assert_eq!(direct, (composed.sine, composed.cosine));
        }
    }

    #[test]
    fn unit_trigonometric_reduction_matches_general_division() {
        for value in [
            rational(-1, 1),
            rational(-1, 3),
            Rational::zero(),
            rational(1, 3),
            Rational::one(),
        ] {
            assert_eq!(divide_rational(&value, &Rational::one()).unwrap(), value,);
            assert_eq!(ceil_absolute_rational_to_u32(&value).unwrap(), 1);
        }
    }

    #[test]
    fn scalar_rational_reduction_matches_general_division() {
        for value in [
            rational(-7, 3),
            rational(-2, 1),
            rational(2, 1),
            rational(7, 3),
        ] {
            for divisor in [1_u32, 2, 3, 17] {
                assert_eq!(
                    divide_rational_by_positive_u32(&value, divisor).unwrap(),
                    divide_rational(&value, &rational_integer(i64::from(divisor))).unwrap(),
                );
            }
        }
        assert_eq!(
            divide_rational_by_positive_u32(&Rational::one(), 0),
            Err(IntervalError::DivisionByIntervalContainingZero),
        );
    }

    #[test]
    fn structural_rational_halving_matches_general_division() {
        for value in [
            rational(-7, 3),
            rational(-2, 1),
            Rational::zero(),
            rational(2, 1),
            rational(7, 3),
        ] {
            assert_eq!(
                halve_rational(&value).unwrap(),
                divide_rational(&value, &rational_integer(2)).unwrap(),
            );
        }
    }

    #[test]
    fn structural_unit_checks_match_exact_rationals() {
        for value in [
            rational(-2, 1),
            rational(-1, 1),
            Rational::zero(),
            Rational::one(),
            rational(2, 1),
        ] {
            assert_eq!(is_negative_one_rational(&value), value == rational(-1, 1));
            assert_eq!(is_positive_one_rational(&value), value == Rational::one());
        }
    }

    #[test]
    fn structural_unit_range_matches_absolute_rational_comparison() {
        let one = Rational::one();
        for value in [
            rational(-2, 1),
            rational(-1, 1),
            rational(-1, 2),
            Rational::zero(),
            rational(1, 2),
            Rational::one(),
            rational(2, 1),
        ] {
            let magnitude = if value.is_negative() {
                value.negate()
            } else {
                value.clone()
            };
            assert_eq!(
                is_unit_rational(&value),
                compare_rationals(&magnitude, &one) != Ordering::Greater,
            );
        }
    }

    #[test]
    fn structural_half_comparison_matches_exact_rationals() {
        let half = rational(1, 2);
        for value in [
            Rational::zero(),
            rational(1, 3),
            rational(499, 1_000),
            half.clone(),
            rational(501, 1_000),
            rational(2, 3),
            rational(9_999, 10_000),
        ] {
            assert_eq!(
                compare_nonnegative_rational_to_half(&value),
                compare_rationals(&value, &half),
            );
        }
    }

    #[test]
    fn direct_complement_square_matches_rational_operations() {
        for value in [
            rational(-1, 1),
            rational(-999, 1_000),
            rational(-1, 2),
            Rational::zero(),
            rational(1, 2),
            rational(2, 3),
            rational(999, 1_000),
            Rational::one(),
        ] {
            assert_eq!(
                one_minus_rational_square(&value).unwrap(),
                Rational::one().subtract(&value.multiply(&value)),
            );
        }
    }

    #[test]
    fn inverse_sine_above_half_preserves_directed_odd_bounds() {
        for positive in [rational(501, 1_000), rational(2, 3)] {
            let (positive_lower, positive_upper) = asin_rational_bounds(&positive, 128).unwrap();
            assert_eq!(
                compare_rationals(&positive_lower, &positive_upper),
                Ordering::Less
            );

            let (negative_lower, negative_upper) =
                asin_rational_bounds(&positive.negate(), 128).unwrap();
            assert_eq!(negative_lower, positive_upper.negate());
            assert_eq!(negative_upper, positive_lower.negate());
        }
    }

    #[test]
    fn primitive_positive_scaling_matches_general_multiplication() {
        for value in [rational(-7, 3), Rational::zero(), rational(7, 3)] {
            for factor in [1_u32, 4, 16] {
                assert_eq!(
                    scale_rational_by_positive_u32(&value, factor).unwrap(),
                    scale_rational(&value, i64::from(factor)),
                );
            }
        }
    }

    #[test]
    fn signed_primitive_scaling_matches_general_multiplication() {
        for value in [rational(-7, 3), Rational::zero(), rational(7, 3)] {
            for factor in [-17_i64, -1, 0, 1, 17] {
                assert_eq!(
                    scale_rational_by_i64(&value, factor).unwrap(),
                    scale_rational(&value, factor),
                );
            }
        }
        assert_eq!(
            scale_rational_by_i64(&Rational::one(), i64::MIN).unwrap(),
            scale_rational(&Rational::one(), i64::MIN),
        );
    }

    #[test]
    fn range_trigonometric_composition_matches_identity_seed() {
        for value in [
            rational(-4, 1),
            rational(-3, 1),
            rational(-2, 1),
            rational(2, 1),
            rational(3, 1),
            rational(4, 1),
        ] {
            let precision_bits = 128;
            let divisor = ceil_absolute_rational_to_u32(&value).unwrap();
            let reduced = divide_rational(&value, &rational_integer(i64::from(divisor))).unwrap();
            let (sin_lower, sin_upper) =
                sin_unit_rational_bounds(&reduced, precision_bits).unwrap();
            let (cos_lower, cos_upper) =
                cos_unit_rational_bounds(&reduced, precision_bits).unwrap();
            let mut factor = TrigPair {
                cosine: from_rational_bounds(&cos_lower, &cos_upper, precision_bits).unwrap(),
                sine: from_rational_bounds(&sin_lower, &sin_upper, precision_bits).unwrap(),
            };
            let mut expected = TrigPair {
                cosine: from_rational(&Rational::one(), precision_bits),
                sine: from_rational(&Rational::zero(), precision_bits),
            };
            let mut remaining = divisor;
            while remaining > 0 {
                if remaining & 1 == 1 {
                    expected = multiply_trig_pairs(&expected, &factor, precision_bits).unwrap();
                }
                remaining >>= 1;
                if remaining > 0 {
                    factor = multiply_trig_pairs(&factor, &factor, precision_bits).unwrap();
                }
            }
            assert_eq!(
                sin_cos_rational(&value, precision_bits).unwrap(),
                (expected.sine, expected.cosine),
            );
        }
    }

    #[test]
    fn common_denominator_euler_series_matches_rational_recurrence() {
        for term_count in [0_u32, 1, 5, 20, 64] {
            let mut sum = Rational::zero();
            let mut factorial = BigInt::one();
            for index in 0..=term_count {
                if index > 0 {
                    factorial *= index;
                }
                sum = sum.add(&rational_from_parts(BigInt::one(), factorial.clone()).unwrap());
            }
            let next_factorial = factorial * (term_count + 1);
            let upper = sum.add(&rational_from_parts(BigInt::from(2_u8), next_factorial).unwrap());
            assert_eq!(
                e_common_denominator_bounds(term_count).unwrap(),
                (sum, upper),
                "term_count={term_count}",
            );
        }
    }

    #[test]
    fn common_denominator_asin_series_matches_rational_recurrence() {
        for value in [
            Rational::zero(),
            rational(1, 7),
            rational(1, 3),
            rational(1, 2),
        ] {
            for term_count in [0_u32, 1, 5, 20] {
                let value_squared = value.multiply(&value);
                let mut sum = Rational::zero();
                let mut term = value.clone();
                for index in 0..=term_count {
                    sum = sum.add(&term);
                    let odd = index * 2 + 1;
                    let numerator = odd * odd;
                    let denominator = (index + 1) * 2 * (odd + 2);
                    term = divide_rational(
                        &term
                            .multiply(&value_squared)
                            .multiply(&rational_integer(i64::from(numerator))),
                        &rational_integer(i64::from(denominator)),
                    )
                    .unwrap();
                }
                assert_eq!(
                    asin_common_denominator_bounds(&value, term_count).unwrap(),
                    (sum.clone(), sum.add(&scale_rational(&term, 2))),
                    "value={value:?}, term_count={term_count}",
                );
            }
        }
    }

    #[test]
    fn reduced_logarithm_identity_has_exact_zero_bounds() {
        let two = rational(2, 1);
        for precision_bits in [1, 64, 256] {
            assert_eq!(
                log_reduced_rational_bounds(&Rational::one(), precision_bits).unwrap(),
                (Rational::zero(), Rational::zero())
            );
            assert_eq!(
                log_rational_bounds(&two, precision_bits).unwrap(),
                log_reduced_rational_bounds(&two, precision_bits).unwrap()
            );
        }
    }

    #[test]
    fn logarithm_series_uses_minimal_geometric_tail_bound() {
        for precision_bits in [0, 1, 16, 128, 256] {
            let term_count = log_series_terms(precision_bits).unwrap();
            let next_denominator = term_count * 2 + 3;
            let denominator =
                BigInt::from(3_u8).pow(next_denominator) * BigInt::from(next_denominator);
            let target = BigInt::one() << (precision_bits + 2);
            assert!(denominator >= target);
            if term_count > 0 {
                let previous_denominator = next_denominator - 2;
                let previous = BigInt::from(3_u8).pow(previous_denominator)
                    * BigInt::from(previous_denominator);
                assert!(previous < target);
            }

            let maximum_width =
                rational_from_parts(BigInt::one(), BigInt::one() << precision_bits).unwrap();
            for value in [rational(1, 1), rational(4, 3), rational(2, 1)] {
                let (lower, upper) = log_reduced_rational_bounds(&value, precision_bits).unwrap();
                assert!(
                    compare_rationals(&upper.subtract(&lower), &maximum_width) != Ordering::Greater
                );
            }
        }
        assert!(log_series_terms(128).unwrap() < series_terms(128).unwrap());
        assert_eq!(
            log_series_terms(u32::MAX),
            Err(IntervalError::ExponentTooLarge)
        );
    }

    #[test]
    fn common_denominator_log_series_matches_rational_recurrence() {
        for z in [
            Rational::zero(),
            rational(1, 7),
            rational(3, 10),
            rational(1, 3),
        ] {
            for term_count in [0, 1, 5, 20] {
                let z_squared = z.multiply(&z);
                let mut sum = Rational::zero();
                let mut term_power = z.clone();
                for k in 0..=term_count {
                    sum = sum.add(
                        &divide_rational(&term_power, &rational_integer(i64::from(2 * k + 1)))
                            .unwrap(),
                    );
                    term_power = term_power.multiply(&z_squared);
                }
                let next = divide_rational(
                    &term_power,
                    &rational_integer(i64::from(2 * term_count + 3)),
                )
                .unwrap();
                let expected_lower = scale_rational(&sum, 2);
                let expected_upper = expected_lower.add(&scale_rational(&next, 4));
                assert_eq!(
                    log_series_common_denominator_bounds(&z, term_count).unwrap(),
                    (expected_lower, expected_upper),
                );
            }
        }
    }

    #[test]
    fn log_series_strategies_match_incremental_recurrence() {
        fn legacy_bounds(
            z: &Rational,
            term_count: u32,
        ) -> Result<(Rational, Rational), IntervalError> {
            let value_numerator = &z.numerator.inner;
            let value_denominator = &z.denominator.inner.inner;
            let numerator_squared = value_numerator * value_numerator;
            let denominator_squared = value_denominator * value_denominator;
            let mut sum_numerator = value_numerator.clone();
            let mut term_numerator = value_numerator.clone();
            let mut odd_product = BigInt::one();
            let mut common_denominator = value_denominator.clone();
            for k in 1..=term_count {
                let odd_denominator = k
                    .checked_mul(2)
                    .and_then(|value| value.checked_add(1))
                    .ok_or(IntervalError::ExponentTooLarge)?;
                term_numerator *= &numerator_squared;
                let denominator_factor = &denominator_squared * odd_denominator;
                sum_numerator *= &denominator_factor;
                sum_numerator += &term_numerator * &odd_product;
                common_denominator *= denominator_factor;
                odd_product *= odd_denominator;
            }
            let state = LogSeriesState {
                sum_numerator,
                term_numerator,
                odd_product,
                common_denominator,
                numerator_squared,
                denominator_squared,
                term_count,
            };
            state.into_bounds()
        }

        for z in [
            Rational::zero(),
            rational(1, 3),
            rational(1, 4),
            rational(1, 5),
            rational(1, 7),
            rational(3, 10),
        ] {
            for term_count in [
                0,
                1,
                5,
                20,
                LOG_BINARY_SPLIT_THRESHOLD - 1,
                LOG_BINARY_SPLIT_THRESHOLD,
                LOG_BINARY_SPLIT_THRESHOLD + 1,
                log_series_terms(64).unwrap(),
                log_series_terms(128).unwrap(),
                log_series_terms(256).unwrap(),
            ] {
                let expected = legacy_bounds(&z, term_count).unwrap();
                assert_eq!(
                    log_series_common_denominator_bounds(&z, term_count).unwrap(),
                    expected,
                );
                assert_eq!(
                    log_series_common_denominator_bound(&z, term_count, BoundDirection::Lower,)
                        .unwrap(),
                    expected.0,
                );
                assert_eq!(
                    log_series_common_denominator_bound(&z, term_count, BoundDirection::Upper,)
                        .unwrap(),
                    expected.1,
                );
            }
        }
    }

    #[test]
    fn log_binary_split_dispatch_keeps_small_and_unit_recurrences() {
        let nonunit = rational(3, 10);
        assert!(matches!(
            log_series_evaluation(&nonunit, LOG_BINARY_SPLIT_THRESHOLD - 1, false).unwrap(),
            LogSeriesEvaluation::Recurrence(_),
        ));
        assert!(matches!(
            log_series_evaluation(&nonunit, LOG_BINARY_SPLIT_THRESHOLD, true).unwrap(),
            LogSeriesEvaluation::BinarySplit(_),
        ));
        let LogSeriesEvaluation::BinarySplit(lower_only) =
            log_series_evaluation(&nonunit, LOG_BINARY_SPLIT_THRESHOLD, false).unwrap()
        else {
            panic!("threshold nonunit log must use binary splitting");
        };
        assert!(lower_only.last_product_numerator.is_none());
        assert!(matches!(
            log_series_evaluation(&rational(1, 3), log_series_terms(256).unwrap(), true,).unwrap(),
            LogSeriesEvaluation::Recurrence(_),
        ));
        assert!(matches!(
            log_series_evaluation(&Rational::zero(), log_series_terms(256).unwrap(), true).unwrap(),
            LogSeriesEvaluation::Recurrence(_),
        ));
        assert!(matches!(
            log_binary_split_state(&nonunit, u32::MAX, true),
            Err(IntervalError::ExponentTooLarge),
        ));
    }

    #[test]
    fn direct_log_series_argument_matches_rational_operations() {
        for value in [
            Rational::one(),
            rational(4, 3),
            rational(3, 2),
            rational(5, 3),
            rational(2, 1),
        ] {
            let expected = divide_rational(
                &value.subtract(&Rational::one()),
                &value.add(&Rational::one()),
            )
            .unwrap();
            assert_eq!(log_series_argument(&value).unwrap(), expected);
        }
    }

    #[test]
    fn scalar_log_range_reduction_matches_rational_operations() {
        let two = rational_integer(2);
        for value in [
            rational(3, 4_096),
            rational(3, 16),
            rational(3, 4),
            Rational::one(),
            rational(3, 2),
            rational(2, 1),
            rational(3, 1),
            rational(8, 1),
            rational(12_288, 5),
        ] {
            let mut expected = value.clone();
            let mut expected_exponent = 0_i64;
            while compare_rationals(&expected, &two) != Ordering::Less {
                expected = divide_rational(&expected, &two).unwrap();
                expected_exponent += 1;
            }
            while compare_rationals(&expected, &Rational::one()) == Ordering::Less {
                expected = expected.multiply(&two);
                expected_exponent -= 1;
            }
            assert_eq!(
                reduce_log_argument_to_unit_range(&value).unwrap(),
                (expected, expected_exponent),
            );
        }
    }

    #[test]
    fn canonical_log_scaling_matches_general_rational_operations() {
        for value in [
            rational(1, 8),
            rational(3, 8),
            rational(1, 3),
            rational(2, 3),
            rational(3, 1),
            rational(8, 3),
            rational(1_024, 3),
            rational(3, 1_024),
        ] {
            assert_eq!(
                halve_log_range_rational(&value),
                divide_rational(&value, &rational_integer(2)).unwrap(),
            );
            assert_eq!(
                double_log_range_rational(&value),
                value.multiply(&rational_integer(2)),
            );
        }
    }

    #[test]
    fn structural_log_range_comparisons_match_rational_constants() {
        let one = Rational::one();
        let two = rational_integer(2);
        for value in [
            rational(1, 8),
            rational(3, 4),
            Rational::one(),
            rational(3, 2),
            rational(2, 1),
            rational(9, 4),
            rational(8, 1),
        ] {
            assert_eq!(
                compare_positive_rational_to_one(&value),
                compare_rationals(&value, &one),
            );
            assert_eq!(
                compare_positive_rational_to_two(&value),
                compare_rationals(&value, &two),
            );
        }
    }

    #[test]
    fn unit_range_exponential_uses_small_series_bounds_directly() {
        let series_calls = Cell::new(0_u32);
        let power_calls = Cell::new(0_u32);
        let reduced_inputs = RefCell::new(Vec::new());
        let bounds = |value: &Rational, _| {
            series_calls.set(series_calls.get() + 1);
            reduced_inputs.borrow_mut().push(value.clone());
            Ok((value.clone(), value.clone()))
        };
        let power = |value: &Rational, _| {
            power_calls.set(power_calls.get() + 1);
            Ok(value.clone())
        };

        exp_nonnegative_rational_bounds_with(&rational(1, 3), 64, bounds, power).unwrap();
        assert_eq!(series_calls.get(), 1);
        assert_eq!(power_calls.get(), 0);
        assert_eq!(reduced_inputs.borrow().as_slice(), &[rational(1, 3)]);

        series_calls.set(0);
        power_calls.set(0);
        reduced_inputs.borrow_mut().clear();
        exp_nonnegative_rational_bounds_with(&rational(3, 2), 64, bounds, power).unwrap();
        assert_eq!(series_calls.get(), 1);
        assert_eq!(power_calls.get(), 2);
        assert_eq!(reduced_inputs.borrow().as_slice(), &[rational(3, 4)]);

        assert_eq!(
            exp_rational_bounds(&Rational::zero(), 64).unwrap(),
            (Rational::one(), Rational::one())
        );
        let positive = rational(1, 3);
        let (positive_lower, positive_upper) =
            exp_nonnegative_rational_bounds(&positive, 64).unwrap();
        assert_eq!(
            exp_rational_bounds(&positive.negate(), 64).unwrap(),
            (
                divide_rational(&Rational::one(), &positive_upper).unwrap(),
                divide_rational(&Rational::one(), &positive_lower).unwrap()
            )
        );
    }

    #[test]
    fn exponential_series_uses_minimal_factorial_tail_bound() {
        for precision_bits in [0, 1, 16, 128, 256] {
            let plan = exp_series_plan(precision_bits).unwrap();
            let term_count = plan.term_count;
            let next_factor = term_count.checked_add(1).unwrap();
            let factorial = (1..=next_factor)
                .map(BigInt::from)
                .fold(BigInt::one(), |product, factor| product * factor);
            let target = BigInt::one() << (precision_bits + 1);
            assert!(factorial >= target);
            if term_count > 0 {
                assert!(&factorial / BigInt::from(next_factor) < target);
            }
            assert_eq!(plan.factorial, factorial / next_factor);
        }
        assert!(exp_series_terms(128).unwrap() < series_terms(128).unwrap());
        assert_eq!(
            exp_series_terms(u32::MAX),
            Err(IntervalError::ExponentTooLarge)
        );
        assert_eq!(
            exp_rational_bound_with_terms(&Rational::zero(), u32::MAX, BoundDirection::Lower,),
            Ok(Rational::one()),
        );
        assert_eq!(
            exp_rational_bound_with_terms(&Rational::one(), u32::MAX, BoundDirection::Lower,),
            Err(IntervalError::ExponentTooLarge),
        );

        for precision_bits in [0, 1, 16, 128] {
            let maximum_width =
                rational_from_parts(BigInt::one(), BigInt::one() << precision_bits).unwrap();
            for value in [Rational::zero(), rational(1, 3), Rational::one()] {
                let (lower, upper) =
                    exp_small_nonnegative_rational_bounds(&value, precision_bits).unwrap();
                assert!(compare_rationals(&lower, &Rational::one()) != Ordering::Less);
                assert!(compare_rationals(&upper, &rational(3, 1)) != Ordering::Greater);
                assert!(
                    compare_rationals(&upper.subtract(&lower), &maximum_width) != Ordering::Greater
                );
            }
        }
    }

    #[test]
    fn trigonometric_series_uses_minimal_alternating_tail_bound() {
        for precision_bits in [0, 1, 16, 128, 256] {
            let term_count = trigonometric_series_terms(precision_bits).unwrap();
            let next_cosine_exponent = term_count * 2 + 2;
            let factorial = (1..=next_cosine_exponent)
                .map(BigInt::from)
                .fold(BigInt::one(), |product, factor| product * factor);
            let target = BigInt::one() << precision_bits;
            assert!(factorial >= target);
            if term_count > 0 {
                let previous_factorial = &factorial
                    / BigInt::from(next_cosine_exponent - 1)
                    / BigInt::from(next_cosine_exponent);
                assert!(previous_factorial < target);
            }
        }

        for precision_bits in [0, 1, 16, 128] {
            let maximum_width =
                rational_from_parts(BigInt::one(), BigInt::one() << precision_bits).unwrap();
            for value in [Rational::zero(), rational(1, 3), Rational::one()] {
                for (lower, upper) in [
                    sin_unit_rational_bounds(&value, precision_bits).unwrap(),
                    cos_unit_rational_bounds(&value, precision_bits).unwrap(),
                ] {
                    assert!(compare_rationals(&lower, &Rational::zero()) != Ordering::Less);
                    assert!(compare_rationals(&upper, &Rational::one()) != Ordering::Greater);
                    assert!(
                        compare_rationals(&upper.subtract(&lower), &maximum_width)
                            != Ordering::Greater
                    );
                }
            }
        }
        assert!(trigonometric_series_terms(128).unwrap() < series_terms(128).unwrap());
    }

    #[test]
    fn arithmetic_intervals_contain_exact_rational_results() {
        let left_value = rational(3, 10);
        let right_value = rational(1, 5);
        let negative_right_value = right_value.negate();
        let left = from_rational(&left_value, 24);
        let right = from_rational(&right_value, 24);
        let negative_right = from_rational(&negative_right_value, 24);

        assert!(contains_rational(&add(&left, &right).unwrap(), &rational(1, 2)).unwrap());
        assert!(
            contains_rational(&add(&left, &negative_right).unwrap(), &rational(1, 10)).unwrap()
        );
        assert!(contains_rational(&multiply(&left, &right).unwrap(), &rational(3, 50)).unwrap());
        assert!(contains_rational(&divide(&left, &right, 24).unwrap(), &rational(3, 2)).unwrap());
        assert!(contains_rational(&pow_i64(&left, 2, 24).unwrap(), &rational(9, 100)).unwrap());
    }

    #[test]
    fn common_denominator_exp_series_matches_rational_recurrence() {
        for value in [
            Rational::zero(),
            rational(1, 3),
            rational(9, 10),
            Rational::one(),
        ] {
            for term_count in [0, 1, 5, 20] {
                let mut expected_sum = Rational::zero();
                let mut expected_term = Rational::one();
                for n in 0..=term_count {
                    expected_sum = expected_sum.add(&expected_term);
                    let next_n = n + 1;
                    expected_term = divide_rational(
                        &expected_term.multiply(&value),
                        &rational_integer(i64::from(next_n)),
                    )
                    .unwrap();
                }
                assert_eq!(
                    exp_series_rational_bounds(&value, term_count).unwrap(),
                    (
                        expected_sum.clone(),
                        expected_sum.add(&scale_rational(&expected_term, 2))
                    )
                );
            }
        }
        assert_eq!(
            exp_series_rational_bounds(&Rational::one(), u32::MAX),
            Err(IntervalError::ExponentTooLarge)
        );
    }

    #[test]
    fn sqrt_interval_contains_irrational_square_root() {
        let interval = sqrt(&from_rational(&rational(2, 1), 32), 32).unwrap();
        let squared = multiply(&interval, &interval).unwrap();
        assert!(contains_rational(&squared, &rational(2, 1)).unwrap());
    }

    #[test]
    fn exact_point_sqrt_shares_scaled_bounds_without_changing_results() {
        for precision_bits in [1, 32, 128] {
            for value in [
                Rational::zero(),
                rational(1, 2),
                Rational::one(),
                rational(3, 2),
                rational(2, 1),
                rational(4, 1),
            ] {
                let input = from_rational(&value, precision_bits);
                assert_eq!(input.lower, input.upper);
                let (lower, upper) = sqrt_dyadic_bounds(&input.lower, precision_bits).unwrap();
                assert_eq!(
                    (lower, upper),
                    (
                        sqrt_dyadic_lower(&input.lower, precision_bits).unwrap(),
                        sqrt_dyadic_upper(&input.upper, precision_bits).unwrap()
                    )
                );
            }

            let subscale_denominator = BigInt::one() << (2 * precision_bits + 1);
            for numerator in [1, 3] {
                let value =
                    rational_from_parts(BigInt::from(numerator), subscale_denominator.clone())
                        .unwrap();
                assert_eq!(
                    sqrt_rational_bounds(&value, precision_bits).unwrap(),
                    (
                        sqrt_rational_lower(&value, precision_bits).unwrap(),
                        sqrt_rational_upper(&value, precision_bits).unwrap()
                    )
                );
            }
        }
        assert_eq!(
            sqrt_dyadic_bounds(&from_rational(&rational(2, 1), 1).lower, u32::MAX),
            Err(IntervalError::ExponentTooLarge)
        );
    }

    #[test]
    fn rational_power_interval_contains_irrational_roots() {
        let square_root = pow_rational(&rational(2, 1), &rational(1, 2), 32).unwrap();
        let squared = pow_i64(&square_root, 2, 32).unwrap();
        assert!(contains_rational(&squared, &rational(2, 1)).unwrap());

        let cube_root = pow_rational(&rational(-2, 1), &rational(1, 3), 32).unwrap();
        let cubed = pow_i64(&cube_root, 3, 32).unwrap();
        assert!(contains_rational(&cubed, &rational(-2, 1)).unwrap());
    }

    #[test]
    fn positive_base_general_power_interval_uses_exp_log_composition() {
        let exponent = pow_rational(&rational(2, 1), &rational(1, 2), 96).unwrap();
        let two_to_sqrt_two =
            pow_positive_base(&from_rational(&rational(2, 1), 96), &exponent, 96).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&two_to_sqrt_two.lower, &rational(2, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&two_to_sqrt_two.upper, &rational(3, 1)).unwrap(),
            Ordering::Less
        );

        let sqrt_two_to_sqrt_two = pow_positive_base(&exponent, &exponent, 96).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&sqrt_two_to_sqrt_two.lower, &rational(1, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&sqrt_two_to_sqrt_two.upper, &rational(2, 1)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn sqrt_interval_rejects_negative_domain() {
        assert_eq!(
            sqrt(&from_rational(&rational(-1, 1), 16), 16),
            Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative))
        );
    }

    #[test]
    fn e_interval_is_inside_coarse_known_bounds() {
        let interval = constant(Constant::Euler, 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&interval.lower, &rational(2, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&interval.upper, &rational(3, 1)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn exp_interval_is_inside_coarse_known_bounds() {
        let interval = exp(&from_rational(&rational(2, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&interval.lower, &rational(7, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&interval.upper, &rational(8, 1)).unwrap(),
            Ordering::Less
        );

        let reciprocal = exp(&from_rational(&rational(-2, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&reciprocal.lower, &rational(1, 8)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&reciprocal.upper, &rational(1, 7)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn large_exponent_binary_scaling_preserves_positive_monotone_bounds() {
        let precision_bits = 128;
        let values = [
            -10001, -10000, -9999, -4097, -4096, -2, -1, 0, 1, 2, 4096, 4097, 9999, 10000, 10001,
        ];
        let intervals = values
            .iter()
            .map(|value| {
                exp(
                    &from_rational(&rational(*value, 1), precision_bits),
                    precision_bits,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        for interval in &intervals {
            assert!(interval.lower.coefficient.inner.is_positive());
            assert!(interval.upper.coefficient.inner.is_positive());
            assert!(compare_dyadic(&interval.lower, &interval.upper).unwrap() != Ordering::Greater);
            assert!(interval.lower.coefficient.inner.bits() <= u64::from(precision_bits + 32));
            assert!(interval.upper.coefficient.inner.bits() <= u64::from(precision_bits + 32));
        }
        for pair in intervals.windows(2) {
            assert!(compare_dyadic(&pair[0].upper, &pair[1].lower).unwrap() == Ordering::Less);
        }

        for magnitude in [2, 4096, 10000] {
            let negative = exp(
                &from_rational(&rational(-magnitude, 1), precision_bits),
                precision_bits,
            )
            .unwrap();
            let positive = exp(
                &from_rational(&rational(magnitude, 1), precision_bits),
                precision_bits,
            )
            .unwrap();
            let product = multiply(&negative, &positive).unwrap();
            assert!(contains_rational(&product, &Rational::one()).unwrap());
        }
        assert_eq!(
            exp(
                &from_rational(&rational(1_000_001, 1), precision_bits),
                precision_bits
            ),
            Err(IntervalError::ExponentTooLarge)
        );
    }

    #[test]
    fn exact_binary_exponential_plan_matches_independent_directions() {
        for precision_bits in [64, 128] {
            for value in [
                rational(-10000, 1),
                rational(-65, 1),
                rational(65, 1),
                rational(10000, 1),
            ] {
                assert_eq!(
                    exp(&from_rational(&value, precision_bits), precision_bits,).unwrap(),
                    CertifiedInterval {
                        lower: exp_binary_scaled_bound(
                            &value,
                            precision_bits,
                            BoundDirection::Lower,
                        )
                        .unwrap(),
                        upper: exp_binary_scaled_bound(
                            &value,
                            precision_bits,
                            BoundDirection::Upper,
                        )
                        .unwrap(),
                    },
                );
            }
        }
    }

    #[test]
    fn log_interval_is_inside_coarse_known_bounds() {
        let interval = log(&from_rational(&rational(2, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&interval.lower, &rational(1, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&interval.upper, &rational(1, 1)).unwrap(),
            Ordering::Less
        );

        let negative = log(&from_rational(&rational(1, 2), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&negative.lower, &rational(-1, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&negative.upper, &rational(-1, 2)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn atan_interval_is_inside_coarse_known_bounds() {
        let small = atan(&from_rational(&rational(1, 3), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&small.lower, &rational(0, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&small.upper, &rational(1, 2)).unwrap(),
            Ordering::Less
        );

        let large = atan(&from_rational(&rational(2, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&large.lower, &rational(1, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&large.upper, &rational(3, 2)).unwrap(),
            Ordering::Less
        );

        let negative = atan(&from_rational(&rational(-2, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&negative.lower, &rational(-3, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&negative.upper, &rational(-1, 1)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn tan_interval_is_inside_coarse_known_bounds() {
        let positive = tan(&from_rational(&rational(1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&positive.lower, &rational(3, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&positive.upper, &rational(8, 5)).unwrap(),
            Ordering::Less
        );

        let small = tan(&from_rational(&rational(1, 3), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&small.lower, &rational(1, 3)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&small.upper, &rational(1, 2)).unwrap(),
            Ordering::Less
        );

        let negative = tan(&from_rational(&rational(-1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&negative.lower, &rational(-8, 5)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&negative.upper, &rational(-3, 2)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn tan_interval_keeps_unsupported_range_separate_from_pole_domain() {
        assert_eq!(
            tan(
                &from_rational_bounds(&rational(0, 1), &rational(2, 1), 16).unwrap(),
                16,
            ),
            Err(IntervalError::UnsupportedExpression)
        );
    }

    #[test]
    fn sin_cos_unit_intervals_are_inside_coarse_known_bounds() {
        let sine = sin(&from_rational(&rational(1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&sine.lower, &rational(4, 5)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&sine.upper, &rational(9, 10)).unwrap(),
            Ordering::Less
        );

        let negative_sine = sin(&from_rational(&rational(-1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&negative_sine.lower, &rational(-9, 10)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&negative_sine.upper, &rational(-4, 5)).unwrap(),
            Ordering::Less
        );

        let cosine = cos(&from_rational(&rational(1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&cosine.lower, &rational(1, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&cosine.upper, &rational(3, 5)).unwrap(),
            Ordering::Less
        );

        let crossing_zero = cos(
            &from_rational_bounds(&rational(-1, 2), &rational(1, 3), 128).unwrap(),
            128,
        )
        .unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&crossing_zero.lower, &rational(4, 5)).unwrap(),
            Ordering::Greater
        );
        assert!(contains_rational(&crossing_zero, &rational(1, 1)).unwrap());
    }

    #[test]
    fn periodic_sin_cos_intervals_include_internal_extrema() {
        let sine = sin(
            &from_rational_bounds(&rational(0, 1), &rational(2, 1), 128).unwrap(),
            128,
        )
        .unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&sine.lower, &rational(0, 1)).unwrap(),
            Ordering::Equal
        );
        assert!(contains_rational(&sine, &rational(1, 1)).unwrap());

        let cosine = cos(
            &from_rational_bounds(&rational(0, 1), &rational(4, 1), 128).unwrap(),
            128,
        )
        .unwrap();
        assert!(contains_rational(&cosine, &rational(-1, 1)).unwrap());
        assert!(contains_rational(&cosine, &rational(1, 1)).unwrap());
    }

    #[test]
    fn wide_sin_cos_intervals_still_use_full_range() {
        let interval = sin(
            &from_rational_bounds(&rational(0, 1), &rational(7, 1), 128).unwrap(),
            128,
        )
        .unwrap();
        assert!(contains_rational(&interval, &rational(-1, 1)).unwrap());
        assert!(contains_rational(&interval, &rational(1, 1)).unwrap());
    }

    #[test]
    fn periodic_tan_intervals_detect_possible_poles_and_monotone_branches() {
        assert_eq!(
            tan(
                &from_rational_bounds(&rational(1, 1), &rational(2, 1), 128).unwrap(),
                128,
            ),
            Err(IntervalError::UnsupportedExpression)
        );

        let branch = tan(
            &from_rational_bounds(&rational(2, 1), &rational(3, 1), 128).unwrap(),
            128,
        )
        .unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&branch.lower, &rational(-9, 4)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&branch.upper, &rational(-1, 10)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn rational_point_trig_range_reduction_is_inside_coarse_known_bounds() {
        let sine = sin_rational(&rational(2, 1), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&sine.lower, &rational(9, 10)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&sine.upper, &rational(1, 1)).unwrap(),
            Ordering::Less
        );

        let cosine = cos_rational(&rational(2, 1), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&cosine.lower, &rational(-1, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&cosine.upper, &rational(-2, 5)).unwrap(),
            Ordering::Less
        );

        let tangent = tan_rational(&rational(2, 1), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&tangent.lower, &rational(-9, 4)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&tangent.upper, &rational(-2, 1)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn asin_acos_intervals_are_inside_coarse_known_bounds() {
        let asin_positive = asin(&from_rational(&rational(1, 2), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&asin_positive.lower, &rational(1, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&asin_positive.upper, &rational(2, 3)).unwrap(),
            Ordering::Less
        );

        let asin_negative = asin(&from_rational(&rational(-1, 2), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&asin_negative.lower, &rational(-2, 3)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&asin_negative.upper, &rational(-1, 2)).unwrap(),
            Ordering::Less
        );

        let acos_positive = acos(&from_rational(&rational(1, 3), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&acos_positive.lower, &rational(1, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&acos_positive.upper, &rational(3, 2)).unwrap(),
            Ordering::Less
        );

        let acos_negative = acos(&from_rational(&rational(-1, 3), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&acos_negative.lower, &rational(3, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&acos_negative.upper, &rational(2, 1)).unwrap(),
            Ordering::Less
        );

        let asin_endpoint = asin(&from_rational(&rational(1, 1), 128), 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&asin_endpoint.lower, &rational(3, 2)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&asin_endpoint.upper, &rational(8, 5)).unwrap(),
            Ordering::Less
        );

        let acos_endpoint = acos(&from_rational(&rational(1, 1), 128), 128).unwrap();
        assert!(contains_rational(&acos_endpoint, &rational(0, 1)).unwrap());
    }

    #[test]
    fn asin_acos_intervals_reject_proven_out_of_range_domain() {
        assert_eq!(
            asin(&from_rational(&rational(2, 1), 16), 16),
            Err(IntervalError::Domain(
                DomainErrorKind::InverseTrigonometricOutOfRange
            ))
        );
        assert_eq!(
            acos(&from_rational(&rational(-2, 1), 16), 16),
            Err(IntervalError::Domain(
                DomainErrorKind::InverseTrigonometricOutOfRange
            ))
        );
        assert_eq!(
            asin(
                &from_rational_bounds(&rational(-2, 1), &rational(0, 1), 16).unwrap(),
                16,
            ),
            Err(IntervalError::UnsupportedExpression)
        );
    }

    #[test]
    fn inverse_trig_structural_domain_units_preserve_interval_classes() {
        for (lower, upper) in [
            (rational(-3, 1), rational(-2, 1)),
            (rational(2, 1), rational(3, 1)),
        ] {
            let interval = from_rational_bounds(&lower, &upper, 16).unwrap();
            assert_eq!(
                inverse_sine_cosine_domain_bounds(&interval),
                Err(IntervalError::Domain(
                    DomainErrorKind::InverseTrigonometricOutOfRange
                )),
            );
        }

        for (lower, upper) in [
            (rational(-2, 1), Rational::zero()),
            (Rational::zero(), rational(2, 1)),
        ] {
            let interval = from_rational_bounds(&lower, &upper, 16).unwrap();
            assert_eq!(
                inverse_sine_cosine_domain_bounds(&interval),
                Err(IntervalError::UnsupportedExpression),
            );
        }

        let lower = rational(-1, 1);
        let upper = Rational::one();
        let interval = from_rational_bounds(&lower, &upper, 16).unwrap();
        assert_eq!(
            inverse_sine_cosine_domain_bounds(&interval).unwrap(),
            (lower, upper),
        );
    }

    #[test]
    fn log_interval_rejects_proven_non_positive_domain() {
        assert_eq!(
            log(&from_rational(&rational(-1, 1), 16), 16),
            Err(IntervalError::Domain(
                DomainErrorKind::LogarithmOfNonPositive
            ))
        );
        assert_eq!(
            log(
                &from_rational_bounds(&rational(-1, 1), &rational(1, 1), 16).unwrap(),
                16,
            ),
            Err(IntervalError::UnsupportedExpression)
        );
    }

    #[test]
    fn pi_interval_is_inside_coarse_known_bounds() {
        let interval = constant(Constant::Pi, 128).unwrap();
        assert_eq!(
            compare_dyadic_to_rational(&interval.lower, &rational(3, 1)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_dyadic_to_rational(&interval.upper, &rational(22, 7)).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn multiplication_orders_negative_endpoint_candidates() {
        let left = from_rational_bounds(&rational(-2, 1), &rational(-1, 1), 8).unwrap();
        let right = from_rational_bounds(&rational(3, 1), &rational(4, 1), 8).unwrap();
        let product = multiply(&left, &right).unwrap();
        assert!(contains_rational(&product, &rational(-8, 1)).unwrap());
        assert!(contains_rational(&product, &rational(-3, 1)).unwrap());
    }

    #[test]
    fn division_preserves_zero_crossing_as_interval_error() {
        let numerator = from_rational(&rational(1, 1), 8);
        let denominator = from_rational_bounds(&rational(-1, 10), &rational(1, 10), 8).unwrap();
        assert_eq!(
            divide(&numerator, &denominator, 8),
            Err(IntervalError::DivisionByIntervalContainingZero)
        );
    }
}
