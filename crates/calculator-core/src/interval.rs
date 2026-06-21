use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::types::{
    ceil_sqrt_nonnegative, floor_sqrt_nonnegative, CertifiedInterval, DomainErrorKind, ExactDyadic,
    Integer, Rational,
};

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

pub(crate) fn contains_rational(
    interval: &CertifiedInterval,
    value: &Rational,
) -> Result<bool, IntervalError> {
    Ok(
        compare_dyadic_to_rational(&interval.lower, value)? != Ordering::Greater
            && compare_dyadic_to_rational(&interval.upper, value)? != Ordering::Less,
    )
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
    fn sqrt_interval_rejects_negative_domain() {
        assert_eq!(
            sqrt(&from_rational(&rational(-1, 1), 16), 16),
            Err(IntervalError::Domain(DomainErrorKind::EvenRootOfNegative))
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
