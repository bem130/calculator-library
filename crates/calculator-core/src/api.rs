use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
};
use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{Signed, Zero};

use crate::expression::{
    evaluate_interval_dag, evaluate_radical_dag, evaluate_rational_evaluation_dag,
    evaluate_rational_pi_multiple_dag, lower_source_expression, ExactExpressionDag,
    PiCoefficientEvaluation, RadicalEvaluation, RationalEvaluation,
};
use crate::interval;
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
    if let Some(reason) = partial_reason(&calculation) {
        return Ok(CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure: partial_certified_enclosure(&evaluation)
                .map_err(CalculatorError::from)?,
        });
    }
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
    let dag = lower_source_expression(&expression.root, request.semantics)?;
    let rational = match evaluate_rational_evaluation_dag(&dag) {
        Ok(rational) => rational,
        Err(error) if should_fallback_to_symbolic_interval(&error) => {
            if let Some(pi_multiple) = evaluate_rational_pi_multiple_dag(&dag)? {
                if pi_multiple.coefficient().is_zero() {
                    return Ok(rational_evaluation_outcome(
                        expression,
                        &dag,
                        RationalEvaluation::direct(pi_multiple.into_coefficient()),
                        request,
                    ));
                }
                let certified_enclosure =
                    rational_pi_enclosure(&dag, &pi_multiple).map_err(|interval_error| {
                        interval_error_to_evaluation_error(interval_error, error.clone())
                    })?;
                let mut methods = vec![
                    MethodTag::SymbolicRetention,
                    MethodTag::CertifiedIntervalEvaluation,
                ];
                if pi_multiple.used_special_angle() {
                    methods.push(MethodTag::SpecialAngle);
                }
                return Ok(EvaluationOutcome {
                    value: EvaluatedValue {
                        exact_expression: ExactExpression {
                            source: expression.source.clone(),
                        },
                        recognized_exact: RecognizedExact::RationalPiMultiple(
                            pi_multiple.into_coefficient(),
                        ),
                        certified_enclosure: CertifiedEnclosureState::Available(
                            certified_enclosure,
                        ),
                    },
                    metadata: EvaluationMetadata {
                        semantic_settings: request.semantics,
                        methods,
                        internal_precision_bits: 128,
                        refinement_rounds: 0,
                    },
                });
            }
            if let Some(radical) = evaluate_radical_dag(&dag)? {
                let certified_enclosure =
                    radical_enclosure(&dag, &radical).map_err(|interval_error| {
                        interval_error_to_evaluation_error(interval_error, error.clone())
                    })?;
                let mut methods = vec![
                    MethodTag::RadicalExtraction,
                    MethodTag::CertifiedIntervalEvaluation,
                ];
                if radical.used_special_angle() {
                    methods.push(MethodTag::SpecialAngle);
                }
                return Ok(EvaluationOutcome {
                    value: EvaluatedValue {
                        exact_expression: ExactExpression {
                            source: expression.source.clone(),
                        },
                        recognized_exact: RecognizedExact::Radical(radical.into_value()),
                        certified_enclosure: CertifiedEnclosureState::Available(
                            certified_enclosure,
                        ),
                    },
                    metadata: EvaluationMetadata {
                        semantic_settings: request.semantics,
                        methods,
                        internal_precision_bits: 128,
                        refinement_rounds: 0,
                    },
                });
            }
            let certified_enclosure = evaluate_interval_dag(&dag).map_err(|interval_error| {
                interval_error_to_evaluation_error(interval_error, error.clone())
            })?;
            return Ok(EvaluationOutcome {
                value: EvaluatedValue {
                    exact_expression: ExactExpression {
                        source: expression.source.clone(),
                    },
                    recognized_exact: RecognizedExact::GeneralSymbolic,
                    certified_enclosure: CertifiedEnclosureState::Available(certified_enclosure),
                },
                metadata: EvaluationMetadata {
                    semantic_settings: request.semantics,
                    methods: vec![
                        MethodTag::SymbolicRetention,
                        MethodTag::CertifiedIntervalEvaluation,
                    ],
                    internal_precision_bits: 128,
                    refinement_rounds: 0,
                },
            });
        }
        Err(error) => return Err(error),
    };
    Ok(rational_evaluation_outcome(
        expression, &dag, rational, request,
    ))
}

