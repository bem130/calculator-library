#![forbid(unsafe_code)]

mod convert;
pub mod dto;

#[cfg(target_arch = "wasm32")]
use js_sys::Reflect;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use dto::UnsupportedProtocolCodeDto;
use dto::{ApiResultDto, CalculationOutcomeDto, CalculationRequestDto};
use dto::{InputActionDto, InputPolicyDto, SessionDispatchResultDto, SessionStateDto};

pub fn protocol_version() -> (u16, u16) {
    let version = calculator_core::ProtocolVersion::CURRENT;
    (version.major, version.minor)
}

pub fn calculate_dto(
    source: &str,
    request: CalculationRequestDto,
) -> ApiResultDto<CalculationOutcomeDto> {
    let request = match calculator_core::CalculationRequest::try_from(request) {
        Ok(request) => request,
        Err(error) => return ApiResultDto::Error { error },
    };
    let mut context = calculator_core::EvaluationContext::default();
    match calculator_core::calculate(source, &request, &mut context) {
        Ok(value) => ApiResultDto::Ok {
            value: value.into(),
        },
        Err(error) => ApiResultDto::Error {
            error: error.into(),
        },
    }
}

pub struct SessionCore {
    state: calculator_core::InputState,
    policy: calculator_core::InputPolicy,
}

impl SessionCore {
    pub fn new(policy: InputPolicyDto) -> Result<Self, dto::CalculatorErrorDto> {
        Ok(Self {
            state: calculator_core::InputState::empty(),
            policy: policy.try_into()?,
        })
    }

    pub fn dispatch_dto(&mut self, action: InputActionDto) -> SessionDispatchResultDto {
        match calculator_core::reduce_input(&self.state, action.into(), &self.policy) {
            Ok(reduction) => {
                self.state = reduction.state.clone();
                reduction.into()
            }
            Err(error) => SessionDispatchResultDto::InputError {
                state: (&self.state).into(),
                error: error.into(),
            },
        }
    }

    pub fn apply_result_dto(
        &mut self,
        result: ApiResultDto<CalculationOutcomeDto>,
    ) -> SessionStateDto {
        let result = match result {
            ApiResultDto::Ok { value } => {
                match calculator_core::CalculationOutcome::try_from(value) {
                    Ok(value) => Ok(value),
                    Err(_) => return self.get_state_dto(),
                }
            }
            ApiResultDto::Error { error } => {
                match calculator_core::CalculatorError::try_from(error) {
                    Ok(error) => Err(error),
                    Err(_) => return self.get_state_dto(),
                }
            }
        };
        self.state = calculator_core::apply_calculation_result(&self.state, result);
        self.get_state_dto()
    }

    pub fn get_state_dto(&self) -> SessionStateDto {
        (&self.state).into()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = CalculatorSession)]
pub struct WasmCalculatorSession {
    inner: SessionCore,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = CalculatorSession)]
impl WasmCalculatorSession {
    #[wasm_bindgen(constructor)]
    pub fn new(policy: JsValue) -> Result<Self, JsValue> {
        let policy = serde_wasm_bindgen::from_value::<InputPolicyDto>(policy)
            .map_err(|_| JsValue::from_str("unsupportedProtocol.unknownTag"))?;
        Ok(Self {
            inner: SessionCore::new(policy)
                .map_err(|_| JsValue::from_str("unsupportedProtocol.unknownCode"))?,
        })
    }

    pub fn dispatch(&mut self, action: JsValue) -> Result<JsValue, JsValue> {
        let action = serde_wasm_bindgen::from_value::<InputActionDto>(action)
            .map_err(|_| JsValue::from_str("unsupportedProtocol.unknownTag"))?;
        serde_wasm_bindgen::to_value(&self.inner.dispatch_dto(action))
            .map_err(|_| JsValue::from_str("internalInvariant.dtoSerialization"))
    }

    #[wasm_bindgen(js_name = applyResult)]
    pub fn apply_result(&mut self, result: JsValue) -> Result<JsValue, JsValue> {
        let result = serde_wasm_bindgen::from_value::<ApiResultDto<CalculationOutcomeDto>>(result)
            .map_err(|_| JsValue::from_str("unsupportedProtocol.unknownTag"))?;
        serde_wasm_bindgen::to_value(&self.inner.apply_result_dto(result))
            .map_err(|_| JsValue::from_str("internalInvariant.dtoSerialization"))
    }

    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.get_state_dto())
            .map_err(|_| JsValue::from_str("internalInvariant.dtoSerialization"))
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn calculate(source: &str, request: JsValue) -> JsValue {
    let result = match deserialize_calculation_request_value(request) {
        Ok(request) => calculate_dto(source, request),
        Err(error) => ApiResultDto::<CalculationOutcomeDto>::Error { error },
    };
    serde_wasm_bindgen::to_value(&result).expect("calculator DTO serialization must succeed")
}

