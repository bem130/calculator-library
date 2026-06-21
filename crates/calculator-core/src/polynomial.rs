use alloc::vec::Vec;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{Signed, Zero};

use crate::types::{Integer, PrimitivePolynomial, PrimitivePolynomialConstructionError};

impl PrimitivePolynomial {
    pub fn new(
        coefficients_low_to_high: Vec<Integer>,
    ) -> Result<Self, PrimitivePolynomialConstructionError> {
        primitive_coefficients(coefficients_low_to_high).map(|coefficients_low_to_high| Self {
            coefficients_low_to_high,
        })
    }

    pub fn degree(&self) -> Option<usize> {
        effective_coefficients(&self.coefficients_low_to_high)
            .len()
            .checked_sub(1)
    }

    pub fn leading_coefficient(&self) -> Option<&Integer> {
        effective_coefficients(&self.coefficients_low_to_high).last()
    }

    pub fn max_coefficient_bits(&self) -> u64 {
        self.coefficients_low_to_high
            .iter()
            .map(|coefficient| coefficient.inner.bits())
            .max()
            .unwrap_or(0)
    }

    pub fn evaluate_integer(&self, x: &Integer) -> Integer {
        let mut value = BigInt::zero();
        for coefficient in effective_coefficients(&self.coefficients_low_to_high)
            .iter()
            .rev()
        {
            value *= &x.inner;
            value += &coefficient.inner;
        }
        Integer::from_bigint(value)
    }
}

fn primitive_coefficients(
    mut coefficients_low_to_high: Vec<Integer>,
) -> Result<Vec<Integer>, PrimitivePolynomialConstructionError> {
    trim_trailing_zeroes(&mut coefficients_low_to_high);
    if coefficients_low_to_high.is_empty() {
        return Err(PrimitivePolynomialConstructionError::ZeroPolynomial);
    }

    let content = coefficients_low_to_high
        .iter()
        .fold(BigInt::zero(), |content, coefficient| {
            content.gcd(&coefficient.inner.abs())
        });
    debug_assert!(!content.is_zero());

    for coefficient in &mut coefficients_low_to_high {
        coefficient.inner /= &content;
    }
    if coefficients_low_to_high
        .last()
        .expect("non-zero polynomial has a leading coefficient")
        .sign()
        == Sign::Minus
    {
        for coefficient in &mut coefficients_low_to_high {
            coefficient.inner = -coefficient.inner.clone();
        }
    }
    Ok(coefficients_low_to_high)
}

fn effective_coefficients(coefficients_low_to_high: &[Integer]) -> &[Integer] {
    let mut len = coefficients_low_to_high.len();
    while len > 0 && coefficients_low_to_high[len - 1].is_zero() {
        len -= 1;
    }
    &coefficients_low_to_high[..len]
}

fn trim_trailing_zeroes(coefficients_low_to_high: &mut Vec<Integer>) {
    while coefficients_low_to_high
        .last()
        .is_some_and(Integer::is_zero)
    {
        coefficients_low_to_high.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integers(values: &[i64]) -> Vec<Integer> {
        values.iter().copied().map(Integer::from).collect()
    }

    #[test]
    fn primitive_polynomial_rejects_zero() {
        assert_eq!(
            PrimitivePolynomial::new(integers(&[0, 0])),
            Err(PrimitivePolynomialConstructionError::ZeroPolynomial)
        );
    }

    #[test]
    fn primitive_polynomial_removes_content_and_makes_leading_coefficient_positive() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 4, -6]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(polynomial.coefficients_low_to_high, integers(&[1, -2, 3]));
        assert_eq!(polynomial.degree(), Some(2));
        assert_eq!(polynomial.leading_coefficient(), Some(&Integer::from(3)));
        assert_eq!(
            polynomial.evaluate_integer(&Integer::from(2)),
            Integer::from(9)
        );
    }

    #[test]
    fn primitive_polynomial_trims_trailing_zeroes_before_normalization() {
        let polynomial = PrimitivePolynomial::new(integers(&[0, -2, 4, 0]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(polynomial.coefficients_low_to_high, integers(&[0, -1, 2]));
    }

    #[test]
    fn primitive_polynomial_methods_tolerate_noncanonical_public_fields() {
        let polynomial = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[1, 2, 0, 0]),
        };

        assert_eq!(polynomial.degree(), Some(1));
        assert_eq!(polynomial.leading_coefficient(), Some(&Integer::from(2)));
        assert_eq!(
            polynomial.evaluate_integer(&Integer::from(3)),
            Integer::from(7)
        );
        assert_eq!(polynomial.max_coefficient_bits(), 2);
    }
}