fn rational_pi_enclosure(
    dag: &ExactExpressionDag,
    value: &PiCoefficientEvaluation,
) -> Result<CertifiedInterval, interval::IntervalError> {
    evaluate_interval_dag(dag).or_else(|error| match error {
        interval::IntervalError::UnsupportedExpression => interval::multiply(
            &interval::from_rational(value.coefficient(), 128),
            &interval::constant(Constant::Pi, 128)?,
        ),
        error => Err(error),
    })
}

fn radical_enclosure(
    dag: &ExactExpressionDag,
    value: &RadicalEvaluation,
) -> Result<CertifiedInterval, interval::IntervalError> {
    evaluate_interval_dag(dag).or_else(|error| match error {
        interval::IntervalError::UnsupportedExpression => {
            let coefficient = interval::from_rational(&value.value().coefficient, 128);
            let radicand = Rational::from_integer(value.value().radicand.inner.clone());
            let radical = interval::sqrt(&interval::from_rational(&radicand, 128), 128)?;
            interval::multiply(&coefficient, &radical)
        }
        error => Err(error),
    })
}

fn rational_evaluation_outcome(
    expression: &ParsedExpression,
    dag: &ExactExpressionDag,
    rational: RationalEvaluation,
    request: &EvaluationRequest,
) -> EvaluationOutcome {
    let certified_enclosure = evaluate_interval_dag(dag)
        .unwrap_or_else(|_| interval::from_rational(rational.value(), 128));
    let mut methods = vec![MethodTag::RationalReduction];
    if rational.used_special_angle() {
        methods.push(MethodTag::SpecialAngle);
    }
    methods.push(MethodTag::CertifiedIntervalEvaluation);
    let rational = rational.into_value();
    EvaluationOutcome {
        value: EvaluatedValue {
            exact_expression: ExactExpression {
                source: expression.source.clone(),
            },
            recognized_exact: RecognizedExact::Rational(rational),
            certified_enclosure: CertifiedEnclosureState::Available(certified_enclosure),
        },
        metadata: EvaluationMetadata {
            semantic_settings: request.semantics,
            methods,
            internal_precision_bits: 128,
            refinement_rounds: 0,
        },
    }
}

pub fn present(
    evaluation: &EvaluationOutcome,
    request: &PresentationRequest,
) -> Result<Calculation, PresentationError> {
    let exact = match (&evaluation.value.recognized_exact, request.exact_output) {
        (_, ExactOutputRequest::Omit) => ExactOutput::Omitted,
        (RecognizedExact::Rational(rational), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(exact_presentation(rational))
        }
        (RecognizedExact::Radical(value), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(radical_presentation(value))
        }
        (RecognizedExact::RationalPiMultiple(value), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(rational_pi_presentation(value))
        }
        (_, ExactOutputRequest::Include { .. }) => ExactOutput::Included(symbolic_presentation(
            &evaluation.value.exact_expression.source,
        )),
    };
    let scientific = match (
        &evaluation.value.recognized_exact,
        request.scientific_output,
    ) {
        (_, ScientificOutputRequest::Omit) => ScientificOutput::Omitted,
        (
            RecognizedExact::Rational(rational),
            ScientificOutputRequest::Include {
                significant_digits,
                rounding_mode,
            },
        ) => ScientificOutput::Included(scientific_presentation(
            rational,
            significant_digits,
            rounding_mode,
        )?),
        (
            _,
            ScientificOutputRequest::Include {
                significant_digits,
                rounding_mode,
            },
        ) => ScientificOutput::Unavailable(UnavailableScientificOutput {
            requested_significant_digits: significant_digits,
            confirmed_significant_digits: 0,
            rounding_mode,
            reason: IncompleteReason::PrecisionLimit {
                requested_digits: significant_digits,
                confirmed_digits: 0,
            },
        }),
    };
    let dyadic_precision_bits = match request.scientific_output {
        ScientificOutputRequest::Include {
            significant_digits, ..
        } => precision_bits_for_decimal_digits(significant_digits.get())?,
        ScientificOutputRequest::Omit => 128,
    };
    let enclosure = match (&evaluation.value.recognized_exact, request.enclosure_output) {
        (_, EnclosureOutputRequest::Omit) => EnclosureOutput::Omitted,
        (RecognizedExact::Rational(rational), EnclosureOutputRequest::Include { format }) => {
            EnclosureOutput::Included(dyadic_interval_presentation(
                rational,
                format,
                dyadic_precision_bits,
            )?)
        }
        (_, EnclosureOutputRequest::Include { format }) => EnclosureOutput::Included(
            certified_interval_state_presentation(&evaluation.value.certified_enclosure, format)?,
        ),
    };
    let confirmed_significant_digits = match &scientific {
        ScientificOutput::Included(value) => value.confirmed_significant_digits,
        ScientificOutput::Omitted | ScientificOutput::Unavailable(_) => 0,
    };
    let simplification_status = simplification_status(&scientific);
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
            exact_representation: exact_representation_kind(&evaluation.value.recognized_exact),
            simplification_status,
            semantic_settings: evaluation.metadata.semantic_settings,
            methods,
            internal_precision_bits: if matches!(
                request.enclosure_output,
                EnclosureOutputRequest::Include { .. }
            ) && matches!(
                evaluation.value.recognized_exact,
                RecognizedExact::Rational(_)
            ) {
                dyadic_precision_bits
            } else {
                evaluation.metadata.internal_precision_bits
            },
            refinement_rounds: evaluation.metadata.refinement_rounds,
            confirmed_significant_digits,
            assurance: assurance_level(&evaluation.value.recognized_exact),
            protocol_version: ProtocolVersion::CURRENT,
        },
    })
}

