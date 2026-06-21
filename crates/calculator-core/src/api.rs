use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
};
use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, Zero};

use crate::expression::{evaluate_rational_dag, lower_source_expression};
use crate::types::*;

pub fn calculate(
    source: &str,
    request: &CalculationRequest,
    context: &mut EvaluationContext,
) -> Result<CalculationOutcome, CalculatorError> {
    let parsed = parse(source, &request.parse).map_err(CalculatorError::Parse)?;
    let evaluation = evaluate(
        &parsed,
        &EvaluationRequest {
            semantics: request.semantics,
            limits: request.limits.clone(),
        },
        context,
    )
    .map_err(CalculatorError::from)?;
    let calculation = present(
        &evaluation,
        &PresentationRequest {
            exact_output: request.exact_output,
            scientific_output: request.scientific_output,
            enclosure_output: request.enclosure_output,
        },
    )
    .map_err(CalculatorError::from)?;
    Ok(CalculationOutcome::Complete(calculation))
}

pub fn parse(source: &str, settings: &ParseSettings) -> Result<ParsedExpression, ParseError> {
    if source.is_empty() {
        return Err(ParseError {
            kind: ParseErrorKind::UnexpectedEnd,
            span: ByteSpan { start: 0, end: 0 },
            expected: alloc::vec![ExpectedToken {
                kind: ExpectedTokenKind::Number,
            }],
        });
    }

    let root = crate::syntax::parse_source(source, settings)?;
    Ok(ParsedExpression {
        source: String::from(source),
        settings: *settings,
        root,
    })
}

pub fn evaluate(
    expression: &ParsedExpression,
    request: &EvaluationRequest,
    _context: &mut EvaluationContext,
) -> Result<EvaluationOutcome, EvaluationError> {
    let dag = lower_source_expression(&expression.root)?;
    let rational = evaluate_rational_dag(&dag)?;
    Ok(EvaluationOutcome {
        value: EvaluatedValue {
            exact_expression: ExactExpression {
                source: expression.source.clone(),
            },
            recognized_exact: RecognizedExact::Rational(rational),
            certified_enclosure: CertifiedEnclosureState::NotRequested,
        },
        metadata: EvaluationMetadata {
            semantic_settings: request.semantics,
            methods: vec![MethodTag::RationalReduction],
            internal_precision_bits: 0,
            refinement_rounds: 0,
        },
    })
}

pub fn present(
    evaluation: &EvaluationOutcome,
    request: &PresentationRequest,
) -> Result<Calculation, PresentationError> {
    let RecognizedExact::Rational(rational) = &evaluation.value.recognized_exact else {
        return Err(PresentationError::InternalInvariant(
            InternalInvariantError {
                code: InternalInvariantCode::PresentationWithoutEvaluation,
            },
        ));
    };

    let exact = match request.exact_output {
        ExactOutputRequest::Omit => ExactOutput::Omitted,
        ExactOutputRequest::Include { .. } => ExactOutput::Included(exact_presentation(rational)),
    };
    let scientific = match request.scientific_output {
        ScientificOutputRequest::Omit => ScientificOutput::Omitted,
        ScientificOutputRequest::Include {
            significant_digits,
            rounding_mode,
        } => ScientificOutput::Included(scientific_presentation(
            rational,
            significant_digits,
            rounding_mode,
        )?),
    };
    let dyadic_precision_bits = match request.scientific_output {
        ScientificOutputRequest::Include {
            significant_digits, ..
        } => precision_bits_for_decimal_digits(significant_digits.get())?,
        ScientificOutputRequest::Omit => 128,
    };
    let enclosure = match request.enclosure_output {
        EnclosureOutputRequest::Omit => EnclosureOutput::Omitted,
        EnclosureOutputRequest::Include { format } => EnclosureOutput::Included(
            dyadic_interval_presentation(rational, format, dyadic_precision_bits)?,
        ),
    };
    let confirmed_significant_digits = match &scientific {
        ScientificOutput::Included(value) => value.confirmed_significant_digits,
        ScientificOutput::Omitted | ScientificOutput::Unavailable(_) => 0,
    };
    let mut methods = evaluation.metadata.methods.clone();
    if matches!(enclosure, EnclosureOutput::Included(_))
        && !methods.contains(&MethodTag::CertifiedIntervalEvaluation)
    {
        methods.push(MethodTag::CertifiedIntervalEvaluation);
    }

    Ok(Calculation {
        exact,
        scientific,
        enclosure,
        metadata: CalculationMetadata {
            exact_representation: exact_representation_kind(rational),
            simplification_status: SimplificationStatus::FullySimplifiedWithinLimits,
            semantic_settings: evaluation.metadata.semantic_settings,
            methods,
            internal_precision_bits: if matches!(
                request.enclosure_output,
                EnclosureOutputRequest::Include { .. }
            ) {
                dyadic_precision_bits
            } else {
                evaluation.metadata.internal_precision_bits
            },
            refinement_rounds: evaluation.metadata.refinement_rounds,
            confirmed_significant_digits,
            assurance: AssuranceLevel::Exact,
            protocol_version: ProtocolVersion::CURRENT,
        },
    })
}

