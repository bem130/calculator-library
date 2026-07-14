extern crate std;

use alloc::{format, string::String, string::ToString};

use crate::{
    calculate, parse, CalculationOutcome, CalculationRequest, CalculatorError,
    ComputationLimitKind, EnclosureOutputRequest, EvaluationContext, ExactFormatPreference,
    ExactOutput, ExactOutputRequest, IncompleteReason, InputLimitErrorKind, ParseError,
    ParseSettings, ResourceLimitRequest, ResourceLimits, ScientificOutputRequest,
};

#[test]
fn source_resource_limits_are_enforced_before_evaluation() {
    let mut limits = ResourceLimits {
        max_input_bytes: 3,
        ..ResourceLimits::default()
    };
    assert_input_limit("1 + 2", limits.clone(), InputLimitErrorKind::InputTooLong);

    limits = ResourceLimits {
        max_source_depth: 4,
        ..ResourceLimits::default()
    };
    assert_input_limit(
        "sqrt(sqrt(sqrt(sqrt(sqrt(1)))))",
        limits.clone(),
        InputLimitErrorKind::SourceAstTooDeep,
    );

    limits = ResourceLimits {
        max_source_ast_nodes: 4,
        ..ResourceLimits::default()
    };
    assert_input_limit(
        "1 + 2 + 3",
        limits.clone(),
        InputLimitErrorKind::SourceAstTooLarge,
    );

    limits = ResourceLimits {
        max_source_depth: 2,
        ..ResourceLimits::default()
    };
    assert_input_limit(
        "1 + (2 + (3 + 4))",
        limits,
        InputLimitErrorKind::SourceAstTooDeep,
    );

    limits = ResourceLimits {
        max_expression_nodes: 2,
        ..ResourceLimits::default()
    };
    assert_input_limit(
        "sin(1) + sin(2)",
        limits,
        InputLimitErrorKind::ExpressionTooLarge,
    );

    let request = CalculationRequest {
        limits: ResourceLimitRequest::Custom(ResourceLimits {
            max_expression_nodes: 1,
            ..ResourceLimits::default()
        }),
        ..CalculationRequest::default()
    };
    assert!(calculate("1 + 2", &request, &mut EvaluationContext::default()).is_ok());
    assert!(calculate("2 * 3", &request, &mut EvaluationContext::default()).is_ok());
    assert_input_limit(
        "sin(1) * cos(1)",
        ResourceLimits {
            max_expression_nodes: 1,
            ..ResourceLimits::default()
        },
        InputLimitErrorKind::ExpressionTooLarge,
    );
}

#[test]
fn additive_literal_fold_charges_structural_logical_work() {
    let source = (1_u32..=256)
        .map(|value| value.to_string())
        .collect::<alloc::vec::Vec<_>>()
        .join("+");
    let default = calculate(
        &source,
        &CalculationRequest::default(),
        &mut EvaluationContext::default(),
    )
    .unwrap();
    let request = |units| CalculationRequest {
        limits: ResourceLimitRequest::Custom(ResourceLimits {
            max_logical_work_units: units,
            ..ResourceLimits::default()
        }),
        ..CalculationRequest::default()
    };
    assert_eq!(
        calculate(&source, &request(261), &mut EvaluationContext::default()).unwrap(),
        default
    );
    let limited = calculate(&source, &request(260), &mut EvaluationContext::default()).unwrap();
    assert!(matches!(
        &limited,
        CalculationOutcome::Partial {
            reason: IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits
            },
            ..
        }
    ));
    assert_eq!(exact_plain_text(&limited), "32896");
}

#[test]
fn additive_decimal_fold_charges_normalization_work() {
    let source = "12345678901234567890.1 + 98765432109876543210.2";
    let default = calculate(
        source,
        &CalculationRequest::default(),
        &mut EvaluationContext::default(),
    )
    .unwrap();
    let calculate_with_work = |units| {
        calculate(
            source,
            &CalculationRequest {
                limits: ResourceLimitRequest::Custom(ResourceLimits {
                    max_logical_work_units: units,
                    ..ResourceLimits::default()
                }),
                ..CalculationRequest::default()
            },
            &mut EvaluationContext::default(),
        )
        .unwrap()
    };
    let mut low = 0;
    let mut high = ResourceLimits::default().max_logical_work_units;
    while low < high {
        let middle = low + (high - low) / 2;
        if calculate_with_work(middle) == default {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(
        low > 10,
        "large decimal normalization must have structural cost"
    );
    assert_eq!(calculate_with_work(low), default);
    let limited = calculate_with_work(low - 1);
    assert!(matches!(
        &limited,
        CalculationOutcome::Partial {
            reason: IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits
            },
            ..
        }
    ));
    assert_eq!(exact_plain_text(&limited), exact_plain_text(&default));
}

#[test]
fn multiplicative_literal_fold_charges_structural_logical_work() {
    let source = (1_u32..=128)
        .map(|value| value.to_string())
        .collect::<alloc::vec::Vec<_>>()
        .join("*");
    let default = calculate(
        &source,
        &CalculationRequest::default(),
        &mut EvaluationContext::default(),
    )
    .unwrap();
    let request = |units| CalculationRequest {
        limits: ResourceLimitRequest::Custom(ResourceLimits {
            max_logical_work_units: units,
            ..ResourceLimits::default()
        }),
        ..CalculationRequest::default()
    };
    assert_eq!(
        calculate(&source, &request(6_377), &mut EvaluationContext::default()).unwrap(),
        default
    );
    let limited = calculate(&source, &request(6_376), &mut EvaluationContext::default()).unwrap();
    assert!(matches!(
        &limited,
        CalculationOutcome::Partial {
            reason: IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits
            },
            ..
        }
    ));
    assert_eq!(exact_plain_text(&limited), exact_plain_text(&default));
}