fn should_fallback_to_symbolic_interval(error: &EvaluationError) -> bool {
    matches!(
        error,
        EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
            feature: UnsupportedFeature::FunctionEvaluation
                | UnsupportedFeature::ConstantEvaluation
                | UnsupportedFeature::NonIntegerPower
        })
    )
}

fn interval_error_to_evaluation_error(
    error: interval::IntervalError,
    fallback: EvaluationError,
) -> EvaluationError {
    match error {
        interval::IntervalError::Domain(kind) => {
            EvaluationError::Domain(DomainError { kind, span: None })
        }
        interval::IntervalError::ExponentTooLarge => {
            EvaluationError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::PrecisionBits,
            })
        }
        interval::IntervalError::InvalidBounds => {
            EvaluationError::InternalInvariant(InternalInvariantError {
                code: InternalInvariantCode::InvalidCertifiedInterval,
            })
        }
        interval::IntervalError::UnsupportedExpression
        | interval::IntervalError::DivisionByIntervalContainingZero => fallback,
    }
}

fn partial_reason(calculation: &Calculation) -> Option<IncompleteReason> {
    match &calculation.scientific {
        ScientificOutput::Unavailable(value) => Some(value.reason.clone()),
        ScientificOutput::Omitted | ScientificOutput::Included(_) => None,
    }
}

fn partial_certified_enclosure(
    evaluation: &EvaluationOutcome,
) -> Result<CertifiedIntervalPresentation, PresentationError> {
    certified_interval_state_presentation(
        &evaluation.value.certified_enclosure,
        EnclosureFormat::ExactDyadic,
    )
}

fn exact_presentation(rational: &Rational) -> ExactPresentation {
    let plain_text = rational.to_string();
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: rational_exact_representation_kind(rational),
        presentation: rational_presentation(rational),
        plain_text,
    }
}

fn radical_presentation(value: &SimpleRadical) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::Radical,
        presentation: radical_presentation_node(value),
        plain_text: radical_plain_text(value),
    }
}

fn radical_plain_text(value: &SimpleRadical) -> String {
    let numerator = value.coefficient.numerator.to_string();
    let denominator = value.coefficient.denominator.inner.to_string();
    let radicand = value.radicand.inner.to_string();
    let radical = format!("sqrt({radicand})");
    if value.coefficient.is_integer() {
        return match numerator.as_str() {
            "1" => radical,
            "-1" => format!("-{radical}"),
            _ => format!("{numerator}{radical}"),
        };
    }

    match numerator.as_str() {
        "1" => format!("{radical}/{denominator}"),
        "-1" => format!("-{radical}/{denominator}"),
        _ => format!("{numerator}{radical}/{denominator}"),
    }
}

