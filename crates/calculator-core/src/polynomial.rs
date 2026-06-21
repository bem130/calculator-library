use alloc::{vec, vec::Vec};

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

    pub fn negate(&self) -> Result<Self, PrimitivePolynomialConstructionError> {
        let coefficients_low_to_high = effective_coefficients(&self.coefficients_low_to_high)
            .iter()
            .map(|coefficient| Integer::from_bigint(-coefficient.inner.clone()))
            .collect();
        Self::new(coefficients_low_to_high)
    }

    pub fn add(&self, rhs: &Self) -> Result<Self, PrimitivePolynomialConstructionError> {
        add_polynomials(
            effective_coefficients(&self.coefficients_low_to_high),
            effective_coefficients(&rhs.coefficients_low_to_high),
        )
    }

    pub fn subtract(&self, rhs: &Self) -> Result<Self, PrimitivePolynomialConstructionError> {
        subtract_polynomials(
            effective_coefficients(&self.coefficients_low_to_high),
            effective_coefficients(&rhs.coefficients_low_to_high),
        )
    }

    pub fn multiply(&self, rhs: &Self) -> Result<Self, PrimitivePolynomialConstructionError> {
        let lhs = effective_coefficients(&self.coefficients_low_to_high);
        let rhs = effective_coefficients(&rhs.coefficients_low_to_high);
        if lhs.is_empty() || rhs.is_empty() {
            return Err(PrimitivePolynomialConstructionError::ZeroPolynomial);
        }

        let mut coefficients_low_to_high = vec![Integer::zero(); lhs.len() + rhs.len() - 1];
        for (lhs_index, lhs_coefficient) in lhs.iter().enumerate() {
            for (rhs_index, rhs_coefficient) in rhs.iter().enumerate() {
                coefficients_low_to_high[lhs_index + rhs_index].inner +=
                    &lhs_coefficient.inner * &rhs_coefficient.inner;
            }
        }
        Self::new(coefficients_low_to_high)
    }

    pub fn derivative(&self) -> Result<Self, PrimitivePolynomialConstructionError> {
        let coefficients = effective_coefficients(&self.coefficients_low_to_high);
        let coefficients_low_to_high = coefficients
            .iter()
            .enumerate()
            .skip(1)
            .map(|(degree, coefficient)| {
                Integer::from_bigint(&coefficient.inner * BigInt::from(degree))
            })
            .collect();
        Self::new(coefficients_low_to_high)
    }
}

fn add_polynomials(
    lhs: &[Integer],
    rhs: &[Integer],
) -> Result<PrimitivePolynomial, PrimitivePolynomialConstructionError> {
    let len = lhs.len().max(rhs.len());
    let mut coefficients_low_to_high = Vec::with_capacity(len);
    for index in 0..len {
        let lhs = lhs.get(index).map(|value| &value.inner);
        let rhs = rhs.get(index).map(|value| &value.inner);
        let coefficient = match (lhs, rhs) {
            (Some(lhs), Some(rhs)) => lhs + rhs,
            (Some(lhs), None) => lhs.clone(),
            (None, Some(rhs)) => rhs.clone(),
            (None, None) => unreachable!("index is bounded by the maximum input length"),
        };
        coefficients_low_to_high.push(Integer::from_bigint(coefficient));
    }
    PrimitivePolynomial::new(coefficients_low_to_high)
}

fn subtract_polynomials(
    lhs: &[Integer],
    rhs: &[Integer],
) -> Result<PrimitivePolynomial, PrimitivePolynomialConstructionError> {
    let len = lhs.len().max(rhs.len());
    let mut coefficients_low_to_high = Vec::with_capacity(len);
    for index in 0..len {
        let lhs = lhs.get(index).map(|value| &value.inner);
        let rhs = rhs.get(index).map(|value| &value.inner);
        let coefficient = match (lhs, rhs) {
            (Some(lhs), Some(rhs)) => lhs - rhs,
            (Some(lhs), None) => lhs.clone(),
            (None, Some(rhs)) => -rhs,
            (None, None) => unreachable!("index is bounded by the maximum input length"),
        };
        coefficients_low_to_high.push(Integer::from_bigint(coefficient));
    }
    PrimitivePolynomial::new(coefficients_low_to_high)
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

    #[test]
    fn primitive_polynomial_addition_and_subtraction_normalize_results() {
        let parabola = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let line =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            parabola.add(&line).unwrap().coefficients_low_to_high,
            integers(&[0, 1, 1])
        );
        assert_eq!(
            parabola.subtract(&line).unwrap().coefficients_low_to_high,
            integers(&[-2, -1, 1])
        );
        assert_eq!(
            parabola.subtract(&parabola),
            Err(PrimitivePolynomialConstructionError::ZeroPolynomial)
        );
    }

    #[test]
    fn primitive_polynomial_multiplication_normalizes_product() {
        let x_minus_one =
            PrimitivePolynomial::new(integers(&[-1, 1])).expect("non-zero polynomial normalizes");
        let x_plus_one =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            x_minus_one
                .multiply(&x_plus_one)
                .unwrap()
                .coefficients_low_to_high,
            integers(&[-1, 0, 1])
        );
    }

    #[test]
    fn primitive_polynomial_derivative_normalizes_and_rejects_zero_derivative() {
        let cubic = PrimitivePolynomial::new(integers(&[-1, -1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let constant =
            PrimitivePolynomial::new(integers(&[7])).expect("non-zero polynomial normalizes");

        assert_eq!(
            cubic.derivative().unwrap().coefficients_low_to_high,
            integers(&[-1, 0, 3])
        );
        assert_eq!(
            constant.derivative(),
            Err(PrimitivePolynomialConstructionError::ZeroPolynomial)
        );
    }

    #[test]
    fn primitive_polynomial_negation_preserves_positive_leading_coefficient() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, -2])).expect("non-zero polynomial normalizes");

        assert_eq!(polynomial.coefficients_low_to_high, integers(&[-1, 2]));
        assert_eq!(
            polynomial.negate().unwrap().coefficients_low_to_high,
            integers(&[-1, 2])
        );
    }
}
