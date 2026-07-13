#![forbid(unsafe_code)]

use std::{env, process};

fn main() {
    match run(env::args().skip(1)) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<String, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.is_empty() {
        let version = calculator_core::ProtocolVersion::CURRENT;
        return Ok(format!(
            "calculator-cli {}.{}\nusage: calculator-cli <expression>",
            version.major, version.minor
        ));
    }

    let source = args.join(" ");
    let mut context = calculator_core::EvaluationContext::default();
    let request = exact_only_request();
    let outcome = calculator_core::calculate(&source, &request, &mut context)
        .map_err(format_calculator_error)?;
    let calculator_core::CalculationOutcome::Complete(calculation) = outcome else {
        return Err(String::from(
            "partial results are not displayed by the Phase 1 CLI",
        ));
    };
    let calculator_core::ExactOutput::Included(exact) = calculation.exact else {
        return Err(String::from("exact output was unexpectedly omitted"));
    };
    Ok(exact.plain_text)
}

fn exact_only_request() -> calculator_core::CalculationRequest {
    calculator_core::CalculationRequest {
        scientific_output: calculator_core::ScientificOutputRequest::Omit,
        enclosure_output: calculator_core::EnclosureOutputRequest::Omit,
        ..calculator_core::CalculationRequest::default()
    }
}

fn format_calculator_error(error: calculator_core::CalculatorError) -> String {
    match error {
        calculator_core::CalculatorError::Parse(error) => {
            format!("parse.{}", parse_error_code(error.kind))
        }
        calculator_core::CalculatorError::Domain(error) => {
            format!("domain.{}", domain_error_code(error.kind))
        }
        calculator_core::CalculatorError::InputLimit(error) => {
            format!("inputLimit.{:?}", error.kind)
        }
        calculator_core::CalculatorError::ComputationLimit(error) => {
            format!("computationLimit.{:?}", error.kind)
        }
        calculator_core::CalculatorError::UnsupportedFeature(error) => {
            format!("unsupportedFeature.{:?}", error.feature)
        }
        calculator_core::CalculatorError::InternalInvariant(error) => {
            format!("internalInvariant.{:?}", error.code)
        }
    }
}

fn parse_error_code(kind: calculator_core::ParseErrorKind) -> &'static str {
    match kind {
        calculator_core::ParseErrorKind::UnexpectedToken => "unexpectedToken",
        calculator_core::ParseErrorKind::UnexpectedEnd => "unexpectedEnd",
        calculator_core::ParseErrorKind::UnknownIdentifier => "unknownIdentifier",
        calculator_core::ParseErrorKind::InvalidNumberLiteral => "invalidNumberLiteral",
        calculator_core::ParseErrorKind::MissingFunctionParenthesis => "missingFunctionParenthesis",
        calculator_core::ParseErrorKind::ImplicitMultiplicationDisabled => {
            "implicitMultiplicationDisabled"
        }
        calculator_core::ParseErrorKind::PercentRejected => "percentRejected",
    }
}

fn domain_error_code(kind: calculator_core::DomainErrorKind) -> &'static str {
    match kind {
        calculator_core::DomainErrorKind::DivisionByZero => "divisionByZero",
        calculator_core::DomainErrorKind::LogarithmOfNonPositive => "logarithmOfNonPositive",
        calculator_core::DomainErrorKind::LogarithmBaseOne => "logarithmBaseOne",
        calculator_core::DomainErrorKind::EvenRootOfNegative => "evenRootOfNegative",
        calculator_core::DomainErrorKind::InverseTrigonometricOutOfRange => {
            "inverseTrigonometricOutOfRange"
        }
        calculator_core::DomainErrorKind::TangentPole => "tangentPole",
        calculator_core::DomainErrorKind::ZeroToNegativePower => "zeroToNegativePower",
        calculator_core::DomainErrorKind::IndeterminateZeroToZero => "indeterminateZeroToZero",
        calculator_core::DomainErrorKind::NonRealPower => "nonRealPower",
        calculator_core::DomainErrorKind::IntegerFunctionRequiresInteger => {
            "integerFunctionRequiresInteger"
        }
        calculator_core::DomainErrorKind::IntegerFunctionRequiresNonNegative => {
            "integerFunctionRequiresNonNegative"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_evaluates_exact_rational_expression() {
        assert_eq!(run(["0.1 + 0.2".to_owned()]).unwrap(), "3/10");
        assert_eq!(run(["1 / 3 + 1 / 6".to_owned()]).unwrap(), "1/2");
        assert_eq!(run(["exp(-10000)".to_owned()]).unwrap(), "exp(-10000)");
        assert_eq!(run(["e^(-10000)".to_owned()]).unwrap(), "exp(-10000)");
        assert_eq!(run(["sin(1)^2+cos(1)^2".to_owned()]).unwrap(), "1");
    }

    #[test]
    fn cli_reports_domain_error_code() {
        assert_eq!(
            run(["1 / 0".to_owned()]).unwrap_err(),
            "domain.divisionByZero"
        );
        assert_eq!(
            run(["gcd(3/2, 1)".to_owned()]).unwrap_err(),
            "domain.integerFunctionRequiresInteger"
        );
        assert_eq!(
            run(["fact(-1)".to_owned()]).unwrap_err(),
            "domain.integerFunctionRequiresNonNegative"
        );
    }
}
