use alloc::{vec, vec::Vec};
use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, Zero};

use crate::types::{
    Integer, PrimitivePolynomial, PrimitivePolynomialConstructionError,
    PrimitivePolynomialDivisionError, PrimitivePolynomialFactor,
    PrimitivePolynomialFactorizationIncompleteReason, PrimitivePolynomialRationalRootFactorization,
    PrimitivePolynomialResultantError, PrimitivePolynomialRootCountingError,
    PrimitivePolynomialRootIsolationError, PrimitivePolynomialSign, PrimitiveSquareFreeFactor,
    Rational, RationalInterval, RealAlgebraic, RealAlgebraicConstructionError,
    SignedPrimitivePolynomial,
};

impl RealAlgebraic {
    pub fn minimal_polynomial(&self) -> &PrimitivePolynomial {
        &self.minimal_polynomial
    }

    pub fn real_root_index(&self) -> u32 {
        self.real_root_index
    }

    pub fn isolating_interval(&self) -> &RationalInterval {
        &self.isolating_interval
    }

    pub(crate) fn from_irreducible_polynomial(
        minimal_polynomial: PrimitivePolynomial,
        isolating_interval: RationalInterval,
        max_root_isolation_steps: u32,
    ) -> Result<Self, RealAlgebraicConstructionError> {
        if !is_nonconstant(&minimal_polynomial) {
            return Err(RealAlgebraicConstructionError::ConstantPolynomial);
        }
        if isolating_interval.lower.compare(&isolating_interval.upper) != Ordering::Less {
            return Err(RealAlgebraicConstructionError::InvalidInterval);
        }
        if minimal_polynomial.evaluate_rational_sign(&isolating_interval.lower) == Sign::NoSign
            || minimal_polynomial.evaluate_rational_sign(&isolating_interval.upper) == Sign::NoSign
        {
            return Err(RealAlgebraicConstructionError::EndpointRoot);
        }
        let root_count = minimal_polynomial
            .distinct_real_root_count_in_interval(&isolating_interval)
            .map_err(RealAlgebraicConstructionError::RootCounting)?;
        if root_count != 1 {
            return Err(RealAlgebraicConstructionError::NonIsolatingInterval);
        }

        let isolated_roots = minimal_polynomial
            .isolate_real_roots(max_root_isolation_steps)
            .map_err(RealAlgebraicConstructionError::RootIsolation)?;
        let Some(root_index) = isolated_roots
            .iter()
            .position(|root| rational_intervals_overlap(root, &isolating_interval))
        else {
            return Err(RealAlgebraicConstructionError::RootIndexNotFound);
        };
        let real_root_index = u32::try_from(root_index)
            .map_err(|_| RealAlgebraicConstructionError::RootIndexOverflow)?;

        Ok(Self {
            minimal_polynomial,
            real_root_index,
            isolating_interval,
        })
    }
}

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

    pub fn evaluate_rational_sign(&self, x: &Rational) -> Sign {
        evaluate_rational_sign_coefficients(
            effective_coefficients(&self.coefficients_low_to_high),
            x,
        )
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

    pub fn sturm_sequence(
        &self,
    ) -> Result<Vec<SignedPrimitivePolynomial>, PrimitivePolynomialConstructionError> {
        let polynomial =
            Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec())?;
        let mut sequence = vec![SignedPrimitivePolynomial {
            sign: PrimitivePolynomialSign::Positive,
            polynomial,
        }];
        if !is_nonconstant(&sequence[0].polynomial) {
            return Ok(sequence);
        }

        sequence.push(SignedPrimitivePolynomial {
            sign: PrimitivePolynomialSign::Positive,
            polynomial: sequence[0]
                .polynomial
                .derivative()
                .expect("non-constant polynomial has a non-zero derivative in characteristic zero"),
        });

        loop {
            let previous = &sequence[sequence.len() - 2];
            let current = &sequence[sequence.len() - 1];
            let remainder = pseudo_remainder_coefficients(
                effective_coefficients(&previous.polynomial.coefficients_low_to_high),
                effective_coefficients(&current.polynomial.coefficients_low_to_high),
            )
            .expect("current Sturm divisor is non-zero");
            let Some(remainder) = signed_primitive_polynomial_from_coefficients(remainder)? else {
                break;
            };
            sequence.push(SignedPrimitivePolynomial {
                sign: negate_polynomial_sign(multiply_polynomial_signs(
                    previous.sign,
                    remainder.sign,
                )),
                polynomial: remainder.polynomial,
            });
        }

        Ok(sequence)
    }

    pub fn sturm_sign_variations_at_rational(
        &self,
        x: &Rational,
    ) -> Result<usize, PrimitivePolynomialConstructionError> {
        let sequence = self.sturm_sequence()?;
        Ok(sturm_sign_variations_at_rational(&sequence, x))
    }

    pub fn distinct_real_root_count_in_interval(
        &self,
        interval: &RationalInterval,
    ) -> Result<u32, PrimitivePolynomialRootCountingError> {
        if interval.lower.compare(&interval.upper) == Ordering::Greater {
            return Err(PrimitivePolynomialRootCountingError::InvalidInterval);
        }

        let sequence = self
            .sturm_sequence()
            .map_err(root_counting_construction_error)?;
        let lower_variations = sturm_sign_variations_at_rational(&sequence, &interval.lower);
        let upper_variations = sturm_sign_variations_at_rational(&sequence, &interval.upper);
        let mut count = lower_variations
            .checked_sub(upper_variations)
            .expect("Sturm variations are monotone over ordered real endpoints");
        if self.evaluate_rational_sign(&interval.lower) == Sign::NoSign {
            count += 1;
        }
        u32::try_from(count).map_err(|_| PrimitivePolynomialRootCountingError::CountOverflow)
    }

    pub fn isolate_real_roots(
        &self,
        max_steps: u32,
    ) -> Result<Vec<RationalInterval>, PrimitivePolynomialRootIsolationError> {
        let polynomial = Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec())
            .map_err(root_isolation_construction_error)?;
        if !is_nonconstant(&polynomial) {
            return Ok(Vec::new());
        }

        let bound = cauchy_root_bound(&polynomial.coefficients_low_to_high);
        let initial_interval = RationalInterval {
            lower: Rational::from_integer(Integer::from_bigint(-bound.clone())),
            upper: Rational::from_integer(Integer::from_bigint(bound)),
        };
        let mut steps = 0;
        let mut pending = vec![initial_interval];
        let mut isolated = Vec::new();

        while let Some(interval) = pending.pop() {
            consume_root_isolation_step(&mut steps, max_steps)?;
            let root_count = polynomial
                .distinct_real_root_count_in_interval(&interval)
                .map_err(root_isolation_counting_error)?;
            match root_count {
                0 => {}
                1 => isolated.push(interval),
                _ => {
                    let split = polynomial.non_root_split(&interval, &mut steps, max_steps)?;
                    pending.push(RationalInterval {
                        lower: split.clone(),
                        upper: interval.upper,
                    });
                    pending.push(RationalInterval {
                        lower: interval.lower,
                        upper: split,
                    });
                }
            }
        }

        isolated.sort_by(|left, right| left.lower.compare(&right.lower));
        Ok(isolated)
    }

    pub fn resultant(&self, rhs: &Self) -> Result<Integer, PrimitivePolynomialResultantError> {
        self.resultant_with_degree_limit(rhs, None)
    }

    pub fn resultant_bounded(
        &self,
        rhs: &Self,
        max_resultant_degree: u32,
    ) -> Result<Integer, PrimitivePolynomialResultantError> {
        self.resultant_with_degree_limit(rhs, Some(max_resultant_degree))
    }

    fn resultant_with_degree_limit(
        &self,
        rhs: &Self,
        max_resultant_degree: Option<u32>,
    ) -> Result<Integer, PrimitivePolynomialResultantError> {
        let lhs = Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec())
            .map_err(resultant_construction_error)?;
        let rhs = Self::new(effective_coefficients(&rhs.coefficients_low_to_high).to_vec())
            .map_err(resultant_construction_error)?;
        let lhs_coefficients = effective_coefficients(&lhs.coefficients_low_to_high);
        let rhs_coefficients = effective_coefficients(&rhs.coefficients_low_to_high);
        let lhs_degree = lhs_coefficients.len() - 1;
        let rhs_degree = rhs_coefficients.len() - 1;
        let matrix_size = lhs_degree
            .checked_add(rhs_degree)
            .ok_or(PrimitivePolynomialResultantError::DegreeOverflow)?;
        if let Some(max_resultant_degree) = max_resultant_degree {
            let max_resultant_degree = max_resultant_degree as usize;
            if matrix_size > max_resultant_degree {
                return Err(PrimitivePolynomialResultantError::DegreeLimitExceeded);
            }
        }

        if lhs_degree == 0 && rhs_degree == 0 {
            return Ok(Integer::one());
        }
        if lhs_degree == 0 {
            return Ok(Integer::from_bigint(pow_bigint_usize(
                &lhs_coefficients[0].inner,
                rhs_degree,
            )));
        }
        if rhs_degree == 0 {
            return Ok(Integer::from_bigint(pow_bigint_usize(
                &rhs_coefficients[0].inner,
                lhs_degree,
            )));
        }

        let mut matrix = vec![vec![BigInt::zero(); matrix_size]; matrix_size];
        let lhs_high_to_low = reversed_bigint_coefficients(lhs_coefficients);
        let rhs_high_to_low = reversed_bigint_coefficients(rhs_coefficients);
        for (row, matrix_row) in matrix.iter_mut().take(rhs_degree).enumerate() {
            matrix_row[row..row + lhs_high_to_low.len()].clone_from_slice(&lhs_high_to_low);
        }
        for (row, matrix_row) in matrix
            .iter_mut()
            .skip(rhs_degree)
            .take(lhs_degree)
            .enumerate()
        {
            matrix_row[row..row + rhs_high_to_low.len()].clone_from_slice(&rhs_high_to_low);
        }

        Ok(Integer::from_bigint(bareiss_determinant(matrix)))
    }

    pub fn factor_rational_roots(
        &self,
        max_work: u32,
    ) -> Result<PrimitivePolynomialRationalRootFactorization, PrimitivePolynomialConstructionError>
    {
        let mut current =
            Self::new(effective_coefficients(&self.coefficients_low_to_high).to_vec())?;
        let mut work = 0;
        let mut factors = Vec::new();

        while is_nonconstant(&current) {
            let factor = match find_rational_linear_factor(&current, &mut work, max_work) {
                Ok(Some(factor)) => factor,
                Ok(None) => break,
                Err(reason) => {
                    return Ok(rational_root_factorization(factors, current, Some(reason)));
                }
            };

            let mut multiplicity = 0;
            loop {
                match current.exact_quotient(&factor) {
                    Ok(Some(quotient)) => {
                        multiplicity += 1;
                        current = quotient;
                        if !is_nonconstant(&current) {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(PrimitivePolynomialDivisionError::NotDivisible) => break,
                    Err(PrimitivePolynomialDivisionError::ZeroDivisor) => {
                        unreachable!("candidate rational-root factor is non-zero")
                    }
                }
                if consume_factorization_work(&mut work, max_work).is_err() {
                    return Ok(rational_root_factorization(
                        push_factor(factors, factor, multiplicity),
                        current,
                        Some(PrimitivePolynomialFactorizationIncompleteReason::WorkLimitExceeded),
                    ));
                }
            }
            factors = push_factor(factors, factor, multiplicity);
        }

        Ok(rational_root_factorization(factors, current, None))
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

fn evaluate_rational_sign_coefficients(coefficients: &[Integer], x: &Rational) -> Sign {
    let Some((leading, remaining)) = coefficients.split_last() else {
        return Sign::NoSign;
    };

    let mut numerator = leading.inner.clone();
    let mut denominator_power = BigInt::one();
    for coefficient in remaining.iter().rev() {
        denominator_power *= &x.denominator.inner.inner;
        numerator *= &x.numerator.inner;
        numerator += &coefficient.inner * &denominator_power;
    }
    numerator.sign()
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

fn cauchy_root_bound(coefficients: &[Integer]) -> BigInt {
    let leading_coefficient = &coefficients
        .last()
        .expect("non-zero primitive polynomial has a leading coefficient")
        .inner;
    debug_assert!(leading_coefficient.sign() == Sign::Plus);

    let mut max_ratio_ceiling = BigInt::zero();
    for coefficient in &coefficients[..coefficients.len() - 1] {
        let ratio_ceiling = coefficient.inner.abs().div_ceil(leading_coefficient);
        if ratio_ceiling > max_ratio_ceiling {
            max_ratio_ceiling = ratio_ceiling;
        }
    }
    max_ratio_ceiling + 1_u8
}

fn rational_midpoint(lower: &Rational, upper: &Rational) -> Rational {
    lower
        .add(upper)
        .divide(&Rational::from_integer(Integer::from(2)))
        .expect("2 is non-zero")
}

fn rational_intervals_overlap(left: &RationalInterval, right: &RationalInterval) -> bool {
    left.lower.compare(&right.upper) != Ordering::Greater
        && right.lower.compare(&left.upper) != Ordering::Greater
}

impl PrimitivePolynomial {
    fn non_root_split(
        &self,
        interval: &RationalInterval,
        steps: &mut u32,
        max_steps: u32,
    ) -> Result<Rational, PrimitivePolynomialRootIsolationError> {
        let mut candidates = vec![(interval.lower.clone(), interval.upper.clone())];
        while let Some((lower, upper)) = candidates.pop() {
            consume_root_isolation_step(steps, max_steps)?;
            let midpoint = rational_midpoint(&lower, &upper);
            if self.evaluate_rational_sign(&midpoint) != Sign::NoSign {
                return Ok(midpoint);
            }
            candidates.push((midpoint.clone(), upper));
            candidates.push((lower, midpoint));
        }
        unreachable!("a non-zero polynomial has finitely many roots")
    }
}

fn consume_root_isolation_step(
    steps: &mut u32,
    max_steps: u32,
) -> Result<(), PrimitivePolynomialRootIsolationError> {
    if *steps >= max_steps {
        return Err(PrimitivePolynomialRootIsolationError::StepLimitExceeded);
    }
    *steps += 1;
    Ok(())
}

fn reversed_bigint_coefficients(coefficients: &[Integer]) -> Vec<BigInt> {
    coefficients
        .iter()
        .rev()
        .map(|coefficient| coefficient.inner.clone())
        .collect()
}

fn pow_bigint_usize(base: &BigInt, exponent: usize) -> BigInt {
    let mut result = BigInt::one();
    let mut factor = base.clone();
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining % 2 == 1 {
            result *= &factor;
        }
        remaining /= 2;
        if remaining > 0 {
            factor = &factor * &factor;
        }
    }
    result
}

fn bareiss_determinant(mut matrix: Vec<Vec<BigInt>>) -> BigInt {
    let size = matrix.len();
    match size {
        0 => return BigInt::one(),
        1 => return matrix[0][0].clone(),
        _ => {}
    }

    let mut sign = BigInt::one();
    let mut previous_pivot = BigInt::one();
    for pivot_index in 0..size - 1 {
        if matrix[pivot_index][pivot_index].is_zero() {
            let Some(swap_row) =
                (pivot_index + 1..size).find(|&row| !matrix[row][pivot_index].is_zero())
            else {
                return BigInt::zero();
            };
            matrix.swap(pivot_index, swap_row);
            sign = -sign;
        }

        let pivot = matrix[pivot_index][pivot_index].clone();
        for row in pivot_index + 1..size {
            for column in pivot_index + 1..size {
                matrix[row][column] = (&matrix[row][column] * &pivot
                    - &matrix[row][pivot_index] * &matrix[pivot_index][column])
                    / &previous_pivot;
            }
        }
        previous_pivot = pivot;
    }

    sign * &matrix[size - 1][size - 1]
}

fn find_rational_linear_factor(
    polynomial: &PrimitivePolynomial,
    work: &mut u32,
    max_work: u32,
) -> Result<Option<PrimitivePolynomial>, PrimitivePolynomialFactorizationIncompleteReason> {
    let coefficients = effective_coefficients(&polynomial.coefficients_low_to_high);
    let constant = &coefficients[0].inner;
    if constant.is_zero() {
        consume_factorization_work(work, max_work)?;
        return Ok(Some(
            PrimitivePolynomial::new(vec![Integer::zero(), Integer::one()])
                .expect("x is a non-zero primitive polynomial"),
        ));
    }

    let leading = &coefficients[coefficients.len() - 1].inner;
    let constant_divisors = positive_divisors_bounded(&constant.abs(), work, max_work)?;
    let leading_divisors = positive_divisors_bounded(leading, work, max_work)?;
    for numerator in &constant_divisors {
        for denominator in &leading_divisors {
            if numerator.gcd(denominator) != BigInt::one() {
                continue;
            }
            if let Some(factor) =
                rational_linear_factor_if_root(polynomial, numerator, denominator, work, max_work)?
            {
                return Ok(Some(factor));
            }
            if let Some(factor) = rational_linear_factor_if_root(
                polynomial,
                &-numerator,
                denominator,
                work,
                max_work,
            )? {
                return Ok(Some(factor));
            }
        }
    }
    Ok(None)
}

fn rational_linear_factor_if_root(
    polynomial: &PrimitivePolynomial,
    numerator: &BigInt,
    denominator: &BigInt,
    work: &mut u32,
    max_work: u32,
) -> Result<Option<PrimitivePolynomial>, PrimitivePolynomialFactorizationIncompleteReason> {
    consume_factorization_work(work, max_work)?;
    let candidate = Rational::new(
        Integer::from_bigint(numerator.clone()),
        Integer::from_bigint(denominator.clone()),
    )
    .expect("rational-root denominator is positive");
    if polynomial.evaluate_rational_sign(&candidate) != Sign::NoSign {
        return Ok(None);
    }

    Ok(Some(
        PrimitivePolynomial::new(vec![
            Integer::from_bigint(-numerator),
            Integer::from_bigint(denominator.clone()),
        ])
        .expect("rational-root linear factor is non-zero and primitive"),
    ))
}

fn positive_divisors_bounded(
    value: &BigInt,
    work: &mut u32,
    max_work: u32,
) -> Result<Vec<BigInt>, PrimitivePolynomialFactorizationIncompleteReason> {
    debug_assert!(value.sign() == Sign::Plus);
    let mut small_divisors = Vec::new();
    let mut large_divisors = Vec::new();
    let mut divisor = BigInt::one();
    while &divisor * &divisor <= *value {
        consume_factorization_work(work, max_work)?;
        let (quotient, remainder) = value.div_rem(&divisor);
        if remainder.is_zero() {
            small_divisors.push(divisor.clone());
            if quotient != divisor {
                large_divisors.push(quotient);
            }
        }
        divisor += 1_u8;
    }
    large_divisors.reverse();
    small_divisors.extend(large_divisors);
    Ok(small_divisors)
}

fn consume_factorization_work(
    work: &mut u32,
    max_work: u32,
) -> Result<(), PrimitivePolynomialFactorizationIncompleteReason> {
    if *work >= max_work {
        return Err(PrimitivePolynomialFactorizationIncompleteReason::WorkLimitExceeded);
    }
    *work += 1;
    Ok(())
}

fn push_factor(
    mut factors: Vec<PrimitivePolynomialFactor>,
    factor: PrimitivePolynomial,
    multiplicity: usize,
) -> Vec<PrimitivePolynomialFactor> {
    if multiplicity == 0 {
        return factors;
    }
    if let Some(existing) = factors
        .iter_mut()
        .find(|existing| existing.factor == factor)
    {
        existing.multiplicity += multiplicity;
    } else {
        factors.push(PrimitivePolynomialFactor {
            factor,
            multiplicity,
        });
    }
    factors
}

fn rational_root_factorization(
    factors: Vec<PrimitivePolynomialFactor>,
    residual: PrimitivePolynomial,
    incomplete_reason: Option<PrimitivePolynomialFactorizationIncompleteReason>,
) -> PrimitivePolynomialRationalRootFactorization {
    PrimitivePolynomialRationalRootFactorization {
        factors,
        residual: is_nonconstant(&residual).then_some(residual),
        incomplete_reason,
    }
}

fn signed_primitive_polynomial_from_coefficients(
    mut coefficients_low_to_high: Vec<Integer>,
) -> Result<Option<SignedPrimitivePolynomial>, PrimitivePolynomialConstructionError> {
    trim_trailing_zeroes(&mut coefficients_low_to_high);
    let Some(leading_coefficient) = coefficients_low_to_high.last() else {
        return Ok(None);
    };
    let sign = match leading_coefficient.sign() {
        Sign::Plus => PrimitivePolynomialSign::Positive,
        Sign::Minus => PrimitivePolynomialSign::Negative,
        Sign::NoSign => unreachable!("zero leading coefficients were trimmed"),
    };
    Ok(Some(SignedPrimitivePolynomial {
        sign,
        polynomial: PrimitivePolynomial::new(coefficients_low_to_high)?,
    }))
}

fn sturm_sign_variations_at_rational(
    sequence: &[SignedPrimitivePolynomial],
    x: &Rational,
) -> usize {
    let mut variations = 0;
    let mut previous_sign = Sign::NoSign;
    for polynomial in sequence {
        let sign = apply_polynomial_sign(
            polynomial.sign,
            polynomial.polynomial.evaluate_rational_sign(x),
        );
        if sign == Sign::NoSign {
            continue;
        }
        if previous_sign != Sign::NoSign && previous_sign != sign {
            variations += 1;
        }
        previous_sign = sign;
    }
    variations
}

fn apply_polynomial_sign(polynomial_sign: PrimitivePolynomialSign, value_sign: Sign) -> Sign {
    match (polynomial_sign, value_sign) {
        (_, Sign::NoSign) => Sign::NoSign,
        (PrimitivePolynomialSign::Positive, sign) => sign,
        (PrimitivePolynomialSign::Negative, Sign::Plus) => Sign::Minus,
        (PrimitivePolynomialSign::Negative, Sign::Minus) => Sign::Plus,
    }
}

fn multiply_polynomial_signs(
    left: PrimitivePolynomialSign,
    right: PrimitivePolynomialSign,
) -> PrimitivePolynomialSign {
    match (left, right) {
        (PrimitivePolynomialSign::Positive, PrimitivePolynomialSign::Positive)
        | (PrimitivePolynomialSign::Negative, PrimitivePolynomialSign::Negative) => {
            PrimitivePolynomialSign::Positive
        }
        (PrimitivePolynomialSign::Positive, PrimitivePolynomialSign::Negative)
        | (PrimitivePolynomialSign::Negative, PrimitivePolynomialSign::Positive) => {
            PrimitivePolynomialSign::Negative
        }
    }
}

fn negate_polynomial_sign(sign: PrimitivePolynomialSign) -> PrimitivePolynomialSign {
    match sign {
        PrimitivePolynomialSign::Positive => PrimitivePolynomialSign::Negative,
        PrimitivePolynomialSign::Negative => PrimitivePolynomialSign::Positive,
    }
}

fn root_counting_construction_error(
    error: PrimitivePolynomialConstructionError,
) -> PrimitivePolynomialRootCountingError {
    match error {
        PrimitivePolynomialConstructionError::ZeroPolynomial => {
            PrimitivePolynomialRootCountingError::ZeroPolynomial
        }
    }
}

fn root_isolation_construction_error(
    error: PrimitivePolynomialConstructionError,
) -> PrimitivePolynomialRootIsolationError {
    match error {
        PrimitivePolynomialConstructionError::ZeroPolynomial => {
            PrimitivePolynomialRootIsolationError::ZeroPolynomial
        }
    }
}

fn root_isolation_counting_error(
    error: PrimitivePolynomialRootCountingError,
) -> PrimitivePolynomialRootIsolationError {
    match error {
        PrimitivePolynomialRootCountingError::ZeroPolynomial => {
            PrimitivePolynomialRootIsolationError::ZeroPolynomial
        }
        PrimitivePolynomialRootCountingError::InvalidInterval => {
            unreachable!("root isolation preserves ordered interval bounds")
        }
        PrimitivePolynomialRootCountingError::CountOverflow => {
            PrimitivePolynomialRootIsolationError::CountOverflow
        }
    }
}

fn resultant_construction_error(
    error: PrimitivePolynomialConstructionError,
) -> PrimitivePolynomialResultantError {
    match error {
        PrimitivePolynomialConstructionError::ZeroPolynomial => {
            PrimitivePolynomialResultantError::ZeroPolynomial
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integers(values: &[i64]) -> Vec<Integer> {
        values.iter().copied().map(Integer::from).collect()
    }

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("test rational denominator is non-zero")
    }

    fn rational_interval(
        lower_numerator: i64,
        lower_denominator: i64,
        upper_numerator: i64,
        upper_denominator: i64,
    ) -> RationalInterval {
        RationalInterval {
            lower: rational(lower_numerator, lower_denominator),
            upper: rational(upper_numerator, upper_denominator),
        }
    }

    fn assert_isolates_one_root(polynomial: &PrimitivePolynomial, interval: &RationalInterval) {
        assert_eq!(interval.lower.compare(&interval.upper), Ordering::Less);
        assert_ne!(
            polynomial.evaluate_rational_sign(&interval.lower),
            Sign::NoSign
        );
        assert_ne!(
            polynomial.evaluate_rational_sign(&interval.upper),
            Sign::NoSign
        );
        assert_eq!(
            polynomial
                .distinct_real_root_count_in_interval(interval)
                .unwrap(),
            1
        );
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
        assert_eq!(
            polynomial.evaluate_rational_sign(&rational(-1, 2)),
            Sign::NoSign
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

    #[test]
    fn sturm_sequence_preserves_negative_remainder_sign() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.sturm_sequence().unwrap(),
            vec![
                SignedPrimitivePolynomial {
                    sign: PrimitivePolynomialSign::Positive,
                    polynomial: PrimitivePolynomial::new(integers(&[1, 0, 1]))
                        .expect("non-zero polynomial normalizes"),
                },
                SignedPrimitivePolynomial {
                    sign: PrimitivePolynomialSign::Positive,
                    polynomial: PrimitivePolynomial::new(integers(&[0, 1]))
                        .expect("non-zero polynomial normalizes"),
                },
                SignedPrimitivePolynomial {
                    sign: PrimitivePolynomialSign::Negative,
                    polynomial: PrimitivePolynomial::new(integers(&[1]))
                        .expect("non-zero polynomial normalizes"),
                },
            ]
        );
    }

    #[test]
    fn sturm_variations_use_exact_rational_signs() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.evaluate_rational_sign(&rational(3, 2)),
            Sign::Plus
        );
        assert_eq!(
            polynomial.evaluate_rational_sign(&rational(-3, 2)),
            Sign::Plus
        );
        assert_eq!(
            polynomial.evaluate_rational_sign(&rational(1, 1)),
            Sign::Minus
        );
        assert_eq!(
            polynomial
                .sturm_sign_variations_at_rational(&rational(-2, 1))
                .unwrap(),
            2
        );
        assert_eq!(
            polynomial
                .sturm_sign_variations_at_rational(&rational(2, 1))
                .unwrap(),
            0
        );
    }

    #[test]
    fn sturm_root_count_counts_distinct_closed_interval_roots() {
        let two_real_roots = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let no_real_roots =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let three_real_roots = PrimitivePolynomial::new(integers(&[0, -1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let repeated_root = PrimitivePolynomial::new(integers(&[1, -2, 1]))
            .expect("non-zero polynomial normalizes");
        let endpoint_root =
            PrimitivePolynomial::new(integers(&[0, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            two_real_roots
                .distinct_real_root_count_in_interval(&rational_interval(-2, 1, 2, 1))
                .unwrap(),
            2
        );
        assert_eq!(
            two_real_roots
                .distinct_real_root_count_in_interval(&rational_interval(0, 1, 2, 1))
                .unwrap(),
            1
        );
        assert_eq!(
            no_real_roots
                .distinct_real_root_count_in_interval(&rational_interval(-10, 1, 10, 1))
                .unwrap(),
            0
        );
        assert_eq!(
            three_real_roots
                .distinct_real_root_count_in_interval(&rational_interval(-2, 1, 2, 1))
                .unwrap(),
            3
        );
        assert_eq!(
            repeated_root
                .distinct_real_root_count_in_interval(&rational_interval(0, 1, 2, 1))
                .unwrap(),
            1
        );
        assert_eq!(
            endpoint_root
                .distinct_real_root_count_in_interval(&rational_interval(0, 1, 1, 1))
                .unwrap(),
            1
        );
        assert_eq!(
            endpoint_root
                .distinct_real_root_count_in_interval(&rational_interval(-1, 1, 0, 1))
                .unwrap(),
            1
        );
    }

    #[test]
    fn sturm_root_count_rejects_invalid_inputs() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[0, 1])).expect("non-zero polynomial normalizes");
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            polynomial.distinct_real_root_count_in_interval(&rational_interval(1, 1, 0, 1)),
            Err(PrimitivePolynomialRootCountingError::InvalidInterval)
        );
        assert_eq!(
            zero.distinct_real_root_count_in_interval(&rational_interval(-1, 1, 1, 1)),
            Err(PrimitivePolynomialRootCountingError::ZeroPolynomial)
        );
    }

    #[test]
    fn isolate_real_roots_returns_ordered_single_root_intervals() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");

        let intervals = polynomial.isolate_real_roots(32).unwrap();

        assert_eq!(intervals.len(), 2);
        for interval in &intervals {
            assert_isolates_one_root(&polynomial, interval);
        }
        assert_ne!(
            intervals[0].upper.compare(&Rational::zero()),
            Ordering::Greater
        );
        assert_ne!(
            intervals[1].lower.compare(&Rational::zero()),
            Ordering::Less
        );
    }

    #[test]
    fn isolate_real_roots_skips_rootless_polynomial_and_counts_repeated_root_once() {
        let rootless =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");
        let repeated_root = PrimitivePolynomial::new(integers(&[1, -2, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(rootless.isolate_real_roots(8).unwrap(), vec![]);

        let intervals = repeated_root.isolate_real_roots(32).unwrap();
        assert_eq!(intervals.len(), 1);
        assert_isolates_one_root(&repeated_root, &intervals[0]);
    }

    #[test]
    fn isolate_real_roots_finds_non_root_split_when_midpoint_is_root() {
        let polynomial = PrimitivePolynomial::new(integers(&[0, -1, 0, 1]))
            .expect("non-zero polynomial normalizes");

        let intervals = polynomial.isolate_real_roots(128).unwrap();

        assert_eq!(intervals.len(), 3);
        for interval in &intervals {
            assert_isolates_one_root(&polynomial, interval);
        }
    }

    #[test]
    fn isolate_real_roots_reports_step_limit_and_zero_polynomial() {
        let polynomial = PrimitivePolynomial::new(integers(&[0, -1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            polynomial.isolate_real_roots(1),
            Err(PrimitivePolynomialRootIsolationError::StepLimitExceeded)
        );
        assert_eq!(
            zero.isolate_real_roots(32),
            Err(PrimitivePolynomialRootIsolationError::ZeroPolynomial)
        );
    }

    #[test]
    fn real_algebraic_constructor_computes_root_index_from_isolation() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let algebraic = RealAlgebraic::from_irreducible_polynomial(
            polynomial.clone(),
            rational_interval(1, 1, 2, 1),
            64,
        )
        .expect("positive square root interval isolates one root");

        assert_eq!(algebraic.minimal_polynomial(), &polynomial);
        assert_eq!(algebraic.real_root_index(), 1);
        assert_eq!(
            algebraic.isolating_interval(),
            &rational_interval(1, 1, 2, 1)
        );
    }

    #[test]
    fn real_algebraic_constructor_rejects_invalid_isolation_data() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            RealAlgebraic::from_irreducible_polynomial(
                PrimitivePolynomial::new(integers(&[1]))
                    .expect("non-zero constant polynomial normalizes"),
                rational_interval(0, 1, 1, 1),
                64,
            ),
            Err(RealAlgebraicConstructionError::ConstantPolynomial)
        );
        assert_eq!(
            RealAlgebraic::from_irreducible_polynomial(
                polynomial.clone(),
                rational_interval(2, 1, 1, 1),
                64,
            ),
            Err(RealAlgebraicConstructionError::InvalidInterval)
        );
        assert_eq!(
            RealAlgebraic::from_irreducible_polynomial(
                PrimitivePolynomial::new(integers(&[-1, 1]))
                    .expect("non-zero polynomial normalizes"),
                rational_interval(1, 1, 2, 1),
                64,
            ),
            Err(RealAlgebraicConstructionError::EndpointRoot)
        );
        assert_eq!(
            RealAlgebraic::from_irreducible_polynomial(
                polynomial,
                rational_interval(-2, 1, 2, 1),
                64,
            ),
            Err(RealAlgebraicConstructionError::NonIsolatingInterval)
        );
    }

    #[test]
    fn real_algebraic_equality_uses_polynomial_and_root_index() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let positive_wide = RealAlgebraic::from_irreducible_polynomial(
            polynomial.clone(),
            rational_interval(1, 1, 2, 1),
            64,
        )
        .expect("positive square root interval isolates one root");
        let positive_refined = RealAlgebraic::from_irreducible_polynomial(
            polynomial.clone(),
            rational_interval(5, 4, 3, 2),
            64,
        )
        .expect("refined positive square root interval isolates the same root");
        let negative = RealAlgebraic::from_irreducible_polynomial(
            polynomial,
            rational_interval(-2, 1, -1, 1),
            64,
        )
        .expect("negative square root interval isolates one root");

        assert_eq!(positive_wide, positive_refined);
        assert_ne!(positive_wide, negative);
    }

    #[test]
    fn resultant_matches_linear_root_difference() {
        let x_minus_two =
            PrimitivePolynomial::new(integers(&[-2, 1])).expect("non-zero polynomial normalizes");
        let x_minus_three =
            PrimitivePolynomial::new(integers(&[-3, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            x_minus_two.resultant(&x_minus_three).unwrap(),
            Integer::from(-1)
        );
        assert_eq!(
            x_minus_three.resultant(&x_minus_two).unwrap(),
            Integer::from(1)
        );
    }

    #[test]
    fn resultant_handles_non_monic_polynomials() {
        let two_x_plus_one =
            PrimitivePolynomial::new(integers(&[1, 2])).expect("non-zero polynomial normalizes");
        let x_minus_three =
            PrimitivePolynomial::new(integers(&[-3, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            two_x_plus_one.resultant(&x_minus_three).unwrap(),
            Integer::from(-7)
        );
        assert_eq!(
            x_minus_three.resultant(&two_x_plus_one).unwrap(),
            Integer::from(7)
        );
    }

    #[test]
    fn resultant_zero_detects_common_factor() {
        let parabola = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let x_minus_one =
            PrimitivePolynomial::new(integers(&[-1, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(parabola.resultant(&x_minus_one).unwrap(), Integer::zero());
    }

    #[test]
    fn resultant_handles_quadratic_linear_and_constants() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let x_minus_one =
            PrimitivePolynomial::new(integers(&[-1, 1])).expect("non-zero polynomial normalizes");
        let constant =
            PrimitivePolynomial::new(integers(&[7])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.resultant(&x_minus_one).unwrap(),
            Integer::from(-1)
        );
        assert_eq!(constant.resultant(&polynomial).unwrap(), Integer::one());
        assert_eq!(polynomial.resultant(&constant).unwrap(), Integer::one());
    }

    #[test]
    fn resultant_bounded_enforces_matrix_degree_before_computing() {
        let polynomial = PrimitivePolynomial::new(integers(&[-2, 0, 1]))
            .expect("non-zero polynomial normalizes");
        let x_minus_three =
            PrimitivePolynomial::new(integers(&[-3, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.resultant_bounded(&x_minus_three, 3).unwrap(),
            Integer::from(7)
        );
        assert_eq!(
            polynomial.resultant_bounded(&x_minus_three, 2),
            Err(PrimitivePolynomialResultantError::DegreeLimitExceeded)
        );
    }

    #[test]
    fn resultant_rejects_zero_polynomial() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, 1])).expect("non-zero polynomial normalizes");
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            polynomial.resultant(&zero),
            Err(PrimitivePolynomialResultantError::ZeroPolynomial)
        );
        assert_eq!(
            zero.resultant(&polynomial),
            Err(PrimitivePolynomialResultantError::ZeroPolynomial)
        );
    }

    #[test]
    fn factor_rational_roots_extracts_linear_integer_factors() {
        let polynomial = PrimitivePolynomial::new(integers(&[-1, 0, 1]))
            .expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.factor_rational_roots(64).unwrap(),
            PrimitivePolynomialRationalRootFactorization {
                factors: vec![
                    PrimitivePolynomialFactor {
                        factor: PrimitivePolynomial::new(integers(&[-1, 1]))
                            .expect("non-zero factor normalizes"),
                        multiplicity: 1,
                    },
                    PrimitivePolynomialFactor {
                        factor: PrimitivePolynomial::new(integers(&[1, 1]))
                            .expect("non-zero factor normalizes"),
                        multiplicity: 1,
                    },
                ],
                residual: None,
                incomplete_reason: None,
            }
        );
    }

    #[test]
    fn factor_rational_roots_extracts_non_monic_and_repeated_factors() {
        let two_x_plus_one =
            PrimitivePolynomial::new(integers(&[1, 2])).expect("non-zero factor normalizes");
        let x_minus_three =
            PrimitivePolynomial::new(integers(&[-3, 1])).expect("non-zero factor normalizes");
        let polynomial = two_x_plus_one
            .multiply(&two_x_plus_one)
            .unwrap()
            .multiply(&x_minus_three)
            .unwrap();

        assert_eq!(
            polynomial.factor_rational_roots(128).unwrap(),
            PrimitivePolynomialRationalRootFactorization {
                factors: vec![
                    PrimitivePolynomialFactor {
                        factor: two_x_plus_one,
                        multiplicity: 2,
                    },
                    PrimitivePolynomialFactor {
                        factor: x_minus_three,
                        multiplicity: 1,
                    },
                ],
                residual: None,
                incomplete_reason: None,
            }
        );
    }

    #[test]
    fn factor_rational_roots_keeps_residual_without_rational_roots() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.factor_rational_roots(64).unwrap(),
            PrimitivePolynomialRationalRootFactorization {
                factors: vec![],
                residual: Some(polynomial),
                incomplete_reason: None,
            }
        );
    }

    #[test]
    fn factor_rational_roots_extracts_zero_root_and_omits_unit_residual() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[0, 0, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.factor_rational_roots(64).unwrap(),
            PrimitivePolynomialRationalRootFactorization {
                factors: vec![PrimitivePolynomialFactor {
                    factor: PrimitivePolynomial::new(integers(&[0, 1]))
                        .expect("non-zero factor normalizes"),
                    multiplicity: 2,
                }],
                residual: None,
                incomplete_reason: None,
            }
        );
    }

    #[test]
    fn factor_rational_roots_returns_partial_result_on_work_limit() {
        let polynomial =
            PrimitivePolynomial::new(integers(&[1, 0, 1])).expect("non-zero polynomial normalizes");

        assert_eq!(
            polynomial.factor_rational_roots(0).unwrap(),
            PrimitivePolynomialRationalRootFactorization {
                factors: vec![],
                residual: Some(polynomial),
                incomplete_reason: Some(
                    PrimitivePolynomialFactorizationIncompleteReason::WorkLimitExceeded,
                ),
            }
        );
    }

    #[test]
    fn factor_rational_roots_rejects_zero_polynomial() {
        let zero = PrimitivePolynomial {
            coefficients_low_to_high: integers(&[0, 0]),
        };

        assert_eq!(
            zero.factor_rational_roots(64),
            Err(PrimitivePolynomialConstructionError::ZeroPolynomial)
        );
    }
}
