use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{Signed, Zero};

use crate::expression::{
    evaluate_interval_dag, evaluate_radical_dag, evaluate_rational_evaluation_dag,
    evaluate_rational_pi_multiple_dag, evaluate_real_algebraic_dag, logarithm_product_reduction,
    logarithm_quotient_identity, logarithm_sum_reduction, lower_source_expression,
    node_is_exact_zero, normalize_exact_subexpressions, structurally_equal_expressions,
    ExactExpressionDag, ExactNormalization, ExactReduction, PiCoefficientEvaluation,
    RadicalEvaluation, RadicalLinearCombinationEvaluation, RadicalReduction, RationalEvaluation,
    RealAlgebraicEvaluation,
};
use crate::interval;
use crate::syntax::{SourceExpr, UnaryOperator};
use crate::types::*;

pub fn calculate(
    source: &str,
    request: &CalculationRequest,
    context: &mut EvaluationContext,
) -> Result<CalculationOutcome, CalculatorError> {
    let limits = resource_limits(&request.limits);
    validate_input_bytes(source, &limits).map_err(CalculatorError::InputLimit)?;
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
    let mut calculation = present_unchecked(
        &evaluation,
        &PresentationRequest {
            exact_output: request.exact_output,
            scientific_output: request.scientific_output,
            enclosure_output: request.enclosure_output,
            limits: request.limits.clone(),
        },
    )
    .map_err(CalculatorError::from)?;
    apply_calculation_symbolic_presentation(&mut calculation, &evaluation);
    validate_calculation_output(&calculation, &limits).map_err(CalculatorError::from)?;
    if let Some(reason) = partial_reason(&calculation) {
        let certified_enclosure =
            partial_certified_enclosure(&evaluation, &limits).map_err(CalculatorError::from)?;
        return Ok(CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        });
    }
    Ok(CalculationOutcome::Complete(calculation))
}

fn apply_calculation_symbolic_presentation(
    calculation: &mut Calculation,
    evaluation: &EvaluationOutcome,
) {
    if !matches!(
        evaluation.value.recognized_exact,
        RecognizedExact::GeneralSymbolic
    ) {
        return;
    }
    let ExactOutput::Included(exact) = &mut calculation.exact else {
        return;
    };
    *exact = symbolic_presentation(&evaluation.value.exact_expression.source);
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

pub fn present_input(
    source: &str,
    request: &CalculationRequest,
) -> Result<PresentationNode, CalculatorError> {
    let limits = resource_limits(&request.limits);
    validate_input_bytes(source, &limits).map_err(CalculatorError::InputLimit)?;
    let parsed = parse(source, &request.parse).map_err(CalculatorError::Parse)?;
    validate_parsed_expression_limits(&parsed, &limits).map_err(CalculatorError::InputLimit)?;
    let presentation = source_presentation(&parsed.root).node;
    validate_presentation_node_output(&presentation, &limits).map_err(CalculatorError::from)?;
    Ok(presentation)
}

pub fn evaluate(
    expression: &ParsedExpression,
    request: &EvaluationRequest,
    _context: &mut EvaluationContext,
) -> Result<EvaluationOutcome, EvaluationError> {
    let limits = resource_limits(&request.limits);
    validate_parsed_expression_limits(expression, &limits).map_err(EvaluationError::InputLimit)?;
    let dag = lower_source_expression(&expression.root, request.semantics, &limits)?;
    validate_expression_dag_limits(&dag, &limits).map_err(EvaluationError::InputLimit)?;
    let (dag, normalization) = normalize_exact_subexpressions(dag, &limits)?;
    let rational = match evaluate_rational_evaluation_dag(&dag) {
        Ok(rational) => rational,
        Err(error) if should_fallback_to_symbolic_interval(&error) => {
            if let Some(kind) = normalization.limit_reached() {
                let mut outcome = EvaluationOutcome {
                    value: EvaluatedValue {
                        exact_expression: ExactExpression {
                            source: symbolic_presentation_from_dag(&dag).plain_text,
                        },
                        recognized_exact: RecognizedExact::GeneralSymbolic,
                        certified_enclosure: CertifiedEnclosureState::Unavailable(
                            IncompleteReason::ComputationLimit { kind },
                        ),
                    },
                    metadata: EvaluationMetadata {
                        semantic_settings: request.semantics,
                        methods: vec![MethodTag::SymbolicRetention],
                        internal_precision_bits: 0,
                        refinement_rounds: 0,
                        incomplete_reason: None,
                    },
                };
                apply_exact_normalization_metadata(&mut outcome, normalization);
                return Ok(outcome);
            }
            if let Some(pi_multiple) = evaluate_rational_pi_multiple_dag(&dag)? {
                if pi_multiple.coefficient().is_zero() {
                    let mut outcome = rational_evaluation_outcome(
                        expression,
                        RationalEvaluation::direct(pi_multiple.into_coefficient()),
                        request,
                    );
                    apply_exact_normalization_metadata(&mut outcome, normalization);
                    return Ok(outcome);
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
                let mut outcome = EvaluationOutcome {
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
                        incomplete_reason: None,
                    },
                };
                apply_exact_normalization_metadata(&mut outcome, normalization);
                return Ok(outcome);
            }
            if let Some(radical) = evaluate_radical_dag(&dag)? {
                match radical {
                    RadicalReduction::Rational(rational) => {
                        let mut outcome = rational_evaluation_outcome_with_methods(
                            expression,
                            rational,
                            request,
                            &[MethodTag::RadicalExtraction],
                        );
                        apply_exact_normalization_metadata(&mut outcome, normalization);
                        return Ok(outcome);
                    }
                    RadicalReduction::Radical(radical) => {
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
                        let mut outcome = EvaluationOutcome {
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
                                incomplete_reason: None,
                            },
                        };
                        apply_exact_normalization_metadata(&mut outcome, normalization);
                        return Ok(outcome);
                    }
                    RadicalReduction::LinearCombination(value) => {
                        let certified_enclosure = radical_linear_combination_enclosure(
                            &dag, &value,
                        )
                        .map_err(|interval_error| {
                            interval_error_to_evaluation_error(interval_error, error.clone())
                        })?;
                        let mut methods = vec![
                            MethodTag::RadicalExtraction,
                            MethodTag::CertifiedIntervalEvaluation,
                        ];
                        if value.used_special_angle() {
                            methods.push(MethodTag::SpecialAngle);
                        }
                        let mut outcome = EvaluationOutcome {
                            value: EvaluatedValue {
                                exact_expression: ExactExpression {
                                    source: expression.source.clone(),
                                },
                                recognized_exact: RecognizedExact::RadicalLinearCombination(
                                    value.into_value(),
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
                                incomplete_reason: None,
                            },
                        };
                        apply_exact_normalization_metadata(&mut outcome, normalization);
                        return Ok(outcome);
                    }
                }
            }
            if let Some(algebraic) = evaluate_real_algebraic_dag(&dag, &limits)? {
                let algebraic = match algebraic {
                    RealAlgebraicEvaluation::Rational(rational) => {
                        let mut outcome = rational_evaluation_outcome_with_methods(
                            expression,
                            rational,
                            request,
                            &[
                                MethodTag::AlgebraicMinimalPolynomial,
                                MethodTag::AlgebraicRootIsolation,
                            ],
                        );
                        apply_exact_normalization_metadata(&mut outcome, normalization);
                        return Ok(outcome);
                    }
                    RealAlgebraicEvaluation::Algebraic(algebraic) => algebraic,
                };
                let certified_enclosure =
                    evaluate_interval_dag(&dag).map_err(|interval_error| {
                        interval_error_to_evaluation_error(interval_error, error.clone())
                    })?;
                let mut methods = vec![
                    MethodTag::AlgebraicMinimalPolynomial,
                    MethodTag::AlgebraicRootIsolation,
                    MethodTag::CertifiedIntervalEvaluation,
                ];
                if contains_trigonometric_function(&dag) {
                    methods.push(MethodTag::CyclotomicReduction);
                }
                let mut outcome = EvaluationOutcome {
                    value: EvaluatedValue {
                        exact_expression: ExactExpression {
                            source: symbolic_presentation_from_dag(&dag).plain_text,
                        },
                        recognized_exact: RecognizedExact::RealAlgebraic(algebraic),
                        certified_enclosure: CertifiedEnclosureState::Available(
                            certified_enclosure,
                        ),
                    },
                    metadata: EvaluationMetadata {
                        semantic_settings: request.semantics,
                        methods,
                        internal_precision_bits: 128,
                        refinement_rounds: 0,
                        incomplete_reason: None,
                    },
                };
                apply_exact_normalization_metadata(&mut outcome, normalization);
                return Ok(outcome);
            }
            let (certified_enclosure, methods) = match evaluate_interval_dag(&dag) {
                Ok(enclosure) => (
                    CertifiedEnclosureState::Available(enclosure),
                    vec![
                        MethodTag::SymbolicRetention,
                        MethodTag::CertifiedIntervalEvaluation,
                    ],
                ),
                Err(interval::IntervalError::UnsupportedExpression) => (
                    CertifiedEnclosureState::Unavailable(normalization.limit_reached().map_or(
                        IncompleteReason::UnsupportedFeature {
                            feature: UnsupportedFeature::FunctionEvaluation,
                        },
                        |kind| IncompleteReason::ComputationLimit { kind },
                    )),
                    vec![MethodTag::SymbolicRetention],
                ),
                Err(interval_error) => {
                    return Err(interval_error_to_evaluation_error(
                        interval_error,
                        error.clone(),
                    ));
                }
            };
            let mut outcome = EvaluationOutcome {
                value: EvaluatedValue {
                    exact_expression: ExactExpression {
                        source: symbolic_presentation_from_dag(&dag).plain_text,
                    },
                    recognized_exact: RecognizedExact::GeneralSymbolic,
                    certified_enclosure,
                },
                metadata: EvaluationMetadata {
                    semantic_settings: request.semantics,
                    methods,
                    internal_precision_bits: 128,
                    refinement_rounds: 0,
                    incomplete_reason: None,
                },
            };
            apply_exact_normalization_metadata(&mut outcome, normalization);
            return Ok(outcome);
        }
        Err(error) => return Err(error),
    };
    let mut outcome = rational_evaluation_outcome(expression, rational, request);
    apply_exact_normalization_metadata(&mut outcome, normalization);
    Ok(outcome)
}

fn apply_exact_normalization_metadata(
    outcome: &mut EvaluationOutcome,
    normalization: ExactNormalization,
) {
    if let Some(kind) = normalization.limit_reached() {
        outcome.metadata.incomplete_reason = Some(IncompleteReason::ComputationLimit { kind });
    }
    for (used, method) in [
        (normalization.used_special_angle(), MethodTag::SpecialAngle),
        (
            normalization.used_radical_reduction(),
            MethodTag::RadicalExtraction,
        ),
        (
            normalization.used_algebraic_reduction(),
            MethodTag::AlgebraicMinimalPolynomial,
        ),
        (
            normalization.used_algebraic_reduction(),
            MethodTag::AlgebraicRootIsolation,
        ),
        (
            normalization.used_cyclotomic_reduction(),
            MethodTag::CyclotomicReduction,
        ),
    ] {
        if used && !outcome.metadata.methods.contains(&method) {
            outcome.metadata.methods.push(method);
        }
    }
}

fn rational_pi_enclosure(
    _dag: &ExactExpressionDag,
    value: &PiCoefficientEvaluation,
) -> Result<CertifiedInterval, interval::IntervalError> {
    interval::multiply(
        &interval::from_rational(value.coefficient(), 128),
        &interval::constant(Constant::Pi, 128)?,
    )
}

fn contains_trigonometric_function(dag: &ExactExpressionDag) -> bool {
    dag.nodes().iter().any(|node| {
        matches!(
            node,
            ExpressionNode::Function {
                function: Function::Sin | Function::Cos | Function::Tan,
                ..
            }
        )
    })
}

fn radical_enclosure(
    _dag: &ExactExpressionDag,
    value: &RadicalEvaluation,
) -> Result<CertifiedInterval, interval::IntervalError> {
    let coefficient = interval::from_rational(&value.value().coefficient, 128);
    let radicand = Rational::from_integer(value.value().radicand.inner.clone());
    let radical = interval::sqrt(&interval::from_rational(&radicand, 128), 128)?;
    interval::multiply(&coefficient, &radical)
}

fn radical_linear_combination_enclosure(
    _dag: &ExactExpressionDag,
    value: &RadicalLinearCombinationEvaluation,
) -> Result<CertifiedInterval, interval::IntervalError> {
    let mut total = interval::from_rational(&value.value().rational, 128);
    for radical in &value.value().radicals {
        let coefficient = interval::from_rational(&radical.coefficient, 128);
        let radicand = Rational::from_integer(radical.radicand.inner.clone());
        let radical = interval::sqrt(&interval::from_rational(&radicand, 128), 128)?;
        let term = interval::multiply(&coefficient, &radical)?;
        total = interval::add(&total, &term)?;
    }
    Ok(total)
}

fn rational_evaluation_outcome(
    expression: &ParsedExpression,
    rational: RationalEvaluation,
    request: &EvaluationRequest,
) -> EvaluationOutcome {
    rational_evaluation_outcome_with_methods(expression, rational, request, &[])
}

fn rational_evaluation_outcome_with_methods(
    expression: &ParsedExpression,
    rational: RationalEvaluation,
    request: &EvaluationRequest,
    extra_methods: &[MethodTag],
) -> EvaluationOutcome {
    let certified_enclosure = interval::from_rational(rational.value(), 128);
    let mut methods = vec![MethodTag::RationalReduction];
    for method in extra_methods {
        if !methods.contains(method) {
            methods.push(*method);
        }
    }
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
            incomplete_reason: None,
        },
    }
}

pub fn present(
    evaluation: &EvaluationOutcome,
    request: &PresentationRequest,
) -> Result<Calculation, PresentationError> {
    let calculation = present_unchecked(evaluation, request)?;
    validate_calculation_output(&calculation, &resource_limits(&request.limits))?;
    Ok(calculation)
}

fn present_unchecked(
    evaluation: &EvaluationOutcome,
    request: &PresentationRequest,
) -> Result<Calculation, PresentationError> {
    let exact = match (&evaluation.value.recognized_exact, request.exact_output) {
        (_, ExactOutputRequest::Omit) => ExactOutput::Omitted,
        (RecognizedExact::Rational(rational), ExactOutputRequest::Include { format }) => {
            ExactOutput::Included(rational_exact_presentation(rational, format))
        }
        (RecognizedExact::Radical(value), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(radical_presentation(value))
        }
        (RecognizedExact::RadicalLinearCombination(value), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(radical_linear_combination_presentation(value))
        }
        (RecognizedExact::RationalPiMultiple(value), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(rational_pi_presentation(value))
        }
        (RecognizedExact::RealAlgebraic(_), ExactOutputRequest::Include { .. }) => {
            ExactOutput::Included(real_algebraic_presentation(
                &evaluation.value.exact_expression.source,
            ))
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
        ) => scientific_presentation_from_certified_interval(
            &evaluation.value.certified_enclosure,
            significant_digits,
            rounding_mode,
        )?
        .map_or_else(
            || {
                unavailable_scientific_output(
                    &evaluation.value.certified_enclosure,
                    significant_digits,
                    rounding_mode,
                )
            },
            ScientificOutput::Included,
        ),
    };
    let dyadic_precision_bits = precision_bits_for_output_request(request)?;
    let enclosure = match (&evaluation.value.recognized_exact, request.enclosure_output) {
        (_, EnclosureOutputRequest::Omit) => EnclosureOutput::Omitted,
        (RecognizedExact::Rational(rational), EnclosureOutputRequest::Include { format }) => {
            EnclosureOutput::Included(dyadic_interval_presentation(
                rational,
                format,
                dyadic_precision_bits,
            )?)
        }
        (_, EnclosureOutputRequest::Include { format }) => {
            match &evaluation.value.certified_enclosure {
                CertifiedEnclosureState::Available(interval) => {
                    EnclosureOutput::Included(certified_interval_presentation(interval, format)?)
                }
                CertifiedEnclosureState::Unavailable(reason) => {
                    EnclosureOutput::Unavailable(UnavailableEnclosureOutput {
                        reason: reason.clone(),
                    })
                }
                CertifiedEnclosureState::NotRequested => {
                    return Err(PresentationError::InternalInvariant(
                        InternalInvariantError {
                            code: InternalInvariantCode::InvalidCertifiedInterval,
                        },
                    ));
                }
            }
        }
    };
    let confirmed_significant_digits = match &scientific {
        ScientificOutput::Included(value) => value.confirmed_significant_digits,
        ScientificOutput::Omitted | ScientificOutput::Unavailable(_) => 0,
    };
    let exact_representation = match &exact {
        ExactOutput::Included(value) => value.representation,
        ExactOutput::Omitted => exact_representation_kind(&evaluation.value.recognized_exact),
    };
    let simplification_status =
        simplification_status(&scientific, evaluation.metadata.incomplete_reason.as_ref());
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
            exact_representation,
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
            assurance: assurance_level(
                &evaluation.value.recognized_exact,
                &evaluation.value.certified_enclosure,
            ),
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

fn resource_limits(request: &ResourceLimitRequest) -> ResourceLimits {
    match request {
        ResourceLimitRequest::Default => ResourceLimits::default(),
        ResourceLimitRequest::Custom(value) => value.clone(),
    }
}

fn validate_input_bytes(source: &str, limits: &ResourceLimits) -> Result<(), InputLimitError> {
    if source.len() > limits.max_input_bytes as usize {
        return Err(InputLimitError {
            kind: InputLimitErrorKind::InputTooLong,
        });
    }
    Ok(())
}

fn validate_parsed_expression_limits(
    expression: &ParsedExpression,
    limits: &ResourceLimits,
) -> Result<(), InputLimitError> {
    validate_input_bytes(&expression.source, limits)?;
    let stats = expression.root.stats().ok_or(InputLimitError {
        kind: InputLimitErrorKind::SourceAstTooLarge,
    })?;
    if stats.depth > limits.max_source_depth {
        return Err(InputLimitError {
            kind: InputLimitErrorKind::SourceAstTooDeep,
        });
    }
    if stats.nodes > limits.max_source_ast_nodes {
        return Err(InputLimitError {
            kind: InputLimitErrorKind::SourceAstTooLarge,
        });
    }
    Ok(())
}

fn validate_expression_dag_limits(
    dag: &ExactExpressionDag,
    limits: &ResourceLimits,
) -> Result<(), InputLimitError> {
    if dag.nodes().len() > limits.max_expression_nodes as usize {
        return Err(InputLimitError {
            kind: InputLimitErrorKind::ExpressionTooLarge,
        });
    }
    Ok(())
}

#[derive(Default)]
struct PresentationOutputStats {
    presentation_nodes: usize,
    output_bytes: usize,
}

fn validate_calculation_output(
    calculation: &Calculation,
    limits: &ResourceLimits,
) -> Result<(), PresentationError> {
    let mut stats = PresentationOutputStats::default();
    collect_exact_output_stats(&calculation.exact, &mut stats)?;
    collect_scientific_output_stats(&calculation.scientific, &mut stats)?;
    collect_enclosure_output_stats(&calculation.enclosure, &mut stats)?;
    validate_presentation_output_stats(&stats, limits)
}

fn validate_presentation_node_output(
    presentation: &PresentationNode,
    limits: &ResourceLimits,
) -> Result<(), PresentationError> {
    let mut stats = PresentationOutputStats::default();
    collect_presentation_node_stats(presentation, &mut stats)?;
    validate_presentation_output_stats(&stats, limits)
}

fn validate_certified_interval_output(
    presentation: &CertifiedIntervalPresentation,
    limits: &ResourceLimits,
) -> Result<(), PresentationError> {
    let mut stats = PresentationOutputStats::default();
    collect_certified_interval_stats(presentation, &mut stats)?;
    validate_presentation_output_stats(&stats, limits)
}

fn validate_presentation_output_stats(
    stats: &PresentationOutputStats,
    limits: &ResourceLimits,
) -> Result<(), PresentationError> {
    if stats.presentation_nodes > limits.max_presentation_nodes as usize {
        return Err(presentation_nodes_limit_error());
    }
    if stats.output_bytes > limits.max_output_bytes as usize {
        return Err(output_too_large_error());
    }
    Ok(())
}

fn collect_exact_output_stats(
    output: &ExactOutput,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    match output {
        ExactOutput::Omitted => Ok(()),
        ExactOutput::Included(value) => {
            collect_presentation_node_stats(&value.presentation, stats)?;
            add_output_text(stats, &value.plain_text)
        }
    }
}

fn collect_scientific_output_stats(
    output: &ScientificOutput,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    match output {
        ScientificOutput::Omitted | ScientificOutput::Unavailable(_) => Ok(()),
        ScientificOutput::Included(value) => {
            add_output_text(stats, &value.significand)?;
            add_output_text(stats, &value.exponent_ten)?;
            collect_presentation_node_stats(&value.presentation, stats)
        }
    }
}

fn collect_enclosure_output_stats(
    output: &EnclosureOutput,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    match output {
        EnclosureOutput::Omitted | EnclosureOutput::Unavailable(_) => Ok(()),
        EnclosureOutput::Included(value) => collect_certified_interval_stats(value, stats),
    }
}

fn collect_certified_interval_stats(
    value: &CertifiedIntervalPresentation,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    collect_presentation_node_stats(&value.presentation, stats)?;
    match &value.bounds {
        CertifiedIntervalBounds::ExactDyadic { lower, upper } => {
            collect_exact_dyadic_stats(lower, stats)?;
            collect_exact_dyadic_stats(upper, stats)
        }
        CertifiedIntervalBounds::DecimalScientific { lower, upper, .. } => {
            collect_decimal_scientific_bound_stats(lower, stats)?;
            collect_decimal_scientific_bound_stats(upper, stats)
        }
    }
}

fn collect_exact_dyadic_stats(
    value: &ExactDyadic,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    add_output_string(stats, value.coefficient.to_string())?;
    add_output_string(stats, value.exponent_two.to_string())
}

fn collect_decimal_scientific_bound_stats(
    value: &DecimalScientificBound,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    add_output_text(stats, &value.significand)?;
    add_output_text(stats, &value.exponent_ten)
}

fn collect_presentation_node_stats(
    node: &PresentationNode,
    stats: &mut PresentationOutputStats,
) -> Result<(), PresentationError> {
    add_presentation_node(stats)?;
    match node {
        PresentationNode::Text(text) => add_output_text(stats, text),
        PresentationNode::Row(children) => {
            for child in children {
                collect_presentation_node_stats(child, stats)?;
            }
            Ok(())
        }
        PresentationNode::Fraction {
            numerator,
            denominator,
        } => {
            add_output_bytes(stats, "/".len())?;
            collect_presentation_node_stats(numerator, stats)?;
            collect_presentation_node_stats(denominator, stats)
        }
        PresentationNode::Superscript { base, exponent } => {
            add_output_bytes(stats, "^".len())?;
            collect_presentation_node_stats(base, stats)?;
            collect_presentation_node_stats(exponent, stats)
        }
        PresentationNode::Subscript { base, subscript } => {
            add_output_bytes(stats, "_".len())?;
            collect_presentation_node_stats(base, stats)?;
            collect_presentation_node_stats(subscript, stats)
        }
        PresentationNode::Radical { index, radicand } => {
            match index {
                RadicalIndex::Square => add_output_bytes(stats, "sqrt".len())?,
                RadicalIndex::Nth(value) => {
                    add_output_bytes(stats, "root".len())?;
                    add_output_string(stats, value.inner.inner.to_string())?;
                }
            }
            collect_presentation_node_stats(radicand, stats)
        }
        PresentationNode::Function { name, argument } => {
            add_output_bytes(stats, function_name_text(*name).len())?;
            collect_presentation_node_stats(argument, stats)
        }
        PresentationNode::Parenthesized(value) => {
            add_output_bytes(stats, "()".len())?;
            collect_presentation_node_stats(value, stats)
        }
    }
}

fn add_presentation_node(stats: &mut PresentationOutputStats) -> Result<(), PresentationError> {
    stats.presentation_nodes = stats
        .presentation_nodes
        .checked_add(1)
        .ok_or_else(presentation_nodes_limit_error)?;
    Ok(())
}

fn add_output_text(
    stats: &mut PresentationOutputStats,
    text: &str,
) -> Result<(), PresentationError> {
    add_output_bytes(stats, text.len())
}

fn add_output_string(
    stats: &mut PresentationOutputStats,
    text: String,
) -> Result<(), PresentationError> {
    add_output_bytes(stats, text.len())
}

fn add_output_bytes(
    stats: &mut PresentationOutputStats,
    bytes: usize,
) -> Result<(), PresentationError> {
    stats.output_bytes = stats
        .output_bytes
        .checked_add(bytes)
        .ok_or_else(output_too_large_error)?;
    Ok(())
}

fn presentation_nodes_limit_error() -> PresentationError {
    PresentationError::ComputationLimit(ComputationLimitError {
        kind: ComputationLimitKind::PresentationNodes,
    })
}

fn output_too_large_error() -> PresentationError {
    PresentationError::InputLimit(InputLimitError {
        kind: InputLimitErrorKind::OutputTooLarge,
    })
}

fn function_name_text(name: FunctionName) -> &'static str {
    match name {
        FunctionName::Sin => "sin",
        FunctionName::Cos => "cos",
        FunctionName::Tan => "tan",
        FunctionName::Asin => "asin",
        FunctionName::Acos => "acos",
        FunctionName::Atan => "atan",
        FunctionName::Sqrt => "sqrt",
        FunctionName::Exp => "exp",
        FunctionName::Log => "log",
        FunctionName::Ln => "ln",
    }
}

