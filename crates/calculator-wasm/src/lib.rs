#![forbid(unsafe_code)]

mod convert;
pub mod dto;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use dto::UnsupportedProtocolCodeDto;
use dto::{ApiResultDto, CalculationOutcomeDto, CalculationRequestDto};

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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn calculate(source: &str, request: JsValue) -> JsValue {
    let result = match serde_wasm_bindgen::from_value::<CalculationRequestDto>(request) {
        Ok(request) => calculate_dto(source, request),
        Err(_) => ApiResultDto::<CalculationOutcomeDto>::Error {
            error: convert::unsupported_protocol_error(UnsupportedProtocolCodeDto::UnknownTag),
        },
    };
    serde_wasm_bindgen::to_value(&result).expect("calculator DTO serialization must succeed")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn calculate(
    source: &str,
    request: CalculationRequestDto,
) -> ApiResultDto<CalculationOutcomeDto> {
    calculate_dto(source, request)
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
                        scientific: ScientificOutputDto::Unavailable {
                            value: UnavailableScientificOutputDto {
                                requested_significant_digits: 50,
                                confirmed_significant_digits: 0,
                                rounding_mode: DecimalRoundingModeDto::NearestTiesToEven,
                                reason: IncompleteReasonDto::ComputationLimit {
                                    kind: ComputationLimitCodeDto::PrecisionBits,
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
                            methods: vec![MethodTagDto::RationalReduction],
                            internal_precision_bits: 0,
                            refinement_rounds: 0,
                            confirmed_significant_digits: 0,
                            assurance: AssuranceLevelDto::Exact,
                            protocol_version: ProtocolVersionDto { major: 1, minor: 0 },
                        },
                    },
                },
            }
        );
    }
}