fn radical_presentation_node(value: &SimpleRadical) -> PresentationNode {
    let radical = PresentationNode::Radical {
        index: RadicalIndex::Square,
        radicand: Box::new(PresentationNode::Text(value.radicand.inner.to_string())),
    };
    let numerator = value.coefficient.numerator.to_string();
    if value.coefficient.is_integer() {
        return radical_numerator_node(&numerator, radical);
    }
    PresentationNode::Fraction {
        numerator: Box::new(radical_numerator_node(&numerator, radical)),
        denominator: Box::new(PresentationNode::Text(
            value.coefficient.denominator.inner.to_string(),
        )),
    }
}

fn radical_numerator_node(numerator: &str, radical: PresentationNode) -> PresentationNode {
    match numerator {
        "1" => radical,
        "-1" => PresentationNode::Row(vec![PresentationNode::Text(String::from("-")), radical]),
        _ => PresentationNode::Row(vec![
            PresentationNode::Text(String::from(numerator)),
            radical,
        ]),
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

fn rational_pi_presentation(coefficient: &Rational) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::RationalPiMultiple,
        presentation: rational_pi_presentation_node(coefficient),
        plain_text: rational_pi_plain_text(coefficient),
    }
}

fn rational_pi_plain_text(coefficient: &Rational) -> String {
    debug_assert!(!coefficient.is_zero());
    let numerator = coefficient.numerator.to_string();
    if coefficient.is_integer() {
        return match numerator.as_str() {
            "1" => String::from("pi"),
            "-1" => String::from("-pi"),
            _ => format!("{numerator}pi"),
        };
    }

    let denominator = coefficient.denominator.inner.to_string();
    match numerator.as_str() {
        "1" => format!("pi/{denominator}"),
        "-1" => format!("-pi/{denominator}"),
        _ => format!("{numerator}pi/{denominator}"),
    }
}

fn rational_pi_presentation_node(coefficient: &Rational) -> PresentationNode {
    if coefficient.is_integer() {
        return pi_numerator_node(&coefficient.numerator.to_string());
    }
    PresentationNode::Fraction {
        numerator: Box::new(pi_numerator_node(&coefficient.numerator.to_string())),
        denominator: Box::new(PresentationNode::Text(
            coefficient.denominator.inner.to_string(),
        )),
    }
}

fn pi_numerator_node(numerator: &str) -> PresentationNode {
    match numerator {
        "1" => PresentationNode::Text(String::from("π")),
        "-1" => PresentationNode::Row(vec![
            PresentationNode::Text(String::from("-")),
            PresentationNode::Text(String::from("π")),
        ]),
        _ => PresentationNode::Row(vec![
            PresentationNode::Text(String::from(numerator)),
            PresentationNode::Text(String::from("π")),
        ]),
    }
}

fn symbolic_presentation(source: &str) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::GeneralSymbolic,
        presentation: PresentationNode::Text(String::from(source)),
        plain_text: String::from(source),
    }
}

fn exact_representation_kind(value: &RecognizedExact) -> ExactRepresentationKind {
    match value {
        RecognizedExact::Rational(rational) => rational_exact_representation_kind(rational),
        RecognizedExact::Radical(_) => ExactRepresentationKind::Radical,
        RecognizedExact::RealAlgebraic(_) => ExactRepresentationKind::RealAlgebraic,
        RecognizedExact::RationalPiMultiple(_) => ExactRepresentationKind::RationalPiMultiple,
        RecognizedExact::GeneralSymbolic => ExactRepresentationKind::GeneralSymbolic,
    }
}

fn rational_exact_representation_kind(rational: &Rational) -> ExactRepresentationKind {
    if rational.is_integer() {
        ExactRepresentationKind::Integer
    } else {
        ExactRepresentationKind::Rational
    }
}

fn simplification_status(scientific: &ScientificOutput) -> SimplificationStatus {
    match scientific {
        ScientificOutput::Unavailable(value) => SimplificationStatus::PartiallySimplified {
            reason: value.reason.clone(),
        },
        ScientificOutput::Omitted | ScientificOutput::Included(_) => {
            SimplificationStatus::FullySimplifiedWithinLimits
        }
    }
}