fn partial_reason(calculation: &Calculation) -> Option<IncompleteReason> {
    if let SimplificationStatus::PartiallySimplified { reason } =
        &calculation.metadata.simplification_status
    {
        return Some(reason.clone());
    }
    match &calculation.scientific {
        ScientificOutput::Unavailable(value) => Some(value.reason.clone()),
        ScientificOutput::Omitted | ScientificOutput::Included(_) => match &calculation.enclosure {
            EnclosureOutput::Unavailable(value) => Some(value.reason.clone()),
            EnclosureOutput::Omitted | EnclosureOutput::Included(_) => None,
        },
    }
}

fn partial_certified_enclosure(
    evaluation: &EvaluationOutcome,
    limits: &ResourceLimits,
) -> Result<Option<CertifiedIntervalPresentation>, PresentationError> {
    let CertifiedEnclosureState::Available(interval) = &evaluation.value.certified_enclosure else {
        return Ok(None);
    };
    let certified_enclosure =
        certified_interval_presentation(interval, EnclosureFormat::ExactDyadic)?;
    validate_certified_interval_output(&certified_enclosure, limits)?;
    Ok(Some(certified_enclosure))
}

fn precision_bits_for_output_request(
    request: &PresentationRequest,
) -> Result<u32, PresentationError> {
    let mut digits = 0;
    if let ScientificOutputRequest::Include {
        significant_digits, ..
    } = request.scientific_output
    {
        digits = digits.max(significant_digits.get());
    }
    if let EnclosureOutputRequest::Include {
        format: EnclosureFormat::DecimalScientific { significant_digits },
    } = request.enclosure_output
    {
        digits = digits.max(significant_digits.get());
    }
    if digits == 0 {
        Ok(128)
    } else {
        precision_bits_for_decimal_digits(digits)
    }
}

fn rational_exact_presentation(
    rational: &Rational,
    format: ExactFormatPreference,
) -> ExactPresentation {
    match format {
        ExactFormatPreference::FiniteDecimal => finite_decimal_presentation(rational)
            .unwrap_or_else(|| canonical_rational_presentation(rational)),
        ExactFormatPreference::MixedFraction => mixed_fraction_presentation(rational)
            .unwrap_or_else(|| canonical_rational_presentation(rational)),
        ExactFormatPreference::Auto
        | ExactFormatPreference::Rational
        | ExactFormatPreference::Symbolic => canonical_rational_presentation(rational),
    }
}

fn canonical_rational_presentation(rational: &Rational) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: rational_exact_representation_kind(rational),
        presentation: rational_presentation(rational),
        plain_text: rational.to_string(),
    }
}

fn finite_decimal_presentation(rational: &Rational) -> Option<ExactPresentation> {
    if rational.is_integer() {
        return Some(canonical_rational_presentation(rational));
    }
    let plain_text = finite_decimal_plain_text(rational)?;
    Some(ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::FiniteDecimal,
        presentation: PresentationNode::Text(plain_text.clone()),
        plain_text,
    })
}

fn finite_decimal_plain_text(rational: &Rational) -> Option<String> {
    let mut denominator = rational.denominator.inner.inner.clone();
    let mut twos = 0_u32;
    while (&denominator % 2_u8).is_zero() {
        denominator /= 2_u8;
        twos = twos.checked_add(1)?;
    }
    let mut fives = 0_u32;
    while (&denominator % 5_u8).is_zero() {
        denominator /= 5_u8;
        fives = fives.checked_add(1)?;
    }
    if denominator != BigInt::from(1_u8) {
        return None;
    }

    let scale = twos.max(fives);
    let mut scaled = rational.numerator.inner.clone();
    if scale > twos {
        scaled *= BigInt::from(2_u8).pow(scale - twos);
    }
    if scale > fives {
        scaled *= BigInt::from(5_u8).pow(scale - fives);
    }

    let negative = scaled.sign() == Sign::Minus;
    let mut digits = scaled.abs().to_string();
    let scale_usize = usize::try_from(scale).ok()?;
    if digits.len() <= scale_usize {
        let leading_zero_count = scale_usize.checked_add(1)?.checked_sub(digits.len())?;
        let mut padded = String::new();
        for _ in 0..leading_zero_count {
            padded.push('0');
        }
        padded.push_str(&digits);
        digits = padded;
    }
    let point_index = digits.len().checked_sub(scale_usize)?;
    let mut text = String::new();
    if negative {
        text.push('-');
    }
    text.push_str(&digits[..point_index]);
    if scale_usize > 0 {
        text.push('.');
        text.push_str(&digits[point_index..]);
    }
    Some(text)
}

fn mixed_fraction_presentation(rational: &Rational) -> Option<ExactPresentation> {
    if rational.is_integer() {
        return Some(canonical_rational_presentation(rational));
    }
    let numerator = rational.numerator.inner.abs();
    let denominator = &rational.denominator.inner.inner;
    if numerator < *denominator {
        return None;
    }
    let (whole, remainder) = numerator.div_rem(denominator);
    if whole.is_zero() || remainder.is_zero() {
        return Some(canonical_rational_presentation(rational));
    }

    let negative = rational.numerator.inner.sign() == Sign::Minus;
    let whole_text = if negative {
        format!("-{whole}")
    } else {
        whole.to_string()
    };
    let fraction = PresentationNode::Fraction {
        numerator: Box::new(PresentationNode::Text(remainder.to_string())),
        denominator: Box::new(PresentationNode::Text(denominator.to_string())),
    };
    let plain_text = format!("{whole_text} {remainder}/{denominator}");
    Some(ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::Rational,
        presentation: PresentationNode::Row(vec![
            PresentationNode::Text(whole_text),
            PresentationNode::Text(String::from(" ")),
            fraction,
        ]),
        plain_text,
    })
}

fn radical_presentation(value: &SimpleRadical) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::Radical,
        presentation: radical_presentation_node(value),
        plain_text: radical_plain_text(value),
    }
}

fn radical_linear_combination_presentation(value: &RadicalLinearCombination) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::Radical,
        presentation: radical_linear_combination_presentation_node(value),
        plain_text: radical_linear_combination_plain_text(value),
    }
}

fn radical_linear_combination_plain_text(value: &RadicalLinearCombination) -> String {
    let mut text = String::new();
    if !value.rational.is_zero() {
        push_plain_text_term(&mut text, value.rational.to_string());
    }
    for radical in &value.radicals {
        push_plain_text_term(&mut text, radical_plain_text(radical));
    }
    text
}

fn push_plain_text_term(target: &mut String, term: String) {
    if target.is_empty() {
        target.push_str(&term);
    } else if let Some(term) = term.strip_prefix('-') {
        target.push_str(" - ");
        target.push_str(term);
    } else {
        target.push_str(" + ");
        target.push_str(&term);
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
            _ => format!("{numerator}*{radical}"),
        };
    }

    match numerator.as_str() {
        "1" => format!("{radical}/{denominator}"),
        "-1" => format!("-{radical}/{denominator}"),
        _ => format!("{numerator}*{radical}/{denominator}"),
    }
}

fn radical_linear_combination_presentation_node(
    value: &RadicalLinearCombination,
) -> PresentationNode {
    let mut children = Vec::new();
    if !value.rational.is_zero() {
        children.push(rational_presentation(&value.rational));
    }
    for radical in &value.radicals {
        push_presentation_term(
            &mut children,
            radical.coefficient.is_negative(),
            radical_presentation_node(&absolute_radical(radical)),
        );
    }

    if children.len() == 1 {
        children
            .pop()
            .expect("single-node radical expression has one child")
    } else {
        PresentationNode::Row(children)
    }
}

fn push_presentation_term(
    children: &mut Vec<PresentationNode>,
    is_negative: bool,
    unsigned_node: PresentationNode,
) {
    if children.is_empty() {
        if is_negative {
            children.push(PresentationNode::Text(String::from("-")));
        }
    } else if is_negative {
        children.push(PresentationNode::Text(String::from(" - ")));
    } else {
        children.push(PresentationNode::Text(String::from(" + ")));
    }
    children.push(unsigned_node);
}

