use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::types::{
    ceil_nth_root_nonnegative, ceil_sqrt_nonnegative, floor_nth_root_nonnegative,
    floor_sqrt_nonnegative, CertifiedInterval, Constant, DomainErrorKind, ExactDyadic, Integer,
    Rational,
};

const MAX_EXP_RANGE_REDUCTION_STEPS: u32 = 4096;
const MAX_LOG_RANGE_REDUCTION_STEPS: u32 = 4096;

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

    Ok(CertifiedInterval {
        lower: sqrt_dyadic_lower(&value.lower, precision_bits)?,
        upper: sqrt_dyadic_upper(&value.upper, precision_bits)?,
    })
}

pub(crate) fn exp(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let (lower, _) = exp_rational_bounds(&lower, precision_bits)?;
    let (_, upper) = exp_rational_bounds(&upper, precision_bits)?;
    from_rational_bounds(&lower, &upper, precision_bits)
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

    let (lower, _) = log_rational_bounds(&lower, precision_bits)?;
    let (_, upper) = log_rational_bounds(&upper, precision_bits)?;
    from_rational_bounds(&lower, &upper, precision_bits)
}

pub(crate) fn atan(
    value: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&value.lower)?;
    let upper = dyadic_to_rational(&value.upper)?;
    let (lower, _) = atan_rational_bounds(&lower, precision_bits)?;
    let (_, upper) = atan_rational_bounds(&upper, precision_bits)?;
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
        let lower = divide_rational(&Rational::one(), &positive_upper)?;
        let upper = divide_rational(&Rational::one(), &positive_lower)?;
        return Ok((lower, upper));
    }
    exp_nonnegative_rational_bounds(value, precision_bits)
}

fn exp_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    let reduction = ceil_nonnegative_rational_to_u32(value)?;
    let divisor = rational_integer(i64::from(reduction));
    let reduced = divide_rational(value, &divisor)?;
    let (lower, upper) = exp_small_nonnegative_rational_bounds(&reduced, precision_bits)?;
    Ok((
        pow_positive_rational(&lower, reduction)?,
        pow_positive_rational(&upper, reduction)?,
    ))
}

fn exp_small_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Greater);
    let term_count = series_terms(precision_bits)?;
    let mut sum = Rational::zero();
    let mut term = Rational::one();
    for n in 0..=term_count {
        sum = sum.add(&term);
        let next_n = n.checked_add(1).ok_or(IntervalError::ExponentTooLarge)?;
        term = divide_rational(&term.multiply(value), &rational_integer(i64::from(next_n)))?;
    }
    let upper = sum.add(&scale_rational(&term, 2));
    Ok((sum, upper))
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
            lower.add(&scale_rational(&log_two_lower, exponent_two)),
            upper.add(&scale_rational(&log_two_upper, exponent_two)),
        ))
    } else {
        Ok((
            lower.add(&scale_rational(&log_two_upper, exponent_two)),
            upper.add(&scale_rational(&log_two_lower, exponent_two)),
        ))
    }
}

fn reduce_log_argument_to_unit_range(value: &Rational) -> Result<(Rational, i64), IntervalError> {
    let mut reduced = value.clone();
    let mut exponent_two = 0_i64;
    let mut steps = 0_u32;
    let two = rational_integer(2);
    while compare_rationals(&reduced, &two) != Ordering::Less {
        guard_log_range_reduction_step(&mut steps)?;
        reduced = divide_rational(&reduced, &two)?;
        exponent_two = exponent_two
            .checked_add(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
    }
    while compare_rationals(&reduced, &Rational::one()) == Ordering::Less {
        guard_log_range_reduction_step(&mut steps)?;
        reduced = reduced.multiply(&two);
        exponent_two = exponent_two
            .checked_sub(1)
            .ok_or(IntervalError::ExponentTooLarge)?;
    }
    Ok((reduced, exponent_two))
}

fn log_reduced_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Less);
    debug_assert!(compare_rationals(value, &rational_integer(2)) != Ordering::Greater);
    let numerator = value.subtract(&Rational::one());
    let denominator = value.add(&Rational::one());
    let z = divide_rational(&numerator, &denominator)?;
    let z_squared = z.multiply(&z);
    let term_count = series_terms(precision_bits)?;
    let mut sum = Rational::zero();
    let mut term_power = z.clone();
    for k in 0..=term_count {
        let denominator = k
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        sum = sum.add(&divide_rational(
            &term_power,
            &rational_integer(i64::from(denominator)),
        )?);
        term_power = term_power.multiply(&z_squared);
    }

    let next_denominator = term_count
        .checked_add(1)
        .and_then(|value| value.checked_mul(2))
        .and_then(|value| value.checked_add(1))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next = divide_rational(&term_power, &rational_integer(i64::from(next_denominator)))?;
    let lower = scale_rational(&sum, 2);
    let upper = lower.add(&scale_rational(&next, 4));
    Ok((lower, upper))
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

