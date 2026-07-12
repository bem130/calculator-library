use calculator_core::{
    calculate, evaluate, parse, present, reduce_input, CalculationOutcome, CalculationRequest,
    CertifiedEnclosureState, EvaluationContext, EvaluationRequest, ExactOutput, InputAction,
    InputPolicy, InputState, Integer, PresentationRequest, Rational, RecognizedExact,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use std::time::Duration;

const CASES: &[(&str, &str)] = &[
    (
        "exact_rational",
        "12345678901234567890/7 + 98765432109876543210/11",
    ),
    ("exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)"),
    ("approximate", "sin(1)+ln(2)+2^sqrt(2)"),
    ("algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)"),
];

#[derive(Clone, Copy)]
enum ExpectedExact {
    GeneralSymbolic,
    Radical,
}

const APPROXIMATE_COMPONENTS: &[(&str, &str, ExpectedExact)] = &[
    ("euler", "e", ExpectedExact::GeneralSymbolic),
    ("exp_one", "exp(1)", ExpectedExact::GeneralSymbolic),
    ("exp_two", "exp(2)", ExpectedExact::GeneralSymbolic),
    (
        "exp_negative_two",
        "exp(-2)",
        ExpectedExact::GeneralSymbolic,
    ),
    (
        "exp_negative_10000",
        "exp(-10000)",
        ExpectedExact::GeneralSymbolic,
    ),
    (
        "exp_positive_10000",
        "exp(10000)",
        ExpectedExact::GeneralSymbolic,
    ),
    ("log_two", "ln(2)", ExpectedExact::GeneralSymbolic),
    (
        "log_non_degenerate",
        "ln(2+sin(1))",
        ExpectedExact::GeneralSymbolic,
    ),
    ("general_power", "2^sqrt(2)", ExpectedExact::GeneralSymbolic),
    ("sin_one", "sin(1)", ExpectedExact::GeneralSymbolic),
    ("cos_one", "cos(1)", ExpectedExact::GeneralSymbolic),
    ("tan_one", "tan(1)", ExpectedExact::GeneralSymbolic),
    ("sin_two", "sin(2)", ExpectedExact::GeneralSymbolic),
    ("cos_two", "cos(2)", ExpectedExact::GeneralSymbolic),
    ("tan_two", "tan(2)", ExpectedExact::GeneralSymbolic),
    ("atan_two", "atan(2)", ExpectedExact::GeneralSymbolic),
    (
        "atan_non_degenerate",
        "atan(2+sin(1))",
        ExpectedExact::GeneralSymbolic,
    ),
    ("atan_half", "atan(1/2)", ExpectedExact::GeneralSymbolic),
    ("asin_third", "asin(1/3)", ExpectedExact::GeneralSymbolic),
    (
        "asin_non_degenerate_unit",
        "asin(sin(1)/3)",
        ExpectedExact::GeneralSymbolic,
    ),
    (
        "asin_non_degenerate_transform",
        "asin((2+sin(1))/3)",
        ExpectedExact::GeneralSymbolic,
    ),
    ("acos_third", "acos(1/3)", ExpectedExact::GeneralSymbolic),
    ("acos_two_thirds", "acos(2/3)", ExpectedExact::GeneralSymbolic),
    (
        "acos_non_degenerate_transform",
        "acos((2+sin(1))/3)",
        ExpectedExact::GeneralSymbolic,
    ),
    ("sqrt_two", "sqrt(2)", ExpectedExact::Radical),
    (
        "power_log_product",
        "sqrt(2)*ln(2)",
        ExpectedExact::GeneralSymbolic,
    ),
    (
        "exp_power_log_product",
        "exp(sqrt(2)*ln(2))",
        ExpectedExact::GeneralSymbolic,
    ),
];

fn calculate_representative_paths(criterion: &mut Criterion) {
    let request = CalculationRequest::default();
    let mut group = criterion.benchmark_group("calculate");
    for &(name, source) in CASES {
        validate_calculation(name, source, &request);
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &source,
            |bencher, source| {
                bencher.iter(|| {
                    let mut context = EvaluationContext::default();
                    black_box(
                        calculate(black_box(source), black_box(&request), &mut context)
                            .expect("representative calculation must not fail"),
                    )
                });
            },
        );
    }
    group.finish();
}

fn calculate_wide_expression(criterion: &mut Criterion) {
    let source = (1..=256)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("+");
    let request = CalculationRequest::default();
    let outcome = calculate(&source, &request, &mut EvaluationContext::default())
        .expect("wide expression preflight must succeed");
    assert_eq!(exact_plain_text(&outcome), "32896");
    let mut group = criterion.benchmark_group("large_expression");
    group.throughput(Throughput::Elements(256));
    group.bench_function("wide_add_256", |bencher| {
        bencher.iter(|| {
            let mut context = EvaluationContext::default();
            black_box(
                calculate(black_box(&source), black_box(&request), &mut context)
                    .expect("wide expression must not fail"),
            )
        });
    });
    group.finish();
}

