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
    evaluate_rational_pi_multiple_dag, evaluate_real_algebraic_dag, lower_source_expression,
    ExactExpressionDag, PiCoefficientEvaluation, RadicalEvaluation,
    RadicalLinearCombinationEvaluation, RadicalReduction, RationalEvaluation,
    RealAlgebraicEvaluation,
};
use crate::interval;
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
    let mut calculation = present(
        &evaluation,
        &PresentationRequest {
            exact_output: request.exact_output,
            scientific_output: request.scientific_output,
            enclosure_output: request.enclosure_output,
        },
    )
    .map_err(CalculatorError::from)?;
    apply_calculation_symbolic_presentation(&mut calculation, &parsed, request, &evaluation);
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

fn apply_calculation_symbolic_presentation(
    calculation: &mut Calculation,
    parsed: &ParsedExpression,
    request: &CalculationRequest,
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
    let Ok(dag) = lower_source_expression(&parsed.root, request.semantics) else {
        return;
    };
    *exact = symbolic_presentation_from_dag(&dag);
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
    let limits = resource_limits(&request.limits);
    validate_parsed_expression_limits(expression, &limits).map_err(EvaluationError::InputLimit)?;
    let dag = lower_source_expression(&expression.root, request.semantics)?;
    validate_expression_dag_limits(&dag, &limits).map_err(EvaluationError::InputLimit)?;
    let rational = match evaluate_rational_evaluation_dag(&dag) {
        Ok(rational) => rational,
        Err(error) if should_fallback_to_symbolic_interval(&error) => {
            if let Some(pi_multiple) = evaluate_rational_pi_multiple_dag(&dag)? {
                if pi_multiple.coefficient().is_zero() {
                    return Ok(rational_evaluation_outcome(
                        expression,
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
                match radical {
                    RadicalReduction::Rational(rational) => {
                        return Ok(rational_evaluation_outcome_with_methods(
                            expression,
                            rational,
                            request,
                            &[MethodTag::RadicalExtraction],
                        ));
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
                        return Ok(EvaluationOutcome {
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
                            },
                        });
                    }
                }
            }
            if let Some(algebraic) = evaluate_real_algebraic_dag(&dag, &limits)? {
                let algebraic = match algebraic {
                    RealAlgebraicEvaluation::Rational(rational) => {
                        return Ok(rational_evaluation_outcome_with_methods(
                            expression,
                            rational,
                            request,
                            &[
                                MethodTag::AlgebraicMinimalPolynomial,
                                MethodTag::AlgebraicRootIsolation,
                            ],
                        ));
                    }
                    RealAlgebraicEvaluation::Algebraic(algebraic) => algebraic,
                };
                let certified_enclosure =
                    real_algebraic_enclosure(&dag, &algebraic).map_err(|interval_error| {
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
                return Ok(EvaluationOutcome {
                    value: EvaluatedValue {
                        exact_expression: ExactExpression {
                            source: expression.source.clone(),
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
    Ok(rational_evaluation_outcome(expression, rational, request))
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

fn real_algebraic_enclosure(
    _dag: &ExactExpressionDag,
    value: &RealAlgebraic,
) -> Result<CertifiedInterval, interval::IntervalError> {
    interval::from_rational_bounds(
        &value.isolating_interval().lower,
        &value.isolating_interval().upper,
        128,
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
            || unavailable_scientific_output(significant_digits, rounding_mode),
            ScientificOutput::Included,
        ),
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
            _ => format!("{numerator}{radical}"),
        };
    }

    match numerator.as_str() {
        "1" => format!("{radical}/{denominator}"),
        "-1" => format!("-{radical}/{denominator}"),
        _ => format!("{numerator}{radical}/{denominator}"),
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

const SYMBOLIC_PRECEDENCE_ADD: u8 = 1;
const SYMBOLIC_PRECEDENCE_MULTIPLY: u8 = 2;
const SYMBOLIC_PRECEDENCE_POWER: u8 = 3;
const SYMBOLIC_PRECEDENCE_PREFIX: u8 = 4;
const SYMBOLIC_PRECEDENCE_ATOM: u8 = 5;

fn render_symbolic_node(dag: &ExactExpressionDag, id: ExprId) -> RenderedSymbolic {
    signed_symbolic_to_rendered(render_signed_symbolic_node(dag, id))
}

fn render_signed_symbolic_node(dag: &ExactExpressionDag, id: ExprId) -> SignedRenderedSymbolic {
    match dag.node(id) {
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
        ExpressionNode::Add(list_id) => render_signed_symbolic_sum(dag, *list_id),
        ExpressionNode::Multiply(list_id) => render_signed_symbolic_product(dag, *list_id),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => render_signed_symbolic_division(dag, *numerator, *denominator),
        ExpressionNode::Function { function, argument } => {
            render_signed_symbolic_function(dag, *function, *argument)
        }
        ExpressionNode::Constant(_) | ExpressionNode::Power { .. } => SignedRenderedSymbolic {
            negative: false,
            value: render_unsigned_symbolic_node(dag, id),
        },
    }
}

fn render_unsigned_symbolic_node(dag: &ExactExpressionDag, id: ExprId) -> RenderedSymbolic {
    match dag.node(id) {
        ExpressionNode::Rational(rational_id) => RenderedSymbolic {
            text: dag.rational(*rational_id).to_string(),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Constant(Constant::Pi) => RenderedSymbolic {
            text: String::from("pi"),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Constant(Constant::Euler) => RenderedSymbolic {
            text: String::from("e"),
            precedence: SYMBOLIC_PRECEDENCE_ATOM,
        },
        ExpressionNode::Add(list_id) => render_symbolic_sum(dag, *list_id),
        ExpressionNode::Multiply(_) => render_symbolic_node(dag, id),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => signed_symbolic_to_rendered(render_signed_symbolic_division(
            dag,
            *numerator,
            *denominator,
        )),
        ExpressionNode::Power { base, exponent } => {
            let base = render_symbolic_node(dag, *base);
            let exponent = render_symbolic_node(dag, *exponent);
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
                    parenthesize_symbolic(&exponent, SYMBOLIC_PRECEDENCE_POWER)
                ),
                precedence: SYMBOLIC_PRECEDENCE_POWER,
            }
        }
        ExpressionNode::Function { function, argument } => {
            let argument = render_symbolic_node(dag, *argument);
            RenderedSymbolic {
                text: format!("{}({})", symbolic_function_name(*function), argument.text),
                precedence: SYMBOLIC_PRECEDENCE_ATOM,
            }
        }
    }
}

fn render_symbolic_sum(dag: &ExactExpressionDag, list_id: ExprListId) -> RenderedSymbolic {
    let terms = dag
        .list(list_id)
        .iter()
        .map(|child| render_signed_symbolic_node(dag, *child))
        .collect::<Vec<_>>();
    render_symbolic_sum_terms(&terms)
}

fn render_signed_symbolic_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> SignedRenderedSymbolic {
    let terms = dag
        .list(list_id)
        .iter()
        .map(|child| render_signed_symbolic_node(dag, *child))
        .collect::<Vec<_>>();
    render_signed_symbolic_sum_terms(&terms)
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

fn render_signed_symbolic_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> SignedRenderedSymbolic {
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
        Function::Sin | Function::Tan | Function::Asin | Function::Atan
            if signed_argument.negative =>
        {
            SignedRenderedSymbolic {
                negative: true,
                value: render_symbolic_function_value(function, &signed_argument.value),
            }
        }
        Function::Cos if signed_argument.negative => SignedRenderedSymbolic {
            negative: false,
            value: render_symbolic_function_value(Function::Cos, &signed_argument.value),
        },
        Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Exp
        | Function::Log => {
            let argument = signed_symbolic_to_rendered(signed_argument);
            SignedRenderedSymbolic {
                negative: false,
                value: render_symbolic_function_value(function, &argument),
            }
        }
    }
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
            | Function::Exp
            | Function::Log => None,
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
            | Function::Exp
            | Function::Log => None,
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
    let ExpressionNode::Add(list_id) = dag.node(argument) else {
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
    match dag.node(id) {
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
        ExpressionNode::Function { .. } | ExpressionNode::Power { .. } => None,
    }
}

fn symbolic_rational_value(dag: &ExactExpressionDag, id: ExprId) -> Option<Rational> {
    match dag.node(id) {
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
        | ExpressionNode::Function { .. }
        | ExpressionNode::Power { .. } => None,
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
        Function::Exp => "exp",
        Function::Log => "log",
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
        RecognizedExact::Rational(_)
        | RecognizedExact::Radical(_)
        | RecognizedExact::RadicalLinearCombination(_) => AssuranceLevel::Exact,
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
    significant_digits: core::num::NonZeroU32,
    rounding_mode: DecimalRoundingMode,
) -> ScientificOutput {
    ScientificOutput::Unavailable(UnavailableScientificOutput {
        requested_significant_digits: significant_digits,
        confirmed_significant_digits: 0,
        rounding_mode,
        reason: IncompleteReason::PrecisionLimit {
            requested_digits: significant_digits,
            confirmed_digits: 0,
        },
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

    fn scientific_text_with_request(source: &str, request: &CalculationRequest) -> String {
        let mut context = EvaluationContext::default();
        let outcome = calculate(source, request, &mut context).expect(source);
        let calculation = match outcome {
            CalculationOutcome::Complete(calculation) => calculation,
            CalculationOutcome::Partial { calculation, .. } => calculation,
        };
        let ScientificOutput::Included(scientific) = calculation.scientific else {
            panic!("{source}: expected scientific output");
        };
        format!("{}e{}", scientific.significand, scientific.exponent_ten)
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

    fn assert_source_symbolic_fallback_with_limits(source: &str, limits: ResourceLimits) {
        let request = CalculationRequest {
            limits: ResourceLimitRequest::Custom(limits),
            ..CalculationRequest::default()
        };
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
        assert_eq!(exact.plain_text, source);
        assert_eq!(
            calculation.metadata.exact_representation,
            ExactRepresentationKind::GeneralSymbolic
        );
        assert!(calculation
            .metadata
            .methods
            .contains(&MethodTag::SymbolicRetention));
        assert!(!calculation
            .metadata
            .methods
            .contains(&MethodTag::AlgebraicMinimalPolynomial));
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
            ("sqrt(6962)", "59sqrt(2)"),
            ("sqrt(1/2)", "sqrt(2)/2"),
            ("sqrt(1/6962)", "sqrt(2)/118"),
            ("2^(1/2)", "sqrt(2)"),
            ("3*sqrt(8)", "6sqrt(2)"),
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
            ("2 * (sin(pi/6) + sqrt(2))", "1 + 2sqrt(2)"),
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
            ("(sqrt(2))^3", "2sqrt(2)"),
            ("(sqrt(2))^-1", "sqrt(2)/2"),
            ("(1 + sqrt(2))^2", "3 + 2sqrt(2)"),
            ("(sqrt(2) + 1) * (sqrt(2) - 1)", "1"),
            ("sqrt(2)/sqrt(2) + 1", "2"),
            ("(sqrt(2)/sqrt(2)) * 2", "2"),
            ("sqrt(8) + sqrt(2)", "3sqrt(2)"),
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
    fn certified_enclosures_confirm_scientific_output_when_digits_are_resolved() {
        let request = scientific_request(5, DecimalRoundingMode::NearestTiesToEven);
        for (source, expected) in [
            ("sqrt(2)", "1.4142e0"),
            ("pi", "3.1416e0"),
            ("pi/6", "5.2360e-1"),
            ("exp(1)", "2.7183e0"),
            ("sin(1)", "8.4147e-1"),
        ] {
            assert_eq!(
                scientific_text_with_request(source, &request),
                expected,
                "{source}"
            );
        }
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
    fn positive_base_general_powers_return_partial_with_certified_enclosure() {
        let mut context = EvaluationContext::default();
        for source in ["2^sqrt(2)", "sqrt(2)^sqrt(2)"] {
            let outcome = calculate(source, &CalculationRequest::default(), &mut context)
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
            assert_eq!(certified_enclosure, enclosure);
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

        let outcome = calculate("2^(1/3)", &CalculationRequest::default(), &mut context).unwrap();
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
        let interval = CertifiedInterval {
            lower: certified_enclosure.lower,
            upper: certified_enclosure.upper,
        };
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

        let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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
        let interval = CertifiedInterval {
            lower: certified_enclosure.lower,
            upper: certified_enclosure.upper,
        };
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
                vec![
                    Integer::one(),
                    Integer::from(3),
                    Integer::from(-3),
                    Integer::one(),
                ],
            ),
            (
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
                vec![
                    Integer::from(-5),
                    Integer::from(12),
                    Integer::from(-12),
                    Integer::from(4),
                ],
            ),
            (
                "1/2^(1/3)+1",
                vec![
                    Integer::from(-3),
                    Integer::from(6),
                    Integer::from(-6),
                    Integer::from(2),
                ],
            ),
        ];

        for (source, coefficients) in cases {
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, source);
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

        let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation for root-sum algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, source);
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

        let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
        let CalculationOutcome::Partial { calculation, .. } = outcome else {
            panic!("expected partial calculation for product algebraic number");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact algebraic output");
        };
        assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
        assert_eq!(exact.plain_text, source);
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
            let CalculationOutcome::Partial { calculation, .. } = outcome else {
                panic!("{source}: expected partial calculation for algebraic integer power");
            };
            let ExactOutput::Included(exact) = calculation.exact else {
                panic!("{source}: expected exact algebraic output");
            };
            assert_eq!(exact.representation, ExactRepresentationKind::RealAlgebraic);
            assert_eq!(exact.plain_text, source);
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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

        let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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
            ("2^(1/3)/2^(1/3)", Rational::one()),
            ("2^(1/3)-2^(1/3)+1", Rational::one()),
            ("(2^(1/3)-2^(1/3))+2^(1/3)-2^(1/3)", Rational::zero()),
            ("(2^(1/3)/2^(1/3))*2^(1/3)/2^(1/3)", Rational::one()),
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
                panic!("{source}: expected rational recognition after algebraic reduction");
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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

            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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
    fn algebraic_product_limits_fall_back_to_symbolic_without_error() {
        let source = "2^(1/3)*2^(1/3)";
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
        assert_source_symbolic_fallback_with_limits(
            "sqrt(2^(1/3))",
            ResourceLimits {
                max_algebraic_degree: 5,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            "sqrt(2^(1/3))",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
        );
    }

    #[test]
    fn algebraic_nth_root_limits_fall_back_to_symbolic_without_error() {
        assert_source_symbolic_fallback_with_limits(
            "(2^(1/3))^(1/5)",
            ResourceLimits {
                max_resultant_degree: 14,
                ..ResourceLimits::default()
            },
        );
        assert_source_symbolic_fallback_with_limits(
            "(2^(1/3))^(1/5)",
            ResourceLimits {
                max_factorization_work: 0,
                ..ResourceLimits::default()
            },
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
    fn guarded_exp_log_identities_are_exact_for_proven_radicals_and_algebraics() {
        for (source, expected) in [
            ("exp(log(sqrt(2)))", "sqrt(2)"),
            ("exp(log(sqrt(2)+sqrt(3)))", "sqrt(2) + sqrt(3)"),
            ("exp(log(sqrt(3)-sqrt(2)))", "sqrt(3) - sqrt(2)"),
            ("exp(log(sqrt(2)+sqrt(3)-3))", "-3 + sqrt(2) + sqrt(3)"),
            ("log(exp(sqrt(2)))", "sqrt(2)"),
            ("log(exp(-sqrt(2)))", "-sqrt(2)"),
        ] {
            assert_eq!(exact_plain_text(source), expected, "{source}");
        }

        for source in ["exp(log(2^(1/3)))", "log(exp(2^(1/3)))"] {
            assert_eq!(
                exact_presentation_for(source).representation,
                ExactRepresentationKind::RealAlgebraic,
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
            ("cos(asin(1/3))", "2sqrt(2)/3"),
            ("sin(acos(1/3))", "2sqrt(2)/3"),
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
    fn transcendental_interval_evaluation_retains_symbolic_exact_expression() {
        let mut context = EvaluationContext::default();
        for (source, expected_plain_text) in [
            ("log(2)", "log(2)"),
            ("log(1/2)", "log(1/2)"),
            ("exp(2)", "exp(2)"),
            ("sqrt(2)+log(2)", "sqrt(2)+log(2)"),
            ("log(sqrt(2))", "log(sqrt(2))"),
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
            let outcome = calculate(source, &CalculationRequest::default(), &mut context).unwrap();
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
            assert_eq!(certified_enclosure, enclosure);
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
        for source in ["log(0)", "log(-1)", "log(sqrt(2)-sqrt(3))"] {
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
            "asin(exp(log(2)))",
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
            "exp(log(0))",
            "exp(log(-1))",
            "exp(log(-sqrt(2)))",
            "exp(log(sqrt(2)-sqrt(2)))",
            "exp(log(sqrt(2)-sqrt(3)))",
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
