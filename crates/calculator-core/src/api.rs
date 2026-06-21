use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
};

use crate::syntax::{SourceExpr, UnaryOperator};
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
    let rational = evaluate_rational(&expression.root)?;
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
        } => ScientificOutput::Unavailable(UnavailableScientificOutput {
            requested_significant_digits: significant_digits,
            confirmed_significant_digits: 0,
            rounding_mode,
            reason: IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::PrecisionBits,
            },
        }),
    };
    let enclosure = match request.enclosure_output {
        EnclosureOutputRequest::Omit => EnclosureOutput::Omitted,
        EnclosureOutputRequest::Include { .. } => EnclosureOutput::Omitted,
    };

    Ok(Calculation {
        exact,
        scientific,
        enclosure,
        metadata: CalculationMetadata {
            exact_representation: exact_representation_kind(rational),
            simplification_status: SimplificationStatus::FullySimplifiedWithinLimits,
            semantic_settings: evaluation.metadata.semantic_settings,
            methods: evaluation.metadata.methods.clone(),
            internal_precision_bits: evaluation.metadata.internal_precision_bits,
            refinement_rounds: evaluation.metadata.refinement_rounds,
            confirmed_significant_digits: 0,
            assurance: AssuranceLevel::Exact,
            protocol_version: ProtocolVersion::CURRENT,
        },
    })
}

fn evaluate_rational(expr: &SourceExpr) -> Result<Rational, EvaluationError> {
    match expr {
        SourceExpr::Number { literal, .. } => {
            Rational::from_decimal_literal(literal).map_err(|_| {
                EvaluationError::InternalInvariant(InternalInvariantError {
                    code: InternalInvariantCode::InvalidParsedNumberLiteral,
                })
            })
        }
        SourceExpr::Constant { .. } => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::ConstantEvaluation,
            },
        )),
        SourceExpr::Function { .. } => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::FunctionEvaluation,
            },
        )),
        SourceExpr::Unary { op, expr, .. } => {
            let value = evaluate_rational(expr)?;
            Ok(match op {
                UnaryOperator::Plus => value,
                UnaryOperator::Negate => value.negate(),
            })
        }
        SourceExpr::Binary {
            op, left, right, ..
        } => {
            let left = evaluate_rational(left)?;
            let right = evaluate_rational(right)?;
            match op {
                BinaryOperator::Add => Ok(left.add(&right)),
                BinaryOperator::Subtract => Ok(left.subtract(&right)),
                BinaryOperator::Multiply => Ok(left.multiply(&right)),
                BinaryOperator::Divide => left.divide(&right).map_err(arithmetic_error),
                BinaryOperator::Power => {
                    let exponent =
                        right
                            .as_i64_if_integer()
                            .ok_or(EvaluationError::UnsupportedFeature(
                                UnsupportedFeatureError {
                                    feature: UnsupportedFeature::NonIntegerPower,
                                },
                            ))?;
                    left.pow_i64(exponent).map_err(arithmetic_error)
                }
            }
        }
        SourceExpr::Percent { expr, .. } => Ok(evaluate_rational(expr)?.percent()),
    }
}

fn arithmetic_error(error: RationalArithmeticError) -> EvaluationError {
    match error {
        RationalArithmeticError::DivisionByZero => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::DivisionByZero,
            span: None,
        }),
        RationalArithmeticError::ZeroToNegativePower => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::ZeroToNegativePower,
            span: None,
        }),
        RationalArithmeticError::ExponentTooLarge => {
            EvaluationError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::LogicalWorkUnits,
            })
        }
    }
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
}
