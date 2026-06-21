use alloc::string::String;

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

    Ok(ParsedExpression {
        source: String::from(source),
        settings: *settings,
    })
}

pub fn evaluate(
    _expression: &ParsedExpression,
    _request: &EvaluationRequest,
    _context: &mut EvaluationContext,
) -> Result<EvaluationOutcome, EvaluationError> {
    Err(EvaluationError::UnsupportedFeature(
        UnsupportedFeatureError {
            feature: UnsupportedFeature::EvaluationEngine,
        },
    ))
}

pub fn present(
    _evaluation: &EvaluationOutcome,
    _request: &PresentationRequest,
) -> Result<Calculation, PresentationError> {
    Err(PresentationError::InternalInvariant(
        InternalInvariantError {
            code: InternalInvariantCode::PresentationWithoutEvaluation,
        },
    ))
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
