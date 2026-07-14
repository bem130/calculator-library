use calculator_core::{
    calculate, CalculationRequest, EvaluationContext, ResourceLimitRequest, ResourceLimits,
};

fn main() {
    let wide = wide_add_source();
    let wide_product = wide_multiply_source();
    let cases = [
        (
            "exact_rational",
            "12345678901234567890/7 + 98765432109876543210/11",
        ),
        ("exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)"),
        ("exact_trig_identity", "sin(1)^2+cos(1)^2"),
        ("approximate", "sin(1)+ln(2)+2^sqrt(2)"),
        ("euler", "e"),
        ("exp_negative_10000", "exp(-10000)"),
        ("exp_positive_10000", "exp(10000)"),
        ("log_non_degenerate", "ln(2+sin(1))"),
        (
            "log_large_positive",
            "ln(340282366920938463463374607431768211457)",
        ),
        ("atan_half", "atan(1/2)"),
        ("atan_two", "atan(2)"),
        ("atan_non_degenerate", "atan(2+sin(1))"),
        ("asin_third", "asin(1/3)"),
        ("asin_non_degenerate_unit", "asin(sin(1)/3)"),
        ("asin_non_degenerate_transform", "asin((2+sin(1))/3)"),
        ("acos_third", "acos(1/3)"),
        ("acos_non_degenerate_transform", "acos((2+sin(1))/3)"),
        ("acos_three_fourths", "acos(3/4)"),
        ("acos_five_eighths", "acos(5/8)"),
        ("sin_one", "sin(1)"),
        ("cos_one", "cos(1)"),
        ("tan_one", "tan(1)"),
        ("sin_two", "sin(2)"),
        ("cos_two", "cos(2)"),
        ("tan_two", "tan(2)"),
        ("sin_periodic_non_degenerate", "sin(100+sin(1))"),
        ("tan_periodic_non_degenerate", "tan(100+sin(1)/100)"),
        ("algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)"),
        ("wide_add_256", wide.as_str()),
        ("wide_multiply_128", wide_product.as_str()),
    ];
    println!("{{\"schemaVersion\":1,\"cases\":[");
    for (index, &(name, source)) in cases.iter().enumerate() {
        let units = minimum_equivalent_logical_work(source);
        let separator = if index + 1 == cases.len() { "" } else { "," };
        println!("{{\"name\":\"{name}\",\"logicalWorkUnits\":{units}}}{separator}");
    }
    println!("]}}");
}

fn minimum_equivalent_logical_work(source: &str) -> u64 {
    let default_limits = ResourceLimits::default();
    let reference = calculate_with_limits(source, default_limits.clone());
    let mut low = 0_u64;
    let mut high = default_limits.max_logical_work_units;
    while low < high {
        let middle = low + (high - low) / 2;
        if calculate_with_units(source, &default_limits, middle) == reference {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert_eq!(
        calculate_with_units(source, &default_limits, low),
        reference
    );
    if low > 0 {
        assert_ne!(
            calculate_with_units(source, &default_limits, low - 1),
            reference
        );
    }
    low
}

fn calculate_with_units(
    source: &str,
    default_limits: &ResourceLimits,
    units: u64,
) -> Result<calculator_core::CalculationOutcome, calculator_core::CalculatorError> {
    let mut limits = default_limits.clone();
    limits.max_logical_work_units = units;
    calculate_with_limits(source, limits)
}

fn calculate_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<calculator_core::CalculationOutcome, calculator_core::CalculatorError> {
    let request = CalculationRequest {
        limits: ResourceLimitRequest::Custom(limits),
        ..CalculationRequest::default()
    };
    calculate(source, &request, &mut EvaluationContext::default())
}

fn wide_add_source() -> String {
    (1..=256)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("+")
}

fn wide_multiply_source() -> String {
    (1..=128)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("*")
}
