use alloc::{vec, vec::Vec};

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{Signed, Zero};

use crate::types::{
    Integer, PrimitivePolynomial, PrimitivePolynomialConstructionError,
    PrimitivePolynomialDivisionError, PrimitiveSquareFreeFactor,
};

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

    pub fn primitive_pseudo_remainder(
        &self,
        divisor: &Self,
    ) -> Result<Option<Self>, PrimitivePolynomialDivisionError> {
        let remainder = pseudo_remainder_coefficients(
            effective_coefficients(&self.coefficients_low_to_high),
            effective_coefficients(&divisor.coefficients_low_to_high),
        )?;
        if remainder.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                Self::new(remainder).expect("non-zero pseudo-remainder normalizes"),
            ))
        }
    }

    pub fn exact_quotient(
        &self,
        divisor: &Self,
    ) -> Result<Option<Self>, PrimitivePolynomialDivisionError> {
        let quotient = exact_quotient_coefficients(
            effective_coefficients(&self.coefficients_low_to_high),
            effective_coefficients(&divisor.coefficients_low_to_high),
        )?;
        if quotient.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                Self::new(quotient).expect("non-zero exact quotient normalizes"),
            ))
        }
    }

    pub fn gcd(&self, rhs: &Self) -> Result<Self, PrimitivePolynomialConstructionError> {
        let lhs = effective_coefficients(&self.coefficients_low_to_high);
        let rhs = effective_coefficients(&rhs.coefficients_low_to_high);
        match (lhs.is_empty(), rhs.is_empty()) {
            (true, true) => return Err(PrimitivePolynomialConstructionError::ZeroPolynomial),
            (true, false) => return Self::new(rhs.to_vec()),
            (false, true) => return Self::new(lhs.to_vec()),
            (false, false) => {}
        }

        let mut previous = Self::new(lhs.to_vec())?;
        let mut current = Self::new(rhs.to_vec())?;
        loop {
            let Some(remainder) = previous
                .primitive_pseudo_remainder(&current)
                .expect("gcd divisor is kept non-zero")
            else {
                return Ok(current);
            };
            previous = current;
            current = remainder;
        }
    }

    pub fn square_free_part(&self) -> Result<Self, PrimitivePolynomialConstructionError> {
        let Ok(derivative) = self.derivative() else {
            return Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec());
        };
        let repeated_factor = self.gcd(&derivative)?;
        self.exact_quotient(&repeated_factor)
            .expect("gcd divides the original polynomial")
            .ok_or(PrimitivePolynomialConstructionError::ZeroPolynomial)
    }

    pub fn square_free_decomposition(
        &self,
    ) -> Result<Vec<PrimitiveSquareFreeFactor>, PrimitivePolynomialConstructionError> {
        let polynomial =
            Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec())?;
        if !is_nonconstant(&polynomial) {
            return Ok(Vec::new());
        }

        let derivative = polynomial
            .derivative()
            .expect("non-constant polynomial has a non-zero derivative in characteristic zero");
        let mut repeated_factor = polynomial.gcd(&derivative)?;
        let mut remaining_square_free = polynomial
            .exact_quotient(&repeated_factor)
            .expect("gcd divides the original polynomial")
            .ok_or(PrimitivePolynomialConstructionError::ZeroPolynomial)?;
        let mut multiplicity = 1;
        let mut factors = Vec::new();

        while is_nonconstant(&remaining_square_free) {
            let common = remaining_square_free.gcd(&repeated_factor)?;
            let factor = remaining_square_free
                .exact_quotient(&common)
                .expect("common factor divides the current square-free layer")
                .ok_or(PrimitivePolynomialConstructionError::ZeroPolynomial)?;
            if is_nonconstant(&factor) {
                factors.push(PrimitiveSquareFreeFactor {
                    factor,
                    multiplicity,
                });
            }

            remaining_square_free = common;
            repeated_factor = repeated_factor
                .exact_quotient(&remaining_square_free)
                .expect("next square-free layer divides the repeated factor")
                .ok_or(PrimitivePolynomialConstructionError::ZeroPolynomial)?;
            multiplicity += 1;
        }

        Ok(factors)
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