fn atan_nonnegative_rational_bounds(
    value: &Rational,
    precision_bits: u32,
) -> Result<(Rational, Rational), IntervalError> {
    debug_assert!(!value.is_negative());
    if value.is_zero() {
        return Ok((Rational::zero(), Rational::zero()));
    }
    if compare_rationals(value, &Rational::one()) == Ordering::Greater {
        let reciprocal = divide_rational(&Rational::one(), value)?;
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
    debug_assert!(compare_rationals(value, &Rational::one()) != Ordering::Greater);
    let value_squared = value.multiply(value);
    let term_count = series_terms(precision_bits)?;
    let mut sum = Rational::zero();
    let mut term_power = value.clone();
    for k in 0..=term_count {
        let denominator = k
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(IntervalError::ExponentTooLarge)?;
        let term = divide_rational(&term_power, &rational_integer(i64::from(denominator)))?;
        if k.is_multiple_of(2) {
            sum = sum.add(&term);
        } else {
            sum = sum.subtract(&term);
        }
        term_power = term_power.multiply(&value_squared);
    }

    let next_denominator = term_count
        .checked_add(1)
        .and_then(|value| value.checked_mul(2))
        .and_then(|value| value.checked_add(1))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let next = divide_rational(&term_power, &rational_integer(i64::from(next_denominator)))?;
    let adjacent = if (term_count + 1).is_multiple_of(2) {
        sum.add(&next)
    } else {
        sum.subtract(&next)
    };
    if compare_rationals(&sum, &adjacent) == Ordering::Less {
        Ok((sum, adjacent))
    } else {
        Ok((adjacent, sum))
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

fn halve_rational(value: &Rational) -> Result<Rational, IntervalError> {
    divide_rational(value, &rational_integer(2))
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

fn reciprocal_interval(
    interval: &CertifiedInterval,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let lower = dyadic_to_rational(&interval.lower)?;
    let upper = dyadic_to_rational(&interval.upper)?;
    let reciprocal_lower = Rational::one()
        .divide(&upper)
        .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    let reciprocal_upper = Rational::one()
        .divide(&lower)
        .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    from_rational_bounds(&reciprocal_lower, &reciprocal_upper, precision_bits)
}

fn sqrt_dyadic_lower(
    value: &ExactDyadic,
    precision_bits: u32,
) -> Result<ExactDyadic, IntervalError> {
    let value = dyadic_to_rational(value)?;
    sqrt_rational_lower(&value, precision_bits)
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
    let mut sum = Rational::zero();
    let mut factorial = BigInt::one();
    for n in 0..=term_count {
        if n > 0 {
            factorial *= n;
        }
        sum = sum.add(&rational_from_parts(BigInt::one(), factorial.clone())?);
    }

    let next_factorial = factorial * BigInt::from(term_count + 1_u32);
    let tail_bound = rational_from_parts(BigInt::from(2_u8), next_factorial)?;
    let upper = sum.add(&tail_bound);
    Ok((sum, upper))
}

fn pi_bounds(precision_bits: u32) -> Result<(Rational, Rational), IntervalError> {
    let term_count = series_terms(precision_bits)?;
    let (atan_1_5_lower, atan_1_5_upper) = arctan_reciprocal_bounds(5, term_count)?;
    let (atan_1_239_lower, atan_1_239_upper) = arctan_reciprocal_bounds(239, term_count)?;

    let lower = scale_rational(&atan_1_5_lower, 16).subtract(&scale_rational(&atan_1_239_upper, 4));
    let upper = scale_rational(&atan_1_5_upper, 16).subtract(&scale_rational(&atan_1_239_lower, 4));
    Ok((lower, upper))
}

fn arctan_reciprocal_bounds(
    reciprocal_denominator: u32,
    term_count: u32,
) -> Result<(Rational, Rational), IntervalError> {
    let denominator_base = BigInt::from(reciprocal_denominator);
    let mut sum = Rational::zero();
    for k in 0..=term_count {
        let term = arctan_reciprocal_term(&denominator_base, k)?;
        if k % 2 == 0 {
            sum = sum.add(&term);
        } else {
            sum = sum.subtract(&term);
        }
    }

    let next = arctan_reciprocal_term(&denominator_base, term_count + 1)?;
    let adjacent = if (term_count + 1).is_multiple_of(2) {
        sum.add(&next)
    } else {
        sum.subtract(&next)
    };
    if compare_rationals(&sum, &adjacent) == Ordering::Less {
        Ok((sum, adjacent))
    } else {
        Ok((adjacent, sum))
    }
}

fn arctan_reciprocal_term(
    denominator_base: &BigInt,
    term_index: u32,
) -> Result<Rational, IntervalError> {
    let power = term_index
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(IntervalError::ExponentTooLarge)?;
    let denominator = denominator_base.pow(power) * BigInt::from(power);
    rational_from_parts(BigInt::one(), denominator)
}

fn series_terms(precision_bits: u32) -> Result<u32, IntervalError> {
    precision_bits
        .checked_div(3)
        .and_then(|value| value.checked_add(16))
        .ok_or(IntervalError::ExponentTooLarge)
}

fn rational_integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn scale_rational(value: &Rational, factor: i64) -> Rational {
    value.multiply(&rational_integer(factor))
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
    Ok(compare_rationals(
        &dyadic_to_rational(left)?,
        &dyadic_to_rational(right)?,
    ))
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

fn dyadic_to_rational(value: &ExactDyadic) -> Result<Rational, IntervalError> {
    let exponent = &value.exponent_two.inner;
    if exponent.sign() == Sign::Minus {
        let denominator = BigInt::one()
            << exponent
                .abs()
                .to_u32()
                .ok_or(IntervalError::ExponentTooLarge)?;
        Rational::new(value.coefficient.clone(), Integer::from_bigint(denominator))
            .map_err(|_| IntervalError::DivisionByIntervalContainingZero)
    } else {
        let numerator =
            &value.coefficient.inner << exponent.to_u32().ok_or(IntervalError::ExponentTooLarge)?;
        Rational::new(Integer::from_bigint(numerator), Integer::one())
            .map_err(|_| IntervalError::DivisionByIntervalContainingZero)
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
    use super::*;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
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
    fn sqrt_interval_contains_irrational_square_root() {
        let interval = sqrt(&from_rational(&rational(2, 1), 32), 32).unwrap();
        let squared = multiply(&interval, &interval).unwrap();
        assert!(contains_rational(&squared, &rational(2, 1)).unwrap());
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
