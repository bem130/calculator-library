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
        "approximate_euler" => Some(String::from("e")),
        "approximate_exp_one" => Some(String::from("exp(1)")),
        "approximate_exp_two" => Some(String::from("exp(2)")),
        "approximate_exp_negative_two" => Some(String::from("exp(-2)")),
        "approximate_exp_negative_10000" => Some(String::from("exp(-10000)")),
        "approximate_exp_positive_10000" => Some(String::from("exp(10000)")),
        "approximate_log_two" => Some(String::from("ln(2)")),
        "approximate_log_non_degenerate" => Some(String::from("ln(2+sin(1))")),
        "approximate_general_power" => Some(String::from("2^sqrt(2)")),
        "approximate_sin_one" => Some(String::from("sin(1)")),
        "approximate_cos_one" => Some(String::from("cos(1)")),
        "approximate_tan_one" => Some(String::from("tan(1)")),
        "approximate_sin_two" => Some(String::from("sin(2)")),
        "approximate_cos_two" => Some(String::from("cos(2)")),
        "approximate_tan_two" => Some(String::from("tan(2)")),
        "approximate_atan_two" => Some(String::from("atan(2)")),
        "approximate_atan_non_degenerate" => Some(String::from("atan(2+sin(1))")),
        "approximate_atan_half" => Some(String::from("atan(1/2)")),
        "approximate_asin_third" => Some(String::from("asin(1/3)")),
        "approximate_asin_non_degenerate_unit" => Some(String::from("asin(sin(1)/3)")),
        "approximate_asin_non_degenerate_transform" => Some(String::from("asin((2+sin(1))/3)")),
        "approximate_acos_third" => Some(String::from("acos(1/3)")),
        "approximate_acos_two_thirds" => Some(String::from("acos(2/3)")),
        "approximate_acos_non_degenerate_transform" => Some(String::from("acos((2+sin(1))/3)")),
        "approximate_sqrt_two" => Some(String::from("sqrt(2)")),
        "approximate_power_log_product" => Some(String::from("sqrt(2)*ln(2)")),
        "approximate_exp_power_log_product" => Some(String::from("exp(sqrt(2)*ln(2))")),
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