fn exact_presentation(rational: &Rational) -> ExactPresentation {
    let plain_text = rational.to_string();
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: exact_representation_kind(rational),
        presentation: rational_presentation(rational),
        plain_text,
    }
}

fn rational_presentation(rational: &Rational) -> PresentationNode {
    if rational.is_integer() {
        PresentationNode::Text(rational.numerator.to_string())
    } else {
        PresentationNode::Fraction {
            numerator: Box::new(PresentationNode::Text(rational.numerator.to_string())),
            denominator: Box::new(PresentationNode::Text(
                rational.denominator.inner.to_string(),
            )),
        }
    }
}

fn exact_representation_kind(rational: &Rational) -> ExactRepresentationKind {
    if rational.is_integer() {
        ExactRepresentationKind::Integer
    } else {
        ExactRepresentationKind::Rational
    }
}

fn scientific_presentation(
    rational: &Rational,
    significant_digits: core::num::NonZeroU32,
    rounding_mode: DecimalRoundingMode,
) -> Result<ScientificPresentation, PresentationError> {
    let digits = significant_digits.get();
    if rational.is_zero() {
        let significand = zero_significand(digits)?;
        return Ok(ScientificPresentation {
            relation: ResultRelation::ApproximatelyEqual,
            significand: significand.clone(),
            exponent_ten: String::from("0"),
            requested_significant_digits: significant_digits,
            confirmed_significant_digits: digits,
            rounding_mode,
            presentation: PresentationNode::Text(format!("{significand}e0")),
        });
    }

    let negative = rational.numerator.sign() == Sign::Minus;
    let numerator = rational.numerator.inner.abs();
    let denominator = &rational.denominator.inner.inner;
    let mut exponent_ten = decimal_exponent(&numerator, denominator)?;
    let scale_exponent = i64::from(digits)
        .checked_sub(1)
        .and_then(|value| value.checked_sub(exponent_ten))
        .ok_or_else(precision_limit_error)?;

    let (scaled_numerator, scaled_denominator) = if scale_exponent >= 0 {
        (numerator * pow10_i64(scale_exponent)?, denominator.clone())
    } else {
        (
            numerator,
            denominator
                * pow10_i64(
                    scale_exponent
                        .checked_neg()
                        .ok_or_else(precision_limit_error)?,
                )?,
        )
    };
    let (quotient, remainder) = scaled_numerator.div_rem(&scaled_denominator);
    let mut rounded = round_scaled_magnitude(
        quotient,
        remainder,
        &scaled_denominator,
        negative,
        rounding_mode,
    );

    let digit_limit = pow10_u32(digits)?;
    if rounded >= digit_limit {
        rounded /= 10_u8;
        exponent_ten = exponent_ten
            .checked_add(1)
            .ok_or_else(precision_limit_error)?;
    }

    let significand = format_significand(negative, &rounded, digits)?;
    Ok(ScientificPresentation {
        relation: ResultRelation::ApproximatelyEqual,
        significand: significand.clone(),
        exponent_ten: exponent_ten.to_string(),
        requested_significant_digits: significant_digits,
        confirmed_significant_digits: digits,
        rounding_mode,
        presentation: PresentationNode::Text(format!("{significand}e{exponent_ten}")),
    })
}