fn assurance_level(value: &RecognizedExact) -> AssuranceLevel {
    match value {
        RecognizedExact::Rational(_) | RecognizedExact::Radical(_) => AssuranceLevel::Exact,
        RecognizedExact::RealAlgebraic(_)
        | RecognizedExact::RationalPiMultiple(_)
        | RecognizedExact::GeneralSymbolic => AssuranceLevel::CertifiedEnclosure,
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
    let interval = interval::from_rational(rational, precision_bits);
    if !interval::contains_rational(&interval, rational).map_err(|_| precision_limit_error())? {
        return Err(PresentationError::InternalInvariant(
            InternalInvariantError {
                code: InternalInvariantCode::InvalidCertifiedInterval,
            },
        ));
    }
    Ok(certified_interval_presentation(&interval, format))
}

fn certified_interval_state_presentation(
    state: &CertifiedEnclosureState,
    format: EnclosureFormat,
) -> Result<CertifiedIntervalPresentation, PresentationError> {
    match state {
        CertifiedEnclosureState::Available(interval) => {
            Ok(certified_interval_presentation(interval, format))
        }
        CertifiedEnclosureState::NotRequested | CertifiedEnclosureState::Unavailable => Err(
            PresentationError::InternalInvariant(InternalInvariantError {
                code: InternalInvariantCode::InvalidCertifiedInterval,
            }),
        ),
    }
}

fn certified_interval_presentation(
    interval: &CertifiedInterval,
    format: EnclosureFormat,
) -> CertifiedIntervalPresentation {
    let lower = interval.lower.clone();
    let upper = interval.upper.clone();
    CertifiedIntervalPresentation {
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
    }
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
        exact_plain_text_from_outcome(outcome)
    }

    fn exact_plain_text_with_request(source: &str, request: &CalculationRequest) -> String {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, request, &mut context).expect(source);
        exact_plain_text_from_outcome(outcome)
    }

    fn exact_plain_text_from_outcome(outcome: CalculationOutcome) -> String {
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
    fn rational_power_real_principal_handles_exact_roots() {
        assert_eq!(exact_plain_text("(-8)^(1/3)"), "-2");
        assert_eq!(exact_plain_text("(-8)^(2/3)"), "4");
        assert_eq!(exact_plain_text("(-27/8)^(2/3)"), "9/4");
        assert_eq!(exact_plain_text("(16/81)^(3/4)"), "8/27");
        assert_eq!(exact_plain_text("8^(-1/3)"), "1/2");
        assert_eq!(exact_plain_text("0^(1/3)"), "0");
    }

    #[test]
    fn rational_pi_multiples_are_recognized_exactly() {
        for (source, expected) in [
            ("pi", "pi"),
            ("pi/6", "pi/6"),
            ("3*pi/4", "3pi/4"),
            ("-11*pi/7", "-11pi/7"),
            ("pi/6 + pi/3", "pi/2"),
            ("pi - pi", "0"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn rational_pi_multiples_use_exact_representation_kind() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("3*pi/4", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(
            exact.representation,
            ExactRepresentationKind::RationalPiMultiple
        );
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::RationalPiMultiple
        );
        assert_eq!(exact.plain_text, "3pi/4");
    }

    #[test]
    fn rational_special_angles_are_exact() {
        for (source, expected) in [
            ("sin(0)", "0"),
            ("sin(pi/6)", "1/2"),
            ("sin(5*pi/6)", "1/2"),
            ("sin(7*pi/6)", "-1/2"),
            ("sin(-pi/6)", "-1/2"),
            ("sin(pi/2)", "1"),
            ("sin(pi)", "0"),
            ("sin(1 - 1)", "0"),
            ("sin(pi/6 + 2*pi)", "1/2"),
            ("cos(0)", "1"),
            ("cos(log(1))", "1"),
            ("cos(pi/3)", "1/2"),
            ("cos(2*pi/3)", "-1/2"),
            ("cos(-pi)", "-1"),
            ("cos(3*pi/2)", "0"),
            ("tan(0)", "0"),
            ("tan(exp(log(2)) - 2)", "0"),
            ("tan(pi/4)", "1"),
            ("tan(-pi/4)", "-1"),
            ("tan(pi)", "0"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn special_angles_honor_angle_unit_semantics() {
        let mut degree_request = exact_only_request();
        degree_request.semantics.angle_unit = AngleUnit::Degree;
        for (source, expected) in [("sin(30)", "1/2"), ("cos(60)", "1/2"), ("tan(45)", "1")] {
            assert_eq!(
                exact_plain_text_with_request(source, &degree_request),
                expected,
                "{source}"
            );
        }

        let mut gradian_request = exact_only_request();
        gradian_request.semantics.angle_unit = AngleUnit::Gradian;
        for (source, expected) in [("sin(100)", "1"), ("cos(200)", "-1"), ("tan(50)", "1")] {
            assert_eq!(
                exact_plain_text_with_request(source, &gradian_request),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn special_angle_metadata_reports_method_tag() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("sin(pi/6)", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Rational);
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::Rational
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SpecialAngle));
        assert_eq!(calculation.metadata.assurance, AssuranceLevel::Exact);
    }

    #[test]
    fn inverse_trigonometric_known_values_are_exact() {
        for (source, expected) in [
            ("asin(-1)", "-pi/2"),
            ("asin(-1/2)", "-pi/6"),
            ("asin(0)", "0"),
            ("asin(1/2)", "pi/6"),
            ("asin(1)", "pi/2"),
            ("acos(-1)", "pi"),
            ("acos(-1/2)", "2pi/3"),
            ("acos(0)", "pi/2"),
            ("acos(1/2)", "pi/3"),
            ("acos(1)", "0"),
            ("atan(-1)", "-pi/4"),
            ("atan(0)", "0"),
            ("atan(1)", "pi/4"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn inverse_trigonometric_known_values_honor_angle_unit_semantics() {
        let mut degree_request = exact_only_request();
        degree_request.semantics.angle_unit = AngleUnit::Degree;
        for (source, expected) in [
            ("asin(1/2)", "30"),
            ("asin(-1/2)", "-30"),
            ("acos(-1)", "180"),
            ("acos(1/2)", "60"),
            ("atan(1)", "45"),
        ] {
            assert_eq!(
                exact_plain_text_with_request(source, &degree_request),
                expected,
                "{source}"
            );
        }

        let mut gradian_request = exact_only_request();
        gradian_request.semantics.angle_unit = AngleUnit::Gradian;
        for (source, expected) in [
            ("asin(1/2)", "100/3"),
            ("acos(-1)", "200"),
            ("atan(1)", "50"),
        ] {
            assert_eq!(
                exact_plain_text_with_request(source, &gradian_request),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn inverse_trigonometric_radian_metadata_reports_special_angle() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("asin(1/2)", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(
            exact.representation,
            ExactRepresentationKind::RationalPiMultiple
        );
        assert_eq!(exact.plain_text, "pi/6");
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SpecialAngle));
        assert_eq!(
            calculation.metadata.assurance,
            AssuranceLevel::CertifiedEnclosure
        );
    }

    #[test]
    fn inverse_trigonometric_radian_values_return_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("asin(1/2)", &CalculationRequest::default(), &mut context)
            .expect("asin(1/2)");
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("expected partial rational pi multiple calculation");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.plain_text, "pi/6");
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected enclosure output");
        };
        assert_eq!(certified_enclosure, enclosure);
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SpecialAngle));
    }

    #[test]
    fn perfect_square_sqrt_is_exact_rational() {
        assert_eq!(exact_plain_text("sqrt(4)"), "2");
        assert_eq!(exact_plain_text("sqrt(9/16)"), "3/4");
    }

    #[test]
    fn simple_radicals_are_recognized_exactly() {
        for (source, expected) in [
            ("sqrt(2)", "sqrt(2)"),
            ("sqrt(72)", "6sqrt(2)"),
            ("sqrt(1/2)", "sqrt(2)/2"),
            ("2^(1/2)", "sqrt(2)"),
            ("3*sqrt(8)", "6sqrt(2)"),
            ("sqrt(2)/2", "sqrt(2)/2"),
            ("sin(pi/4)", "sqrt(2)/2"),
            ("cos(pi/6)", "sqrt(3)/2"),
            ("tan(pi/3)", "sqrt(3)"),
            ("tan(pi/6)", "sqrt(3)/3"),
            ("sin(pi/4) + cos(pi/4)", "sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn radical_metadata_reports_exact_representation_and_methods() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("sin(pi/4)", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Radical);
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::Radical
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::RadicalExtraction));
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SpecialAngle));
        assert_eq!(calculation.metadata.assurance, AssuranceLevel::Exact);
    }

    #[test]
    fn irrational_sqrt_returns_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("sqrt(2)", &CalculationRequest::default(), &mut context).unwrap();
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("expected partial calculation");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected retained exact expression");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Radical);
        assert_eq!(exact.plain_text, "sqrt(2)");
        let ScientificOutput::Unavailable(scientific) = calculation.scientific else {
            panic!("expected unavailable scientific output");
        };
        assert_eq!(scientific.confirmed_significant_digits, 0);
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected requested enclosure output");
        };
        assert_eq!(certified_enclosure, enclosure);
        let interval = CertifiedInterval {
            lower: certified_enclosure.lower,
            upper: certified_enclosure.upper,
        };
        let squared = interval::multiply(&interval, &interval).unwrap();
        assert!(
            interval::contains_rational(&squared, &Rational::from_integer(Integer::from(2)),)
                .unwrap()
        );
        assert_eq!(calculation.metadata.assurance, AssuranceLevel::Exact);
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::RadicalExtraction));
    }

    #[test]
    fn irrational_rational_power_returns_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("2^(1/2)", &CalculationRequest::default(), &mut context).unwrap();
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("expected partial calculation");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected retained exact expression");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Radical);
        assert_eq!(exact.plain_text, "sqrt(2)");
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected requested enclosure output");
        };
        assert_eq!(certified_enclosure, enclosure);
        let interval = CertifiedInterval {
            lower: certified_enclosure.lower,
            upper: certified_enclosure.upper,
        };
        let squared = interval::multiply(&interval, &interval).unwrap();
        assert!(
            interval::contains_rational(&squared, &Rational::from_integer(Integer::from(2)),)
                .unwrap()
        );
        assert_eq!(calculation.metadata.assurance, AssuranceLevel::Exact);
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::RadicalExtraction));
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::CertifiedIntervalEvaluation));
    }

    #[test]
    fn constants_return_partial_with_certified_enclosures() {
        let source = "e";
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &CalculationRequest::default(), &mut context)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("{source}: expected partial calculation");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("{source}: expected retained exact expression");
        };
        assert_eq!(
            exact.representation,
            ExactRepresentationKind::GeneralSymbolic
        );
        assert_eq!(exact.plain_text, source);
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("{source}: expected requested enclosure output");
        };
        assert_eq!(certified_enclosure, enclosure);
        assert_eq!(
            calculation.metadata.assurance,
            AssuranceLevel::CertifiedEnclosure
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::CertifiedIntervalEvaluation));
    }

    #[test]
    fn non_rational_pi_multiples_return_partial_with_certified_enclosures() {
        for source in ["pi", "pi/6"] {
            let mut context = EvaluationContext::default();
            let outcome = calculate(source, &CalculationRequest::default(), &mut context)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let CalculationOutcome::Partial {
                calculation,
                reason,
                certified_enclosure,
            } = outcome
            else {
                panic!("{source}: expected partial calculation");
            };
            assert_eq!(
                reason,
                IncompleteReason::PrecisionLimit {
                    requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                    confirmed_digits: 0,
                }
            );
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected retained exact expression");
            };
            assert_eq!(
                exact.representation,
                ExactRepresentationKind::RationalPiMultiple
            );
            let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
                panic!("{source}: expected requested enclosure output");
            };
            assert_eq!(certified_enclosure, enclosure);
            assert_eq!(
                calculation.metadata.assurance,
                AssuranceLevel::CertifiedEnclosure
            );
        }
    }

    #[test]
    fn initial_exp_log_identities_are_exact() {
        assert_eq!(exact_plain_text("exp(0)"), "1");
        assert_eq!(exact_plain_text("log(1)"), "0");
    }

    #[test]
    fn guarded_exp_log_identities_are_exact_for_proven_rationals() {
        assert_eq!(exact_plain_text("exp(log(2))"), "2");
        assert_eq!(exact_plain_text("exp(log(1/3))"), "1/3");
        assert_eq!(exact_plain_text("exp(log(0.1 + 0.2))"), "3/10");
        assert_eq!(exact_plain_text("log(exp(2))"), "2");
        assert_eq!(exact_plain_text("log(exp(-2))"), "-2");
        assert_eq!(exact_plain_text("log(exp(1/3))"), "1/3");
    }

    #[test]
    fn exp_one_returns_partial_euler_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("exp(1)", &CalculationRequest::default(), &mut context).unwrap();
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("expected partial calculation");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected retained exact expression");
        };
        assert_eq!(exact.plain_text, "exp(1)");
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected requested enclosure output");
        };
        assert_eq!(certified_enclosure, enclosure);
        assert_eq!(
            calculation.metadata.assurance,
            AssuranceLevel::CertifiedEnclosure
        );
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
        assert!(interval::contains_rational(
            &CertifiedInterval {
                lower: enclosure.lower,
                upper: enclosure.upper,
            },
            &rational
        )
        .unwrap());
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::CertifiedIntervalEvaluation));
    }

    #[test]
    fn evaluation_carries_certified_interval_for_rational_dag() {
        let parsed = parse("0.1 + 0.2", &ParseSettings::default()).unwrap();
        let mut context = EvaluationContext::default();
        let evaluation = evaluate(
            &parsed,
            &EvaluationRequest {
                semantics: SemanticSettings::default(),
                limits: ResourceLimitRequest::Default,
            },
            &mut context,
        )
        .unwrap();
        let CertifiedEnclosureState::Available(interval) = &evaluation.value.certified_enclosure
        else {
            panic!("expected certified interval");
        };
        let rational = Rational::new(Integer::from(3), Integer::from(10)).unwrap();
        assert!(interval::contains_rational(interval, &rational).unwrap());
        assert_eq!(evaluation.metadata.internal_precision_bits, 128);
        assert!(evaluation
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

    #[test]
    fn sqrt_of_negative_is_domain_error() {
        let mut context = EvaluationContext::default();
        let error =
            calculate("sqrt(-1)", &exact_only_request(), &mut context).expect_err("sqrt(-1)");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );
    }

    #[test]
    fn log_of_non_positive_is_domain_error() {
        for source in ["log(0)", "log(-1)"] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::LogarithmOfNonPositive,
                    span: None,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn tangent_poles_are_domain_errors() {
        for source in ["tan(pi/2)", "tan(3*pi/2)", "tan(-pi/2)"] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::TangentPole,
                    span: None,
                }),
                "{source}"
            );
        }

        let mut request = exact_only_request();
        request.semantics.angle_unit = AngleUnit::Degree;
        let mut context = EvaluationContext::default();
        let error = calculate("tan(90)", &request, &mut context).expect_err("tan(90)");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::TangentPole,
                span: None,
            })
        );
    }

    #[test]
    fn inverse_trigonometric_out_of_range_is_domain_error() {
        for source in ["asin(2)", "asin(exp(log(2)))", "acos(-2)"] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::InverseTrigonometricOutOfRange,
                    span: None,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn exp_log_identity_requires_positive_inner_value() {
        for source in ["exp(log(0))", "exp(log(-1))"] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::LogarithmOfNonPositive,
                    span: None,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn rational_power_domain_errors_follow_real_principal_semantics() {
        for (source, kind) in [
            ("0^0", DomainErrorKind::IndeterminateZeroToZero),
            ("0^-1", DomainErrorKind::ZeroToNegativePower),
            ("0^(-1/3)", DomainErrorKind::ZeroToNegativePower),
            ("(-8)^(1/2)", DomainErrorKind::NonRealPower),
            ("(-1)^(sqrt(2))", DomainErrorKind::NonRealPower),
            ("(-1)^pi", DomainErrorKind::NonRealPower),
            ("(-1)^(2^(1/2))", DomainErrorKind::NonRealPower),
        ] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError { kind, span: None }),
                "{source}"
            );
        }
    }

    #[test]
    fn negative_base_power_preserves_exponent_domain_errors() {
        let mut context = EvaluationContext::default();
        let error = calculate("(-1)^(sqrt(-1))", &exact_only_request(), &mut context)
            .expect_err("(-1)^(sqrt(-1))");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );
    }
}