fn absolute_radical(value: &SimpleRadical) -> SimpleRadical {
    SimpleRadical {
        coefficient: if value.coefficient.is_negative() {
            value.coefficient.negate()
        } else {
            value.coefficient.clone()
        },
        radicand: value.radicand.clone(),
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

fn real_algebraic_presentation(source: &str) -> ExactPresentation {
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::RealAlgebraic,
        presentation: PresentationNode::Text(String::from(source)),
        plain_text: String::from(source),
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

fn symbolic_presentation_from_dag(dag: &ExactExpressionDag) -> ExactPresentation {
    let rendered = render_symbolic_node(dag, dag.root());
    ExactPresentation {
        relation: ResultRelation::ExactEqual,
        representation: ExactRepresentationKind::GeneralSymbolic,
        presentation: PresentationNode::Text(rendered.text.clone()),
        plain_text: rendered.text,
    }
}

#[derive(Clone, Debug)]
struct RenderedSymbolic {
    text: String,
    precedence: u8,
}

#[derive(Clone, Debug)]
struct SignedRenderedSymbolic {
    negative: bool,
    value: RenderedSymbolic,
}

#[derive(Clone, Debug)]
struct SymbolicPiCoefficient {
    coefficient: Rational,
    contains_pi: bool,
}

#[derive(Clone, Debug)]
struct SymbolicPiShiftArgument {
    phase: Rational,
    remainder: SignedRenderedSymbolic,
}

#[derive(Clone, Debug)]
struct SymbolicLogTerm {
    coefficient: Rational,
    argument: ExprId,
}

const SYMBOLIC_PRECEDENCE_ADD: u8 = 1;
const SYMBOLIC_PRECEDENCE_MULTIPLY: u8 = 2;
const SYMBOLIC_PRECEDENCE_POWER: u8 = 3;
const SYMBOLIC_PRECEDENCE_PREFIX: u8 = 4;
const SYMBOLIC_PRECEDENCE_ATOM: u8 = 5;

#[derive(Clone, Debug)]
struct RenderedSource {
    node: PresentationNode,
    precedence: u8,
}

fn source_presentation(expression: &SourceExpr) -> RenderedSource {
    match expression {
        SourceExpr::Number { literal, .. } => RenderedSource {
            node: PresentationNode::Text(literal.clone()),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        SourceExpr::Constant {
            constant: Constant::Pi,
            ..
        } => RenderedSource {
            node: PresentationNode::Text(String::from("π")),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        SourceExpr::Constant {
            constant: Constant::Euler,
            ..
        } => RenderedSource {
            node: PresentationNode::Text(String::from("e")),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        SourceExpr::Unary { op, expr, .. } => {
            let expr = source_parenthesize(source_presentation(expr), SYMBOLIC_PRECEDENCE_PREFIX);
            let sign = match op {
                UnaryOperator::Plus => "+",
                UnaryOperator::Negate => "-",
            };
            RenderedSource {
                node: PresentationNode::Row(vec![PresentationNode::Text(String::from(sign)), expr]),
                precedence: SYMBOLIC_PRECEDENCE_PREFIX,
            }
        }
        SourceExpr::Binary {
            op, left, right, ..
        } => source_binary_presentation(*op, left, right),
        SourceExpr::Percent { expr, .. } => RenderedSource {
            node: PresentationNode::Row(vec![
                source_parenthesize(source_presentation(expr), SYMBOLIC_PRECEDENCE_ATOM),
                PresentationNode::Text(String::from("%")),
            ]),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        SourceExpr::Function {
            function,
            argument,
            base,
            ..
        } => source_function_presentation(*function, argument, base.as_deref()),
    }
}

fn source_binary_presentation(
    op: BinaryOperator,
    left: &SourceExpr,
    right: &SourceExpr,
) -> RenderedSource {
    match op {
        BinaryOperator::Add | BinaryOperator::Subtract => {
            let operator = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Power => {
                    unreachable!("operator is constrained by outer match")
                }
            };
            RenderedSource {
                node: PresentationNode::Row(vec![
                    source_parenthesize(source_presentation(left), SYMBOLIC_PRECEDENCE_ADD),
                    PresentationNode::Text(String::from(operator)),
                    source_parenthesize(source_presentation(right), SYMBOLIC_PRECEDENCE_ADD),
                ]),
                precedence: SYMBOLIC_PRECEDENCE_ADD,
            }
        }
        BinaryOperator::Multiply => RenderedSource {
            node: PresentationNode::Row(vec![
                source_parenthesize(source_presentation(left), SYMBOLIC_PRECEDENCE_MULTIPLY),
                PresentationNode::Text(String::from("×")),
                source_parenthesize(source_presentation(right), SYMBOLIC_PRECEDENCE_MULTIPLY),
            ]),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        },
        BinaryOperator::Divide => RenderedSource {
            node: PresentationNode::Fraction {
                numerator: Box::new(source_presentation(left).node),
                denominator: Box::new(source_presentation(right).node),
            },
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        BinaryOperator::Power => RenderedSource {
            node: PresentationNode::Superscript {
                base: Box::new(source_parenthesize(
                    source_presentation(left),
                    SYMBOLIC_PRECEDENCE_POWER,
                )),
                exponent: Box::new(source_parenthesize(
                    source_presentation(right),
                    SYMBOLIC_PRECEDENCE_POWER,
                )),
            },
            precedence: SYMBOLIC_PRECEDENCE_POWER,
        },
    }
}

fn source_function_presentation(
    function: Function,
    argument: &SourceExpr,
    base: Option<&SourceExpr>,
) -> RenderedSource {
    let argument = source_presentation(argument).node;
    let node = match (function, base) {
        (Function::Ln, None) => PresentationNode::Function {
            name: FunctionName::Ln,
            argument: Box::new(argument),
        },
        (Function::Log, Some(base)) => {
            log_base_presentation(argument, source_presentation(base).node)
        }
        (Function::Exp, None) => PresentationNode::Superscript {
            base: Box::new(PresentationNode::Text(String::from("e"))),
            exponent: Box::new(argument),
        },
        (Function::Exp, Some(base)) => PresentationNode::Superscript {
            base: Box::new(source_presentation(base).node),
            exponent: Box::new(argument),
        },
        (Function::Sqrt, None) => PresentationNode::Radical {
            index: RadicalIndex::Square,
            radicand: Box::new(argument),
        },
        (Function::Root, Some(index)) => root_source_presentation(argument, index),
        (Function::Abs, None) => fenced_source_presentation("|", argument, "|"),
        (Function::Floor, None) => fenced_source_presentation("⌊", argument, "⌋"),
        (Function::Factorial, None) => {
            PresentationNode::Row(vec![argument, PresentationNode::Text(String::from("!"))])
        }
        (
            function @ (Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm),
            Some(right),
        ) => source_binary_function_presentation(
            symbolic_function_name(function),
            argument,
            source_presentation(right).node,
        ),
        (
            function @ (Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Log),
            None,
        ) => PresentationNode::Function {
            name: legacy_function_presentation_name(function)
                .expect("legacy function source arm only passes FunctionName-backed functions"),
            argument: Box::new(argument),
        },
        (
            function @ (Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Ln),
            Some(right),
        ) => source_binary_function_presentation(
            symbolic_function_name(function),
            argument,
            source_presentation(right).node,
        ),
        (
            function @ (Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh),
            None,
        ) => text_function_presentation(symbolic_function_name(function), argument),
        (
            Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh,
            _,
        ) => text_function_presentation(symbolic_function_name(function), argument),
    };
    RenderedSource {
        node,
        precedence: SYMBOLIC_PRECEDENCE_ATOM,
    }
}

fn root_source_presentation(argument: PresentationNode, index: &SourceExpr) -> PresentationNode {
    if let Some(index) = source_positive_integer(index) {
        PresentationNode::Radical {
            index: RadicalIndex::Nth(index),
            radicand: Box::new(argument),
        }
    } else {
        source_binary_function_presentation("root", argument, source_presentation(index).node)
    }
}

fn source_positive_integer(expression: &SourceExpr) -> Option<PositiveInteger> {
    let SourceExpr::Number { literal, .. } = expression else {
        return None;
    };
    let value = Rational::from_decimal_literal(literal).ok()?;
    if value.is_integer() {
        PositiveInteger::new(value.numerator)
    } else {
        None
    }
}

fn fenced_source_presentation(
    left: &str,
    argument: PresentationNode,
    right: &str,
) -> PresentationNode {
    PresentationNode::Row(vec![
        PresentationNode::Text(String::from(left)),
        argument,
        PresentationNode::Text(String::from(right)),
    ])
}

fn source_binary_function_presentation(
    name: &str,
    left: PresentationNode,
    right: PresentationNode,
) -> PresentationNode {
    PresentationNode::Row(vec![
        PresentationNode::Text(String::from(name)),
        PresentationNode::Text(String::from("(")),
        left,
        PresentationNode::Text(String::from(",")),
        right,
        PresentationNode::Text(String::from(")")),
    ])
}

fn text_function_presentation(name: &str, argument: PresentationNode) -> PresentationNode {
    PresentationNode::Row(vec![
        PresentationNode::Text(String::from(name)),
        PresentationNode::Text(String::from("(")),
        argument,
        PresentationNode::Text(String::from(")")),
    ])
}

fn log_base_presentation(argument: PresentationNode, base: PresentationNode) -> PresentationNode {
    PresentationNode::Row(vec![
        PresentationNode::Subscript {
            base: Box::new(PresentationNode::Text(String::from("log"))),
            subscript: Box::new(base),
        },
        PresentationNode::Text(String::from("(")),
        argument,
        PresentationNode::Text(String::from(")")),
    ])
}

fn legacy_function_presentation_name(function: Function) -> Option<FunctionName> {
    match function {
        Function::Sin => Some(FunctionName::Sin),
        Function::Cos => Some(FunctionName::Cos),
        Function::Tan => Some(FunctionName::Tan),
        Function::Asin => Some(FunctionName::Asin),
        Function::Acos => Some(FunctionName::Acos),
        Function::Atan => Some(FunctionName::Atan),
        Function::Sqrt => Some(FunctionName::Sqrt),
        Function::Exp => Some(FunctionName::Exp),
        Function::Log => Some(FunctionName::Log),
        Function::Ln => Some(FunctionName::Ln),
        Function::Root
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => None,
    }
}

fn source_parenthesize(value: RenderedSource, parent_precedence: u8) -> PresentationNode {
    if value.precedence <= parent_precedence {
        PresentationNode::Parenthesized(Box::new(value.node))
    } else {
        value.node
    }
}

fn render_symbolic_node(dag: &ExactExpressionDag, id: ExprId) -> RenderedSymbolic {
    signed_symbolic_to_rendered(render_signed_symbolic_node(dag, id))
}

fn render_signed_symbolic_node(dag: &ExactExpressionDag, id: ExprId) -> SignedRenderedSymbolic {
    render_signed_symbolic_expression_node(dag, dag.node(id))
}

fn render_signed_symbolic_expression_node(
    dag: &ExactExpressionDag,
    node: &ExpressionNode,
) -> SignedRenderedSymbolic {
    match node {
        ExpressionNode::Rational(rational_id) => {
            let rational = dag.rational(*rational_id);
            if rational.is_negative() {
                SignedRenderedSymbolic {
                    negative: true,
                    value: RenderedSymbolic {
                        text: rational.negate().to_string(),
                        precedence: SYMBOLIC_PRECEDENCE_ATOM,
                    },
                }
            } else {
                SignedRenderedSymbolic {
                    negative: false,
                    value: RenderedSymbolic {
                        text: rational.to_string(),
                        precedence: SYMBOLIC_PRECEDENCE_ATOM,
                    },
                }
            }
        }
        ExpressionNode::Exact(value) => render_exact_reduction(dag, *value),
        ExpressionNode::Add(list_id) => render_signed_symbolic_sum(dag, *list_id),
        ExpressionNode::Multiply(list_id) => render_signed_symbolic_product(dag, *list_id),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => render_signed_symbolic_division(dag, *numerator, *denominator),
        ExpressionNode::Function { function, argument } => {
            render_signed_symbolic_function(dag, *function, *argument)
        }
        ExpressionNode::Constant(_)
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => SignedRenderedSymbolic {
            negative: false,
            value: render_unsigned_symbolic_expression_node(dag, node),
        },
    }
}

fn render_unsigned_symbolic_expression_node(
    dag: &ExactExpressionDag,
    node: &ExpressionNode,
) -> RenderedSymbolic {
    match node {
        ExpressionNode::Rational(rational_id) => RenderedSymbolic {
            text: dag.rational(*rational_id).to_string(),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Exact(value) => {
            signed_symbolic_to_rendered(render_exact_reduction(dag, *value))
        }
        ExpressionNode::Constant(Constant::Pi) => RenderedSymbolic {
            text: String::from("pi"),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Constant(Constant::Euler) => RenderedSymbolic {
            text: String::from("e"),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Add(list_id) => render_symbolic_sum(dag, *list_id),
        ExpressionNode::Multiply(list_id) => {
            signed_symbolic_to_rendered(render_signed_symbolic_product(dag, *list_id))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => signed_symbolic_to_rendered(render_signed_symbolic_division(
            dag,
            *numerator,
            *denominator,
        )),
        ExpressionNode::Power { base, exponent } => {
            let exponent_id = *exponent;
            let base = render_symbolic_node(dag, *base);
            let exponent = render_symbolic_node(dag, exponent_id);
            let base_text =
                if base.text.starts_with('-') || base.precedence <= SYMBOLIC_PRECEDENCE_POWER {
                    format!("({})", base.text)
                } else {
                    parenthesize_symbolic(&base, SYMBOLIC_PRECEDENCE_POWER)
                };
            RenderedSymbolic {
                text: format!(
                    "{}^{}",
                    base_text,
                    if symbolic_power_exponent_needs_parentheses(dag, exponent_id) {
                        format!("({})", exponent.text)
                    } else {
                        parenthesize_symbolic(&exponent, SYMBOLIC_PRECEDENCE_POWER)
                    }
                ),
                precedence: SYMBOLIC_PRECEDENCE_POWER,
            }
        }
        ExpressionNode::LogBase { argument, base } => {
            let argument = render_symbolic_node(dag, *argument);
            let base = render_symbolic_node(dag, *base);
            RenderedSymbolic {
                text: format!("log({},{})", argument.text, base.text),
                precedence: SYMBOLIC_PRECEDENCE_ATOM,
            }
        }
        ExpressionNode::Function { function, argument } => {
            let argument = render_symbolic_node(dag, *argument);
            RenderedSymbolic {
                text: format!("{}({})", symbolic_function_name(*function), argument.text),
                precedence: SYMBOLIC_PRECEDENCE_ATOM,
            }
        }
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => {
            let left = render_symbolic_node(dag, *left);
            let right = render_symbolic_node(dag, *right);
            RenderedSymbolic {
                text: format!(
                    "{}({},{})",
                    symbolic_function_name(*function),
                    left.text,
                    right.text
                ),
                precedence: SYMBOLIC_PRECEDENCE_ATOM,
            }
        }
    }
}

fn render_exact_reduction(dag: &ExactExpressionDag, id: ExactValueId) -> SignedRenderedSymbolic {
    match dag.exact_value(id) {
        ExactReduction::PiMultiple(value) => {
            signed_rendered_atom(rational_pi_plain_text(value.coefficient()))
        }
        ExactReduction::Radical(RadicalReduction::Rational(value)) => {
            signed_rendered_atom(value.value().to_string())
        }
        ExactReduction::Radical(RadicalReduction::Radical(value)) => {
            signed_rendered_atom(symbolic_simple_radical_text(value.value()))
        }
        ExactReduction::Radical(RadicalReduction::LinearCombination(value)) => {
            let text = radical_linear_combination_plain_text(value.value());
            SignedRenderedSymbolic {
                negative: false,
                value: RenderedSymbolic {
                    precedence: if text.contains(" + ") || text.contains(" - ") {
                        SYMBOLIC_PRECEDENCE_ADD
                    } else {
                        SYMBOLIC_PRECEDENCE_MULTIPLY
                    },
                    text,
                },
            }
        }
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Rational(value)) => {
            signed_rendered_atom(value.value().to_string())
        }
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(_)) => {
            render_signed_symbolic_expression_node(dag, dag.exact_presentation(id))
        }
        ExactReduction::Symbolic => {
            render_signed_symbolic_expression_node(dag, dag.exact_presentation(id))
        }
    }
}

fn symbolic_simple_radical_text(value: &SimpleRadical) -> String {
    let coefficient = &value.coefficient;
    let radical = format!("sqrt({})", value.radicand.inner);
    if coefficient == &Rational::one() {
        radical
    } else if coefficient == &Rational::from_integer(Integer::from(-1)) {
        format!("-{radical}")
    } else {
        format!("{coefficient}*{radical}")
    }
}

fn signed_rendered_atom(text: String) -> SignedRenderedSymbolic {
    let precedence = if text.contains('*') || text.contains('/') {
        SYMBOLIC_PRECEDENCE_MULTIPLY
    } else {
        SYMBOLIC_PRECEDENCE_ATOM
    };
    if let Some(magnitude) = text.strip_prefix('-') {
        SignedRenderedSymbolic {
            negative: true,
            value: RenderedSymbolic {
                text: String::from(magnitude),
                precedence,
            },
        }
    } else {
        SignedRenderedSymbolic {
            negative: false,
            value: RenderedSymbolic { text, precedence },
        }
    }
}

fn symbolic_power_exponent_needs_parentheses(dag: &ExactExpressionDag, id: ExprId) -> bool {
    matches!(
        dag.node(id),
        ExpressionNode::Rational(rational) if !dag.rational(*rational).is_integer()
    )
}

fn render_symbolic_sum(dag: &ExactExpressionDag, list_id: ExprListId) -> RenderedSymbolic {
    let terms = dag
        .list(list_id)
        .iter()
        .filter(|child| !node_is_exact_zero(dag, **child))
        .map(|child| render_signed_symbolic_node(dag, *child))
        .collect::<Vec<_>>();
    render_symbolic_sum_terms(&terms)
}

fn render_signed_symbolic_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> SignedRenderedSymbolic {
    if dag.symbolic_rewrites_allowed() {
        if let Ok(Some(reduction)) = logarithm_sum_reduction(dag, dag.list(list_id)) {
            let numerator =
                render_symbolic_log_argument_product(dag, &reduction.numerator_arguments);
            let denominator =
                render_symbolic_log_argument_product(dag, &reduction.denominator_arguments);
            let argument = if reduction.denominator_arguments.is_empty() {
                numerator
            } else {
                format!(
                    "{numerator}/{}",
                    if reduction.denominator_arguments.len() > 1 {
                        format!("({denominator})")
                    } else {
                        denominator
                    }
                )
            };
            let base = render_symbolic_node(dag, reduction.base);
            let logarithm =
                render_scaled_symbolic_log_text(&argument, &base.text, &reduction.scale);
            if reduction.offset.value().is_zero() {
                return logarithm;
            }
            return render_signed_symbolic_sum_terms(&[
                signed_rational_symbolic(reduction.offset.value()),
                logarithm,
            ]);
        }
    }
    let terms = dag
        .list(list_id)
        .iter()
        .filter(|child| !node_is_exact_zero(dag, **child))
        .map(|child| render_signed_symbolic_node(dag, *child))
        .collect::<Vec<_>>();
    render_signed_symbolic_sum_terms(&terms)
}

fn render_symbolic_log_argument_product(dag: &ExactExpressionDag, values: &[ExprId]) -> String {
    if values.is_empty() {
        return String::from("1");
    }
    values
        .iter()
        .map(|value| {
            parenthesize_symbolic(
                &render_symbolic_node(dag, *value),
                SYMBOLIC_PRECEDENCE_MULTIPLY,
            )
        })
        .collect::<Vec<_>>()
        .join("*")
}

fn render_signed_symbolic_sum_terms(terms: &[SignedRenderedSymbolic]) -> SignedRenderedSymbolic {
    if terms.is_empty() {
        return SignedRenderedSymbolic {
            negative: false,
            value: RenderedSymbolic {
                text: String::from("0"),
                precedence: SYMBOLIC_PRECEDENCE_ATOM,
            },
        };
    }
    if terms.len() == 1 {
        return terms[0].clone();
    }
    if terms.iter().all(|term| term.negative) {
        let positive_terms = terms
            .iter()
            .map(|term| SignedRenderedSymbolic {
                negative: false,
                value: term.value.clone(),
            })
            .collect::<Vec<_>>();
        SignedRenderedSymbolic {
            negative: true,
            value: render_symbolic_sum_terms(&positive_terms),
        }
    } else {
        SignedRenderedSymbolic {
            negative: false,
            value: render_symbolic_sum_terms(terms),
        }
    }
}

fn render_symbolic_sum_terms(terms: &[SignedRenderedSymbolic]) -> RenderedSymbolic {
    let mut text = String::new();
    for signed in terms {
        let term = parenthesize_symbolic(&signed.value, SYMBOLIC_PRECEDENCE_ADD);
        if text.is_empty() {
            if signed.negative {
                text.push('-');
            }
            text.push_str(&term);
        } else if signed.negative {
            text.push('-');
            text.push_str(&term);
        } else {
            text.push('+');
            text.push_str(&term);
        }
    }
    RenderedSymbolic {
        text,
        precedence: SYMBOLIC_PRECEDENCE_ADD,
    }
}

fn render_signed_symbolic_division(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
) -> SignedRenderedSymbolic {
    if dag.symbolic_rewrites_allowed() {
        if let Ok(Some((argument, base, scale))) =
            logarithm_quotient_identity(dag, numerator, denominator)
        {
            return render_scaled_symbolic_log_base(dag, argument, base, &scale);
        }
    }
    let numerator = render_signed_symbolic_node(dag, numerator);
    let denominator = render_signed_symbolic_node(dag, denominator);
    SignedRenderedSymbolic {
        negative: numerator.negative ^ denominator.negative,
        value: RenderedSymbolic {
            text: format!(
                "{}/{}",
                parenthesize_symbolic(&numerator.value, SYMBOLIC_PRECEDENCE_MULTIPLY),
                parenthesize_symbolic(&denominator.value, SYMBOLIC_PRECEDENCE_MULTIPLY)
            ),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        },
    }
}

fn render_signed_symbolic_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> SignedRenderedSymbolic {
    let children = dag.list(list_id);
    if dag.symbolic_rewrites_allowed() {
        if let Ok(Some(reduction)) = logarithm_product_reduction(dag, children) {
            if let Some((argument, base)) = reduction.logarithm {
                return render_scaled_symbolic_log_base(dag, argument, base, &reduction.scale);
            }
            return signed_rational_symbolic(reduction.scale.value());
        }
    }
    let mut negative = false;
    let mut factors = Vec::new();
    for child in dag.list(list_id) {
        let signed = render_signed_symbolic_node(dag, *child);
        negative ^= signed.negative;
        if signed.value.text == "1" {
            continue;
        }
        factors.push(parenthesize_symbolic(
            &signed.value,
            SYMBOLIC_PRECEDENCE_MULTIPLY,
        ));
    }
    if factors.is_empty() {
        factors.push(String::from("1"));
    }
    SignedRenderedSymbolic {
        negative,
        value: RenderedSymbolic {
            text: factors.join("*"),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        },
    }
}

fn signed_rational_symbolic(value: &Rational) -> SignedRenderedSymbolic {
    let negative = value.is_negative();
    SignedRenderedSymbolic {
        negative,
        value: RenderedSymbolic {
            text: if negative {
                value.negate().to_string()
            } else {
                value.to_string()
            },
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
    }
}

fn render_scaled_symbolic_log_base(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
    scale: &RationalEvaluation,
) -> SignedRenderedSymbolic {
    let argument = render_symbolic_node(dag, argument);
    let base = render_symbolic_node(dag, base);
    render_scaled_symbolic_log_text(&argument.text, &base.text, scale)
}

fn render_scaled_symbolic_log_text(
    argument: &str,
    base: &str,
    scale: &RationalEvaluation,
) -> SignedRenderedSymbolic {
    let logarithm = RenderedSymbolic {
        text: format!("log({argument},{base})"),
        precedence: SYMBOLIC_PRECEDENCE_ATOM,
    };
    let negative = scale.value().is_negative();
    let magnitude = if negative {
        scale.value().negate()
    } else {
        scale.value().clone()
    };
    let value = if magnitude == Rational::one() {
        logarithm
    } else {
        RenderedSymbolic {
            text: format!("{}*{}", magnitude, logarithm.text),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        }
    };
    SignedRenderedSymbolic { negative, value }
}

fn render_signed_symbolic_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> SignedRenderedSymbolic {
    if function == Function::Sqrt {
        if let Some(rendered) = render_symbolic_square_root_of_square(dag, argument) {
            return rendered;
        }
    }

    if matches!(function, Function::Log | Function::Ln) {
        if let Some(rendered) = render_symbolic_log_expansion(dag, argument) {
            return rendered;
        }
    }

    if matches!(function, Function::Sin | Function::Cos | Function::Tan) {
        if let Some(shifted_argument) = render_symbolic_pi_shift_argument(dag, argument) {
            if let Some(rendered) =
                render_symbolic_shifted_trig_function(function, shifted_argument)
            {
                return rendered;
            }
        }
    }

    render_signed_symbolic_function_from_signed_argument(
        function,
        render_signed_symbolic_node(dag, argument),
    )
}

fn render_signed_symbolic_function_from_signed_argument(
    function: Function,
    signed_argument: SignedRenderedSymbolic,
) -> SignedRenderedSymbolic {
    match function {
        Function::Sin
        | Function::Tan
        | Function::Asin
        | Function::Atan
        | Function::Sinh
        | Function::Tanh
        | Function::Asinh
        | Function::Atanh
            if signed_argument.negative =>
        {
            SignedRenderedSymbolic {
                negative: true,
                value: render_symbolic_function_value(function, &signed_argument.value),
            }
        }
        Function::Cos | Function::Cosh | Function::Abs if signed_argument.negative => {
            SignedRenderedSymbolic {
                negative: false,
                value: render_symbolic_function_value(function, &signed_argument.value),
            }
        }
        Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            let argument = signed_symbolic_to_rendered(signed_argument);
            SignedRenderedSymbolic {
                negative: false,
                value: render_symbolic_function_value(function, &argument),
            }
        }
    }
}

fn render_symbolic_square_root_of_square(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Option<SignedRenderedSymbolic> {
    let base = symbolic_square_base(dag, argument)?;
    if !symbolic_nonnegative_proof(dag, base) {
        return None;
    }
    Some(render_signed_symbolic_node(dag, base))
}

fn symbolic_square_base(dag: &ExactExpressionDag, id: ExprId) -> Option<ExprId> {
    match dag.semantic_node(id) {
        ExpressionNode::Power { base, exponent } => {
            (symbolic_rational_value(dag, *exponent)? == symbolic_rational(2, 1)).then_some(*base)
        }
        ExpressionNode::Multiply(list_id) => {
            let factors = dag.list(*list_id);
            if factors.len() == 2 && structurally_equal_expressions(dag, factors[0], factors[1]) {
                Some(factors[0])
            } else {
                None
            }
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn render_symbolic_log_expansion(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Option<SignedRenderedSymbolic> {
    if let ExpressionNode::Exact(value) = dag.node(argument) {
        if let ExactReduction::Radical(RadicalReduction::Radical(value)) = dag.exact_value(*value) {
            let value = value.value();
            if value.coefficient.is_negative() || value.coefficient.is_zero() {
                return None;
            }
            let mut terms = Vec::new();
            if value.coefficient != Rational::one() {
                terms.push(render_symbolic_log_term(
                    Rational::one(),
                    RenderedSymbolic {
                        text: value.coefficient.to_string(),
                        precedence: SYMBOLIC_PRECEDENCE_ATOM,
                    },
                ));
            }
            terms.push(render_symbolic_log_term(
                symbolic_rational(1, 2),
                RenderedSymbolic {
                    text: value.radicand.inner.to_string(),
                    precedence: SYMBOLIC_PRECEDENCE_ATOM,
                },
            ));
            return Some(render_signed_symbolic_sum_terms(&terms));
        }
    }
    if let ExpressionNode::Rational(rational) = dag.node(argument) {
        let value = dag.rational(*rational);
        if !value.is_integer() && !value.is_negative() && !value.is_zero() {
            let numerator = value.numerator.to_string();
            let denominator = value.denominator.inner.to_string();
            let mut terms = Vec::new();
            if numerator != "1" {
                terms.push(SignedRenderedSymbolic {
                    negative: false,
                    value: render_symbolic_function_value(
                        Function::Ln,
                        &RenderedSymbolic {
                            text: numerator,
                            precedence: SYMBOLIC_PRECEDENCE_ATOM,
                        },
                    ),
                });
            }
            terms.push(SignedRenderedSymbolic {
                negative: true,
                value: render_symbolic_function_value(
                    Function::Ln,
                    &RenderedSymbolic {
                        text: denominator,
                        precedence: SYMBOLIC_PRECEDENCE_ATOM,
                    },
                ),
            });
            return Some(render_signed_symbolic_sum_terms(&terms));
        }
    }
    let terms = symbolic_log_expansion_terms(dag, argument)?;
    Some(render_symbolic_log_terms(dag, terms))
}

fn symbolic_log_expansion_terms(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Option<Vec<SymbolicLogTerm>> {
    match dag.semantic_node(argument) {
        ExpressionNode::Multiply(list_id) => {
            let mut terms = Vec::new();
            for factor in dag.list(*list_id) {
                terms.extend(symbolic_log_expansion_terms(dag, *factor)?);
            }
            Some(terms)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let mut terms = symbolic_log_expansion_terms(dag, *numerator)?;
            let mut denominator_terms = symbolic_log_expansion_terms(dag, *denominator)?;
            for term in &mut denominator_terms {
                term.coefficient = term.coefficient.negate();
            }
            terms.extend(denominator_terms);
            Some(terms)
        }
        ExpressionNode::Power { base, exponent } => {
            if !symbolic_positive_proof(dag, *base) {
                return None;
            }
            let exponent = symbolic_rational_value(dag, *exponent)?;
            let mut terms = symbolic_log_expansion_terms(dag, *base)?;
            for term in &mut terms {
                term.coefficient = term.coefficient.multiply(&exponent);
            }
            Some(terms)
        }
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument: radicand,
        } => {
            if !symbolic_positive_proof(dag, *radicand) {
                return None;
            }
            let mut terms = symbolic_log_expansion_terms(dag, *radicand)?;
            for term in &mut terms {
                term.coefficient = term.coefficient.multiply(&symbolic_rational(1, 2));
            }
            Some(terms)
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Exp
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => {
            if !symbolic_positive_proof(dag, argument) {
                return None;
            }
            if matches!(
                symbolic_rational_value(dag, argument),
                Some(value) if value == Rational::one()
            ) {
                return Some(Vec::new());
            }
            Some(vec![SymbolicLogTerm {
                coefficient: Rational::one(),
                argument,
            }])
        }
    }
}

fn render_symbolic_log_terms(
    dag: &ExactExpressionDag,
    terms: Vec<SymbolicLogTerm>,
) -> SignedRenderedSymbolic {
    let mut rendered_terms: Vec<(Rational, RenderedSymbolic)> = Vec::new();
    for term in terms {
        if term.coefficient.is_zero() {
            continue;
        }
        let argument = render_symbolic_node(dag, term.argument);
        if argument.text == "1" {
            continue;
        }
        if let Some((coefficient, _)) = rendered_terms
            .iter_mut()
            .find(|(_, existing)| existing.text == argument.text)
        {
            *coefficient = coefficient.add(&term.coefficient);
        } else {
            rendered_terms.push((term.coefficient, argument));
        }
    }

    let terms = rendered_terms
        .into_iter()
        .filter(|(coefficient, _)| !coefficient.is_zero())
        .map(|(coefficient, argument)| render_symbolic_log_term(coefficient, argument))
        .collect::<Vec<_>>();
    render_signed_symbolic_sum_terms(&terms)
}

fn render_symbolic_log_term(
    coefficient: Rational,
    argument: RenderedSymbolic,
) -> SignedRenderedSymbolic {
    let negative = coefficient.is_negative();
    let magnitude = if negative {
        coefficient.negate()
    } else {
        coefficient
    };
    let logarithm = render_symbolic_function_value(Function::Ln, &argument);
    let value = if magnitude == Rational::one() {
        logarithm
    } else {
        RenderedSymbolic {
            text: format!(
                "{}*{}",
                magnitude,
                parenthesize_symbolic(&logarithm, SYMBOLIC_PRECEDENCE_MULTIPLY)
            ),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        }
    };
    SignedRenderedSymbolic { negative, value }
}

fn symbolic_positive_proof(dag: &ExactExpressionDag, id: ExprId) -> bool {
    if let Some(value) = symbolic_rational_value(dag, id) {
        return !value.is_negative() && !value.is_zero();
    }

    match dag.semantic_node(id) {
        ExpressionNode::Exact(value) => {
            symbolic_exact_reduction_sign(dag.exact_value(*value)) == Some(Ordering::Greater)
        }
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => true,
        ExpressionNode::Add(list_id) => dag
            .list(*list_id)
            .iter()
            .all(|child| symbolic_positive_proof(dag, *child)),
        ExpressionNode::Multiply(list_id) => dag
            .list(*list_id)
            .iter()
            .all(|child| symbolic_positive_proof(dag, *child)),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => symbolic_positive_proof(dag, *numerator) && symbolic_positive_proof(dag, *denominator),
        ExpressionNode::Power { base, .. } => symbolic_positive_proof(dag, *base),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => symbolic_positive_proof(dag, *argument),
        ExpressionNode::Function {
            function: Function::Exp,
            ..
        } => true,
        ExpressionNode::Function {
            function: Function::Log | Function::Ln,
            argument,
        } => symbolic_log_positive_proof(dag, *argument),
        ExpressionNode::LogBase { argument, base } => {
            symbolic_log_base_positive_proof(dag, *argument, *base)
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::BinaryFunction { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        } => false,
    }
}

fn symbolic_nonnegative_proof(dag: &ExactExpressionDag, id: ExprId) -> bool {
    if let Some(value) = symbolic_rational_value(dag, id) {
        return !value.is_negative();
    }
    if symbolic_positive_proof(dag, id) {
        return true;
    }

    match dag.semantic_node(id) {
        ExpressionNode::Exact(value) => matches!(
            symbolic_exact_reduction_sign(dag.exact_value(*value)),
            Some(Ordering::Equal | Ordering::Greater)
        ),
        ExpressionNode::Add(list_id) | ExpressionNode::Multiply(list_id) => dag
            .list(*list_id)
            .iter()
            .all(|child| symbolic_nonnegative_proof(dag, *child)),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            symbolic_nonnegative_proof(dag, *numerator)
                && symbolic_positive_proof(dag, *denominator)
        }
        ExpressionNode::Power { base, .. } => symbolic_positive_proof(dag, *base),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => symbolic_nonnegative_proof(dag, *argument),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Exp
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => false,
    }
}

fn symbolic_log_positive_proof(dag: &ExactExpressionDag, argument: ExprId) -> bool {
    symbolic_rational_value(dag, argument)
        .is_some_and(|value| value.compare(&Rational::one()) == Ordering::Greater)
}

fn symbolic_real_algebraic_sign(value: &RealAlgebraic) -> Option<Ordering> {
    let zero = Rational::zero();
    let interval = value.isolating_interval();
    if interval.lower.compare(&zero) != Ordering::Less {
        Some(Ordering::Greater)
    } else if interval.upper.compare(&zero) != Ordering::Greater {
        Some(Ordering::Less)
    } else {
        None
    }
}

fn symbolic_exact_reduction_sign(value: &ExactReduction) -> Option<Ordering> {
    match value {
        ExactReduction::PiMultiple(value) => Some(rational_ordering(value.coefficient())),
        ExactReduction::Radical(RadicalReduction::Rational(value)) => {
            Some(rational_ordering(value.value()))
        }
        ExactReduction::Radical(RadicalReduction::Radical(value)) => {
            let coefficient = &value.value().coefficient;
            Some(rational_ordering(coefficient))
        }
        ExactReduction::Radical(RadicalReduction::LinearCombination(_)) => None,
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Rational(value)) => {
            Some(rational_ordering(value.value()))
        }
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) => {
            symbolic_real_algebraic_sign(value)
        }
        ExactReduction::Symbolic => None,
    }
}

fn rational_ordering(value: &Rational) -> Ordering {
    if value.is_negative() {
        Ordering::Less
    } else if value.is_zero() {
        Ordering::Equal
    } else {
        Ordering::Greater
    }
}

fn symbolic_log_base_positive_proof(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> bool {
    let Some(argument) = symbolic_rational_value(dag, argument) else {
        return false;
    };
    let Some(base) = symbolic_rational_value(dag, base) else {
        return false;
    };
    if argument.is_negative()
        || argument.is_zero()
        || base.is_negative()
        || base.is_zero()
        || base == Rational::one()
    {
        return false;
    }

    let one = Rational::one();
    (argument.compare(&one) == Ordering::Greater && base.compare(&one) == Ordering::Greater)
        || (argument.compare(&one) == Ordering::Less && base.compare(&one) == Ordering::Less)
}

fn render_symbolic_shifted_trig_function(
    function: Function,
    shifted_argument: SymbolicPiShiftArgument,
) -> Option<SignedRenderedSymbolic> {
    let phase = shifted_argument.phase.modulo_integer(2);
    if phase.is_integer() {
        let mut rendered = render_signed_symbolic_function_from_signed_argument(
            function,
            shifted_argument.remainder,
        );
        if phase.numerator.inner.is_odd() && matches!(function, Function::Sin | Function::Cos) {
            rendered.negative = !rendered.negative;
        }
        return Some(rendered);
    }

    if phase == symbolic_rational(1, 2) {
        return match function {
            Function::Sin => Some(render_signed_symbolic_function_from_signed_argument(
                Function::Cos,
                shifted_argument.remainder,
            )),
            Function::Cos => {
                let mut rendered = render_signed_symbolic_function_from_signed_argument(
                    Function::Sin,
                    shifted_argument.remainder,
                );
                rendered.negative = !rendered.negative;
                Some(rendered)
            }
            Function::Tan => Some(render_symbolic_tangent_half_pi_shift(
                shifted_argument.remainder,
            )),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => None,
        };
    }

    if phase == symbolic_rational(3, 2) {
        return match function {
            Function::Sin => {
                let mut rendered = render_signed_symbolic_function_from_signed_argument(
                    Function::Cos,
                    shifted_argument.remainder,
                );
                rendered.negative = !rendered.negative;
                Some(rendered)
            }
            Function::Cos => Some(render_signed_symbolic_function_from_signed_argument(
                Function::Sin,
                shifted_argument.remainder,
            )),
            Function::Tan => Some(render_symbolic_tangent_half_pi_shift(
                shifted_argument.remainder,
            )),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => None,
        };
    }

    None
}

fn render_symbolic_tangent_half_pi_shift(
    remainder: SignedRenderedSymbolic,
) -> SignedRenderedSymbolic {
    let tangent = render_signed_symbolic_function_from_signed_argument(Function::Tan, remainder);
    SignedRenderedSymbolic {
        negative: !tangent.negative,
        value: RenderedSymbolic {
            text: format!(
                "1/{}",
                parenthesize_symbolic(&tangent.value, SYMBOLIC_PRECEDENCE_MULTIPLY)
            ),
            precedence: SYMBOLIC_PRECEDENCE_MULTIPLY,
        },
    }
}

fn render_symbolic_function_value(
    function: Function,
    argument: &RenderedSymbolic,
) -> RenderedSymbolic {
    RenderedSymbolic {
        text: format!("{}({})", symbolic_function_name(function), argument.text),
        precedence: SYMBOLIC_PRECEDENCE_ATOM,
    }
}

fn render_symbolic_pi_shift_argument(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Option<SymbolicPiShiftArgument> {
    let ExpressionNode::Add(list_id) = dag.semantic_node(argument) else {
        return None;
    };

    let mut phase = Rational::zero();
    let mut contains_pi = false;
    let mut remainder_terms = Vec::new();
    for child in dag.list(*list_id) {
        match symbolic_pi_multiple_coefficient(dag, *child) {
            Some(coefficient) => {
                contains_pi |= coefficient.contains_pi;
                phase = phase.add(&coefficient.coefficient);
            }
            None => remainder_terms.push(render_signed_symbolic_node(dag, *child)),
        }
    }

    if !contains_pi || remainder_terms.is_empty() {
        return None;
    }

    Some(SymbolicPiShiftArgument {
        phase,
        remainder: render_signed_symbolic_sum_terms(&remainder_terms),
    })
}

fn symbolic_rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("symbolic rational helper uses non-zero denominators")
}

fn symbolic_pi_multiple_coefficient(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Option<SymbolicPiCoefficient> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(rational_id) => {
            let rational = dag.rational(*rational_id);
            rational.is_zero().then(|| SymbolicPiCoefficient {
                coefficient: Rational::zero(),
                contains_pi: false,
            })
        }
        ExpressionNode::Constant(Constant::Pi) => Some(SymbolicPiCoefficient {
            coefficient: Rational::one(),
            contains_pi: true,
        }),
        ExpressionNode::Exact(value) => match dag.exact_value(*value) {
            ExactReduction::PiMultiple(value) => Some(SymbolicPiCoefficient {
                coefficient: value.coefficient().clone(),
                contains_pi: true,
            }),
            ExactReduction::Radical(_) | ExactReduction::RealAlgebraic(_) => None,
            ExactReduction::Symbolic => None,
        },
        ExpressionNode::Constant(Constant::Euler) => None,
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            let mut contains_pi = false;
            for child in dag.list(*list_id) {
                let coefficient = symbolic_pi_multiple_coefficient(dag, *child)?;
                total = total.add(&coefficient.coefficient);
                contains_pi |= coefficient.contains_pi;
            }
            Some(SymbolicPiCoefficient {
                coefficient: total,
                contains_pi,
            })
        }
        ExpressionNode::Multiply(list_id) => {
            let mut scalar = Rational::one();
            let mut pi_coefficient = None;
            for child in dag.list(*list_id) {
                if let Some(rational) = symbolic_rational_value(dag, *child) {
                    scalar = scalar.multiply(&rational);
                    continue;
                }

                let coefficient = symbolic_pi_multiple_coefficient(dag, *child)?;
                if pi_coefficient.is_some() {
                    return None;
                }
                pi_coefficient = Some(coefficient);
            }
            pi_coefficient.map(|coefficient: SymbolicPiCoefficient| SymbolicPiCoefficient {
                coefficient: scalar.multiply(&coefficient.coefficient),
                contains_pi: coefficient.contains_pi,
            })
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let numerator = symbolic_pi_multiple_coefficient(dag, *numerator)?;
            let denominator = symbolic_rational_value(dag, *denominator)?;
            numerator
                .coefficient
                .divide(&denominator)
                .ok()
                .map(|coefficient| SymbolicPiCoefficient {
                    coefficient,
                    contains_pi: numerator.contains_pi,
                })
        }
        ExpressionNode::Function { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn symbolic_rational_value(dag: &ExactExpressionDag, id: ExprId) -> Option<Rational> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(rational_id) => Some(dag.rational(*rational_id).clone()),
        ExpressionNode::Add(list_id) => dag
            .list(*list_id)
            .iter()
            .try_fold(Rational::zero(), |total, child| {
                Some(total.add(&symbolic_rational_value(dag, *child)?))
            }),
        ExpressionNode::Multiply(list_id) => dag
            .list(*list_id)
            .iter()
            .try_fold(Rational::one(), |product, child| {
                Some(product.multiply(&symbolic_rational_value(dag, *child)?))
            }),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => symbolic_rational_value(dag, *numerator)?
            .divide(&symbolic_rational_value(dag, *denominator)?)
            .ok(),
        ExpressionNode::Constant(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Function { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn signed_symbolic_to_rendered(value: SignedRenderedSymbolic) -> RenderedSymbolic {
    if value.negative {
        let text = if value.value.precedence == SYMBOLIC_PRECEDENCE_MULTIPLY {
            format!("-{}", value.value.text)
        } else {
            format!(
                "-{}",
                parenthesize_symbolic(&value.value, SYMBOLIC_PRECEDENCE_PREFIX)
            )
        };
        RenderedSymbolic {
            text,
            precedence: SYMBOLIC_PRECEDENCE_PREFIX,
        }
    } else {
        value.value
    }
}

fn parenthesize_symbolic(value: &RenderedSymbolic, parent_precedence: u8) -> String {
    if value.precedence < parent_precedence {
        format!("({})", value.text)
    } else {
        value.text.clone()
    }
}

fn symbolic_function_name(function: Function) -> &'static str {
    match function {
        Function::Sin => "sin",
        Function::Cos => "cos",
        Function::Tan => "tan",
        Function::Asin => "asin",
        Function::Acos => "acos",
        Function::Atan => "atan",
        Function::Sqrt => "sqrt",
        Function::Root => "root",
        Function::Exp => "exp",
        Function::Log | Function::Ln => "ln",
        Function::Abs => "abs",
        Function::Floor => "floor",
        Function::Factorial => "fact",
        Function::Permutation => "perm",
        Function::Combination => "comb",
        Function::Modulo => "mod",
        Function::Gcd => "gcd",
        Function::Lcm => "lcm",
        Function::Sinh => "sinh",
        Function::Cosh => "cosh",
        Function::Tanh => "tanh",
        Function::Asinh => "asinh",
        Function::Acosh => "acosh",
        Function::Atanh => "atanh",
    }
}

fn exact_representation_kind(value: &RecognizedExact) -> ExactRepresentationKind {
    match value {
        RecognizedExact::Rational(rational) => rational_exact_representation_kind(rational),
        RecognizedExact::Radical(_) | RecognizedExact::RadicalLinearCombination(_) => {
            ExactRepresentationKind::Radical
        }
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

fn simplification_status(
    scientific: &ScientificOutput,
    evaluation_reason: Option<&IncompleteReason>,
) -> SimplificationStatus {
    if let Some(reason) = evaluation_reason {
        return SimplificationStatus::PartiallySimplified {
            reason: reason.clone(),
        };
    }
    match scientific {
        ScientificOutput::Unavailable(value) => SimplificationStatus::PartiallySimplified {
            reason: value.reason.clone(),
        },
        ScientificOutput::Omitted | ScientificOutput::Included(_) => {
            SimplificationStatus::FullySimplifiedWithinLimits
        }
    }
}

fn assurance_level(value: &RecognizedExact, enclosure: &CertifiedEnclosureState) -> AssuranceLevel {
    match value {
        RecognizedExact::Rational(_)
        | RecognizedExact::Radical(_)
        | RecognizedExact::RadicalLinearCombination(_) => AssuranceLevel::Exact,
        RecognizedExact::RealAlgebraic(_) | RecognizedExact::RationalPiMultiple(_) => {
            AssuranceLevel::CertifiedEnclosure
        }
        RecognizedExact::GeneralSymbolic => match enclosure {
            CertifiedEnclosureState::Available(_) => AssuranceLevel::CertifiedEnclosure,
            CertifiedEnclosureState::NotRequested | CertifiedEnclosureState::Unavailable(_) => {
                AssuranceLevel::Exact
            }
        },
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
        let exponent_ten = String::from("0");
        let presentation = scientific_notation_presentation(&significand, &exponent_ten);
        return Ok(ScientificPresentation {
            relation: ResultRelation::ApproximatelyEqual,
            significand,
            exponent_ten,
            requested_significant_digits: significant_digits,
            confirmed_significant_digits: digits,
            rounding_mode,
            presentation,
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
    let exponent_ten = exponent_ten.to_string();
    let presentation = scientific_notation_presentation(&significand, &exponent_ten);
    Ok(ScientificPresentation {
        relation: ResultRelation::ApproximatelyEqual,
        significand,
        exponent_ten,
        requested_significant_digits: significant_digits,
        confirmed_significant_digits: digits,
        rounding_mode,
        presentation,
    })
}

fn scientific_presentation_from_certified_interval(
    state: &CertifiedEnclosureState,
    significant_digits: core::num::NonZeroU32,
    rounding_mode: DecimalRoundingMode,
) -> Result<Option<ScientificPresentation>, PresentationError> {
    let CertifiedEnclosureState::Available(interval) = state else {
        return Ok(None);
    };
    let lower = match interval::dyadic_to_rational(&interval.lower) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let upper = match interval::dyadic_to_rational(&interval.upper) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let lower = scientific_presentation(&lower, significant_digits, rounding_mode)?;
    let upper = scientific_presentation(&upper, significant_digits, rounding_mode)?;
    if lower == upper {
        Ok(Some(lower))
    } else {
        Ok(None)
    }
}

fn unavailable_scientific_output(
    enclosure: &CertifiedEnclosureState,
    significant_digits: core::num::NonZeroU32,
    rounding_mode: DecimalRoundingMode,
) -> ScientificOutput {
    let reason = match enclosure {
        CertifiedEnclosureState::Unavailable(reason) => reason.clone(),
        CertifiedEnclosureState::NotRequested | CertifiedEnclosureState::Available(_) => {
            IncompleteReason::PrecisionLimit {
                requested_digits: significant_digits,
                confirmed_digits: 0,
            }
        }
    };
    ScientificOutput::Unavailable(UnavailableScientificOutput {
        requested_significant_digits: significant_digits,
        confirmed_significant_digits: 0,
        rounding_mode,
        reason,
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
    certified_interval_presentation(&interval, format)
}

fn certified_interval_presentation(
    interval: &CertifiedInterval,
    format: EnclosureFormat,
) -> Result<CertifiedIntervalPresentation, PresentationError> {
    match format {
        EnclosureFormat::ExactDyadic => {
            let lower = interval.lower.clone();
            let upper = interval.upper.clone();
            Ok(CertifiedIntervalPresentation {
                relation: ResultRelation::ElementOf,
                presentation: PresentationNode::Row(vec![
                    PresentationNode::Text(String::from("[")),
                    dyadic_text_node(&lower),
                    PresentationNode::Text(String::from(", ")),
                    dyadic_text_node(&upper),
                    PresentationNode::Text(String::from("]")),
                ]),
                bounds: CertifiedIntervalBounds::ExactDyadic { lower, upper },
            })
        }
        EnclosureFormat::DecimalScientific { significant_digits } => {
            let lower = decimal_scientific_bound_from_dyadic(
                &interval.lower,
                significant_digits,
                DecimalRoundingMode::TowardNegativeInfinity,
            )?;
            let upper = decimal_scientific_bound_from_dyadic(
                &interval.upper,
                significant_digits,
                DecimalRoundingMode::TowardPositiveInfinity,
            )?;
            Ok(CertifiedIntervalPresentation {
                relation: ResultRelation::ElementOf,
                presentation: PresentationNode::Row(vec![
                    PresentationNode::Text(String::from("[")),
                    decimal_scientific_bound_presentation(&lower),
                    PresentationNode::Text(String::from(", ")),
                    decimal_scientific_bound_presentation(&upper),
                    PresentationNode::Text(String::from("]")),
                ]),
                bounds: CertifiedIntervalBounds::DecimalScientific {
                    lower,
                    upper,
                    requested_significant_digits: significant_digits,
                },
            })
        }
    }
}

fn decimal_scientific_bound_from_dyadic(
    value: &ExactDyadic,
    significant_digits: core::num::NonZeroU32,
    rounding_mode: DecimalRoundingMode,
) -> Result<DecimalScientificBound, PresentationError> {
    let rational = interval::dyadic_to_rational(value).map_err(|_| precision_limit_error())?;
    let presentation = scientific_presentation(&rational, significant_digits, rounding_mode)?;
    Ok(DecimalScientificBound {
        significand: presentation.significand,
        exponent_ten: presentation.exponent_ten,
    })
}

fn decimal_scientific_bound_presentation(value: &DecimalScientificBound) -> PresentationNode {
    scientific_notation_presentation(&value.significand, &value.exponent_ten)
}

fn scientific_notation_presentation(significand: &str, exponent_ten: &str) -> PresentationNode {
    PresentationNode::Row(vec![
        PresentationNode::Text(String::from(significand)),
        PresentationNode::Text(String::from(" × ")),
        PresentationNode::Superscript {
            base: Box::new(PresentationNode::Text(String::from("10"))),
            exponent: Box::new(PresentationNode::Text(String::from(exponent_ten))),
        },
    ])
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

    fn exact_format_request(format: ExactFormatPreference) -> CalculationRequest {
        CalculationRequest {
            exact_output: ExactOutputRequest::Include { format },
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Omit,
            ..CalculationRequest::default()
        }
    }

    fn scientific_plain_text_with_request(source: &str, request: &CalculationRequest) -> String {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, request, &mut context).expect(source);
        let calculation = match outcome {
            CalculationOutcome::Complete(calculation) => calculation,
            CalculationOutcome::Partial { calculation, .. } => calculation,
        };
        let ScientificOutput::Included(scientific) = calculation.scientific else {
            panic!("{source}: expected scientific output");
        };
        render_plain_text_for_test(&scientific.presentation)
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

    fn high_precision_enclosure_request() -> CalculationRequest {
        CalculationRequest {
            scientific_output: ScientificOutputRequest::Include {
                significant_digits: core::num::NonZeroU32::new(50).unwrap(),
                rounding_mode: DecimalRoundingMode::NearestTiesToEven,
            },
            enclosure_output: EnclosureOutputRequest::Include {
                format: EnclosureFormat::ExactDyadic,
            },
            ..CalculationRequest::default()
        }
    }

    fn exact_dyadic_interval(enclosure: &CertifiedIntervalPresentation) -> CertifiedInterval {
        let CertifiedIntervalBounds::ExactDyadic { lower, upper } = &enclosure.bounds else {
            panic!("expected exact dyadic certified interval bounds");
        };
        CertifiedInterval {
            lower: lower.clone(),
            upper: upper.clone(),
        }
    }

    fn decimal_scientific_plain_text(source: &str) -> String {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
        let calculation = match outcome {
            CalculationOutcome::Complete(calculation) => calculation,
            CalculationOutcome::Partial { calculation, .. } => calculation,
        };
        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("expected enclosure output");
        };
        let CertifiedIntervalBounds::DecimalScientific {
            requested_significant_digits,
            ..
        } = enclosure.bounds
        else {
            panic!("expected decimal scientific certified interval bounds");
        };
        assert_eq!(requested_significant_digits.get(), 5);
        render_plain_text_for_test(&enclosure.presentation)
    }

    fn exact_output_with_format(source: &str, format: ExactFormatPreference) -> ExactPresentation {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &exact_format_request(format), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("{source}: expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("{source}: expected exact output");
        };
        assert_eq!(
            calculation.metadata.exact_representation,
            exact.representation
        );
        exact
    }

    fn render_plain_text_for_test(node: &PresentationNode) -> String {
        match node {
            PresentationNode::Text(text) => text.clone(),
            PresentationNode::Row(children) => children
                .iter()
                .map(render_plain_text_for_test)
                .collect::<String>(),
            PresentationNode::Fraction {
                numerator,
                denominator,
            } => {
                format!(
                    "{}/{}",
                    render_plain_text_for_test(numerator),
                    render_plain_text_for_test(denominator)
                )
            }
            PresentationNode::Superscript { base, exponent } => {
                format!(
                    "{}^{}",
                    render_plain_text_for_test(base),
                    render_plain_text_for_test(exponent)
                )
            }
            PresentationNode::Subscript { base, subscript } => {
                format!(
                    "{}_{}",
                    render_plain_text_for_test(base),
                    render_plain_text_for_test(subscript)
                )
            }
            PresentationNode::Radical { index, radicand } => match index {
                RadicalIndex::Square => format!("sqrt({})", render_plain_text_for_test(radicand)),
                RadicalIndex::Nth(value) => {
                    format!(
                        "root({}, {})",
                        value.inner.inner,
                        render_plain_text_for_test(radicand)
                    )
                }
            },
            PresentationNode::Function { name, argument } => {
                format!("{name:?}({})", render_plain_text_for_test(argument))
            }
            PresentationNode::Parenthesized(value) => {
                format!("({})", render_plain_text_for_test(value))
            }
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

    fn exact_plain_text_from_partial_outcome(outcome: CalculationOutcome) -> String {
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        exact.plain_text
    }

    fn exact_presentation_for(source: &str) -> ExactPresentation {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &exact_only_request(), &mut context).expect(source);
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        exact
    }

    fn symbolic_plain_text_from_source(source: &str) -> String {
        let parsed = parse(source, &ParseSettings::default()).unwrap();
        let dag = lower_source_expression(
            &parsed.root,
            SemanticSettings::default(),
            &ResourceLimits::default(),
        )
        .unwrap();
        symbolic_presentation_from_dag(&dag).plain_text
    }

    #[test]
    fn arbitrary_base_logarithm_and_exponential_have_exact_cases() {
        assert_eq!(exact_presentation_for("log(8,2)").plain_text, "3");
        assert_eq!(exact_presentation_for("log(1/8,2)").plain_text, "-3");
        assert_eq!(exact_presentation_for("log(2^(1/3),2)").plain_text, "1/3");
        assert_eq!(exact_presentation_for("log(2^(-2/3),2)").plain_text, "-2/3");
        assert_eq!(exact_presentation_for("log(sqrt(2),2)").plain_text, "1/2");
        assert_eq!(exact_presentation_for("log(2,sqrt(2))").plain_text, "2");
        assert_eq!(exact_presentation_for("log(8,sqrt(2))").plain_text, "6");
        assert_eq!(exact_presentation_for("log(sqrt(2),8)").plain_text, "1/6");
        assert_eq!(
            exact_presentation_for("log(2^(1/3),sqrt(2))").plain_text,
            "2/3"
        );
        assert_eq!(exact_presentation_for("log(1,sqrt(2))").plain_text, "0");
        assert_eq!(
            exact_presentation_for("log((sin(pi/6)*4)^(3/2),2)").plain_text,
            "3/2"
        );
        assert_eq!(exact_presentation_for("exp(3,2)").plain_text, "8");
        assert_eq!(exact_presentation_for("ln(exp(2))").plain_text, "2");
        assert_eq!(exact_presentation_for("log(exp(2), e)").plain_text, "2");
    }

    #[test]
    fn logarithm_change_of_base_and_chain_identities_are_exact() {
        for (source, expected) in [
            ("ln(8)/ln(2)", "3"),
            ("ln(sqrt(2))/ln(8)", "1/6"),
            ("log(8,3)/log(2,3)", "3"),
            ("log(3,2)/log(3,8)", "3"),
            ("log(8,3)*log(3,2)", "3"),
            ("log(3,2)*log(8,3)", "3"),
            ("2*log(8,3)*log(3,2)", "6"),
            ("log(2,3)*log(3,2)", "1"),
            ("log(8,7)*log(7,3)*log(3,2)", "3"),
            ("log(3,2)*log(8,7)*log(7,3)", "3"),
            ("log(2,10)+log(5,10)", "1"),
            ("log(6,2)-log(3,2)", "1"),
            ("2*log(2,10)+2*log(5,10)", "2"),
            ("log(2,10)+log(5,10)+log(2,10)+log(5,10)", "2"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn logarithm_change_of_base_normalizes_symbolically_when_not_numeric() {
        for (source, expected) in [
            ("ln(3)/ln(2)", "log(3,2)"),
            ("log(5,3)/log(2,3)", "log(5,2)"),
            ("log(3,2)/log(3,5)", "log(5,2)"),
            ("log(5,3)*log(3,2)", "log(5,2)"),
            ("2*ln(3)/(3*ln(2))", "2/3*log(3,2)"),
            ("(2*ln(3)/3)/ln(2)", "2/3*log(3,2)"),
            ("log(5,7)*log(7,3)*log(3,2)", "log(5,2)"),
            ("log(2,10)+log(3,10)", "log(2*3,10)"),
            ("log(6,10)-log(3,10)", "log(6/3,10)"),
            ("2*log(2,10)+2*log(3,10)", "2*log(2*3,10)"),
            ("-log(5,3)/log(2,3)", "-log(5,2)"),
            ("log(5,3)/(-log(2,3))", "-log(5,2)"),
            ("-log(2,10)-log(3,10)", "-log(2*3,10)"),
            ("-log(5,3)*log(3,2)", "-log(5,2)"),
            ("log(2,10)+log(3,10)+log(5,10)", "log(2*3*5,10)"),
            ("log(2,10)+(log(3,10)+log(5,10))", "log(2*3*5,10)"),
            ("log(2,10)-log(3,10)-log(5,10)", "log(2/(3*5),10)"),
            ("log(2,10)-(log(3,10)+log(5,10))", "log(2/(3*5),10)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        for source in ["ln(sin(1))/ln(2)", "log(sin(1),3)/log(2,3)"] {
            assert_eq!(exact_plain_text(source), source, "{source}");
        }
    }

    #[test]
    fn logarithm_identity_reduction_preserves_domain_errors() {
        let mut context = EvaluationContext::default();
        let error = calculate("ln(2)/ln(1)", &exact_only_request(), &mut context)
            .expect_err("the denominator logarithm is zero");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::DivisionByZero,
                span: None,
            })
        );

        let error = calculate("log(2,1)*log(1,3)", &exact_only_request(), &mut context)
            .expect_err("base one must be rejected before logarithm cancellation");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::LogarithmBaseOne,
                span: None,
            })
        );
    }

    #[test]
    fn logarithm_renderer_does_not_bypass_reduction_budget() {
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequest::Custom(ResourceLimits {
            max_rewrite_steps: 0,
            max_logical_work_units: 0,
            ..ResourceLimits::default()
        });
        for (source, expected) in [
            ("log(2,10)+log(3,10)", "log(2,10)+log(3,10)"),
            ("log(5,7)*log(7,3)*log(3,2)", "log(3,2)*log(5,7)*log(7,3)"),
        ] {
            let mut context = EvaluationContext::default();
            let outcome = calculate(source, &request, &mut context).unwrap();
            assert_eq!(exact_plain_text_from_partial_outcome(outcome), expected);
        }
    }

    #[test]
    fn logarithm_base_one_is_a_domain_error() {
        let mut context = EvaluationContext::default();
        for source in ["log(2,1)", "log(2,2^0)"] {
            let error = calculate(source, &exact_only_request(), &mut context)
                .expect_err("log base one should be rejected");
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::LogarithmBaseOne,
                    span: None,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn input_presentation_renders_log_bases_and_natural_log_alias() {
        let log = present_input("log(8,2)", &CalculationRequest::default()).unwrap();
        let PresentationNode::Row(children) = log else {
            panic!("expected log presentation to be a row");
        };
        assert!(matches!(
            children.first(),
            Some(PresentationNode::Subscript { .. })
        ));

        let natural = present_input("ln(e)", &CalculationRequest::default()).unwrap();
        assert!(matches!(
            natural,
            PresentationNode::Function {
                name: FunctionName::Ln,
                ..
            }
        ));
    }

    #[test]
    fn integer_and_hyperbolic_functions_reduce_exact_cases() {
        for (source, expected) in [
            ("abs(-3/2)", "3/2"),
            ("floor(7/3)", "2"),
            ("floor(-7/3)", "-3"),
            ("5!", "120"),
            ("fact(5)", "120"),
            ("root(27,3)", "3"),
            ("gcd(84,30)", "6"),
            ("lcm(12,18)", "36"),
            ("lcd(12,18)", "36"),
            ("mod(17,5)", "2"),
            ("perm(5,2)", "20"),
            ("comb(5,2)", "10"),
            ("sinh(0)", "0"),
            ("cosh(0)", "1"),
            ("tanh(0)", "0"),
            ("asinh(0)", "0"),
            ("acosh(1)", "0"),
            ("atanh(0)", "0"),
            ("exp(sinh(0))", "1"),
            ("ln(cosh(0))", "0"),
            ("sqrt(abs(-4))", "2"),
            ("(sin(pi/2)+4)!", "120"),
        ] {
            assert_eq!(
                exact_presentation_for(source).plain_text,
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn input_presentation_renders_extended_function_notation() {
        let factorial = present_input("5!", &CalculationRequest::default()).unwrap();
        assert!(matches!(factorial, PresentationNode::Row(_)));

        let absolute = present_input("abs(-2)", &CalculationRequest::default()).unwrap();
        let PresentationNode::Row(children) = absolute else {
            panic!("expected absolute value presentation to be a row");
        };
        assert!(matches!(
            children.first(),
            Some(PresentationNode::Text(text)) if text == "|"
        ));

        let root = present_input("root(27,3)", &CalculationRequest::default()).unwrap();
        assert!(matches!(
            root,
            PresentationNode::Radical {
                index: RadicalIndex::Nth(_),
                ..
            }
        ));
    }

    #[test]
    fn present_input_enforces_presentation_resource_limits() {
        let output_limited_request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_output_bytes: 1,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let error = present_input("12345", &output_limited_request)
            .expect_err("input presentation text should be checked against max_output_bytes");
        assert_eq!(
            error,
            CalculatorError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::OutputTooLarge,
            })
        );

        let node_limited_request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_presentation_nodes: 1,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let error = present_input("1+2", &node_limited_request)
            .expect_err("input presentation tree should be checked against max_presentation_nodes");
        assert_eq!(
            error,
            CalculatorError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::PresentationNodes,
            })
        );
    }

    #[test]
    fn present_enforces_presentation_resource_limits() {
        let parsed = parse("12345", &ParseSettings::default()).unwrap();
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
        let error = present(
            &evaluation,
            &PresentationRequest {
                exact_output: ExactOutputRequest::Include {
                    format: ExactFormatPreference::Auto,
                },
                scientific_output: ScientificOutputRequest::Omit,
                enclosure_output: EnclosureOutputRequest::Omit,
                limits: ResourceLimitRequest::Custom(ResourceLimits {
                    max_output_bytes: 1,
                    ..ResourceLimits::default()
                }),
            },
        )
        .expect_err("present should check exact output text against max_output_bytes");
        assert_eq!(
            error,
            PresentationError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::OutputTooLarge,
            })
        );
    }

    #[test]
    fn calculate_enforces_output_presentation_resource_limits() {
        let output_limited_request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_output_bytes: 1,
                ..ResourceLimits::default()
            }),
            ..exact_only_request()
        };
        let mut context = EvaluationContext::default();
        let error = calculate("12345", &output_limited_request, &mut context)
            .expect_err("calculation output should be checked against max_output_bytes");
        assert_eq!(
            error,
            CalculatorError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::OutputTooLarge,
            })
        );

        let node_limited_request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_presentation_nodes: 1,
                ..ResourceLimits::default()
            }),
            ..exact_only_request()
        };
        let error = calculate("1/2", &node_limited_request, &mut context)
            .expect_err("calculation output should be checked against max_presentation_nodes");
        assert_eq!(
            error,
            CalculatorError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::PresentationNodes,
            })
        );
    }

    #[test]
    fn calculate_checks_final_symbolic_presentation_not_intermediate_source_text() {
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_output_bytes: 20,
                ..ResourceLimits::default()
            }),
            ..exact_only_request()
        };
        let mut context = EvaluationContext::default();
        let outcome = calculate(
            "sin(                    pi/2+1/10                    )",
            &request,
            &mut context,
        )
        .expect("normalized symbolic output should fit the output byte limit");
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete normalized symbolic calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.plain_text, "cos(1/10)");
    }

    #[test]
    fn partial_certified_enclosure_enforces_output_byte_limit() {
        let request = CalculationRequest {
            exact_output: ExactOutputRequest::Omit,
            scientific_output: ScientificOutputRequest::Include {
                significant_digits: core::num::NonZeroU32::new(50).unwrap(),
                rounding_mode: DecimalRoundingMode::NearestTiesToEven,
            },
            enclosure_output: EnclosureOutputRequest::Omit,
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_output_bytes: 1,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let error = calculate("sqrt(2)", &request, &mut context)
            .expect_err("partial certified enclosure should be checked against max_output_bytes");
        assert_eq!(
            error,
            CalculatorError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::OutputTooLarge,
            })
        );
    }

    fn assert_source_symbolic_fallback_with_limits(source: &str, limits: ResourceLimits) {
        assert_symbolic_fallback_exact_with_limits(source, source, limits, false);
    }

    fn assert_source_symbolic_fallback_with_limits_and_nested_algebraic(
        source: &str,
        limits: ResourceLimits,
        expects_nested_algebraic: bool,
    ) {
        assert_symbolic_fallback_exact_with_limits(
            source,
            source,
            limits,
            expects_nested_algebraic,
        );
    }

    fn assert_symbolic_fallback_exact_with_limits(
        source: &str,
        expected_exact: &str,
        limits: ResourceLimits,
        expects_nested_algebraic: bool,
    ) {
        let mut request = high_precision_enclosure_request();
        request.limits = ResourceLimitRequest::Custom(limits);
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &request, &mut context)
            .expect("bounded algebraic fallback should calculate");
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial symbolic fallback");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected retained exact output");
        };
        assert_eq!(
            exact.representation,
            ExactRepresentationKind::GeneralSymbolic
        );
        assert_eq!(exact.plain_text, expected_exact);
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::GeneralSymbolic
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SymbolicRetention));
        assert_eq!(
            calculation
                .metadata
                .methods
                .contains(&MethodTag::AlgebraicMinimalPolynomial),
            expects_nested_algebraic
        );
    }

    fn assert_symbolic_fallback_with_limits(limits: ResourceLimits) {
        assert_source_symbolic_fallback_with_limits("2^(1/3)", limits);
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
    fn large_negative_exponential_keeps_scientific_digits_and_symbolic_aliases() {
        assert_eq!(exact_plain_text("exp(-10000)"), "exp(-10000)");
        assert_eq!(exact_plain_text("e^(-10000)"), "exp(-10000)");
        assert_eq!(
            scientific_parts("exp(-10000)", 5, DecimalRoundingMode::NearestTiesToEven),
            (String::from("1.1355"), String::from("-4343"))
        );
        assert_eq!(
            scientific_parts("e^(-10000)", 5, DecimalRoundingMode::NearestTiesToEven),
            (String::from("1.1355"), String::from("-4343"))
        );
        let (significand, exponent) =
            scientific_parts("exp(-10000)", 20, DecimalRoundingMode::NearestTiesToEven);
        assert_eq!(significand, "1.1354838653147360985");
        assert_eq!(exponent, "-4343");
    }

    #[test]
    fn large_exponential_work_is_charged_before_interval_evaluation() {
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_logical_work_units: 100,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let CalculationOutcome::Partial {
            reason,
            certified_enclosure,
            ..
        } = calculate("exp(-10000)", &request, &mut EvaluationContext::default())
            .expect("insufficient large-exp work must return a typed partial")
        else {
            panic!("large exp must not evaluate outside its logical-work budget");
        };
        assert_eq!(
            reason,
            IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits,
            }
        );
        assert!(certified_enclosure.is_none());

        for source in ["e^(-10000)", "exp(100*pi)"] {
            let outcome = calculate(source, &request, &mut EvaluationContext::default())
                .expect("large exponential work must produce a typed outcome");
            assert!(
                matches!(
                    outcome,
                    CalculationOutcome::Partial {
                        reason: IncompleteReason::ComputationLimit {
                            kind: ComputationLimitKind::LogicalWorkUnits,
                        },
                        certified_enclosure: None,
                        ..
                    }
                ),
                "{source} bypassed the large exponential work reservation"
            );
        }
    }

    #[test]
    fn ordinary_exponentials_do_not_reserve_binary_scaling_work() {
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_logical_work_units: 100,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        for source in ["exp(-2)", "exp(2)"] {
            assert!(
                matches!(
                    calculate(source, &request, &mut EvaluationContext::default())
                        .expect("ordinary exponential must calculate"),
                    CalculationOutcome::Complete { .. }
                ),
                "{source} was charged for the binary scaling path"
            );
        }
    }

    #[test]
    fn exact_format_preference_controls_rational_presentation() {
        let finite = exact_output_with_format("0.1 + 0.2", ExactFormatPreference::FiniteDecimal);
        assert_eq!(
            finite.representation,
            ExactRepresentationKind::FiniteDecimal
        );
        assert_eq!(finite.plain_text, "0.3");
        assert_eq!(render_plain_text_for_test(&finite.presentation), "0.3");

        let non_finite = exact_output_with_format("1/3", ExactFormatPreference::FiniteDecimal);
        assert_eq!(non_finite.representation, ExactRepresentationKind::Rational);
        assert_eq!(non_finite.plain_text, "1/3");

        let proper = exact_output_with_format("1/2", ExactFormatPreference::MixedFraction);
        assert_eq!(proper.representation, ExactRepresentationKind::Rational);
        assert_eq!(proper.plain_text, "1/2");

        let mixed = exact_output_with_format("7/3", ExactFormatPreference::MixedFraction);
        assert_eq!(mixed.representation, ExactRepresentationKind::Rational);
        assert_eq!(mixed.plain_text, "2 1/3");
        assert_eq!(render_plain_text_for_test(&mixed.presentation), "2 1/3");

        let negative = exact_output_with_format("-7/3", ExactFormatPreference::MixedFraction);
        assert_eq!(negative.representation, ExactRepresentationKind::Rational);
        assert_eq!(negative.plain_text, "-2 1/3");
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
            ("cos(ln(1))", "1"),
            ("cos(pi/3)", "1/2"),
            ("cos(2*pi/3)", "-1/2"),
            ("cos(-pi)", "-1"),
            ("cos(3*pi/2)", "0"),
            ("tan(0)", "0"),
            ("tan(exp(ln(2)) - 2)", "0"),
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
    fn cyclotomic_trigonometric_values_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        for (source, coefficients) in [
            (
                "sin(pi/5)",
                vec![
                    Integer::from(5),
                    Integer::zero(),
                    Integer::from(-20),
                    Integer::zero(),
                    Integer::from(16),
                ],
            ),
            (
                "cos(pi/5)",
                vec![Integer::from(-1), Integer::from(-2), Integer::from(4)],
            ),
            (
                "tan(pi/5)",
                vec![
                    Integer::from(5),
                    Integer::zero(),
                    Integer::from(-10),
                    Integer::zero(),
                    Integer::one(),
                ],
            ),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected cyclotomic real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap()
            );
            assert_eq!(
                algebraic
                    .minimal_polynomial
                    .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                    .unwrap(),
                1
            );
            assert!(evaluation
                .metadata
                .methods
                .contains(&MethodTag::CyclotomicReduction));

            let outcome = calculate(source, &exact_only_request(), &mut context).unwrap();
            let CalculationOutcome::Complete(calculation) = outcome else {
                panic!("{source}: expected complete exact-only calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, source);
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::CyclotomicReduction));
            assert_eq!(
                calculation.metadata.exact_representation,
                ExactRepresentationKind::RealAlgebraic
            );
        }
    }

    #[test]
    fn cyclotomic_trig_limits_fall_back_to_symbolic_without_error() {
        assert_source_symbolic_fallback_with_limits(
            "sin(pi/5)",
            ResourceLimits {
                max_cyclotomic_order: 4,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            "sin(pi/5)",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            "tan(pi/5)",
            ResourceLimits {
                max_cyclotomic_order: 4,
                ..ResourceLimits::default()
            },
        );
    }

    #[test]
    fn inverse_trigonometric_known_values_are_exact() {
        for (source, expected) in [
            ("asin(-1)", "-pi/2"),
            ("asin(-1/2)", "-pi/6"),
            ("asin(0)", "0"),
            ("asin(1/2)", "pi/6"),
            ("asin(sqrt(2)/2)", "pi/4"),
            ("asin(sqrt(3)/2)", "pi/3"),
            ("asin(1)", "pi/2"),
            ("acos(-1)", "pi"),
            ("acos(-1/2)", "2pi/3"),
            ("acos(-sqrt(2)/2)", "3pi/4"),
            ("acos(0)", "pi/2"),
            ("acos(sqrt(3)/2)", "pi/6"),
            ("acos(1/2)", "pi/3"),
            ("acos(1)", "0"),
            ("atan(-1)", "-pi/4"),
            ("atan(-sqrt(3))", "-pi/3"),
            ("atan(-sqrt(3)/3)", "-pi/6"),
            ("atan(0)", "0"),
            ("atan(sqrt(3)/3)", "pi/6"),
            ("atan(1)", "pi/4"),
            ("atan(sqrt(3))", "pi/3"),
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
            ("asin(sqrt(2)/2)", "45"),
            ("asin(-1/2)", "-30"),
            ("acos(-1)", "180"),
            ("acos(-sqrt(2)/2)", "135"),
            ("acos(1/2)", "60"),
            ("atan(1)", "45"),
            ("atan(sqrt(3))", "60"),
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
            ("asin(sqrt(2)/2)", "50"),
            ("acos(-1)", "200"),
            ("acos(sqrt(3)/2)", "100/3"),
            ("atan(1)", "50"),
            ("atan(sqrt(3)/3)", "100/3"),
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
        let outcome = calculate(
            "asin(1/2)",
            &high_precision_enclosure_request(),
            &mut context,
        )
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
        assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
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
            ("sqrt(72)", "6*sqrt(2)"),
            ("sqrt(6962)", "59*sqrt(2)"),
            ("sqrt(1/2)", "sqrt(2)/2"),
            ("sqrt(1/6962)", "sqrt(2)/118"),
            ("2^(1/2)", "sqrt(2)"),
            ("3*sqrt(8)", "6*sqrt(2)"),
            ("sqrt(2)/2", "sqrt(2)/2"),
            ("sin(pi/4)", "sqrt(2)/2"),
            ("cos(pi/6)", "sqrt(3)/2"),
            ("tan(pi/3)", "sqrt(3)"),
            ("tan(pi/6)", "sqrt(3)/3"),
            ("sin(pi/12)", "sqrt(6)/4 - sqrt(2)/4"),
            ("cos(pi/12)", "sqrt(2)/4 + sqrt(6)/4"),
            ("sin(5*pi/12)", "sqrt(2)/4 + sqrt(6)/4"),
            ("cos(5*pi/12)", "sqrt(6)/4 - sqrt(2)/4"),
            ("sin(-pi/12)", "sqrt(2)/4 - sqrt(6)/4"),
            ("cos(7*pi/12)", "sqrt(2)/4 - sqrt(6)/4"),
            ("sin(17*pi/12)", "-sqrt(2)/4 - sqrt(6)/4"),
            ("cos(11*pi/12)", "-sqrt(2)/4 - sqrt(6)/4"),
            ("tan(pi/12)", "2 - sqrt(3)"),
            ("tan(5*pi/12)", "2 + sqrt(3)"),
            ("tan(7*pi/12)", "-2 - sqrt(3)"),
            ("tan(11*pi/12)", "-2 + sqrt(3)"),
            ("sin(pi/4) + cos(pi/4)", "sqrt(2)"),
            ("sin(pi/6) + sqrt(2)", "1/2 + sqrt(2)"),
            ("sqrt(3) + sqrt(2)", "sqrt(2) + sqrt(3)"),
            ("2 * (sin(pi/6) + sqrt(2))", "1 + 2*sqrt(2)"),
            ("(sin(pi/6) + sqrt(2)) / 2", "1/4 + sqrt(2)/2"),
            ("-(sin(pi/6) + sqrt(2))", "-1/2 - sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn simple_radical_algebra_reduces_products_quotients_and_like_terms() {
        for (source, expected) in [
            ("sqrt(2) * sqrt(2)", "2"),
            ("sqrt(2) * sqrt(3)", "sqrt(6)"),
            ("sqrt(2) * sqrt(8)", "4"),
            ("sqrt(8) / sqrt(2)", "2"),
            ("sqrt(2) / sqrt(8)", "1/2"),
            ("1 / sqrt(2)", "sqrt(2)/2"),
            ("(sqrt(2))^2", "2"),
            ("(sqrt(2))^3", "2*sqrt(2)"),
            ("(sqrt(2))^-1", "sqrt(2)/2"),
            ("(1 + sqrt(2))^2", "3 + 2*sqrt(2)"),
            ("(sqrt(2) + 1) * (sqrt(2) - 1)", "1"),
            ("sqrt(2)/sqrt(2) + 1", "2"),
            ("(sqrt(2)/sqrt(2)) * 2", "2"),
            ("sqrt(8) + sqrt(2)", "3*sqrt(2)"),
            ("sqrt(8) - 2 * sqrt(2)", "0"),
            ("sin(pi/4) * cos(pi/4)", "1/2"),
            ("tan(pi/3) / sqrt(3)", "1"),
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
    fn radical_algebra_rational_results_report_radical_extraction() {
        let mut context = EvaluationContext::default();
        let outcome =
            calculate("sin(pi/4) * cos(pi/4)", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Rational);
        assert_eq!(exact.plain_text, "1/2");
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::Rational
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
    fn radical_linear_combinations_report_radical_representation() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("tan(pi/12)", &exact_only_request(), &mut context).unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::Radical);
        assert_eq!(exact.plain_text, "2 - sqrt(3)");
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
        let outcome =
            calculate("sqrt(2)", &high_precision_enclosure_request(), &mut context).unwrap();
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
        assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
        let interval = exact_dyadic_interval(
            certified_enclosure
                .as_ref()
                .expect("partial result has enclosure"),
        );
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
    fn certified_enclosures_confirm_scientific_output_when_digits_are_resolved() {
        let request = scientific_request(5, DecimalRoundingMode::NearestTiesToEven);
        for (source, expected) in [
            ("0", "0.0000 × 10^0"),
            ("sqrt(2)", "1.4142 × 10^0"),
            ("pi", "3.1416 × 10^0"),
            ("pi/6", "5.2360 × 10^-1"),
            ("exp(1)", "2.7183 × 10^0"),
            ("sin(1)", "8.4147 × 10^-1"),
        ] {
            assert_eq!(
                scientific_plain_text_with_request(source, &request),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn default_request_uses_five_digit_decimal_scientific_enclosure() {
        assert_eq!(
            decimal_scientific_plain_text("sqrt(2)"),
            "[1.4142 × 10^0, 1.4143 × 10^0]"
        );
    }

    #[test]
    fn irrational_rational_power_returns_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome =
            calculate("2^(1/2)", &high_precision_enclosure_request(), &mut context).unwrap();
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
        assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
        let interval = exact_dyadic_interval(
            certified_enclosure
                .as_ref()
                .expect("partial result has enclosure"),
        );
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
    fn positive_base_general_powers_return_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        for source in ["2^sqrt(2)", "sqrt(2)^sqrt(2)"] {
            let outcome = calculate(source, &high_precision_enclosure_request(), &mut context)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let CalculationOutcome::Partial {
                calculation,
                reason,
                certified_enclosure,
            } = outcome
            else {
                panic!("{source}: expected partial symbolic calculation");
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
            assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
            assert_eq!(
                calculation.metadata.assurance,
                AssuranceLevel::CertifiedEnclosure
            );
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::SymbolicRetention));
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::CertifiedIntervalEvaluation));
        }
    }

    #[test]
    fn prime_root_rational_power_is_real_algebraic() {
        let parsed = parse("2^(1/3)", &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-2),
                Integer::zero(),
                Integer::zero(),
                Integer::one(),
            ])
            .unwrap()
        );
        assert_eq!(algebraic.real_root_index, 0);
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );
        assert!(evaluation
            .metadata
            .methods
            .contains(&MethodTag::AlgebraicMinimalPolynomial));
        assert!(evaluation
            .metadata
            .methods
            .contains(&MethodTag::AlgebraicRootIsolation));

        let outcome =
            calculate("2^(1/3)", &high_precision_enclosure_request(), &mut context).unwrap();
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = outcome
        else {
            panic!("expected partial calculation for non-rational algebraic number");
        };
        assert_eq!(
            reason,
            IncompleteReason::PrecisionLimit {
                requested_digits: core::num::NonZeroU32::new(50).unwrap(),
                confirmed_digits: 0,
            }
        );
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, "2^(1/3)");
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::RealAlgebraic
        );
        assert_eq!(
            calculation.metadata.assurance,
            AssuranceLevel::CertifiedEnclosure
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::AlgebraicMinimalPolynomial));
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::AlgebraicRootIsolation));
        let interval = exact_dyadic_interval(
            certified_enclosure
                .as_ref()
                .expect("partial result has enclosure"),
        );
        let cubed = interval::pow_i64(&interval, 3, 128).unwrap();
        assert!(
            interval::contains_rational(&cubed, &Rational::from_integer(Integer::from(2)),)
                .unwrap()
        );
    }

    #[test]
    fn negative_prime_root_rational_power_is_real_algebraic() {
        let parsed = parse("(-2)^(1/3)", &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(2),
                Integer::zero(),
                Integer::zero(),
                Integer::one(),
            ])
            .unwrap()
        );
        assert_eq!(algebraic.real_root_index, 0);
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );
        let CertifiedEnclosureState::Available(enclosure) = &evaluation.value.certified_enclosure
        else {
            panic!("real algebraic recognition should include a certified enclosure");
        };
        let cubed = interval::pow_i64(enclosure, 3, 128).unwrap();
        assert!(
            interval::contains_rational(&cubed, &Rational::from_integer(Integer::from(-2)),)
                .unwrap()
        );
    }

    #[test]
    fn translated_prime_root_sum_is_real_algebraic() {
        let source = "2^(1/3)+1";
        let parsed = parse(source, &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected translated real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-3),
                Integer::from(3),
                Integer::from(-3),
                Integer::one(),
            ])
            .unwrap()
        );
        assert_eq!(algebraic.real_root_index, 0);
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );

        let outcome = calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
        let CalculationOutcome::Partial {
            calculation,
            certified_enclosure,
            ..
        } = outcome
        else {
            panic!("expected partial calculation for translated algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact translated algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, source);
        let interval = exact_dyadic_interval(
            certified_enclosure
                .as_ref()
                .expect("partial result has enclosure"),
        );
        let shifted = interval::add(
            &interval,
            &interval::from_rational(&Rational::from_integer(Integer::from(-1)), 128),
        )
        .unwrap();
        let cubed = interval::pow_i64(&shifted, 3, 128).unwrap();
        assert!(
            interval::contains_rational(&cubed, &Rational::from_integer(Integer::from(2)),)
                .unwrap()
        );
    }

    #[test]
    fn affine_prime_root_expressions_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        let cases = [
            (
                "1-2^(1/3)",
                "1-2^(1/3)",
                vec![
                    Integer::one(),
                    Integer::from(3),
                    Integer::from(-3),
                    Integer::one(),
                ],
            ),
            (
                "2*2^(1/3)+1",
                "2*2^(1/3)+1",
                vec![
                    Integer::from(-17),
                    Integer::from(3),
                    Integer::from(-3),
                    Integer::one(),
                ],
            ),
            (
                "2^(1/3)/2+1",
                "1/2*2^(1/3)+1",
                vec![
                    Integer::from(-5),
                    Integer::from(12),
                    Integer::from(-12),
                    Integer::from(4),
                ],
            ),
            (
                "1/2^(1/3)+1",
                "1/2^(1/3)+1",
                vec![
                    Integer::from(-3),
                    Integer::from(6),
                    Integer::from(-6),
                    Integer::from(2),
                ],
            ),
        ];

        for (source, expected_exact, coefficients) in cases {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected affine real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap()
            );
            assert_eq!(
                algebraic
                    .minimal_polynomial
                    .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                    .unwrap(),
                1
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, expected_exact);
        }
    }

    #[test]
    fn repeated_prime_root_sum_is_real_algebraic() {
        let source = "2^(1/3)+2^(1/3)";
        let parsed = parse(source, &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected root-sum real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-16),
                Integer::zero(),
                Integer::zero(),
                Integer::one(),
            ])
            .unwrap()
        );
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );

        let outcome = calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation for root-sum algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, "2*2^(1/3)");
    }

    #[test]
    fn repeated_prime_root_product_is_real_algebraic() {
        let source = "2^(1/3)*2^(1/3)";
        let parsed = parse(source, &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected product real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-4),
                Integer::zero(),
                Integer::zero(),
                Integer::one(),
            ])
            .unwrap()
        );
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );

        let outcome = calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation for product algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, "(2^(1/3))^2");
    }

    #[test]
    fn algebraic_integer_powers_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        for (source, coefficients, cube) in [
            (
                "(2^(1/3))^2",
                vec![
                    Integer::from(-4),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                Rational::from_integer(Integer::from(4)),
            ),
            (
                "(2^(1/3))^-1",
                vec![
                    Integer::from(-1),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::from(2),
                ],
                Rational::new(Integer::one(), Integer::from(2)).unwrap(),
            ),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected integer power real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap()
            );
            assert_eq!(
                algebraic
                    .minimal_polynomial
                    .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                    .unwrap(),
                1
            );
            let CertifiedEnclosureState::Available(enclosure) =
                &evaluation.value.certified_enclosure
            else {
                panic!("{source}: real algebraic recognition should include an enclosure");
            };
            let cubed = interval::pow_i64(enclosure, 3, 128).unwrap();
            assert!(
                interval::contains_rational(&cubed, &cube).unwrap(),
                "{source}: cubed enclosure should contain {cube}"
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic integer power");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            let expected = if source == "(2^(1/3))^-1" {
                "1/2^(1/3)"
            } else {
                source
            };
            assert_eq!(exact.plain_text, expected);
        }
    }

    #[test]
    fn algebraic_square_roots_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        for (source, coefficients, real_root_index, power) in [
            (
                "sqrt(2^(1/3))",
                vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                1,
                6,
            ),
            (
                "sqrt((2^(1/3))^2)",
                vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                0,
                3,
            ),
            (
                "((2^(1/3))^2)^(1/2)",
                vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                0,
                3,
            ),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap(),
                "{source}"
            );
            assert_eq!(algebraic.real_root_index, real_root_index, "{source}");
            assert_eq!(
                algebraic
                    .minimal_polynomial
                    .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                    .unwrap(),
                1,
                "{source}"
            );
            let CertifiedEnclosureState::Available(enclosure) =
                &evaluation.value.certified_enclosure
            else {
                panic!("{source}: real algebraic recognition should include an enclosure");
            };
            let powered = interval::pow_i64(enclosure, power, 128).unwrap();
            assert!(
                interval::contains_rational(&powered, &Rational::from_integer(Integer::from(2)),)
                    .unwrap(),
                "{source}: powered enclosure should contain 2"
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic square root");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, source);
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::AlgebraicMinimalPolynomial));
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::AlgebraicRootIsolation));
        }
    }

    #[test]
    fn rational_nth_root_powers_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        for (source, coefficients, real_root_index, power, target) in [
            (
                "2^(1/4)",
                vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                1,
                4,
                Integer::from(2),
            ),
            (
                "2^(2/3)",
                vec![
                    Integer::from(-4),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                0,
                3,
                Integer::from(4),
            ),
            (
                "8^(1/6)",
                vec![Integer::from(-2), Integer::zero(), Integer::one()],
                1,
                2,
                Integer::from(2),
            ),
            (
                "(-2)^(2/3)",
                vec![
                    Integer::from(-4),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                0,
                3,
                Integer::from(4),
            ),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap(),
                "{source}"
            );
            assert_eq!(algebraic.real_root_index, real_root_index, "{source}");
            let CertifiedEnclosureState::Available(enclosure) =
                &evaluation.value.certified_enclosure
            else {
                panic!("{source}: real algebraic recognition should include an enclosure");
            };
            let powered = interval::pow_i64(enclosure, power, 128).unwrap();
            assert!(
                interval::contains_rational(&powered, &Rational::from_integer(target)).unwrap(),
                "{source}: powered enclosure should contain target"
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic nth root");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, source);
        }
    }

    #[test]
    fn algebraic_nth_root_powers_are_real_algebraic() {
        let mut context = EvaluationContext::default();
        for (source, coefficients, real_root_index, power, target) in [
            (
                "(2^(1/3))^(1/5)",
                {
                    let mut coefficients = vec![Integer::zero(); 16];
                    coefficients[0] = Integer::from(-2);
                    coefficients[15] = Integer::one();
                    coefficients
                },
                0,
                15,
                Integer::from(2),
            ),
            (
                "(2^(1/3))^(2/5)",
                {
                    let mut coefficients = vec![Integer::zero(); 16];
                    coefficients[0] = Integer::from(-4);
                    coefficients[15] = Integer::one();
                    coefficients
                },
                0,
                15,
                Integer::from(4),
            ),
            (
                "sqrt(2)^(1/2)",
                vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ],
                1,
                4,
                Integer::from(2),
            ),
            (
                "(-1*2^(1/3))^(1/3)",
                {
                    let mut coefficients = vec![Integer::zero(); 10];
                    coefficients[0] = Integer::from(2);
                    coefficients[9] = Integer::one();
                    coefficients
                },
                0,
                9,
                Integer::from(-2),
            ),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(coefficients).unwrap(),
                "{source}"
            );
            assert_eq!(algebraic.real_root_index, real_root_index, "{source}");
            let CertifiedEnclosureState::Available(enclosure) =
                &evaluation.value.certified_enclosure
            else {
                panic!("{source}: real algebraic recognition should include an enclosure");
            };
            let powered = interval::pow_i64(enclosure, power, 128).unwrap();
            assert!(
                interval::contains_rational(&powered, &Rational::from_integer(target)).unwrap(),
                "{source}: powered enclosure should contain target"
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic nth root");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            let expected = if source == "(-1*2^(1/3))^(1/3)" {
                "(-2^(1/3))^(1/3)"
            } else {
                source
            };
            assert_eq!(exact.plain_text, expected);
        }
    }

    #[test]
    fn prime_root_quotient_is_real_algebraic() {
        let source = "2^(1/3)/4^(1/3)";
        let parsed = parse(source, &ParseSettings::default()).unwrap();
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
        let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact else {
            panic!("expected quotient real algebraic recognition");
        };
        assert_eq!(
            algebraic.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-1),
                Integer::zero(),
                Integer::zero(),
                Integer::from(2),
            ])
            .unwrap()
        );
        assert_eq!(
            algebraic
                .minimal_polynomial
                .distinct_real_root_count_in_interval(&algebraic.isolating_interval)
                .unwrap(),
            1
        );

        let outcome = calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation for quotient algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, source);
    }

    #[test]
    fn degree_one_algebraic_results_collapse_to_rational() {
        let mut context = EvaluationContext::default();
        for (source, expected) in [
            ("2^(1/3)-2^(1/3)", Rational::zero()),
            ("2^(2/3)/4^(1/3)", Rational::one()),
            ("2^(1/3)-2^(1/3)+1", Rational::one()),
            ("(2^(1/3)-2^(1/3))+2^(1/3)-2^(1/3)", Rational::zero()),
            ("((2^(1/3))^2/4^(1/3))*2^(2/3)/4^(1/3)", Rational::one()),
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::Rational(value) = &evaluation.value.recognized_exact else {
                panic!(
                    "{source}: expected rational recognition after algebraic reduction: {evaluation:#?}"
                );
            };
            assert_eq!(value, &expected);
            assert!(evaluation
                .metadata
                .methods
                .contains(&MethodTag::AlgebraicMinimalPolynomial));
            assert!(evaluation
                .metadata
                .methods
                .contains(&MethodTag::AlgebraicRootIsolation));

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Complete(calculation) = outcome else {
                panic!("{source}: expected complete rational calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact rational output");
            };
            assert_eq!(
                calculation.metadata.exact_representation,
                exact.representation
            );
        }
    }

    #[test]
    fn nested_degree_one_algebraic_collapse_feeds_later_algebraic_operations() {
        let mut context = EvaluationContext::default();
        for source in [
            "(2^(1/3)-2^(1/3))+2^(1/3)",
            "(2^(1/3)/2^(1/3))*2^(1/3)",
            "((2^(1/3)-2^(1/3))+2)^(1/3)",
            "((2^(1/3)/2^(1/3))*2)^(1/3)",
        ] {
            let parsed = parse(source, &ParseSettings::default()).unwrap();
            let evaluation = evaluate(
                &parsed,
                &EvaluationRequest {
                    semantics: SemanticSettings::default(),
                    limits: ResourceLimitRequest::Default,
                },
                &mut context,
            )
            .unwrap();
            let RecognizedExact::RealAlgebraic(algebraic) = &evaluation.value.recognized_exact
            else {
                panic!("{source}: expected real algebraic recognition");
            };
            assert_eq!(
                algebraic.minimal_polynomial,
                PrimitivePolynomial::new(vec![
                    Integer::from(-2),
                    Integer::zero(),
                    Integer::zero(),
                    Integer::one(),
                ])
                .unwrap()
            );

            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic value");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        }
    }

    #[test]
    fn algebraic_sum_limits_fall_back_to_symbolic_without_error() {
        let source = "2^(1/3)+2^(1/3)";
        assert_symbolic_fallback_exact_with_limits(
            source,
            "2*2^(1/3)",
            ResourceLimits {
                max_resultant_degree: 2,
                ..ResourceLimits::default()
            },
            false,
        );
        assert_symbolic_fallback_exact_with_limits(
            source,
            "2*2^(1/3)",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
            false,
        );
    }

    #[test]
    fn algebraic_product_limits_fall_back_to_symbolic_without_error() {
        let source = "2^(1/3)*2^(1/3)";
        assert_symbolic_fallback_exact_with_limits(
            source,
            "(2^(1/3))^2",
            ResourceLimits {
                max_resultant_degree: 2,
                ..ResourceLimits::default()
            },
            false,
        );
        assert_symbolic_fallback_exact_with_limits(
            source,
            "(2^(1/3))^2",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
            false,
        );
    }

    #[test]
    fn algebraic_power_limits_fall_back_to_symbolic_without_error() {
        let source = "(2^(1/3))^2";
        assert_source_symbolic_fallback_with_limits(
            source,
            ResourceLimits {
                max_resultant_degree: 2,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            source,
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
        );
    }

    #[test]
    fn algebraic_quotient_limits_fall_back_to_symbolic_without_error() {
        let source = "2^(1/3)/4^(1/3)";
        assert_source_symbolic_fallback_with_limits(
            source,
            ResourceLimits {
                max_resultant_degree: 2,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            source,
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
        );
    }

    #[test]
    fn algebraic_square_root_limits_fall_back_to_symbolic_without_error() {
        assert_source_symbolic_fallback_with_limits_and_nested_algebraic(
            "sqrt(2^(1/3))",
            ResourceLimits {
                max_algebraic_degree: 5,
                ..ResourceLimits::default()
            },
            true,
        );
        assert_source_symbolic_fallback_with_limits_and_nested_algebraic(
            "sqrt(2^(1/3))",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
            false,
        );
    }

    #[test]
    fn algebraic_nth_root_limits_fall_back_to_symbolic_without_error() {
        assert_source_symbolic_fallback_with_limits_and_nested_algebraic(
            "(2^(1/3))^(1/5)",
            ResourceLimits {
                max_resultant_degree: 14,
                ..ResourceLimits::default()
            },
            true,
        );
        assert_source_symbolic_fallback_with_limits_and_nested_algebraic(
            "(2^(1/3))^(1/5)",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
            false,
        );
        assert_source_symbolic_fallback_with_limits(
            "2^(1/4)",
            ResourceLimits {
                max_resultant_degree: 3,
                ..ResourceLimits::default()
            },
        );
    }

    #[test]
    fn algebraic_root_limits_fall_back_to_symbolic_without_error() {
        assert_symbolic_fallback_with_limits(ResourceLimits {
            max_algebraic_degree: 2,
            ..ResourceLimits::default()
        });

        assert_symbolic_fallback_with_limits(ResourceLimits {
            max_polynomial_coefficient_bits: 1,
            ..ResourceLimits::default()
        });

        assert_symbolic_fallback_with_limits(ResourceLimits {
            max_root_isolation_steps: 0,
            ..ResourceLimits::default()
        });
    }

    #[test]
    fn constants_return_partial_with_certified_enclosures() {
        let source = "e";
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &high_precision_enclosure_request(), &mut context)
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
        assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
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
            let outcome = calculate(source, &high_precision_enclosure_request(), &mut context)
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
            assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
            assert_eq!(
                calculation.metadata.assurance,
                AssuranceLevel::CertifiedEnclosure
            );
        }
    }

    #[test]
    fn initial_exp_log_identities_are_exact() {
        assert_eq!(exact_plain_text("exp(0)"), "1");
        assert_eq!(exact_plain_text("ln(1)"), "0");
    }

    #[test]
    fn guarded_exp_log_identities_are_exact_for_proven_rationals() {
        assert_eq!(exact_plain_text("exp(ln(2))"), "2");
        assert_eq!(exact_plain_text("exp(ln(1/3))"), "1/3");
        assert_eq!(exact_plain_text("exp(ln(0.1 + 0.2))"), "3/10");
        assert_eq!(exact_plain_text("ln(exp(2))"), "2");
        assert_eq!(exact_plain_text("ln(exp(-2))"), "-2");
        assert_eq!(exact_plain_text("ln(exp(1/3))"), "1/3");
    }

    #[test]
    fn guarded_exp_log_identities_are_exact_for_proven_radicals_and_algebraics() {
        for (source, expected) in [
            ("exp(ln(sqrt(2)))", "sqrt(2)"),
            ("exp(ln(sqrt(2)+sqrt(3)))", "sqrt(2) + sqrt(3)"),
            ("exp(ln(sqrt(3)-sqrt(2)))", "sqrt(3) - sqrt(2)"),
            ("exp(ln(sqrt(2)+sqrt(3)-3))", "-3 + sqrt(2) + sqrt(3)"),
            ("ln(exp(sqrt(2)))", "sqrt(2)"),
            ("ln(exp(-sqrt(2)))", "-sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        for source in ["exp(ln(2^(1/3)))", "ln(exp(2^(1/3)))"] {
            assert_eq!(
                exact_presentation_for(source).representation,
                ExactRepresentationKind::RealAlgebraic,
                "{source}"
            );
        }
    }

    #[test]
    fn nonzero_self_division_identities_are_exact_only_when_proven() {
        for source in [
            "pi/pi",
            "e/e",
            "exp(2)/exp(2)",
            "exp(sin(1))/exp(sin(1))",
            "ln(2)/ln(2)",
            "log(3,2)/log(3,2)",
        ] {
            assert_eq!(exact_plain_text(source), "1", "{source}");
        }

        assert_eq!(exact_plain_text("sin(1)/sin(1)"), "sin(1)/sin(1)");
    }

    #[test]
    fn symbolic_logarithm_presentation_expands_only_proven_positive_factors() {
        for (source, expected) in [
            ("ln(2*3)", "ln(6)"),
            ("ln(2*2*3)", "ln(12)"),
            ("ln(2/3)", "ln(2)-ln(3)"),
            ("ln(2^3)", "ln(8)"),
            ("ln(2^(-3))", "-ln(8)"),
            ("ln(sqrt(2))", "1/2*ln(2)"),
            ("ln(sqrt(2)^2)", "ln(2)"),
            ("ln((2+3)*5)", "ln(25)"),
            ("ln(pi*e)", "ln(pi)+ln(e)"),
        ] {
            assert_eq!(
                symbolic_plain_text_from_source(source),
                expected,
                "{source}"
            );
        }

        for (source, expected) in [("ln((-2)*(-3))", "ln(6)"), ("ln(sin(1)*2)", "ln(2*sin(1))")] {
            assert_eq!(
                symbolic_plain_text_from_source(source),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn symbolic_square_roots_of_squares_expand_only_when_base_is_nonnegative() {
        for (source, expected) in [
            ("sqrt(exp(2)^2)", "exp(2)"),
            ("sqrt(exp(sin(1))^2)", "exp(sin(1))"),
            ("sqrt(ln(2)^2)", "ln(2)"),
            ("sqrt(log(3,2)^2)", "log(3,2)"),
            ("sqrt(sqrt(2)^2)", "sqrt(2)"),
            ("sqrt((2+3)^2)", "sqrt(25)"),
            ("sqrt(exp(2)*exp(2))", "exp(2)"),
        ] {
            assert_eq!(
                symbolic_plain_text_from_source(source),
                expected,
                "{source}"
            );
        }

        for (source, expected) in [
            ("sqrt(ln(1/2)^2)", "sqrt((-ln(2))^2)"),
            ("sqrt(sin(1)^2)", "sqrt(sin(1)^2)"),
            ("sqrt((-exp(2))^2)", "exp(2)"),
        ] {
            assert_eq!(
                symbolic_plain_text_from_source(source),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn guarded_inverse_trig_compositions_are_exact_for_proven_values() {
        for (source, expected) in [
            ("sin(asin(1/3))", "1/3"),
            ("cos(acos(-1/3))", "-1/3"),
            ("tan(atan(1/3))", "1/3"),
            ("cos(asin(1/3))", "2*sqrt(2)/3"),
            ("sin(acos(1/3))", "2*sqrt(2)/3"),
            ("sin(asin(sqrt(2)/3))", "sqrt(2)/3"),
            ("cos(acos(sqrt(3)/3))", "sqrt(3)/3"),
            ("tan(atan(sqrt(2)))", "sqrt(2)"),
            ("cos(asin(sqrt(2)/3))", "sqrt(7)/3"),
            ("sin(acos(sqrt(3)/3))", "sqrt(6)/3"),
            ("sin(asin(sqrt(3)-sqrt(2)))", "sqrt(3) - sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        let mut context = EvaluationContext::default();
        for source in [
            "tan(atan(2^(1/3)))",
            "cos(asin(2^(1/3)-1))",
            "sin(acos(2^(1/3)-1))",
        ] {
            let outcome = calculate(source, &exact_only_request(), &mut context).unwrap();
            let CalculationOutcome::Complete(calculation) = outcome else {
                panic!("{source}: expected complete real algebraic calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact output");
            };
            assert_eq!(
                exact.representation,
                ExactRepresentationKind::RealAlgebraic,
                "{source}"
            );
            assert_eq!(
                calculation.metadata.exact_representation,
                ExactRepresentationKind::RealAlgebraic,
                "{source}"
            );
            assert!(
                calculation
                    .metadata
                    .methods
                    .contains(&MethodTag::AlgebraicMinimalPolynomial),
                "{source}"
            );
            assert!(
                calculation
                    .metadata
                    .methods
                    .contains(&MethodTag::AlgebraicRootIsolation),
                "{source}"
            );
        }
    }

    #[test]
    fn guarded_inverse_trig_compositions_honor_angle_unit_semantics() {
        let mut degree_request = exact_only_request();
        degree_request.semantics.angle_unit = AngleUnit::Degree;
        assert_eq!(
            exact_plain_text_with_request("sin(asin(1/3))", &degree_request),
            "1/3"
        );
        assert_eq!(
            exact_plain_text_with_request("cos(acos(sqrt(2)/3))", &degree_request),
            "sqrt(2)/3"
        );
        assert_eq!(
            exact_plain_text_with_request("cos(asin(sqrt(2)/3))", &degree_request),
            "sqrt(7)/3"
        );

        let mut gradian_request = exact_only_request();
        gradian_request.semantics.angle_unit = AngleUnit::Gradian;
        assert_eq!(
            exact_plain_text_with_request("tan(atan(1/3))", &gradian_request),
            "1/3"
        );
        assert_eq!(
            exact_plain_text_with_request("sin(acos(sqrt(3)/3))", &gradian_request),
            "sqrt(6)/3"
        );
    }

    #[test]
    fn symbolic_function_parity_presentation_normalizes_negative_arguments() {
        for (source, expected) in [
            ("sin(-1)", "-sin(1)"),
            ("cos(-1)", "cos(1)"),
            ("tan(-1)", "-tan(1)"),
            ("asin(-1/3)", "-asin(1/3)"),
            ("atan(-sqrt(2))", "-atan(sqrt(2))"),
            ("exp(sin(-1))", "exp(-sin(1))"),
            ("acos(-1/3)", "acos(-1/3)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn symbolic_trig_integer_pi_shift_presentation_normalizes_remainders() {
        for (source, expected) in [
            ("sin(pi+1/10)", "-sin(1/10)"),
            ("sin(1/10+pi)", "-sin(1/10)"),
            ("sin(2*pi+1/10)", "sin(1/10)"),
            ("sin((pi+pi)+1/10)", "sin(1/10)"),
            ("sin((1+1)*pi+1/10)", "sin(1/10)"),
            ("sin(pi-1/10)", "sin(1/10)"),
            ("sin(-pi+1/10)", "-sin(1/10)"),
            ("cos(pi+1/10)", "-cos(1/10)"),
            ("cos(2*pi+1/10)", "cos(1/10)"),
            ("cos(pi-1/10)", "-cos(1/10)"),
            ("tan(pi+1/10)", "tan(1/10)"),
            ("tan(2*pi+1/10)", "tan(1/10)"),
            ("tan(pi-1/10)", "-tan(1/10)"),
            ("exp(sin(pi+1/10))", "exp(-sin(1/10))"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn symbolic_trig_half_pi_shift_presentation_normalizes_cofunctions() {
        for (source, expected) in [
            ("sin(pi/2+1/10)", "cos(1/10)"),
            ("sin(1/10+pi/2)", "cos(1/10)"),
            ("sin(pi/2-1/10)", "cos(1/10)"),
            ("sin(3*pi/2+1/10)", "-cos(1/10)"),
            ("sin(-pi/2+1/10)", "-cos(1/10)"),
            ("cos(pi/2+1/10)", "-sin(1/10)"),
            ("cos(pi/2-1/10)", "sin(1/10)"),
            ("cos(3*pi/2+1/10)", "sin(1/10)"),
            ("cos(-pi/2+1/10)", "sin(1/10)"),
            ("tan(pi/2+1/10)", "-1/tan(1/10)"),
            ("tan(pi/2-1/10)", "1/tan(1/10)"),
            ("tan(3*pi/2+1/10)", "-1/tan(1/10)"),
            ("tan(-pi/2+1/10)", "-1/tan(1/10)"),
            ("exp(sin(pi/2+1/10))", "exp(cos(1/10))"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn exp_one_returns_partial_euler_enclosure() {
        let mut context = EvaluationContext::default();
        let outcome =
            calculate("exp(1)", &high_precision_enclosure_request(), &mut context).unwrap();
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
        assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
        assert_eq!(
            calculation.metadata.assurance,
            AssuranceLevel::CertifiedEnclosure
        );
    }

    #[test]
    fn transcendental_interval_evaluation_retains_symbolic_exact_expression() {
        let mut context = EvaluationContext::default();
        for (source, expected_plain_text) in [
            ("ln(2)", "ln(2)"),
            ("ln(1/2)", "-ln(2)"),
            ("exp(2)", "exp(2)"),
            ("sqrt(2)+ln(2)", "ln(2)+sqrt(2)"),
            ("ln(sqrt(2))", "1/2*ln(2)"),
            ("sqrt(exp(2)^2)", "exp(2)"),
            ("exp(sqrt(2))", "exp(sqrt(2))"),
            ("atan(1/3)", "atan(1/3)"),
            ("asin(1/3)", "asin(1/3)"),
            ("acos(1/3)", "acos(1/3)"),
            ("tan(1)", "tan(1)"),
            ("sin(1)", "sin(1)"),
            ("cos(1)", "cos(1)"),
            ("sin(2)", "sin(2)"),
            ("cos(2)", "cos(2)"),
            ("tan(2)", "tan(2)"),
            ("sin(pi+1/10)", "-sin(1/10)"),
            ("cos(pi+1/10)", "-cos(1/10)"),
            ("tan(pi+1/10)", "tan(1/10)"),
            ("sin(pi/2+1/10)", "cos(1/10)"),
            ("cos(pi/2+1/10)", "-sin(1/10)"),
            ("tan(pi/2+1/10)", "-1/tan(1/10)"),
        ] {
            let outcome =
                calculate(source, &high_precision_enclosure_request(), &mut context).unwrap();
            let CalculationOutcome::Partial {
                calculation,
                certified_enclosure,
                ..
            } = outcome
            else {
                panic!("{source}: expected partial symbolic calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected retained exact expression");
            };
            assert_eq!(
                exact.representation,
                ExactRepresentationKind::GeneralSymbolic
            );
            assert_eq!(exact.plain_text, expected_plain_text, "{source}");
            let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
                panic!("{source}: expected requested enclosure output");
            };
            assert_eq!(certified_enclosure.as_ref(), Some(&enclosure));
            assert_eq!(
                calculation.metadata.exact_representation,
                ExactRepresentationKind::GeneralSymbolic
            );
            assert_eq!(
                calculation.metadata.assurance,
                AssuranceLevel::CertifiedEnclosure
            );
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::SymbolicRetention));
            assert!(calculation
                .metadata
                .methods
                .contains(&MethodTag::CertifiedIntervalEvaluation));
        }
    }

    #[test]
    fn nested_exact_reduction_precedes_outer_evaluation_mode() {
        for (source, expected) in [
            ("floor(sqrt(2)*sqrt(2))", "2"),
            ("abs(sqrt(2)*sqrt(2))", "2"),
            ("fact(sqrt(2)*sqrt(2))", "2"),
            ("gcd(sqrt(2)*sqrt(2),4)", "2"),
            ("sin(sqrt(2)*sqrt(2))", "sin(2)"),
            ("exp(sqrt(2)*sqrt(2))", "exp(2)"),
            ("sin((sqrt(2)*sqrt(2))*pi/6)", "sqrt(3)/2"),
            ("sin((2^(1/3)-2^(1/3))+pi/6)", "1/2"),
            ("(2^(1/3)-2^(1/3))+2^(1/3)", "2^(1/3)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn exact_reduction_values_feed_sign_floor_and_integer_function_consumers() {
        for (source, expected) in [
            ("abs(sqrt(2))", "sqrt(2)"),
            ("floor(sqrt(2))", "1"),
            ("abs(pi)", "pi"),
            ("floor(pi)", "3"),
            ("abs(2^(1/3))", "2^(1/3)"),
            ("abs(-(2^(1/3)))", "abs(2^(1/3))"),
            ("abs(-sin(pi/5))", "abs(sin(pi/5))"),
            ("floor(sin(pi/5))", "0"),
            ("floor(sin(1))", "0"),
            ("fact(sin(1)-sin(1))", "1"),
            ("gcd(sin(1)-sin(1),2)", "2"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        for source in ["fact(sqrt(2))", "gcd(sqrt(2),2)"] {
            let mut context = EvaluationContext::default();
            let error = calculate(source, &exact_only_request(), &mut context).expect_err(source);
            assert_eq!(
                error,
                CalculatorError::Domain(DomainError {
                    kind: DomainErrorKind::IntegerFunctionRequiresInteger,
                    span: None,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn hyperbolic_lowering_uses_nested_scaled_exp_log_identities() {
        for (source, expected) in [
            ("exp(-ln(2))", "1/2"),
            ("exp(2*ln(3))", "9"),
            ("sinh(ln(2))", "3/4"),
            ("cosh(100)-sinh(100)", "exp(-100)"),
            ("cosh(1000)-sinh(1000)-e^(-1000)", "0"),
            ("e^sin(1)-exp(sin(1))", "0"),
            ("exp(sin(1),e)-exp(sin(1))", "0"),
            ("sin(1)*cos(1)-cos(1)*sin(1)", "0"),
            ("2*(sin(1)*cos(1))-2*(cos(1)*sin(1))", "0"),
            ("(2*sin(1))*cos(1)-2*(sin(1)*cos(1))", "0"),
            ("(sin(1)*cos(1))*exp(1)-sin(1)*(cos(1)*exp(1))", "0"),
            ("exp(cosh(100)-sinh(100)-e^(-100))", "1"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn hyperbolic_cancellation_numeric_outputs_match_the_reduced_exponential() {
        fn numeric_outputs(source: &str) -> (ScientificOutput, EnclosureOutput) {
            let mut context = EvaluationContext::default();
            let outcome = calculate(source, &CalculationRequest::default(), &mut context)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let calculation = match outcome {
                CalculationOutcome::Complete(calculation)
                | CalculationOutcome::Partial { calculation, .. } => calculation,
            };
            (calculation.scientific, calculation.enclosure)
        }

        assert_eq!(
            numeric_outputs("cosh(100)-sinh(100)"),
            numeric_outputs("exp(-100)")
        );
    }

    #[test]
    fn hyperbolic_cancellation_is_normalized_before_numeric_evaluation() {
        let mut context = EvaluationContext::default();
        let outcome = calculate(
            "cosh(1000)-sinh(1000)-e^(-1000)",
            &CalculationRequest::default(),
            &mut context,
        )
        .unwrap();
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("exact zero must confirm every requested output");
        };

        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("default request includes exact output");
        };
        assert_eq!(exact.plain_text, "0");

        let ScientificOutput::Included(scientific) = calculation.scientific else {
            panic!("exact zero must have scientific output");
        };
        assert_eq!(scientific.significand, "0.0000");
        assert_eq!(scientific.exponent_ten, "0");

        let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
            panic!("exact zero must have a certified enclosure");
        };
        let CertifiedIntervalBounds::DecimalScientific { lower, upper, .. } = enclosure.bounds
        else {
            panic!("default request uses decimal scientific enclosure bounds");
        };
        assert_eq!(lower.significand, "0.0000");
        assert_eq!(upper.significand, "0.0000");
    }

    #[test]
    fn arithmetic_normalization_uses_canonical_factor_and_polynomial_forms() {
        for (source, expected) in [
            ("sin(1)*sin(1)", "sin(1)^2"),
            ("sin(1)^2*sin(1)^3", "sin(1)^5"),
            ("(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)", "sin(1)*cos(1)"),
            ("exp(1)*(sin(1)+cos(1))-exp(1)*sin(1)-exp(1)*cos(1)", "0"),
            ("(exp(1)+sin(1))^2-exp(1)^2-2*exp(1)*sin(1)-sin(1)^2", "0"),
            ("(exp(1)*sin(1))/exp(1)", "sin(1)"),
            ("(exp(1)*sin(1)+exp(1)*cos(1))/exp(1)", "sin(1)+cos(1)"),
            ("sin(pi/2)*exp(1)-exp(1)", "0"),
            ("sqrt(2)*sqrt(2)*exp(1)-2*exp(1)", "0"),
            ("(sin(pi/6)+sin(pi/6))*exp(1)-exp(1)", "0"),
            ("(2^(1/3))^2-2^(2/3)", "0"),
            ("exp(1)*exp(2)", "exp(3)"),
            ("exp(sin(1))*exp(cos(1))", "exp(sin(1)+cos(1))"),
            ("exp(sin(1))*exp(-sin(1))", "1"),
            ("exp(1)/exp(2)", "exp(-1)"),
            ("sqrt(exp(sin(1)))", "exp(1/2*sin(1))"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn trigonometric_square_complements_normalize_canonically() {
        for (source, expected) in [
            ("sin(1)^2+cos(1)^2", "1"),
            ("cos(1)*cos(1)+sin(1)*sin(1)", "1"),
            ("3*sin(1)^2+3*cos(1)^2", "3"),
            ("-2*sin(1)^2-2*cos(1)^2", "-2"),
            ("3*sin(1)^2+2*cos(1)^2", "sin(1)^2+2"),
            ("-3*sin(1)^2-2*cos(1)^2", "-(2+sin(1)^2)"),
            ("exp(1)*sin(1)^2+exp(1)*cos(1)^2", "exp(1)"),
            ("sin(1)^2+cos(1)^2+sin(2)", "sin(2)+1"),
            ("sin(1)^2-cos(1)^2", "sin(1)^2-cos(1)^2"),
            ("sin(1)^2+cos(2)^2", "sin(1)^2+cos(2)^2"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        let mut degree_request = exact_only_request();
        degree_request.semantics.angle_unit = AngleUnit::Degree;
        assert_eq!(
            exact_plain_text_with_request("sin(1)^2+cos(1)^2", &degree_request),
            "1"
        );

        let mut context = EvaluationContext::default();
        let error = calculate(
            "sin(ln(-1))^2+cos(ln(-1))^2",
            &exact_only_request(),
            &mut context,
        )
        .expect_err("undefined argument must not be hidden by the identity");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::LogarithmOfNonPositive,
                span: None,
            })
        );
    }

    #[test]
    fn arithmetic_normalization_is_independent_of_order_and_factoring() {
        for equivalent_sources in [
            [
                "sin(1)*cos(1)*exp(1)",
                "exp(1)*(cos(1)*sin(1))",
                "(sin(1)*exp(1))*cos(1)",
            ],
            [
                "exp(1)*(sin(1)+cos(1))",
                "exp(1)*sin(1)+exp(1)*cos(1)",
                "cos(1)*exp(1)+sin(1)*exp(1)",
            ],
            [
                "(exp(1)+sin(1))/exp(2)",
                "exp(1)/exp(2)+sin(1)/exp(2)",
                "sin(1)*exp(-2)+exp(-1)",
            ],
            ["sin(pi/2)*exp(1)+cos(1)", "exp(1)+cos(1)", "cos(1)+exp(1)"],
            [
                "exp(sin(pi/2)*exp(1)+cos(1))",
                "exp(exp(1)+cos(1))",
                "exp(cos(1)+exp(1))",
            ],
            [
                "sqrt(2)*sqrt(2)*exp(1)+cos(1)",
                "2*exp(1)+cos(1)",
                "cos(1)+exp(1)*2",
            ],
            ["sin(pi/2)*exp(1)+exp(2)", "exp(1)+exp(2)", "exp(2)+exp(1)"],
            ["cos(0)*sin(1)+sin(2)", "sin(1)+sin(2)", "sin(2)+sin(1)"],
            [
                "2*sin(pi/6)*2^(1/3)",
                "2^(1/3)",
                "sqrt(2)*sqrt(2)/2*2^(1/3)",
            ],
        ] {
            let expected = exact_plain_text(equivalent_sources[0]);
            for source in &equivalent_sources[1..] {
                assert_eq!(exact_plain_text(source), expected, "{source}");
            }
        }
    }

    #[test]
    fn canonical_polynomial_expansion_uses_the_shared_rewrite_budget() {
        let request = CalculationRequest {
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Omit,
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_rewrite_steps: 2,
                max_logical_work_units: 2,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let outcome = calculate("(sin(1)+cos(1)+exp(1))^8", &request, &mut context)
            .expect("bounded canonicalization must return a typed partial outcome");
        let CalculationOutcome::Partial { reason, .. } = outcome else {
            panic!("insufficient canonicalization budget must be visible");
        };
        assert!(matches!(
            reason,
            IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::RewriteSteps | ComputationLimitKind::LogicalWorkUnits,
            }
        ));
    }

    #[test]
    fn factor_normalization_never_cancels_domain_obligations() {
        for source in [
            "(ln(-1)*exp(1))/ln(-1)",
            "(exp(1)+sin(1))*ln(-1)-exp(1)*ln(-1)-sin(1)*ln(-1)",
        ] {
            for request in [
                exact_only_request(),
                CalculationRequest {
                    scientific_output: ScientificOutputRequest::Omit,
                    enclosure_output: EnclosureOutputRequest::Omit,
                    limits: ResourceLimitRequest::Custom(ResourceLimits {
                        max_rewrite_steps: 0,
                        max_logical_work_units: 0,
                        ..ResourceLimits::default()
                    }),
                    ..CalculationRequest::default()
                },
            ] {
                let mut context = EvaluationContext::default();
                let error = calculate(source, &request, &mut context).expect_err(source);
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
    }

    #[test]
    fn exhausted_budget_uses_structural_domain_validation_only() {
        let request = CalculationRequest {
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Omit,
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_rewrite_steps: 0,
                max_logical_work_units: 0,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let error = calculate("sqrt(-exp(1000000))", &request, &mut context)
            .expect_err("a structurally negative radicand is a domain error");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );
    }

    #[test]
    fn symbolic_cancellation_does_not_hide_domain_errors() {
        for (source, kind) in [
            ("ln(-1)-ln(-1)", DomainErrorKind::LogarithmOfNonPositive),
            (
                "ln(sin(-1))-ln(sin(-1))",
                DomainErrorKind::LogarithmOfNonPositive,
            ),
            (
                "sqrt(-exp(1))-sqrt(-exp(1))",
                DomainErrorKind::EvenRootOfNegative,
            ),
            ("0*ln(sin(-1))", DomainErrorKind::LogarithmOfNonPositive),
            ("log(2,2/2)-log(2,2/2)", DomainErrorKind::LogarithmBaseOne),
            (
                "log(2,exp(0))-log(2,exp(0))",
                DomainErrorKind::LogarithmBaseOne,
            ),
            ("0*log(2,2/2)", DomainErrorKind::LogarithmBaseOne),
            (
                "sqrt(((-1)^(1/2))^2)-sqrt(((-1)^(1/2))^2)",
                DomainErrorKind::NonRealPower,
            ),
            (
                "fact(-1)",
                DomainErrorKind::IntegerFunctionRequiresNonNegative,
            ),
            ("asin(2)", DomainErrorKind::InverseTrigonometricOutOfRange),
            ("acos(2)", DomainErrorKind::InverseTrigonometricOutOfRange),
            ("tan(pi/2)", DomainErrorKind::TangentPole),
            (
                "perm(-1,1)",
                DomainErrorKind::IntegerFunctionRequiresNonNegative,
            ),
            (
                "comb(-1,1)",
                DomainErrorKind::IntegerFunctionRequiresNonNegative,
            ),
            ("mod(1,0)", DomainErrorKind::DivisionByZero),
            (
                "gcd(1/2,1)",
                DomainErrorKind::IntegerFunctionRequiresInteger,
            ),
            (
                "fact(sqrt(2))",
                DomainErrorKind::IntegerFunctionRequiresInteger,
            ),
            (
                "gcd(sqrt(2),2)",
                DomainErrorKind::IntegerFunctionRequiresInteger,
            ),
            ("fact(pi)", DomainErrorKind::IntegerFunctionRequiresInteger),
            ("gcd(pi,2)", DomainErrorKind::IntegerFunctionRequiresInteger),
        ] {
            for request in [
                CalculationRequest::default(),
                CalculationRequest {
                    limits: ResourceLimitRequest::Custom(ResourceLimits {
                        max_rewrite_steps: 0,
                        max_logical_work_units: 0,
                        ..ResourceLimits::default()
                    }),
                    ..CalculationRequest::default()
                },
            ] {
                let mut context = EvaluationContext::default();
                let error = match calculate(source, &request, &mut context) {
                    Err(error) => error,
                    Ok(outcome) => panic!("{source}: undefined terms were cancelled: {outcome:?}"),
                };
                assert_eq!(
                    error,
                    CalculatorError::Domain(DomainError { kind, span: None }),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn nested_non_rational_exact_values_are_canonicalized_before_symbolic_retention() {
        for (source, expected) in [
            ("sin(sqrt(8))", "sin(2*sqrt(2))"),
            ("exp(sin(pi/4))", "exp(1/2*sqrt(2))"),
            ("cos(sqrt(18))", "cos(3*sqrt(2))"),
            ("ln(exp(sqrt(8)))", "2*sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }
    }

    #[test]
    fn nested_exact_reduction_is_shared_by_symbolic_and_interval_outputs() {
        fn enclosure(source: &str) -> CertifiedIntervalPresentation {
            let mut context = EvaluationContext::default();
            let outcome = calculate(source, &high_precision_enclosure_request(), &mut context)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let calculation = match outcome {
                CalculationOutcome::Complete(calculation)
                | CalculationOutcome::Partial { calculation, .. } => calculation,
            };
            let EnclosureOutput::Included(enclosure) = calculation.enclosure else {
                panic!("{source}: expected certified enclosure");
            };
            enclosure
        }

        assert_eq!(enclosure("sin(sqrt(2)*sqrt(2))"), enclosure("sin(2)"));
        assert_eq!(
            enclosure("sin((sqrt(2)*sqrt(2))*pi/6)"),
            enclosure("sqrt(3)/2")
        );
        assert_eq!(
            enclosure("sin((2^(1/3)+2^(1/3))/2)"),
            enclosure("sin(2^(1/3))")
        );
    }

    #[test]
    fn nested_reduction_preserves_input_presentation_and_method_provenance() {
        let source = "exp(sin(pi/4))";
        let request = exact_only_request();
        let input = present_input(source, &request).unwrap();
        assert_eq!(render_plain_text_for_test(&input), "e^Sin(π/4)");

        let mut context = EvaluationContext::default();
        let outcome = calculate(source, &request, &mut context).unwrap();
        let calculation = match outcome {
            CalculationOutcome::Complete(calculation)
            | CalculationOutcome::Partial { calculation, .. } => calculation,
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact symbolic output");
        };
        assert_eq!(exact.plain_text, "exp(1/2*sqrt(2))");
        assert_eq!(present_input(source, &request).unwrap(), input);
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SpecialAngle));
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::RadicalExtraction));
    }

    #[test]
    fn nested_domain_errors_are_not_hidden_by_symbolic_or_interval_fallback() {
        let mut context = EvaluationContext::default();
        let error = calculate(
            "sin(sqrt(-1)+sqrt(2)*sqrt(2))",
            &high_precision_enclosure_request(),
            &mut context,
        )
        .expect_err("the inner square root must remain a domain error");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );
    }

    #[test]
    fn nested_reduction_budget_falls_back_to_symbolic_without_hiding_domain_errors() {
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequest::Custom(ResourceLimits {
            max_rewrite_steps: 0,
            max_logical_work_units: 0,
            ..ResourceLimits::default()
        });
        let mut context = EvaluationContext::default();
        let outcome = calculate("sin(sqrt(2)*sqrt(2))", &request, &mut context).unwrap();
        assert_eq!(
            exact_plain_text_from_partial_outcome(outcome),
            "sin(sqrt(2)*sqrt(2))"
        );

        let error = calculate("sin(sqrt(-1))", &request, &mut context)
            .expect_err("domain errors must precede reduction-budget fallback");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );

        let mut factorial_request = exact_only_request();
        factorial_request.limits = ResourceLimitRequest::Custom(ResourceLimits {
            max_rewrite_steps: 1,
            max_logical_work_units: 1,
            ..ResourceLimits::default()
        });
        let outcome = calculate("fact(10000)", &factorial_request, &mut context).unwrap();
        assert_eq!(
            exact_plain_text_from_partial_outcome(outcome),
            "fact(10000)"
        );
    }

    #[test]
    fn symbolic_budget_fallback_is_partial_without_a_fabricated_enclosure() {
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_rewrite_steps: 1,
                max_logical_work_units: 1,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let CalculationOutcome::Partial {
            calculation,
            reason,
            certified_enclosure,
        } = calculate("fact(10000)", &request, &mut context)
            .expect("budget fallback remains a typed partial result")
        else {
            panic!("budget fallback must be partial");
        };
        assert_eq!(
            reason,
            IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits,
            }
        );
        assert!(certified_enclosure.is_none());
        assert!(matches!(
            calculation.scientific,
            ScientificOutput::Unavailable(UnavailableScientificOutput {
                reason: IncompleteReason::ComputationLimit {
                    kind: ComputationLimitKind::LogicalWorkUnits,
                },
                ..
            })
        ));
        assert!(matches!(
            calculation.enclosure,
            EnclosureOutput::Unavailable(UnavailableEnclosureOutput {
                reason: IncompleteReason::ComputationLimit {
                    kind: ComputationLimitKind::LogicalWorkUnits,
                },
            })
        ));
    }

    #[test]
    fn canonical_lowering_handles_wide_sums_before_zero_rewrite_budget_fallback() {
        let source = (1..=400)
            .map(|value| format!("sin({value})"))
            .collect::<Vec<_>>()
            .join("+");
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(ResourceLimits {
                max_rewrite_steps: 0,
                max_logical_work_units: 0,
                ..ResourceLimits::default()
            }),
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let outcome = calculate(&source, &request, &mut context)
            .expect("bounded wide input must return a typed outcome");
        assert!(matches!(outcome, CalculationOutcome::Partial { .. }));
    }

    #[test]
    fn algebraic_construction_reserves_shared_logical_work() {
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequest::Custom(ResourceLimits {
            max_rewrite_steps: 8,
            max_logical_work_units: 2,
            ..ResourceLimits::default()
        });
        let mut context = EvaluationContext::default();
        let outcome = calculate("2^(1/3)", &request, &mut context)
            .expect("insufficient shared work retains the algebraic expression symbolically");
        let CalculationOutcome::Partial {
            calculation,
            reason,
            ..
        } = outcome
        else {
            panic!("shared algebraic budget exhaustion must remain visible");
        };
        assert_eq!(
            reason,
            IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits,
            }
        );
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::GeneralSymbolic
        );
        assert_eq!(
            calculation.metadata.simplification_status,
            SimplificationStatus::PartiallySimplified {
                reason: IncompleteReason::ComputationLimit {
                    kind: ComputationLimitKind::LogicalWorkUnits,
                },
            }
        );
    }

    #[test]
    fn symbolic_absolute_value_keeps_a_certified_interval() {
        let mut context = EvaluationContext::default();
        let outcome = calculate("abs(sin(1))", &CalculationRequest::default(), &mut context)
            .expect("absolute value composes over a symbolic certified interval");
        let calculation = match outcome {
            CalculationOutcome::Complete(calculation)
            | CalculationOutcome::Partial { calculation, .. } => calculation,
        };
        assert!(matches!(
            calculation.enclosure,
            EnclosureOutput::Included(_)
        ));
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
        assert!(
            interval::contains_rational(&exact_dyadic_interval(&enclosure), &rational).unwrap()
        );
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

        let error =
            calculate("2^(1/3) / 0", &exact_only_request(), &mut context).expect_err("2^(1/3) / 0");
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

        let error = calculate("sqrt(2^(1/3)-2)", &exact_only_request(), &mut context)
            .expect_err("sqrt(2^(1/3)-2)");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::EvenRootOfNegative,
                span: None,
            })
        );

        let error = calculate("(2^(1/3)-2)^(1/2)", &exact_only_request(), &mut context)
            .expect_err("(2^(1/3)-2)^(1/2)");
        assert_eq!(
            error,
            CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::NonRealPower,
                span: None,
            })
        );
    }

    #[test]
    fn log_of_non_positive_is_domain_error() {
        for source in ["ln(0)", "ln(-1)", "ln(sqrt(2)-sqrt(3))"] {
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
        for source in [
            "asin(2)",
            "asin(exp(ln(2)))",
            "asin(sqrt(2))",
            "acos(-2)",
            "acos(-2*sqrt(2))",
            "sin(asin(sqrt(2)+sqrt(3)))",
            "cos(acos(-sqrt(2)-sqrt(3)))",
        ] {
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
        for source in [
            "exp(ln(0))",
            "exp(ln(-1))",
            "exp(ln(-sqrt(2)))",
            "exp(ln(sqrt(2)-sqrt(2)))",
            "exp(ln(sqrt(2)-sqrt(3)))",
        ] {
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
