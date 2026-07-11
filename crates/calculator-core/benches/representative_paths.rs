use calculator_core::{
    calculate, reduce_input, CalculationRequest, EvaluationContext, InputAction, InputPolicy,
    InputState,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

const CASES: &[(&str, &str)] = &[
    (
        "exact_rational",
        "12345678901234567890/7 + 98765432109876543210/11",
    ),
    ("exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)"),
    ("approximate", "sin(1)+ln(2)+2^sqrt(2)"),
    ("algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)"),
];

fn calculate_representative_paths(criterion: &mut Criterion) {
    let request = CalculationRequest::default();
    let mut group = criterion.benchmark_group("calculate");
    for &(name, source) in CASES {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &source,
            |bencher, source| {
                bencher.iter(|| {
                    let mut context = EvaluationContext::default();
                    black_box(calculate(
                        black_box(source),
                        black_box(&request),
                        &mut context,
                    ))
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
    let mut group = criterion.benchmark_group("large_expression");
    group.throughput(Throughput::Elements(256));
    group.bench_function("wide_add_256", |bencher| {
        bencher.iter(|| {
            let mut context = EvaluationContext::default();
            black_box(calculate(
                black_box(&source),
                black_box(&request),
                &mut context,
            ))
        });
    });
    group.finish();
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

criterion_group!(
    representative_paths,
    calculate_representative_paths,
    calculate_wide_expression,
    reduce_session_input
);
criterion_main!(representative_paths);