fn dyadic_interval_presentation(
    rational: &Rational,
    format: EnclosureFormat,
    precision_bits: u32,
) -> Result<CertifiedIntervalPresentation, PresentationError> {
    let scale = BigInt::one() << precision_bits;
    let scaled_numerator = &rational.numerator.inner * scale;
    let denominator = &rational.denominator.inner.inner;
    let lower = scaled_numerator.div_floor(denominator);
    let upper = scaled_numerator.div_ceil(denominator);
    let exponent_two = -BigInt::from(precision_bits);
    let lower = normalize_dyadic(lower, exponent_two.clone());
    let upper = normalize_dyadic(upper, exponent_two);

    Ok(CertifiedIntervalPresentation {
        relation: ResultRelation::ElementOf,
        presentation: PresentationNode::Row(vec![
            PresentationNode::Text(String::from("[")),
            dyadic_text_node(&lower),
            PresentationNode::Text(String::from(", ")),
            dyadic_text_node(&upper),
            PresentationNode::Text(String::from("]")),
        ]),
        lower,
        upper,
        format,
    })
}

fn decimal_exponent(numerator: &BigInt, denominator: &BigInt) -> Result<i64, PresentationError> {
    let numerator_digits = decimal_digit_count(numerator)?;
    let denominator_digits = decimal_digit_count(denominator)?;
    let mut exponent = numerator_digits
        .checked_sub(denominator_digits)
        .ok_or_else(precision_limit_error)?;

    while compare_rational_abs_to_power10(numerator, denominator, exponent)? == Ordering::Less {
        exponent = exponent.checked_sub(1).ok_or_else(precision_limit_error)?;
    }
    while compare_rational_abs_to_power10(
        numerator,
        denominator,
        exponent.checked_add(1).ok_or_else(precision_limit_error)?,
    )? != Ordering::Less
    {
        exponent = exponent.checked_add(1).ok_or_else(precision_limit_error)?;
    }
    Ok(exponent)
}

fn compare_rational_abs_to_power10(
    numerator: &BigInt,
    denominator: &BigInt,
    exponent: i64,
) -> Result<Ordering, PresentationError> {
    if exponent >= 0 {
        Ok(numerator.cmp(&(denominator * pow10_i64(exponent)?)))
    } else {
        Ok(
            (numerator * pow10_i64(exponent.checked_neg().ok_or_else(precision_limit_error)?)?)
                .cmp(denominator),
        )
    }
}

fn round_scaled_magnitude(
    quotient: BigInt,
    remainder: BigInt,
    denominator: &BigInt,
    negative: bool,
    rounding_mode: DecimalRoundingMode,
) -> BigInt {
    if remainder.is_zero() {
        return quotient;
    }

    match rounding_mode {
        DecimalRoundingMode::TowardPositiveInfinity => {
            if negative {
                quotient
            } else {
                quotient + 1_u8
            }
        }
        DecimalRoundingMode::TowardNegativeInfinity => {
            if negative {
                quotient + 1_u8
            } else {
                quotient
            }
        }
        DecimalRoundingMode::TowardZero => quotient,
        DecimalRoundingMode::AwayFromZero => quotient + 1_u8,
        DecimalRoundingMode::NearestTiesAwayFromZero => {
            round_nearest_magnitude(quotient, remainder, denominator, TieRule::AwayFromZero)
        }
        DecimalRoundingMode::NearestTiesToEven => {
            round_nearest_magnitude(quotient, remainder, denominator, TieRule::ToEven)
        }
    }
}

#[derive(Clone, Copy)]
enum TieRule {
    ToEven,
    AwayFromZero,
}

