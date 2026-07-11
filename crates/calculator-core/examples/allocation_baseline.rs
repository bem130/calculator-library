use calculator_core::{calculate, CalculationRequest, EvaluationContext};
use std::{env, hint::black_box};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    let source = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("sin(1)+ln(2)+2^sqrt(2)"));
    let iterations = env::var("CALCULATOR_ALLOCATION_ITERATIONS")
        .ok()
        .map(|value| value.parse::<u32>().expect("iterations must be a u32"))
        .unwrap_or(10);
    assert!(iterations > 0, "iterations must be positive");
    let _profiler = dhat::Profiler::new_heap();
    let request = CalculationRequest::default();
    for _ in 0..iterations {
        let mut context = EvaluationContext::default();
        black_box(
            calculate(black_box(&source), black_box(&request), &mut context)
                .expect("allocation baseline calculation must not fail"),
        );
    }
}