fn validate_calculation(name: &str, source: &str, request: &CalculationRequest) {
    let outcome = calculate(source, request, &mut EvaluationContext::default())
        .expect("representative calculation preflight must succeed");
    let exact = exact_plain_text(&outcome);
    assert!(!exact.is_empty(), "{name} must retain exact output");
    if name == "exact_symbolic" {
        assert_eq!(exact, "sin(1)*cos(1)");
    }
}

fn exact_plain_text(outcome: &CalculationOutcome) -> &str {
    let calculation = match outcome {
        CalculationOutcome::Complete(calculation) => calculation,
        CalculationOutcome::Partial { calculation, .. } => calculation,
    };
    match &calculation.exact {
        ExactOutput::Included(exact) => &exact.plain_text,
        ExactOutput::Omitted => panic!("benchmark request must include exact output"),
    }
}

fn reduce_session_input(criterion: &mut Criterion) {
    let actions = [
        InputAction::Digit(1),
        InputAction::Digit(2),
        InputAction::Digit(3),
        InputAction::DecimalPoint,
        InputAction::Digit(4),
        InputAction::Digit(5),
        InputAction::Percent,
        InputAction::Evaluate,
    ];
    let policy = InputPolicy::default();
    let mut group = criterion.benchmark_group("session");
    group.throughput(Throughput::Elements(actions.len() as u64));
    group.bench_function("dispatch_sequence", |bencher| {
        bencher.iter(|| {
            let mut state = InputState::empty();
            for action in actions.iter().cloned() {
                state = reduce_input(black_box(&state), black_box(action), black_box(&policy))
                    .expect("representative actions are valid")
                    .state;
            }
            black_box(state)
        });
    });
    group.finish();
}

fn profile_approximate_stages(criterion: &mut Criterion) {
    let source = "sin(1)+ln(2)+2^sqrt(2)";
    let request = CalculationRequest::default();
    let parsed = parse(source, &request.parse).expect("approximate stage parse must succeed");
    let evaluation_request = EvaluationRequest {
        semantics: request.semantics,
        limits: request.limits.clone(),
    };
    let evaluation = evaluate(
        &parsed,
        &evaluation_request,
        &mut EvaluationContext::default(),
    )
    .expect("approximate stage evaluation must succeed");
    let presentation_request = PresentationRequest {
        exact_output: request.exact_output,
        scientific_output: request.scientific_output,
        enclosure_output: request.enclosure_output,
        limits: request.limits.clone(),
    };
    let mut group = criterion.benchmark_group("approximate_stages");
    group.bench_function("parse", |bencher| {
        bencher.iter(|| black_box(parse(black_box(source), black_box(&request.parse)).unwrap()));
    });
    group.bench_function("evaluate", |bencher| {
        bencher.iter(|| {
            black_box(
                evaluate(
                    black_box(&parsed),
                    black_box(&evaluation_request),
                    &mut EvaluationContext::default(),
                )
                .unwrap(),
            )
        });
    });
    group.bench_function("present", |bencher| {
        bencher.iter(|| {
            black_box(present(black_box(&evaluation), black_box(&presentation_request)).unwrap())
        });
    });
    group.finish();
}

fn profile_approximate_components(criterion: &mut Criterion) {
    let request = CalculationRequest::default();
    let evaluation_request = EvaluationRequest {
        semantics: request.semantics,
        limits: request.limits.clone(),
    };
    let parsed = APPROXIMATE_COMPONENTS
        .iter()
        .map(|&(name, source, expected_exact)| {
            let parsed = parse(source, &request.parse).expect("component parse must succeed");
            let evaluation = evaluate(
                &parsed,
                &evaluation_request,
                &mut EvaluationContext::default(),
            )
            .expect("component evaluation preflight must succeed");
            assert!(match expected_exact {
                ExpectedExact::GeneralSymbolic => matches!(
                    evaluation.value.recognized_exact,
                    RecognizedExact::GeneralSymbolic
                ),
                ExpectedExact::Radical => match &evaluation.value.recognized_exact {
                    RecognizedExact::Radical(value) => {
                        value.coefficient == Rational::one()
                            && value.radicand.inner == Integer::from(2)
                    }
                    RecognizedExact::Rational(_)
                    | RecognizedExact::RadicalLinearCombination(_)
                    | RecognizedExact::RealAlgebraic(_)
                    | RecognizedExact::RationalPiMultiple(_)
                    | RecognizedExact::GeneralSymbolic => false,
                },
            });
            assert!(matches!(
                evaluation.value.certified_enclosure,
                CertifiedEnclosureState::Available(_)
            ));
            (name, parsed)
        })
        .collect::<Vec<_>>();
    let mut group = criterion.benchmark_group("approximate_components");
    for (name, parsed) in &parsed {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            parsed,
            |bencher, parsed| {
                bencher.iter(|| {
                    black_box(
                        evaluate(
                            black_box(parsed),
                            black_box(&evaluation_request),
                            &mut EvaluationContext::default(),
                        )
                        .unwrap(),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = representative_paths;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(1));
    targets = calculate_representative_paths, calculate_wide_expression, reduce_session_input, profile_approximate_stages, profile_approximate_components
}
criterion_main!(representative_paths);