#[cfg(target_arch = "wasm32")]
fn deserialize_calculation_request_value(
    request: JsValue,
) -> Result<CalculationRequestDto, dto::CalculatorErrorDto> {
    validate_calculation_request_value(&request)?;
    serde_wasm_bindgen::from_value::<CalculationRequestDto>(request)
        .map_err(|_| convert::unsupported_protocol_error(UnsupportedProtocolCodeDto::UnknownTag))
}

#[cfg(target_arch = "wasm32")]
fn validate_calculation_request_value(request: &JsValue) -> Result<(), dto::CalculatorErrorDto> {
    if request.is_null() || request.is_undefined() {
        return Err(convert::unsupported_protocol_error(
            UnsupportedProtocolCodeDto::UnknownTag,
        ));
    }
    validate_parse_settings_value(request)?;
    validate_semantic_settings_value(request)?;
    validate_exact_output_request_value(request)?;
    validate_scientific_output_request_value(request)?;
    validate_enclosure_output_request_value(request)?;
    validate_resource_limit_request_value(request)
}

#[cfg(target_arch = "wasm32")]
fn validate_parse_settings_value(request: &JsValue) -> Result<(), dto::CalculatorErrorDto> {
    let parse = js_property(request, "parse")?;
    validate_required_code(&js_property(&parse, "grammar")?, &["default"])?;
    validate_required_code(
        &js_property(&parse, "implicitMultiplication")?,
        &["enabled", "disabled"],
    )?;
    validate_required_code(
        &js_property(&parse, "unicodeAliases")?,
        &["mathematicalAliases", "asciiOnly"],
    )?;
    validate_required_code(
        &js_property(&parse, "percent")?,
        &["postfixPercent", "rejectPercent"],
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_semantic_settings_value(request: &JsValue) -> Result<(), dto::CalculatorErrorDto> {
    let semantics = js_property(request, "semantics")?;
    validate_required_code(&js_property(&semantics, "domain")?, &["real"])?;
    validate_required_code(
        &js_property(&semantics, "angleUnit")?,
        &["radian", "degree", "gradian"],
    )?;
    validate_required_code(
        &js_property(&semantics, "powerSemantics")?,
        &["realPrincipal"],
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_exact_output_request_value(request: &JsValue) -> Result<(), dto::CalculatorErrorDto> {
    let exact_output = js_property(request, "exactOutput")?;
    let tag = validate_required_tag(&js_property(&exact_output, "tag")?, &["omit", "include"])?;
    if tag == "include" {
        validate_required_code(
            &js_property(&exact_output, "format")?,
            &[
                "auto",
                "rational",
                "finiteDecimal",
                "mixedFraction",
                "symbolic",
            ],
        )?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_scientific_output_request_value(
    request: &JsValue,
) -> Result<(), dto::CalculatorErrorDto> {
    let scientific_output = js_property(request, "scientificOutput")?;
    let tag = validate_required_tag(
        &js_property(&scientific_output, "tag")?,
        &["omit", "include"],
    )?;
    if tag != "include" {
        return Ok(());
    }

    let significant_digits = js_property(&scientific_output, "significantDigits")?;
    validate_required_u32_number(
        &significant_digits,
        1,
        dto::InputLimitErrorCodeDto::InvalidSignificantDigits,
    )?;
    validate_required_code(
        &js_property(&scientific_output, "roundingMode")?,
        &[
            "nearestTiesToEven",
            "nearestTiesAwayFromZero",
            "towardPositiveInfinity",
            "towardNegativeInfinity",
            "towardZero",
            "awayFromZero",
        ],
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_enclosure_output_request_value(
    request: &JsValue,
) -> Result<(), dto::CalculatorErrorDto> {
    let enclosure_output = js_property(request, "enclosureOutput")?;
    let tag = validate_required_tag(
        &js_property(&enclosure_output, "tag")?,
        &["omit", "include"],
    )?;
    if tag == "include" {
        validate_required_code(&js_property(&enclosure_output, "format")?, &["exactDyadic"])?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_resource_limit_request_value(request: &JsValue) -> Result<(), dto::CalculatorErrorDto> {
    let limits = js_property(request, "limits")?;
    let tag = validate_required_tag(&js_property(&limits, "tag")?, &["default", "custom"])?;
    if tag != "custom" {
        return Ok(());
    }

    let value = js_property(&limits, "value")?;
    for field in RESOURCE_LIMIT_U32_FIELDS {
        validate_required_u32_number(
            &js_property(&value, field)?,
            0,
            dto::InputLimitErrorCodeDto::InvalidResourceLimit,
        )?;
    }

    let max_logical_work_units = js_property(&value, "maxLogicalWorkUnits")?;
    if max_logical_work_units.as_f64().is_some() {
        return Err(convert::input_limit_error(
            dto::InputLimitErrorCodeDto::InvalidResourceLimit,
        ));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
const RESOURCE_LIMIT_U32_FIELDS: &[&str] = &[
    "maxInputBytes",
    "maxSourceAstNodes",
    "maxSourceDepth",
    "maxExpressionNodes",
    "maxIntegerBits",
    "maxAlgebraicDegree",
    "maxPolynomialCoefficientBits",
    "maxRewriteSteps",
    "maxPrecisionBits",
    "maxRefinementRounds",
    "maxPresentationNodes",
    "maxOutputBytes",
];

#[cfg(target_arch = "wasm32")]
fn validate_required_u32_number(
    value: &JsValue,
    minimum: u32,
    code: dto::InputLimitErrorCodeDto,
) -> Result<(), dto::CalculatorErrorDto> {
    if value.is_null() || value.is_undefined() {
        return Err(convert::unsupported_protocol_error(
            UnsupportedProtocolCodeDto::UnknownTag,
        ));
    }
    let Some(number) = value.as_f64() else {
        return Err(convert::unsupported_protocol_error(
            UnsupportedProtocolCodeDto::UnknownTag,
        ));
    };
    if !number.is_finite()
        || number.fract() != 0.0
        || number.is_sign_negative()
        || number < f64::from(minimum)
        || number > f64::from(u32::MAX)
    {
        return Err(convert::input_limit_error(code));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn validate_required_tag(
    value: &JsValue,
    allowed: &[&str],
) -> Result<String, dto::CalculatorErrorDto> {
    validate_required_string_union(value, allowed, UnsupportedProtocolCodeDto::UnknownTag)
}

#[cfg(target_arch = "wasm32")]
fn validate_required_code(
    value: &JsValue,
    allowed: &[&str],
) -> Result<String, dto::CalculatorErrorDto> {
    validate_required_string_union(value, allowed, UnsupportedProtocolCodeDto::UnknownCode)
}

#[cfg(target_arch = "wasm32")]
fn validate_required_string_union(
    value: &JsValue,
    allowed: &[&str],
    unknown_code: UnsupportedProtocolCodeDto,
) -> Result<String, dto::CalculatorErrorDto> {
    if value.is_null() || value.is_undefined() {
        return Err(convert::unsupported_protocol_error(
            UnsupportedProtocolCodeDto::UnknownTag,
        ));
    }
    let Some(value) = value.as_string() else {
        return Err(convert::unsupported_protocol_error(
            UnsupportedProtocolCodeDto::UnknownTag,
        ));
    };
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(convert::unsupported_protocol_error(unknown_code))
    }
}

#[cfg(target_arch = "wasm32")]
fn js_property(value: &JsValue, property: &str) -> Result<JsValue, dto::CalculatorErrorDto> {
    Reflect::get(value, &JsValue::from_str(property))
        .map_err(|_| convert::unsupported_protocol_error(UnsupportedProtocolCodeDto::UnknownTag))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn calculate(
    source: &str,
    request: CalculationRequestDto,
) -> ApiResultDto<CalculationOutcomeDto> {
    calculate_dto(source, request)
}

#[cfg(test)]
mod dto_conformance {
    use serde::Deserialize;

    use crate::dto::CalculationRequestDto;

    const FIXTURE: &str = include_str!("../fixtures/native_wasm_dto_conformance.json");

    #[derive(Debug, Deserialize)]
    pub struct Fixture {
        pub cases: Vec<Case>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Case {
        pub id: String,
        pub source: String,
        pub request: CalculationRequestDto,
        pub expected_result: serde_json::Value,
    }

    pub fn load_fixture() -> Fixture {
        serde_json::from_str(FIXTURE).expect("native/wasm DTO conformance fixture must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::*;

    fn exact_only_request() -> CalculationRequestDto {
        CalculationRequestDto {
            parse: ParseSettingsDto {
                grammar: GrammarProfileDto::Default,
                implicit_multiplication: ImplicitMultiplicationPolicyDto::Enabled,
                unicode_aliases: UnicodeAliasPolicyDto::MathematicalAliases,
                percent: PercentParsePolicyDto::PostfixPercent,
            },
            semantics: SemanticSettingsDto {
                domain: EvaluationDomainDto::Real,
                angle_unit: AngleUnitDto::Radian,
                power_semantics: PowerSemanticsDto::RealPrincipal,
            },
            exact_output: ExactOutputRequestDto::Include {
                format: ExactFormatPreferenceDto::Auto,
            },
            scientific_output: ScientificOutputRequestDto::Omit,
            enclosure_output: EnclosureOutputRequestDto::Omit,
            limits: ResourceLimitRequestDto::Default,
        }
    }

    fn input_policy() -> InputPolicyDto {
        InputPolicyDto {
            calculation_request: exact_only_request(),
            percent_policy: PercentPolicyDto::ExpressionPercent,
        }
    }

    fn resource_limits() -> ResourceLimitsDto {
        ResourceLimitsDto {
            max_input_bytes: 1_001,
            max_source_ast_nodes: 1_002,
            max_source_depth: 1_003,
            max_expression_nodes: 1_004,
            max_integer_bits: 1_005,
            max_algebraic_degree: 1_006,
            max_polynomial_coefficient_bits: 1_007,
            max_rewrite_steps: 1_008,
            max_precision_bits: 1_009,
            max_refinement_rounds: 1_010,
            max_logical_work_units: String::from("1011"),
            max_presentation_nodes: 1_012,
            max_output_bytes: 1_013,
        }
    }

    #[test]
    fn wasm_dto_calculates_exact_rational_expression() {
        let result = calculate_dto("0.1 + 0.2", exact_only_request());
        let ApiResultDto::Ok {
            value:
                CalculationOutcomeDto::Complete {
                    calculation:
                        CalculationDto {
                            exact: ExactOutputDto::Included { value: exact },
                            ..
                        },
                },
        } = result
        else {
            panic!("expected exact successful calculation");
        };
        assert_eq!(exact.plain_text, "3/10");
    }

    #[test]
    fn wasm_dto_reports_domain_error_code() {
        let result = calculate_dto("1 / 0", exact_only_request());
        assert_eq!(
            result,
            ApiResultDto::Error {
                error: CalculatorErrorDto::Domain {
                    code: DomainErrorCodeDto::DivisionByZero,
                    span: OptionalTextSpanDto::None,
                },
            }
        );
    }

    #[test]
    fn wasm_dto_rejects_zero_significant_digits_before_core() {
        let mut request = exact_only_request();
        request.scientific_output = ScientificOutputRequestDto::Include {
            significant_digits: 0,
            rounding_mode: DecimalRoundingModeDto::NearestTiesToEven,
        };

        assert_eq!(
            calculate_dto("1", request),
            ApiResultDto::Error {
                error: CalculatorErrorDto::InputLimit {
                    code: InputLimitErrorCodeDto::InvalidSignificantDigits,
                },
            }
        );
    }

    #[test]
    fn wasm_dto_custom_resource_limits_preserve_all_core_fields() {
        let limits = resource_limits();
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequestDto::Custom {
            value: limits.clone(),
        };

        let request = calculator_core::CalculationRequest::try_from(request)
            .expect("custom resource limits should convert");
        let calculator_core::ResourceLimitRequest::Custom(converted) = request.limits else {
            panic!("expected custom resource limits");
        };
        assert_eq!(converted.max_input_bytes, limits.max_input_bytes);
        assert_eq!(converted.max_source_ast_nodes, limits.max_source_ast_nodes);
        assert_eq!(converted.max_source_depth, limits.max_source_depth);
        assert_eq!(converted.max_expression_nodes, limits.max_expression_nodes);
        assert_eq!(converted.max_integer_bits, limits.max_integer_bits);
        assert_eq!(converted.max_algebraic_degree, limits.max_algebraic_degree);
        assert_eq!(
            converted.max_polynomial_coefficient_bits,
            limits.max_polynomial_coefficient_bits
        );
        assert_eq!(converted.max_rewrite_steps, limits.max_rewrite_steps);
        assert_eq!(converted.max_precision_bits, limits.max_precision_bits);
        assert_eq!(
            converted.max_refinement_rounds,
            limits.max_refinement_rounds
        );
        assert_eq!(
            converted.max_logical_work_units.to_string(),
            limits.max_logical_work_units
        );
        assert_eq!(
            converted.max_presentation_nodes,
            limits.max_presentation_nodes
        );
        assert_eq!(converted.max_output_bytes, limits.max_output_bytes);
    }

    #[test]
    fn wasm_dto_rejects_non_canonical_resource_limit_decimal_string() {
        let mut limits = resource_limits();
        limits.max_logical_work_units = String::from("01");
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequestDto::Custom { value: limits };

        assert_eq!(
            calculate_dto("1", request),
            ApiResultDto::Error {
                error: CalculatorErrorDto::InputLimit {
                    code: InputLimitErrorCodeDto::InvalidResourceLimit,
                },
            }
        );
    }

    #[test]
    fn calculation_outcome_partial_requires_certified_enclosure() {
        let ApiResultDto::Ok {
            value: CalculationOutcomeDto::Complete { calculation },
        } = calculate_dto("1", exact_only_request())
        else {
            panic!("expected complete calculation to build partial fixture");
        };
        let partial = CalculationOutcomeDto::Partial {
            calculation,
            reason: IncompleteReasonDto::ComputationLimit {
                kind: ComputationLimitCodeDto::LogicalWorkUnits,
            },
            certified_enclosure: CertifiedIntervalPresentationDto {
                relation: ResultRelationDto::ExactEqual,
                lower: ExactDyadicDto {
                    coefficient: String::from("1"),
                    exponent_two: String::from("0"),
                },
                upper: ExactDyadicDto {
                    coefficient: String::from("1"),
                    exponent_two: String::from("0"),
                },
                format: EnclosureFormatDto::ExactDyadic,
                presentation: PresentationNodeDto::Text {
                    text: String::from("[1, 1]"),
                },
            },
        };
        let mut value = serde_json::to_value(partial).expect("partial DTO must serialize");
        let serde_json::Value::Object(ref mut object) = value else {
            panic!("partial DTO must serialize as object");
        };
        object.remove("certifiedEnclosure");

        assert!(serde_json::from_value::<CalculationOutcomeDto>(value).is_err());
    }

    #[test]
    fn native_serialized_outputs_match_dto_golden_fixture() {
        for case in dto_conformance::load_fixture().cases {
            let actual = serde_json::to_value(calculate_dto(&case.source, case.request))
                .unwrap_or_else(|error| panic!("{}: failed to serialize result: {error}", case.id));
            assert_eq!(actual, case.expected_result, "{}", case.id);
        }
    }

    #[test]
    fn wasm_dto_accepts_camel_case_scientific_request_fields() {
        let request: CalculationRequestDto = serde_json::from_value(serde_json::json!({
            "parse": {
                "grammar": "default",
                "implicitMultiplication": "enabled",
                "unicodeAliases": "mathematicalAliases",
                "percent": "postfixPercent"
            },
            "semantics": {
                "domain": "real",
                "angleUnit": "degree",
                "powerSemantics": "realPrincipal"
            },
            "exactOutput": {
                "tag": "include",
                "format": "auto"
            },
            "scientificOutput": {
                "tag": "include",
                "significantDigits": 50,
                "roundingMode": "nearestTiesToEven"
            },
            "enclosureOutput": {
                "tag": "omit"
            },
            "limits": {
                "tag": "default"
            }
        }))
        .expect("generated TypeScript DTO casing must deserialize into Rust DTO");

        assert_eq!(
            calculate_dto("0.1 + 0.2", request),
            ApiResultDto::Ok {
                value: CalculationOutcomeDto::Complete {
                    calculation: CalculationDto {
                        exact: ExactOutputDto::Included {
                            value: ExactPresentationDto {
                                relation: ResultRelationDto::ExactEqual,
                                representation: ExactRepresentationKindDto::Rational,
                                presentation: PresentationNodeDto::Fraction {
                                    numerator: Box::new(PresentationNodeDto::Text {
                                        text: String::from("3"),
                                    }),
                                    denominator: Box::new(PresentationNodeDto::Text {
                                        text: String::from("10"),
                                    }),
                                },
                                plain_text: String::from("3/10"),
                            },
                        },
                        scientific: ScientificOutputDto::Included {
                            value: ScientificPresentationDto {
                                relation: ResultRelationDto::ApproximatelyEqual,
                                significand: String::from(
                                    "3.0000000000000000000000000000000000000000000000000",
                                ),
                                exponent_ten: String::from("-1"),
                                requested_significant_digits: 50,
                                confirmed_significant_digits: 50,
                                rounding_mode: DecimalRoundingModeDto::NearestTiesToEven,
                                presentation: PresentationNodeDto::Text {
                                    text: String::from(
                                        "3.0000000000000000000000000000000000000000000000000e-1",
                                    ),
                                },
                            },
                        },
                        enclosure: EnclosureOutputDto::Omitted,
                        metadata: CalculationMetadataDto {
                            exact_representation: ExactRepresentationKindDto::Rational,
                            simplification_status:
                                SimplificationStatusDto::FullySimplifiedWithinLimits,
                            semantic_settings: SemanticSettingsDto {
                                domain: EvaluationDomainDto::Real,
                                angle_unit: AngleUnitDto::Degree,
                                power_semantics: PowerSemanticsDto::RealPrincipal,
                            },
                            methods: vec![
                                MethodTagDto::RationalReduction,
                                MethodTagDto::CertifiedIntervalEvaluation,
                            ],
                            internal_precision_bits: 128,
                            refinement_rounds: 0,
                            confirmed_significant_digits: 50,
                            assurance: AssuranceLevelDto::Exact,
                            protocol_version: ProtocolVersionDto { major: 1, minor: 0 },
                        },
                    },
                },
            }
        );
    }

    #[test]
    fn session_dispatch_returns_calculate_command_and_applies_result() {
        let mut session = SessionCore::new(input_policy()).unwrap();
        session.dispatch_dto(InputActionDto::Digit { value: 1 });
        session.dispatch_dto(InputActionDto::BinaryOperator {
            value: BinaryOperatorDto::Add,
        });
        session.dispatch_dto(InputActionDto::Digit { value: 2 });

        let SessionDispatchResultDto::Calculate {
            source, request, ..
        } = session.dispatch_dto(InputActionDto::Evaluate)
        else {
            panic!("expected calculate command");
        };
        assert_eq!(source, "1+2");

        let result = calculate_dto(&source, request);
        let state = session.apply_result_dto(result);
        let SessionDisplayDto::Result { calculation } = state.display else {
            panic!("expected result display");
        };
        let ExactOutputDto::Included { value } = calculation.exact else {
            panic!("expected exact output");
        };
        assert_eq!(value.plain_text, "3");
        assert!(state.has_ans);
    }

    #[test]
    fn session_dispatch_honors_calculator_percent_policy() {
        let mut policy = input_policy();
        policy.percent_policy = PercentPolicyDto::CalculatorPercent;
        let mut session = SessionCore::new(policy).unwrap();
        for action in [
            InputActionDto::Digit { value: 1 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::BinaryOperator {
                value: BinaryOperatorDto::Add,
            },
            InputActionDto::Digit { value: 1 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::Percent,
        ] {
            session.dispatch_dto(action);
        }

        let SessionDispatchResultDto::Calculate {
            source, request, ..
        } = session.dispatch_dto(InputActionDto::Evaluate)
        else {
            panic!("expected calculate command");
        };
        assert_eq!(source, "100+((100)*(10)/100)");

        let result = calculate_dto(&source, request);
        let ApiResultDto::Ok {
            value:
                CalculationOutcomeDto::Complete {
                    calculation:
                        CalculationDto {
                            exact: ExactOutputDto::Included { value },
                            ..
                        },
                },
        } = result
        else {
            panic!("expected exact successful calculation");
        };
        assert_eq!(value.plain_text, "110");
    }

    #[test]
    fn session_dispatch_reports_input_error_without_mutating_state() {
        let mut session = SessionCore::new(input_policy()).unwrap();
        let result = session.dispatch_dto(InputActionDto::MemoryRecall);
        assert_eq!(
            result,
            SessionDispatchResultDto::InputError {
                state: session.get_state_dto(),
                error: InputErrorDto {
                    code: InputErrorCodeDto::MemoryEmpty,
                },
            }
        );
        assert_eq!(session.get_state_dto().source, "");
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
pub mod wasm_tests {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::{calculate, calculate_dto, dto_conformance, SessionCore};
    use crate::dto::*;

    fn exact_only_request() -> CalculationRequestDto {
        CalculationRequestDto {
            parse: ParseSettingsDto {
                grammar: GrammarProfileDto::Default,
                implicit_multiplication: ImplicitMultiplicationPolicyDto::Enabled,
                unicode_aliases: UnicodeAliasPolicyDto::MathematicalAliases,
                percent: PercentParsePolicyDto::PostfixPercent,
            },
            semantics: SemanticSettingsDto {
                domain: EvaluationDomainDto::Real,
                angle_unit: AngleUnitDto::Radian,
                power_semantics: PowerSemanticsDto::RealPrincipal,
            },
            exact_output: ExactOutputRequestDto::Include {
                format: ExactFormatPreferenceDto::Auto,
            },
            scientific_output: ScientificOutputRequestDto::Omit,
            enclosure_output: EnclosureOutputRequestDto::Omit,
            limits: ResourceLimitRequestDto::Default,
        }
    }

    fn exact_plain_text(result: ApiResultDto<CalculationOutcomeDto>) -> String {
        let ApiResultDto::Ok {
            value:
                CalculationOutcomeDto::Complete {
                    calculation:
                        CalculationDto {
                            exact: ExactOutputDto::Included { value },
                            ..
                        },
                },
        } = result
        else {
            panic!("expected exact successful calculation");
        };
        value.plain_text
    }

    fn resource_limits() -> ResourceLimitsDto {
        ResourceLimitsDto {
            max_input_bytes: 1_001,
            max_source_ast_nodes: 1_002,
            max_source_depth: 1_003,
            max_expression_nodes: 1_004,
            max_integer_bits: 1_005,
            max_algebraic_degree: 1_006,
            max_polynomial_coefficient_bits: 1_007,
            max_rewrite_steps: 1_008,
            max_precision_bits: 1_009,
            max_refinement_rounds: 1_010,
            max_logical_work_units: String::from("1011"),
            max_presentation_nodes: 1_012,
            max_output_bytes: 1_013,
        }
    }

    fn js_exact_request() -> JsValue {
        serde_wasm_bindgen::to_value(&exact_only_request()).expect("request DTO must serialize")
    }

    fn js_custom_limit_request() -> JsValue {
        let mut request = exact_only_request();
        request.limits = ResourceLimitRequestDto::Custom {
            value: resource_limits(),
        };
        serde_wasm_bindgen::to_value(&request).expect("request DTO must serialize")
    }

    fn js_property(value: &JsValue, property: &str) -> JsValue {
        Reflect::get(value, &JsValue::from_str(property)).expect("property read must succeed")
    }

    fn set_js_property(value: &JsValue, property: &str, property_value: &JsValue) {
        Reflect::set(value, &JsValue::from_str(property), property_value)
            .expect("property write must succeed");
    }

    fn calculate_js_error(request: JsValue) -> CalculatorErrorDto {
        let result: ApiResultDto<CalculationOutcomeDto> =
            serde_wasm_bindgen::from_value(calculate("1", request))
                .expect("wasm calculate result must deserialize");
        let ApiResultDto::Error { error } = result else {
            panic!("expected calculator error");
        };
        error
    }

    #[wasm_bindgen_test]
    fn wasm32_calculates_exact_rational_expression() {
        let result = calculate_dto("0.1 + 0.2", exact_only_request());
        assert_eq!(exact_plain_text(result), "3/10");
    }

    #[wasm_bindgen_test]
    fn wasm32_session_dispatches_calculator_percent() {
        let request = exact_only_request();
        let mut session = SessionCore::new(InputPolicyDto {
            calculation_request: request,
            percent_policy: PercentPolicyDto::CalculatorPercent,
        })
        .expect("policy should be accepted");
        for action in [
            InputActionDto::Digit { value: 1 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::BinaryOperator {
                value: BinaryOperatorDto::Add,
            },
            InputActionDto::Digit { value: 1 },
            InputActionDto::Digit { value: 0 },
            InputActionDto::Percent,
        ] {
            session.dispatch_dto(action);
        }

        let SessionDispatchResultDto::Calculate {
            source, request, ..
        } = session.dispatch_dto(InputActionDto::Evaluate)
        else {
            panic!("expected calculate command");
        };
        assert_eq!(source, "100+((100)*(10)/100)");
        assert_eq!(exact_plain_text(calculate_dto(&source, request)), "110");
    }

    #[wasm_bindgen_test]
    fn wasm32_calculate_rejects_null_undefined_and_unknown_tag_as_unsupported_protocol() {
        for request in [JsValue::NULL, JsValue::UNDEFINED] {
            assert_eq!(
                calculate_js_error(request),
                CalculatorErrorDto::UnsupportedProtocol {
                    code: UnsupportedProtocolCodeDto::UnknownTag,
                }
            );
        }

        let request = js_exact_request();
        let exact_output = js_property(&request, "exactOutput");
        set_js_property(&exact_output, "tag", &JsValue::from_str("futureOutput"));
        assert_eq!(
            calculate_js_error(request),
            CalculatorErrorDto::UnsupportedProtocol {
                code: UnsupportedProtocolCodeDto::UnknownTag,
            }
        );
    }

    #[wasm_bindgen_test]
    fn wasm32_calculate_rejects_unknown_code_as_unsupported_protocol() {
        let request = js_exact_request();
        let semantics = js_property(&request, "semantics");
        set_js_property(&semantics, "angleUnit", &JsValue::from_str("turn"));
        assert_eq!(
            calculate_js_error(request),
            CalculatorErrorDto::UnsupportedProtocol {
                code: UnsupportedProtocolCodeDto::UnknownCode,
            }
        );
    }

    #[wasm_bindgen_test]
    fn wasm32_calculate_rejects_invalid_significant_digit_numbers() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            1.5,
            -1.0,
            -0.0,
            0.0,
            9_007_199_254_740_992.0,
        ] {
            let request = js_exact_request();
            let scientific_output = js_property(&request, "scientificOutput");
            set_js_property(&scientific_output, "tag", &JsValue::from_str("include"));
            set_js_property(
                &scientific_output,
                "significantDigits",
                &JsValue::from_f64(value),
            );
            set_js_property(
                &scientific_output,
                "roundingMode",
                &JsValue::from_str("nearestTiesToEven"),
            );

            assert_eq!(
                calculate_js_error(request),
                CalculatorErrorDto::InputLimit {
                    code: InputLimitErrorCodeDto::InvalidSignificantDigits,
                },
                "{value:?} must be rejected as invalid significant digits"
            );
        }
    }

    #[wasm_bindgen_test]
    fn wasm32_calculate_rejects_invalid_resource_limit_numbers() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            1.5,
            -1.0,
            -0.0,
            9_007_199_254_740_992.0,
        ] {
            let request = js_custom_limit_request();
            let limits = js_property(&request, "limits");
            let resource_limits = js_property(&limits, "value");
            set_js_property(&resource_limits, "maxInputBytes", &JsValue::from_f64(value));

            assert_eq!(
                calculate_js_error(request),
                CalculatorErrorDto::InputLimit {
                    code: InputLimitErrorCodeDto::InvalidResourceLimit,
                },
                "{value:?} must be rejected as an invalid resource limit"
            );
        }
    }

    #[wasm_bindgen_test]
    fn wasm32_calculate_rejects_non_canonical_resource_limit_decimal_string() {
        let request = js_custom_limit_request();
        let limits = js_property(&request, "limits");
        let resource_limits = js_property(&limits, "value");
        set_js_property(
            &resource_limits,
            "maxLogicalWorkUnits",
            &JsValue::from_str("01"),
        );

        assert_eq!(
            calculate_js_error(request),
            CalculatorErrorDto::InputLimit {
                code: InputLimitErrorCodeDto::InvalidResourceLimit,
            }
        );
    }

    #[wasm_bindgen_test]
    fn wasm32_serialized_outputs_match_dto_golden_fixture() {
        for case in dto_conformance::load_fixture().cases {
            let request = serde_wasm_bindgen::to_value(&case.request).unwrap_or_else(|error| {
                panic!("{}: failed to serialize request: {error:?}", case.id)
            });
            let actual: serde_json::Value =
                serde_wasm_bindgen::from_value(calculate(&case.source, request)).unwrap_or_else(
                    |error| panic!("{}: failed to deserialize wasm result: {error:?}", case.id),
                );
            assert_eq!(actual, case.expected_result, "{}", case.id);
        }
    }
}