#[test]
fn multiplicative_decimal_fold_charges_normalization_work() {
    let source = "12345678901234567890.1 * 98765432109876543210.2";
    let default = calculate(
        source,
        &CalculationRequest::default(),
        &mut EvaluationContext::default(),
    )
    .unwrap();
    let calculate_with_work = |units| {
        calculate(
            source,
            &CalculationRequest {
                limits: ResourceLimitRequest::Custom(ResourceLimits {
                    max_logical_work_units: units,
                    ..ResourceLimits::default()
                }),
                ..CalculationRequest::default()
            },
            &mut EvaluationContext::default(),
        )
        .unwrap()
    };
    let mut low = 0;
    let mut high = ResourceLimits::default().max_logical_work_units;
    while low < high {
        let middle = low + (high - low) / 2;
        if calculate_with_work(middle) == default {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 10);
    assert_eq!(calculate_with_work(low), default);
    let limited = calculate_with_work(low - 1);
    assert!(matches!(
        &limited,
        CalculationOutcome::Partial {
            reason: IncompleteReason::ComputationLimit {
                kind: ComputationLimitKind::LogicalWorkUnits
            },
            ..
        }
    ));
    assert_eq!(exact_plain_text(&limited), exact_plain_text(&default));
}

fn exact_plain_text(outcome: &CalculationOutcome) -> &str {
    let calculation = match outcome {
        CalculationOutcome::Complete(calculation) => calculation,
        CalculationOutcome::Partial { calculation, .. } => calculation,
    };
    match &calculation.exact {
        ExactOutput::Included(exact) => &exact.plain_text,
        ExactOutput::Omitted => panic!("default request must include exact output"),
    }
}

#[test]
fn huge_decimal_exponent_is_reported_as_input_limit() {
    assert_input_limit(
        "1e999999999999999999999999",
        ResourceLimits::default(),
        InputLimitErrorKind::IntegerTooLarge,
    );
}

#[test]
fn warm_and_cold_contexts_return_identical_results() {
    for source in [
        "0.1 + 0.2",
        "sqrt(2)",
        "sin(pi/6)",
        "2^(1/3)+1",
        "tan(pi/2)",
    ] {
        let request = CalculationRequest::default();
        let mut cold_context = EvaluationContext::default();
        let cold = calculate(source, &request, &mut cold_context);

        let mut warm_context = EvaluationContext::default();
        let _ = calculate(source, &request, &mut warm_context);
        let warm = calculate(source, &request, &mut warm_context);

        assert_eq!(cold, warm, "warm/cold mismatch for {source}");
    }
}

#[test]
fn generated_parser_inputs_do_not_panic_and_keep_error_spans_on_boundaries() {
    let mut rng = Rng::new(0x243f_6a88_85a3_08d3);
    for _ in 0..256 {
        let source = parser_source(&mut rng, 0);
        if let Err(error) = parse(&source, &ParseSettings::default()) {
            assert_parse_error_span(&source, &error);
        }
    }
}

#[test]
fn generated_calculation_inputs_return_typed_results() {
    let mut rng = Rng::new(0x1319_8a2e_0370_7344);
    let request = CalculationRequest {
        exact_output: ExactOutputRequest::Include {
            format: ExactFormatPreference::Auto,
        },
        scientific_output: ScientificOutputRequest::Omit,
        enclosure_output: EnclosureOutputRequest::Omit,
        limits: ResourceLimitRequest::Custom(ResourceLimits {
            max_input_bytes: 256,
            max_source_ast_nodes: 256,
            max_source_depth: 64,
            max_expression_nodes: 512,
            ..ResourceLimits::default()
        }),
        ..CalculationRequest::default()
    };

    for _ in 0..96 {
        let source = calculation_source(&mut rng, 0);
        let mut context = EvaluationContext::default();
        match calculate(&source, &request, &mut context) {
            Ok(_) => {}
            Err(CalculatorError::Parse(error)) => assert_parse_error_span(&source, &error),
            Err(CalculatorError::InternalInvariant(error)) => {
                panic!(
                    "input reached internal invariant for {source:?}: {:?}",
                    error.code
                );
            }
            Err(_) => {}
        }
    }
}

fn assert_input_limit(source: &str, limits: ResourceLimits, expected: InputLimitErrorKind) {
    let request = CalculationRequest {
        scientific_output: ScientificOutputRequest::Omit,
        enclosure_output: EnclosureOutputRequest::Omit,
        limits: ResourceLimitRequest::Custom(limits),
        ..CalculationRequest::default()
    };
    let mut context = EvaluationContext::default();
    match calculate(source, &request, &mut context) {
        Err(CalculatorError::InputLimit(error)) => assert_eq!(error.kind, expected),
        other => panic!("expected {expected:?} for {source:?}, got {other:?}"),
    }
}

fn assert_parse_error_span(source: &str, error: &ParseError) {
    let start = error.span.start as usize;
    let end = error.span.end as usize;
    assert!(start <= end, "invalid parse error span order: {error:?}");
    assert!(
        end <= source.len(),
        "parse error span escapes input: {error:?}"
    );
    assert!(
        source.is_char_boundary(start),
        "parse error span starts inside utf-8 scalar: {error:?} in {source:?}",
    );
    assert!(
        source.is_char_boundary(end),
        "parse error span ends inside utf-8 scalar: {error:?} in {source:?}",
    );
}

fn parser_source(rng: &mut Rng, depth: u8) -> String {
    if depth >= 4 {
        return parser_atom(rng);
    }

    match rng.below(13) {
        0 => parser_atom(rng),
        1 => format!(
            "{}{}+{}{}",
            parser_source(rng, depth + 1),
            whitespace(rng),
            whitespace(rng),
            parser_source(rng, depth + 1)
        ),
        2 => format!(
            "{}{}*{}{}",
            parser_source(rng, depth + 1),
            whitespace(rng),
            whitespace(rng),
            parser_source(rng, depth + 1)
        ),
        3 => format!("({})", parser_source(rng, depth + 1)),
        4 => format!("-{}", parser_source(rng, depth + 1)),
        5 => format!(
            "{}^{}",
            parser_source(rng, depth + 1),
            parser_source(rng, depth + 1)
        ),
        6 => format!("sqrt({})", parser_source(rng, depth + 1)),
        7 => format!(
            "{}{}",
            parser_source(rng, depth + 1),
            parser_source(rng, depth + 1)
        ),
        8 => format!("({}", parser_source(rng, depth + 1)),
        9 => format!("{})", parser_source(rng, depth + 1)),
        10 => String::from("1e999999999999999999999999"),
        11 => String::from("unknown"),
        _ => String::new(),
    }
}

fn parser_atom(rng: &mut Rng) -> String {
    let atoms = [
        "0", "1", "2", "3.25", "1e3", "pi", "\u{03c0}", ".", "1e+", "\u{221a}",
    ];
    String::from(atoms[rng.below(atoms.len())])
}

fn calculation_source(rng: &mut Rng, depth: u8) -> String {
    if depth >= 3 {
        return calculation_atom(rng);
    }

    match rng.below(9) {
        0 => calculation_atom(rng),
        1 => binary_calculation(rng, depth, "+"),
        2 => binary_calculation(rng, depth, "-"),
        3 => binary_calculation(rng, depth, "*"),
        4 => binary_calculation(rng, depth, "/"),
        5 => format!("-{}", calculation_source(rng, depth + 1)),
        6 => format!("({})", calculation_source(rng, depth + 1)),
        7 => format!("sqrt({})", calculation_source(rng, depth + 1)),
        _ => format!("sin({})", calculation_source(rng, depth + 1)),
    }
}

fn binary_calculation(rng: &mut Rng, depth: u8, operator: &str) -> String {
    format!(
        "{}{}{}{}{}",
        calculation_source(rng, depth + 1),
        whitespace(rng),
        operator,
        whitespace(rng),
        calculation_source(rng, depth + 1)
    )
}

fn calculation_atom(rng: &mut Rng) -> String {
    let atoms = ["0", "1", "2", "3", "4", "5", "0.25", "1.5", "pi"];
    String::from(atoms[rng.below(atoms.len())])
}

fn whitespace(rng: &mut Rng) -> &'static str {
    let values = ["", " ", "\t", "\n", "\u{00a0}"];
    values[rng.below(values.len())]
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn below(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next() as usize) % upper
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}