fn round_nearest_magnitude(
    quotient: BigInt,
    remainder: BigInt,
    denominator: &BigInt,
    tie_rule: TieRule,
) -> BigInt {
    match (&remainder * 2_u8).cmp(denominator) {
        Ordering::Less => quotient,
        Ordering::Greater => quotient + 1_u8,
        Ordering::Equal => match tie_rule {
            TieRule::AwayFromZero => quotient + 1_u8,
            TieRule::ToEven => {
                if quotient.is_even() {
                    quotient
                } else {
                    quotient + 1_u8
                }
            }
        },
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

fn dyadic_text_node(value: &ExactDyadic) -> PresentationNode {
    PresentationNode::Text(format!("{}*2^{}", value.coefficient, value.exponent_two))
}

fn format_significand(
    negative: bool,
    rounded_magnitude: &BigInt,
    digits: u32,
) -> Result<String, PresentationError> {
    let width = usize::try_from(digits).map_err(|_| precision_limit_error())?;
    let mut text = rounded_magnitude.to_string();
    while text.len() < width {
        text.insert(0, '0');
    }
    let body = if digits == 1 {
        text
    } else {
        let (head, tail) = text.split_at(1);
        format!("{head}.{tail}")
    };
    if negative {
        Ok(format!("-{body}"))
    } else {
        Ok(body)
    }
}

fn zero_significand(digits: u32) -> Result<String, PresentationError> {
    let width = usize::try_from(digits).map_err(|_| precision_limit_error())?;
    if width == 1 {
        Ok(String::from("0"))
    } else {
        Ok(format!("0.{}", "0".repeat(width - 1)))
    }
}

fn decimal_digit_count(value: &BigInt) -> Result<i64, PresentationError> {
    i64::try_from(value.to_string().len()).map_err(|_| precision_limit_error())
}

fn precision_bits_for_decimal_digits(digits: u32) -> Result<u32, PresentationError> {
    digits
        .checked_mul(4)
        .and_then(|value| value.checked_add(16))
        .map(|value| value.max(64))
        .ok_or_else(precision_limit_error)
}

fn pow10_i64(exponent: i64) -> Result<BigInt, PresentationError> {
    let exponent = u32::try_from(exponent).map_err(|_| precision_limit_error())?;
    pow10_u32(exponent)
}

fn pow10_u32(exponent: u32) -> Result<BigInt, PresentationError> {
    Ok(BigInt::from(10_u8).pow(exponent))
}

fn precision_limit_error() -> PresentationError {
    PresentationError::ComputationLimit(ComputationLimitError {
        kind: ComputationLimitKind::PrecisionBits,
    })
}

impl From<EvaluationError> for CalculatorError {
    fn from(value: EvaluationError) -> Self {
        match value {
            EvaluationError::Domain(error) => Self::Domain(error),
            EvaluationError::InputLimit(error) => Self::InputLimit(error),
            EvaluationError::ComputationLimit(error) => Self::ComputationLimit(error),
            EvaluationError::UnsupportedFeature(error) => Self::UnsupportedFeature(error),
            EvaluationError::InternalInvariant(error) => Self::InternalInvariant(error),
        }
    }
}

impl From<PresentationError> for CalculatorError {
    fn from(value: PresentationError) -> Self {
        match value {
            PresentationError::InputLimit(error) => Self::InputLimit(error),
            PresentationError::ComputationLimit(error) => Self::ComputationLimit(error),
            PresentationError::InternalInvariant(error) => Self::InternalInvariant(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_only_request() -> CalculationRequest {
        CalculationRequest {
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Omit,
            ..CalculationRequest::default()
        }
    }

    fn scientific_request(
        significant_digits: u32,
        rounding_mode: DecimalRoundingMode,
    ) -> CalculationRequest {
        CalculationRequest {
            scientific_output: ScientificOutputRequest::Include {
                significant_digits: core::num::NonZeroU32::new(significant_digits).unwrap(),
                rounding_mode,
            },
            enclosure_output: EnclosureOutputRequest::Omit,
            ..CalculationRequest::default()
        }
    }

    fn enclosure_request() -> CalculationRequest {
        CalculationRequest {
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Include {
                format: EnclosureFormat::ExactDyadic,
            },
            ..CalculationRequest::default()
        }
    }

    fn exact_plain_text(source: &str) -> String {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &exact_only_request(), &mut context).expect(source);
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        exact.plain_text
    }

    fn scientific_parts(
        source: &str,
        significant_digits: u32,
        rounding_mode: DecimalRoundingMode,
    ) -> (String, String) {
        let mut context = EvaluationContext::default();
        let outcome = calculate(
            source,
            &scientific_request(significant_digits, rounding_mode),
            &mut context,
        )
        .expect(source);
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ScientificOutput::Included(scientific) = calculation.scientific else {
            panic!("expected scientific output");
        };
        assert_eq!(scientific.confirmed_significant_digits, significant_digits);
        assert_eq!(
            calculation.metadata.confirmed_significant_digits,
            significant_digits
        );
        (scientific.significand, scientific.exponent_ten)
    }

    #[test]
    fn decimal_addition_is_exact() {
        assert_eq!(exact_plain_text("0.1 + 0.2"), "3/10");
    }

    #[test]
    fn rational_addition_is_exact() {
        assert_eq!(exact_plain_text("1 / 3 + 1 / 6"), "1/2");
    }

    #[test]
    fn integer_power_and_percent_are_exact() {
        assert_eq!(exact_plain_text("2^-3"), "1/8");
        assert_eq!(exact_plain_text("50%"), "1/2");
    }

    #[test]
    fn rational_scientific_output_uses_exact_rounding_modes() {
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::NearestTiesToEven),
            (String::from("1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::NearestTiesAwayFromZero),
            (String::from("1.3"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::TowardPositiveInfinity),
            (String::from("1.3"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::TowardNegativeInfinity),
            (String::from("1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::TowardZero),
            (String::from("1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("1.25", 2, DecimalRoundingMode::AwayFromZero),
            (String::from("1.3"), String::from("0"))
        );

        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::NearestTiesToEven),
            (String::from("-1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::NearestTiesAwayFromZero),
            (String::from("-1.3"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::TowardPositiveInfinity),
            (String::from("-1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::TowardNegativeInfinity),
            (String::from("-1.3"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::TowardZero),
            (String::from("-1.2"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("-1.25", 2, DecimalRoundingMode::AwayFromZero),
            (String::from("-1.3"), String::from("0"))
        );
    }

    #[test]
    fn rational_scientific_output_handles_carry_and_zero() {
        assert_eq!(
            scientific_parts("999", 2, DecimalRoundingMode::NearestTiesToEven),
            (String::from("1.0"), String::from("3"))
        );
        assert_eq!(
            scientific_parts("0", 3, DecimalRoundingMode::NearestTiesToEven),
            (String::from("0.00"), String::from("0"))
        );
        assert_eq!(
            scientific_parts("0.1 + 0.2", 4, DecimalRoundingMode::NearestTiesToEven),
            (String::from("3.000"), String::from("-1"))
        );
    }

    #[test]
    fn rational_enclosure_output_contains_exact_value() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("0.1 + 0.2", &enclosure_request(), &mut context)
            .expect("rational enclosure should calculate");
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected enclosure output");
        };
        let rational = Rational::new(Integer::from(3), Integer::from(10)).unwrap();
        assert!(dyadic_le_rational(&enclosure.lower, &rational));
        assert!(dyadic_ge_rational(&enclosure.upper, &rational));
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::CertifiedIntervalEvaluation));
    }

    #[test]
    fn division_by_zero_is_domain_error() {
        let mut context = EvaluationContext::default();
        let error = calculate("1 / 0", &exact_only_request(), &mut context).expect_err("1 / 0");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::DivisionByZero,
                span: None,
            })
        );
    }

    fn dyadic_le_rational(dyadic: &ExactDyadic, rational: &Rational) -> bool {
        compare_dyadic_to_rational(dyadic, rational) != Ordering::Greater
    }

    fn dyadic_ge_rational(dyadic: &ExactDyadic, rational: &Rational) -> bool {
        compare_dyadic_to_rational(dyadic, rational) != Ordering::Less
    }

    fn compare_dyadic_to_rational(dyadic: &ExactDyadic, rational: &Rational) -> Ordering {
        let exponent = &dyadic.exponent_two.inner;
        if exponent.sign() == Sign::Minus {
            let scale = BigInt::one() << exponent.abs().to_string().parse::<u32>().unwrap();
            (&dyadic.coefficient.inner * &rational.denominator.inner.inner)
                .cmp(&(&rational.numerator.inner * scale))
        } else {
            let scale = BigInt::one() << exponent.to_string().parse::<u32>().unwrap();
            (&dyadic.coefficient.inner * scale * &rational.denominator.inner.inner)
                .cmp(&rational.numerator.inner)
        }
    }
}
