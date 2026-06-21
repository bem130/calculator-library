extern crate std;

use alloc::{format, string::String, vec::Vec};

use serde::Deserialize;

use crate::{
    session::{apply_calculation_result, reduce_input},
    syntax::{parse_source, SourceExpr, UnaryOperator},
    types::*,
};

const FIXTURE: &str = include_str!("../fixtures/parser_session_conformance.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    default_parse_settings: ParseSettingsFixture,
    default_semantic_settings: SemanticSettingsFixture,
    source_cases: Vec<SourceCase>,
    parse_error_cases: Vec<ParseErrorCase>,
    session_cases: Vec<SessionCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseSettingsFixture {
    grammar: String,
    implicit_multiplication: String,
    unicode_aliases: String,
    percent: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SemanticSettingsFixture {
    domain: String,
    angle_unit: String,
    power_semantics: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceCase {
    id: String,
    source: String,
    expected_parse_tree: String,
    expected_exact: Option<String>,
    expected_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseErrorCase {
    id: String,
    source: String,
    expected_error: String,
    expected_span_utf8: SpanFixture,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpanFixture {
    start: u32,
    end: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionCase {
    id: String,
    percent_policy: String,
    actions: Vec<String>,
    expected_command_source: Option<String>,
    expected_exact: Option<String>,
    expected_error: Option<String>,
    apply_command_result: Option<bool>,
    expected_final_state: SessionStateFixture,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionStateFixture {
    source: String,
    cursor_utf8: u32,
    has_ans: bool,
    has_memory: bool,
}

#[test]
fn golden_source_cases_match_parser_lowering_and_evaluation_contract() {
    let fixture = load_fixture();
    let parse_settings = parse_settings(&fixture.default_parse_settings);
    let request = exact_only_request(
        parse_settings,
        semantic_settings(&fixture.default_semantic_settings),
    );

    for case in fixture.source_cases {
        let parsed = parse_source(&case.source, &parse_settings)
            .unwrap_or_else(|error| panic!("{}: parse failed: {error:?}", case.id));
        assert_eq!(
            source_shape(&parsed),
            case.expected_parse_tree,
            "{}: parse tree shape changed",
            case.id
        );

        let mut context = EvaluationContext::default();
        match crate::calculate(&case.source, &request, &mut context) {
            Ok(outcome) => {
                let expected = case
                    .expected_exact
                    .as_deref()
                    .unwrap_or_else(|| panic!("{}: fixture expected an error", case.id));
                assert_eq!(
                    exact_plain_text(outcome),
                    expected,
                    "{}: exact output",
                    case.id
                );
            }
            Err(error) => {
                let expected = case
                    .expected_error
                    .as_deref()
                    .unwrap_or_else(|| panic!("{}: unexpected error: {error:?}", case.id));
                assert_eq!(
                    calculator_error_code(&error),
                    expected,
                    "{}: error",
                    case.id
                );
            }
        }
    }
}

#[test]
fn golden_parse_error_cases_match_error_kind_and_utf8_span() {
    let fixture = load_fixture();
    let parse_settings = parse_settings(&fixture.default_parse_settings);

    for case in fixture.parse_error_cases {
        let error = parse_source(&case.source, &parse_settings)
            .expect_err(&format!("{}: parse should fail", case.id));
        assert_eq!(
            parse_error_code(error.kind),
            case.expected_error,
            "{}: parse error kind",
            case.id
        );
        assert_eq!(
            error.span,
            ByteSpan {
                start: case.expected_span_utf8.start,
                end: case.expected_span_utf8.end,
            },
            "{}: parse error UTF-8 span",
            case.id
        );
    }
}

#[test]
fn golden_session_cases_match_command_and_state_contract() {
    let fixture = load_fixture();
    let request = exact_only_request(
        parse_settings(&fixture.default_parse_settings),
        semantic_settings(&fixture.default_semantic_settings),
    );

    for case in fixture.session_cases {
        let policy = InputPolicy {
            calculation_request: request.clone(),
            percent_policy: percent_policy(&case.percent_policy),
        };
        let mut state = InputState::empty();
        let mut last_command: Option<(String, CalculationRequest)> = None;
        let mut observed_command_source: Option<String> = None;

        for action in &case.actions {
            if action == "applyCommandResult" {
                state = apply_last_command(&case, &mut last_command, &state);
                continue;
            }

            let reduction = reduce_input(&state, input_action(action), &policy)
                .unwrap_or_else(|error| panic!("{}: action {action} failed: {error:?}", case.id));
            state = reduction.state;
            if let SessionCommand::Calculate { source, request } = reduction.command {
                observed_command_source = Some(source.clone());
                last_command = Some((source, request));
            }
        }

        if case.apply_command_result.unwrap_or(false) {
            state = apply_last_command(&case, &mut last_command, &state);
        }

        if let Some(expected) = &case.expected_command_source {
            let Some(source) = &observed_command_source else {
                panic!("{}: expected calculate command", case.id);
            };
            assert_eq!(source, expected, "{}: command source", case.id);
        }

        assert_eq!(
            state.source(),
            case.expected_final_state.source,
            "{}: final source",
            case.id
        );
        assert_eq!(
            state.cursor_utf8(),
            case.expected_final_state.cursor_utf8,
            "{}: final cursor",
            case.id
        );
        assert_eq!(
            state.has_ans(),
            case.expected_final_state.has_ans,
            "{}: final Ans flag",
            case.id
        );
        assert_eq!(
            state.has_memory(),
            case.expected_final_state.has_memory,
            "{}: final memory flag",
            case.id
        );
    }
}

fn load_fixture() -> Fixture {
    serde_json::from_str(FIXTURE).expect("parser/session conformance fixture must be valid JSON")
}

fn parse_settings(value: &ParseSettingsFixture) -> ParseSettings {
    ParseSettings {
        grammar: match value.grammar.as_str() {
            "default" => GrammarProfile::Default,
            other => panic!("unsupported grammar fixture value: {other}"),
        },
        implicit_multiplication: match value.implicit_multiplication.as_str() {
            "enabled" => ImplicitMultiplicationPolicy::Enabled,
            "disabled" => ImplicitMultiplicationPolicy::Disabled,
            other => panic!("unsupported implicit multiplication fixture value: {other}"),
        },
        unicode_aliases: match value.unicode_aliases.as_str() {
            "mathematicalAliases" => UnicodeAliasPolicy::MathematicalAliases,
            "asciiOnly" => UnicodeAliasPolicy::AsciiOnly,
            other => panic!("unsupported unicode aliases fixture value: {other}"),
        },
        percent: match value.percent.as_str() {
            "postfixPercent" => PercentParsePolicy::PostfixPercent,
            "rejectPercent" => PercentParsePolicy::RejectPercent,
            other => panic!("unsupported percent fixture value: {other}"),
        },
    }
}

fn semantic_settings(value: &SemanticSettingsFixture) -> SemanticSettings {
    SemanticSettings {
        domain: match value.domain.as_str() {
            "real" => EvaluationDomain::Real,
            other => panic!("unsupported domain fixture value: {other}"),
        },
        angle_unit: match value.angle_unit.as_str() {
            "radian" => AngleUnit::Radian,
            "degree" => AngleUnit::Degree,
            "gradian" => AngleUnit::Gradian,
            other => panic!("unsupported angle unit fixture value: {other}"),
        },
        power_semantics: match value.power_semantics.as_str() {
            "realPrincipal" => PowerSemantics::RealPrincipal,
            other => panic!("unsupported power semantics fixture value: {other}"),
        },
    }
}

fn exact_only_request(parse: ParseSettings, semantics: SemanticSettings) -> CalculationRequest {
    CalculationRequest {
        parse,
        semantics,
        scientific_output: ScientificOutputRequest::Omit,
        enclosure_output: EnclosureOutputRequest::Omit,
        ..CalculationRequest::default()
    }
}

fn source_shape(expression: &SourceExpr) -> String {
    match expression {
        SourceExpr::Number { literal, .. } => format!("number({literal})"),
        SourceExpr::Constant { constant, .. } => format!("constant({})", constant_name(*constant)),
        SourceExpr::Unary { op, expr, .. } => {
            format!("{}({})", unary_name(*op), source_shape(expr))
        }
        SourceExpr::Binary {
            op,
            left,
            right,
            implicit,
            ..
        } => {
            let suffix = if *implicit { "[implicit]" } else { "" };
            format!(
                "{}{}({},{})",
                binary_name(*op),
                suffix,
                source_shape(left),
                source_shape(right)
            )
        }
        SourceExpr::Percent { expr, .. } => format!("percent({})", source_shape(expr)),
        SourceExpr::Function {
            function, argument, ..
        } => {
            format!("{}({})", function_name(*function), source_shape(argument))
        }
    }
}

fn exact_plain_text(outcome: CalculationOutcome) -> String {
    let CalculationOutcome::Complete(calculation) = outcome else {
        panic!("expected complete calculation");
    };
    let ExactOutput::Included(exact) = calculation.exact else {
        panic!("expected included exact output");
    };
    exact.plain_text
}

fn apply_last_command(
    case: &SessionCase,
    last_command: &mut Option<(String, CalculationRequest)>,
    state: &InputState,
) -> InputState {
    let Some((source, request)) = last_command.take() else {
        panic!("{}: no calculate command to apply", case.id);
    };
    let mut context = EvaluationContext::default();
    let result = crate::calculate(&source, &request, &mut context);
    match (&result, &case.expected_exact, &case.expected_error) {
        (Ok(outcome), Some(expected), _) => {
            assert_eq!(
                exact_plain_text(outcome.clone()),
                *expected,
                "{}: command exact output",
                case.id
            );
        }
        (Err(error), _, Some(expected)) => {
            assert_eq!(
                calculator_error_code(error),
                expected,
                "{}: command error",
                case.id
            );
        }
        (Ok(_), None, Some(expected)) => {
            panic!("{}: expected command error {expected}", case.id);
        }
        (Err(error), Some(_), _) => {
            panic!("{}: unexpected command error: {error:?}", case.id);
        }
        _ => {}
    }
    apply_calculation_result(state, result)
}

fn input_action(value: &str) -> InputAction {
    match value {
        "decimalPoint" => InputAction::DecimalPoint,
        "percent" => InputAction::Percent,
        "openParenthesis" => InputAction::OpenParenthesis,
        "closeParenthesis" => InputAction::CloseParenthesis,
        "deleteBackward" => InputAction::DeleteBackward,
        "clearEntry" => InputAction::ClearEntry,
        "clearAll" => InputAction::ClearAll,
        "memoryClear" => InputAction::MemoryClear,
        "memoryRecall" => InputAction::MemoryRecall,
        "memoryAdd" => InputAction::MemoryAdd,
        "memorySubtract" => InputAction::MemorySubtract,
        "evaluate" => InputAction::Evaluate,
        _ => prefixed_input_action(value),
    }
}

fn prefixed_input_action(value: &str) -> InputAction {
    let Some((kind, payload)) = value.split_once(':') else {
        panic!("unsupported input action fixture value: {value}");
    };
    match kind {
        "digit" => {
            let digit = payload
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("invalid digit fixture value: {value}"));
            InputAction::Digit(digit)
        }
        "constant" => InputAction::Constant(match payload {
            "pi" => Constant::Pi,
            "e" => Constant::Euler,
            other => panic!("unsupported constant fixture value: {other}"),
        }),
        "function" => InputAction::Function(match payload {
            "sin" => Function::Sin,
            "cos" => Function::Cos,
            "tan" => Function::Tan,
            "asin" => Function::Asin,
            "acos" => Function::Acos,
            "atan" => Function::Atan,
            "sqrt" => Function::Sqrt,
            "exp" => Function::Exp,
            "log" => Function::Log,
            other => panic!("unsupported function fixture value: {other}"),
        }),
        "binary" => InputAction::BinaryOperator(match payload {
            "add" => BinaryOperator::Add,
            "subtract" => BinaryOperator::Subtract,
            "multiply" => BinaryOperator::Multiply,
            "divide" => BinaryOperator::Divide,
            "power" => BinaryOperator::Power,
            other => panic!("unsupported binary operator fixture value: {other}"),
        }),
        other => panic!("unsupported action fixture kind: {other}"),
    }
}

fn percent_policy(value: &str) -> PercentPolicy {
    match value {
        "expressionPercent" => PercentPolicy::ExpressionPercent,
        "calculatorPercent" => PercentPolicy::CalculatorPercent,
        other => panic!("unsupported percent policy fixture value: {other}"),
    }
}

fn parse_error_code(value: ParseErrorKind) -> &'static str {
    match value {
        ParseErrorKind::UnexpectedToken => "unexpectedToken",
        ParseErrorKind::UnexpectedEnd => "unexpectedEnd",
        ParseErrorKind::UnknownIdentifier => "unknownIdentifier",
        ParseErrorKind::InvalidNumberLiteral => "invalidNumberLiteral",
        ParseErrorKind::MissingFunctionParenthesis => "missingFunctionParenthesis",
        ParseErrorKind::ImplicitMultiplicationDisabled => "implicitMultiplicationDisabled",
        ParseErrorKind::PercentRejected => "percentRejected",
    }
}

fn calculator_error_code(value: &CalculatorError) -> &'static str {
    match value {
        CalculatorError::Parse(error) => parse_error_code(error.kind),
        CalculatorError::Domain(error) => match error.kind {
            DomainErrorKind::DivisionByZero => "domain.divisionByZero",
            DomainErrorKind::LogarithmOfNonPositive => "domain.logarithmOfNonPositive",
            DomainErrorKind::EvenRootOfNegative => "domain.evenRootOfNegative",
            DomainErrorKind::InverseTrigonometricOutOfRange => {
                "domain.inverseTrigonometricOutOfRange"
            }
            DomainErrorKind::TangentPole => "domain.tangentPole",
            DomainErrorKind::ZeroToNegativePower => "domain.zeroToNegativePower",
            DomainErrorKind::IndeterminateZeroToZero => "domain.indeterminateZeroToZero",
            DomainErrorKind::NonRealPower => "domain.nonRealPower",
        },
        CalculatorError::InputLimit(error) => match error.kind {
            InputLimitErrorKind::InputTooLong => "inputLimit.inputTooLong",
            InputLimitErrorKind::SourceAstTooDeep => "inputLimit.sourceAstTooDeep",
            InputLimitErrorKind::SourceAstTooLarge => "inputLimit.sourceAstTooLarge",
            InputLimitErrorKind::ExpressionTooLarge => "inputLimit.expressionTooLarge",
            InputLimitErrorKind::IntegerTooLarge => "inputLimit.integerTooLarge",
            InputLimitErrorKind::OutputTooLarge => "inputLimit.outputTooLarge",
            InputLimitErrorKind::InvalidSignificantDigits => "inputLimit.invalidSignificantDigits",
            InputLimitErrorKind::InvalidResourceLimit => "inputLimit.invalidResourceLimit",
        },
        CalculatorError::ComputationLimit(error) => match error.kind {
            ComputationLimitKind::AlgebraicDegree => "computationLimit.algebraicDegree",
            ComputationLimitKind::PolynomialCoefficientBits => {
                "computationLimit.polynomialCoefficientBits"
            }
            ComputationLimitKind::ResultantDegree => "computationLimit.resultantDegree",
            ComputationLimitKind::FactorizationWork => "computationLimit.factorizationWork",
            ComputationLimitKind::RootIsolationSteps => "computationLimit.rootIsolationSteps",
            ComputationLimitKind::RewriteSteps => "computationLimit.rewriteSteps",
            ComputationLimitKind::PrecisionBits => "computationLimit.precisionBits",
            ComputationLimitKind::RefinementRounds => "computationLimit.refinementRounds",
            ComputationLimitKind::LogicalWorkUnits => "computationLimit.logicalWorkUnits",
            ComputationLimitKind::PresentationNodes => "computationLimit.presentationNodes",
        },
        CalculatorError::UnsupportedFeature(error) => match error.feature {
            UnsupportedFeature::ComplexDomain => "unsupportedFeature.complexDomain",
            UnsupportedFeature::PortableProofCertificate => {
                "unsupportedFeature.portableProofCertificate"
            }
            UnsupportedFeature::EvaluationEngine => "unsupportedFeature.evaluationEngine",
            UnsupportedFeature::ConstantEvaluation => "unsupportedFeature.constantEvaluation",
            UnsupportedFeature::FunctionEvaluation => "unsupportedFeature.functionEvaluation",
            UnsupportedFeature::NonIntegerPower => "unsupportedFeature.nonIntegerPower",
        },
        CalculatorError::InternalInvariant(error) => match error.code {
            InternalInvariantCode::NonCanonicalRational => "internalInvariant.nonCanonicalRational",
            InternalInvariantCode::InvalidAlgebraicIsolation => {
                "internalInvariant.invalidAlgebraicIsolation"
            }
            InternalInvariantCode::InvalidCertifiedInterval => {
                "internalInvariant.invalidCertifiedInterval"
            }
            InternalInvariantCode::NonDeterministicCacheAccounting => {
                "internalInvariant.nonDeterministicCacheAccounting"
            }
            InternalInvariantCode::PresentationWithoutEvaluation => {
                "internalInvariant.presentationWithoutEvaluation"
            }
            InternalInvariantCode::InvalidParsedNumberLiteral => {
                "internalInvariant.invalidParsedNumberLiteral"
            }
        },
    }
}

fn constant_name(value: Constant) -> &'static str {
    match value {
        Constant::Pi => "pi",
        Constant::Euler => "e",
    }
}

fn function_name(value: Function) -> &'static str {
    match value {
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

fn unary_name(value: UnaryOperator) -> &'static str {
    match value {
        UnaryOperator::Plus => "plus",
        UnaryOperator::Negate => "negate",
    }
}

fn binary_name(value: BinaryOperator) -> &'static str {
    match value {
        BinaryOperator::Add => "add",
        BinaryOperator::Subtract => "subtract",
        BinaryOperator::Multiply => "multiply",
        BinaryOperator::Divide => "divide",
        BinaryOperator::Power => "power",
    }
}
