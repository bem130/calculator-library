use calculator_core::{
    calculate, reduce_input, CalculationRequest, EvaluationContext, InputAction, InputPolicy,
    InputState,
};
use std::{env, hint::black_box};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    let case = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("approximate"));
    let iterations = env::var("CALCULATOR_ALLOCATION_ITERATIONS")
        .ok()
        .map(|value| value.parse::<u32>().expect("iterations must be a u32"))
        .unwrap_or(10);
    assert!(iterations > 0, "iterations must be positive");
    let request = CalculationRequest::default();
    let policy = InputPolicy::default();
    let source = match case.as_str() {
        "exact_rational" => Some(String::from(
            "12345678901234567890/7 + 98765432109876543210/11",
        )),
        "exact_symbolic" => Some(String::from("(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)")),
        "approximate" => Some(String::from("sin(1)+ln(2)+2^sqrt(2)")),
        "algebraic" => Some(String::from("((2^(1/3)-2^(1/3))+2)^(1/3)")),
        "wide_add_256" => Some(wide_add_source()),
        "session_dispatch_sequence" => None,
        _ => panic!("unknown allocation case: {case}"),
    };
    let _profiler = dhat::Profiler::new_heap();
    for _ in 0..iterations {
        if let Some(source) = &source {
            calculate_case(source, &request);
        } else {
            session_case(&policy);
        }
    }
}

fn calculate_case(source: &str, request: &CalculationRequest) {
    black_box(
        calculate(
            black_box(source),
            black_box(request),
            &mut EvaluationContext::default(),
        )
        .expect("allocation baseline calculation must not fail"),
    );
}

fn session_case(policy: &InputPolicy) {
    let mut state = InputState::empty();
    for action in [
        InputAction::Digit(1),
        InputAction::Digit(2),
        InputAction::Digit(3),
        InputAction::DecimalPoint,
        InputAction::Digit(4),
        InputAction::Digit(5),
        InputAction::Percent,
        InputAction::Evaluate,
    ] {
        state = reduce_input(&state, action, policy)
            .expect("allocation session action must succeed")
            .state;
    }
    black_box(state);
}

fn wide_add_source() -> String {
    (1..=256)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("+")
}
