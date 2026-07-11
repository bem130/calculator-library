use calculator_core::{
    calculate, CalculationRequest, EvaluationContext, ResourceLimitRequest, ResourceLimits,
};

const CASES: &[(&str, &str)] = &[
    (
        "exact_rational",
        "12345678901234567890/7 + 98765432109876543210/11",
    ),
    ("exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)"),
    ("approximate", "sin(1)+ln(2)+2^sqrt(2)"),
    ("algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)"),
];

fn main() {
    println!("{{\"schemaVersion\":1,\"cases\":[");
    for (index, &(name, source)) in CASES.iter().enumerate() {
        let units = minimum_equivalent_logical_work(source);
        let separator = if index + 1 == CASES.len() { "" } else { "," };
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
        let mut limits = default_limits.clone();
        limits.max_logical_work_units = middle;
        if calculate_with_limits(source, limits) == reference {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
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