fn pseudo_remainder_coefficients(
    dividend: &[Integer],
    divisor: &[Integer],
) -> Result<Vec<Integer>, PrimitivePolynomialDivisionError> {
    if divisor.is_empty() {
        return Err(PrimitivePolynomialDivisionError::ZeroDivisor);
    }

    let divisor_degree = divisor.len() - 1;
    let divisor_leading = &divisor[divisor_degree].inner;
    let mut remainder = dividend.to_vec();
    trim_trailing_zeroes(&mut remainder);
    while remainder.len() > divisor_degree {
        let degree_delta = remainder.len() - divisor.len();
        let remainder_leading = remainder
            .last()
            .expect("remainder degree is at least divisor degree")
            .inner
            .clone();

        for coefficient in &mut remainder {
            coefficient.inner *= divisor_leading;
        }
        for (index, divisor_coefficient) in divisor.iter().enumerate() {
            remainder[index + degree_delta].inner -=
                &remainder_leading * &divisor_coefficient.inner;
        }
        trim_trailing_zeroes(&mut remainder);
    }
    Ok(remainder)
}

fn exact_quotient_coefficients(
    dividend: &[Integer],
    divisor: &[Integer],
) -> Result<Vec<Integer>, PrimitivePolynomialDivisionError> {
    if divisor.is_empty() {
        return Err(PrimitivePolynomialDivisionError::ZeroDivisor);
    }
    if dividend.is_empty() {
        return Ok(Vec::new());
    }
    if dividend.len() < divisor.len() {
        return Err(PrimitivePolynomialDivisionError::NotDivisible);
    }

    let divisor_degree = divisor.len() - 1;
    let divisor_leading = &divisor[divisor_degree].inner;
    let mut remainder = dividend.to_vec();
    let mut quotient = vec![Integer::zero(); dividend.len() - divisor.len() + 1];
    while remainder.len() >= divisor.len() {
        let degree_delta = remainder.len() - divisor.len();
        let leading = remainder
            .last()
            .expect("remainder degree is at least divisor degree")
            .inner
            .clone();
        let (term, remainder_after_division) = leading.div_rem(divisor_leading);
        if !remainder_after_division.is_zero() {
            return Err(PrimitivePolynomialDivisionError::NotDivisible);
        }

        quotient[degree_delta] = Integer::from_bigint(term.clone());
        for (index, divisor_coefficient) in divisor.iter().enumerate() {
            remainder[index + degree_delta].inner -= &term * &divisor_coefficient.inner;
        }
        trim_trailing_zeroes(&mut remainder);
    }

    if remainder.is_empty() {
        trim_trailing_zeroes(&mut quotient);
        Ok(quotient)
    } else {
        Err(PrimitivePolynomialDivisionError::NotDivisible)
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

fn is_nonconstant(polynomial: &PrimitivePolynomial) -> bool {
    polynomial.degree().is_some_and(|degree| degree > 0)
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

    #[test]
    fn primitive_pseudo_remainder_rejects_zero_divisor() {
        let dividend =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let zero_divisor = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            dividend.primitive_pseudo_remainder(&zero_divisor),
            Err(PrimitivePolynomialDivisionError::ZeroDivisor)
        );
    }

    #[test]
    fn primitive_pseudo_remainder_returns_none_for_exact_division() {
        let dividend = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let divisor =
            PrimitivePolynomial::new(integers(&[-1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(dividend.primitive_pseudo_remainder(&divisor), Ok(None));
    }

    #[test]
    fn primitive_pseudo_remainder_normalizes_nonzero_remainder() {
        let dividend =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let divisor =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            dividend
                .primitive_pseudo_remainder(&divisor)
                .unwrap()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[1])
        );
    }

    #[test]
    fn primitive_polynomial_gcd_finds_common_factor() {
        let left = PrimitivePolynomial::new(integers(&[-1, -1, 1, 1]))
            .expect("non-zero polynomial normalizes");
        let right = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            left.gcd(&right).unwrap().coefficients_low_to_high,
            integers(&[-1, 0, 1])
        );
        assert_eq!(
            right.gcd(&left).unwrap().coefficients_low_to_high,
            integers(&[-1, 0, 1])
        );
    }

    #[test]
    fn primitive_polynomial_gcd_handles_non_monic_common_factor() {
        let left =
            PrimitivePolynomial::new(integers(&[1, 3, 2])).expect("non-zero polynomial normalizes");
        let right = PrimitivePolynomial::new(integers(&[-1, -1, 2]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            left.gcd(&right).unwrap().coefficients_low_to_high,
            integers(&[1, 2])
        );
    }

    #[test]
    fn primitive_polynomial_gcd_returns_one_for_coprime_polynomials() {
        let left =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let right =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            left.gcd(&right).unwrap().coefficients_low_to_high,
            integers(&[1])
        );
    }

    #[test]
    fn primitive_polynomial_gcd_handles_zero_inputs() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[2, 4])).expect("non-zero polynomial normalizes");
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            polynomial.gcd(&zero).unwrap().coefficients_low_to_high,
            integers(&[1, 2])
        );
        assert_eq!(
            zero.gcd(&polynomial).unwrap().coefficients_low_to_high,
            integers(&[1, 2])
        );
        assert_eq!(
            zero.gcd(&zero),
            Err(PrimitivePolynomialConstructionError::ZeroPolynomial)
        );
    }

    #[test]
    fn exact_quotient_divides_primitive_polynomials() {
        let dividend =
            PrimitivePolynomial::new(integers(&[1, 3, 2])).expect("non-zero polynomial normalizes");
        let divisor =
            PrimitivePolynomial::new(integers(&[1, 2])).expect("non-zero polynomial normalizes");

        assert_eq!(
            dividend
                .exact_quotient(&divisor)
                .unwrap()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[1, 1])
        );
    }

    #[test]
    fn exact_quotient_reports_non_divisibility() {
        let dividend =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let divisor =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            dividend.exact_quotient(&divisor),
            Err(PrimitivePolynomialDivisionError::NotDivisible)
        );
    }

    #[test]
    fn exact_quotient_handles_zero_and_zero_divisor() {
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(zero.exact_quotient(&polynomial), Ok(None));
        assert_eq!(
            polynomial.exact_quotient(&zero),
            Err(PrimitivePolynomialDivisionError::ZeroDivisor)
        );
    }

    #[test]
    fn square_free_part_removes_repeated_factors() {
        let polynomial = PrimitivePolynomial::new(integers(&[-1, 3, -3, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial
                .square_free_part()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[-1, 1])
        );
    }

    #[test]
    fn square_free_part_removes_distinct_repeated_factors() {
        let polynomial = PrimitivePolynomial::new(integers(&[-4, 8, -1, -5, 1, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial
                .square_free_part()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[-2, 1, 1])
        );
    }

    #[test]
    fn square_free_part_keeps_square_free_polynomial() {
        let polynomial = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial
                .square_free_part()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[-1, 0, 1])
        );
    }

    #[test]
    fn square_free_part_keeps_constant_polynomial() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[7])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial
                .square_free_part()
                .unwrap()
                .coefficients_low_to_high,
            integers(&[1])
        );
    }

    #[test]
    fn square_free_decomposition_groups_factors_by_multiplicity() {
        let polynomial = PrimitivePolynomial::new(integers(&[-4, 8, -1, -5, 1, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.square_free_decomposition().unwrap(),
            vec![
                PrimitiveSquareFreeFactor {
                    factor: PrimitivePolynomial::new(integers(&[2, 1]))
                        .expect("non-zero factor normalizes"),
                    multiplicity: 2,
                },
                PrimitiveSquareFreeFactor {
                    factor: PrimitivePolynomial::new(integers(&[-1, 1]))
                        .expect("non-zero factor normalizes"),
                    multiplicity: 3,
                },
            ]
        );
    }

    #[test]
    fn square_free_decomposition_reports_square_free_polynomial_at_multiplicity_one() {
        let polynomial = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.square_free_decomposition().unwrap(),
            vec![PrimitiveSquareFreeFactor {
                factor: PrimitivePolynomial::new(integers(&[-1, 0, 1]))
                    .expect("non-zero factor normalizes"),
                multiplicity: 1,
            }]
        );
    }

    #[test]
    fn square_free_decomposition_omits_constant_polynomial() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[7])).expect("non-zero polynomial normalizes");

        assert_eq!(polynomial.square_free_decomposition().unwrap(), vec![]);
    }
}
