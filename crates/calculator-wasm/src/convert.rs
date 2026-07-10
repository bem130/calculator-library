use std::{num::NonZeroU32, str::FromStr};

use calculator_core as core;
use num_bigint::BigInt;

use crate::dto::*;

impl TryFrom<CalculationRequestDto> for core::CalculationRequest {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculationRequestDto) -> Result<Self, Self::Error> {
        Ok(Self {
            parse: value.parse.into(),
            semantics: value.semantics.into(),
            exact_output: value.exact_output.into(),
            scientific_output: value.scientific_output.try_into()?,
            enclosure_output: value.enclosure_output.try_into()?,
            limits: value.limits.try_into()?,
        })
    }
}

impl From<ParseSettingsDto> for core::ParseSettings {
    fn from(value: ParseSettingsDto) -> Self {
        Self {
            grammar: value.grammar.into(),
            implicit_multiplication: value.implicit_multiplication.into(),
            unicode_aliases: value.unicode_aliases.into(),
            percent: value.percent.into(),
        }
    }
}

impl From<GrammarProfileDto> for core::GrammarProfile {
    fn from(value: GrammarProfileDto) -> Self {
        match value {
            GrammarProfileDto::Default => Self::Default,
        }
    }
}

impl From<ImplicitMultiplicationPolicyDto> for core::ImplicitMultiplicationPolicy {
    fn from(value: ImplicitMultiplicationPolicyDto) -> Self {
        match value {
            ImplicitMultiplicationPolicyDto::Enabled => Self::Enabled,
            ImplicitMultiplicationPolicyDto::Disabled => Self::Disabled,
        }
    }
}

impl From<UnicodeAliasPolicyDto> for core::UnicodeAliasPolicy {
    fn from(value: UnicodeAliasPolicyDto) -> Self {
        match value {
            UnicodeAliasPolicyDto::MathematicalAliases => Self::MathematicalAliases,
            UnicodeAliasPolicyDto::AsciiOnly => Self::AsciiOnly,
        }
    }
}

impl From<PercentParsePolicyDto> for core::PercentParsePolicy {
    fn from(value: PercentParsePolicyDto) -> Self {
        match value {
            PercentParsePolicyDto::PostfixPercent => Self::PostfixPercent,
            PercentParsePolicyDto::RejectPercent => Self::RejectPercent,
        }
    }
}

impl From<SemanticSettingsDto> for core::SemanticSettings {
    fn from(value: SemanticSettingsDto) -> Self {
        Self {
            domain: value.domain.into(),
            angle_unit: value.angle_unit.into(),
            power_semantics: value.power_semantics.into(),
        }
    }
}

impl From<EvaluationDomainDto> for core::EvaluationDomain {
    fn from(value: EvaluationDomainDto) -> Self {
        match value {
            EvaluationDomainDto::Real => Self::Real,
        }
    }
}

impl From<AngleUnitDto> for core::AngleUnit {
    fn from(value: AngleUnitDto) -> Self {
        match value {
            AngleUnitDto::Radian => Self::Radian,
            AngleUnitDto::Degree => Self::Degree,
            AngleUnitDto::Gradian => Self::Gradian,
        }
    }
}

impl From<PowerSemanticsDto> for core::PowerSemantics {
    fn from(value: PowerSemanticsDto) -> Self {
        match value {
            PowerSemanticsDto::RealPrincipal => Self::RealPrincipal,
        }
    }
}

impl From<ExactOutputRequestDto> for core::ExactOutputRequest {
    fn from(value: ExactOutputRequestDto) -> Self {
        match value {
            ExactOutputRequestDto::Omit => Self::Omit,
            ExactOutputRequestDto::Include { format } => Self::Include {
                format: format.into(),
            },
        }
    }
}

impl From<ExactFormatPreferenceDto> for core::ExactFormatPreference {
    fn from(value: ExactFormatPreferenceDto) -> Self {
        match value {
            ExactFormatPreferenceDto::Auto => Self::Auto,
            ExactFormatPreferenceDto::Rational => Self::Rational,
            ExactFormatPreferenceDto::FiniteDecimal => Self::FiniteDecimal,
            ExactFormatPreferenceDto::MixedFraction => Self::MixedFraction,
            ExactFormatPreferenceDto::Symbolic => Self::Symbolic,
        }
    }
}

impl TryFrom<ScientificOutputRequestDto> for core::ScientificOutputRequest {
    type Error = CalculatorErrorDto;

    fn try_from(value: ScientificOutputRequestDto) -> Result<Self, Self::Error> {
        match value {
            ScientificOutputRequestDto::Omit => Ok(Self::Omit),
            ScientificOutputRequestDto::Include {
                significant_digits,
                rounding_mode,
            } => {
                let Some(significant_digits) = NonZeroU32::new(significant_digits) else {
                    return Err(input_limit_error(
                        InputLimitErrorCodeDto::InvalidSignificantDigits,
                    ));
                };
                Ok(Self::Include {
                    significant_digits,
                    rounding_mode: rounding_mode.into(),
                })
            }
        }
    }
}

impl From<DecimalRoundingModeDto> for core::DecimalRoundingMode {
    fn from(value: DecimalRoundingModeDto) -> Self {
        match value {
            DecimalRoundingModeDto::NearestTiesToEven => Self::NearestTiesToEven,
            DecimalRoundingModeDto::NearestTiesAwayFromZero => Self::NearestTiesAwayFromZero,
            DecimalRoundingModeDto::TowardPositiveInfinity => Self::TowardPositiveInfinity,
            DecimalRoundingModeDto::TowardNegativeInfinity => Self::TowardNegativeInfinity,
            DecimalRoundingModeDto::TowardZero => Self::TowardZero,
            DecimalRoundingModeDto::AwayFromZero => Self::AwayFromZero,
        }
    }
}

impl From<core::DecimalRoundingMode> for DecimalRoundingModeDto {
    fn from(value: core::DecimalRoundingMode) -> Self {
        match value {
            core::DecimalRoundingMode::NearestTiesToEven => Self::NearestTiesToEven,
            core::DecimalRoundingMode::NearestTiesAwayFromZero => Self::NearestTiesAwayFromZero,
            core::DecimalRoundingMode::TowardPositiveInfinity => Self::TowardPositiveInfinity,
            core::DecimalRoundingMode::TowardNegativeInfinity => Self::TowardNegativeInfinity,
            core::DecimalRoundingMode::TowardZero => Self::TowardZero,
            core::DecimalRoundingMode::AwayFromZero => Self::AwayFromZero,
        }
    }
}

impl TryFrom<EnclosureOutputRequestDto> for core::EnclosureOutputRequest {
    type Error = CalculatorErrorDto;

    fn try_from(value: EnclosureOutputRequestDto) -> Result<Self, Self::Error> {
        match value {
            EnclosureOutputRequestDto::Omit => Ok(Self::Omit),
            EnclosureOutputRequestDto::Include { format } => Ok(Self::Include {
                format: format.try_into()?,
            }),
        }
    }
}

impl TryFrom<EnclosureFormatDto> for core::EnclosureFormat {
    type Error = CalculatorErrorDto;

    fn try_from(value: EnclosureFormatDto) -> Result<Self, Self::Error> {
        match value {
            EnclosureFormatDto::ExactDyadic => Ok(Self::ExactDyadic),
            EnclosureFormatDto::DecimalScientific { significant_digits } => {
                let Some(significant_digits) = NonZeroU32::new(significant_digits) else {
                    return Err(input_limit_error(
                        InputLimitErrorCodeDto::InvalidSignificantDigits,
                    ));
                };
                Ok(Self::DecimalScientific { significant_digits })
            }
        }
    }
}

impl From<core::EnclosureFormat> for EnclosureFormatDto {
    fn from(value: core::EnclosureFormat) -> Self {
        match value {
            core::EnclosureFormat::ExactDyadic => Self::ExactDyadic,
            core::EnclosureFormat::DecimalScientific { significant_digits } => {
                Self::DecimalScientific {
                    significant_digits: significant_digits.get(),
                }
            }
        }
    }
}

impl TryFrom<ResourceLimitRequestDto> for core::ResourceLimitRequest {
    type Error = CalculatorErrorDto;

    fn try_from(value: ResourceLimitRequestDto) -> Result<Self, Self::Error> {
        match value {
            ResourceLimitRequestDto::Default => Ok(Self::Default),
            ResourceLimitRequestDto::Custom { value } => {
                let limits = core::ResourceLimits {
                    max_input_bytes: value.max_input_bytes,
                    max_source_ast_nodes: value.max_source_ast_nodes,
                    max_source_depth: value.max_source_depth,
                    max_expression_nodes: value.max_expression_nodes,
                    max_integer_bits: value.max_integer_bits,
                    max_cyclotomic_order: value.max_cyclotomic_order,
                    max_algebraic_degree: value.max_algebraic_degree,
                    max_polynomial_coefficient_bits: value.max_polynomial_coefficient_bits,
                    max_resultant_degree: value.max_resultant_degree,
                    max_factorization_work: value.max_factorization_work,
                    max_root_isolation_steps: value.max_root_isolation_steps,
                    max_rewrite_steps: value.max_rewrite_steps,
                    max_precision_bits: value.max_precision_bits,
                    max_refinement_rounds: value.max_refinement_rounds,
                    max_logical_work_units: parse_u64(&value.max_logical_work_units)?,
                    max_presentation_nodes: value.max_presentation_nodes,
                    max_output_bytes: value.max_output_bytes,
                };
                Ok(Self::Custom(limits))
            }
        }
    }
}

impl From<core::CalculationOutcome> for CalculationOutcomeDto {
    fn from(value: core::CalculationOutcome) -> Self {
        match value {
            core::CalculationOutcome::Complete(calculation) => Self::Complete {
                calculation: calculation.into(),
            },
            core::CalculationOutcome::Partial {
                calculation,
                reason,
                certified_enclosure,
            } => Self::Partial {
                calculation: calculation.into(),
                reason: reason.into(),
                certified_enclosure: certified_enclosure.map(Into::into),
            },
        }
    }
}

impl From<core::Calculation> for CalculationDto {
    fn from(value: core::Calculation) -> Self {
        Self {
            exact: value.exact.into(),
            scientific: value.scientific.into(),
            enclosure: value.enclosure.into(),
            metadata: value.metadata.into(),
        }
    }
}

impl From<core::ExactOutput> for ExactOutputDto {
    fn from(value: core::ExactOutput) -> Self {
        match value {
            core::ExactOutput::Omitted => Self::Omitted,
            core::ExactOutput::Included(value) => Self::Included {
                value: value.into(),
            },
        }
    }
}

impl From<core::ScientificOutput> for ScientificOutputDto {
    fn from(value: core::ScientificOutput) -> Self {
        match value {
            core::ScientificOutput::Omitted => Self::Omitted,
            core::ScientificOutput::Included(value) => Self::Included {
                value: value.into(),
            },
            core::ScientificOutput::Unavailable(value) => Self::Unavailable {
                value: value.into(),
            },
        }
    }
}

impl From<core::EnclosureOutput> for EnclosureOutputDto {
    fn from(value: core::EnclosureOutput) -> Self {
        match value {
            core::EnclosureOutput::Omitted => Self::Omitted,
            core::EnclosureOutput::Included(value) => Self::Included {
                value: value.into(),
            },
            core::EnclosureOutput::Unavailable(value) => Self::Unavailable {
                reason: value.reason.into(),
            },
        }
    }
}

impl From<core::ExactPresentation> for ExactPresentationDto {
    fn from(value: core::ExactPresentation) -> Self {
        Self {
            relation: value.relation.into(),
            representation: value.representation.into(),
            presentation: value.presentation.into(),
            plain_text: value.plain_text,
        }
    }
}

impl From<core::ScientificPresentation> for ScientificPresentationDto {
    fn from(value: core::ScientificPresentation) -> Self {
        Self {
            relation: value.relation.into(),
            significand: value.significand,
            exponent_ten: value.exponent_ten,
            requested_significant_digits: value.requested_significant_digits.get(),
            confirmed_significant_digits: value.confirmed_significant_digits,
            rounding_mode: value.rounding_mode.into(),
            presentation: value.presentation.into(),
        }
    }
}

impl From<core::UnavailableScientificOutput> for UnavailableScientificOutputDto {
    fn from(value: core::UnavailableScientificOutput) -> Self {
        Self {
            requested_significant_digits: value.requested_significant_digits.get(),
            confirmed_significant_digits: value.confirmed_significant_digits,
            rounding_mode: value.rounding_mode.into(),
            reason: value.reason.into(),
        }
    }
}

impl From<core::CertifiedIntervalPresentation> for CertifiedIntervalPresentationDto {
    fn from(value: core::CertifiedIntervalPresentation) -> Self {
        Self {
            relation: value.relation.into(),
            bounds: value.bounds.into(),
            presentation: value.presentation.into(),
        }
    }
}

impl From<core::CertifiedIntervalBounds> for CertifiedIntervalBoundsDto {
    fn from(value: core::CertifiedIntervalBounds) -> Self {
        match value {
            core::CertifiedIntervalBounds::ExactDyadic { lower, upper } => Self::ExactDyadic {
                lower: lower.into(),
                upper: upper.into(),
            },
            core::CertifiedIntervalBounds::DecimalScientific {
                lower,
                upper,
                requested_significant_digits,
            } => Self::DecimalScientific {
                lower: lower.into(),
                upper: upper.into(),
                requested_significant_digits: requested_significant_digits.get(),
            },
        }
    }
}

impl From<core::DecimalScientificBound> for DecimalScientificBoundDto {
    fn from(value: core::DecimalScientificBound) -> Self {
        Self {
            significand: value.significand,
            exponent_ten: value.exponent_ten,
        }
    }
}

impl From<core::ExactDyadic> for ExactDyadicDto {
    fn from(value: core::ExactDyadic) -> Self {
        Self {
            coefficient: value.coefficient.to_string(),
            exponent_two: value.exponent_two.to_string(),
        }
    }
}

impl From<core::CalculationMetadata> for CalculationMetadataDto {
    fn from(value: core::CalculationMetadata) -> Self {
        Self {
            exact_representation: value.exact_representation.into(),
            simplification_status: value.simplification_status.into(),
            semantic_settings: value.semantic_settings.into(),
            methods: value.methods.into_iter().map(Into::into).collect(),
            internal_precision_bits: value.internal_precision_bits,
            refinement_rounds: value.refinement_rounds,
            confirmed_significant_digits: value.confirmed_significant_digits,
            assurance: value.assurance.into(),
            protocol_version: value.protocol_version.into(),
        }
    }
}

impl From<core::SemanticSettings> for SemanticSettingsDto {
    fn from(value: core::SemanticSettings) -> Self {
        Self {
            domain: value.domain.into(),
            angle_unit: value.angle_unit.into(),
            power_semantics: value.power_semantics.into(),
        }
    }
}

impl From<core::EvaluationDomain> for EvaluationDomainDto {
    fn from(value: core::EvaluationDomain) -> Self {
        match value {
            core::EvaluationDomain::Real => Self::Real,
        }
    }
}

impl From<core::AngleUnit> for AngleUnitDto {
    fn from(value: core::AngleUnit) -> Self {
        match value {
            core::AngleUnit::Radian => Self::Radian,
            core::AngleUnit::Degree => Self::Degree,
            core::AngleUnit::Gradian => Self::Gradian,
        }
    }
}

impl From<core::PowerSemantics> for PowerSemanticsDto {
    fn from(value: core::PowerSemantics) -> Self {
        match value {
            core::PowerSemantics::RealPrincipal => Self::RealPrincipal,
        }
    }
}

impl From<core::IncompleteReason> for IncompleteReasonDto {
    fn from(value: core::IncompleteReason) -> Self {
        match value {
            core::IncompleteReason::PrecisionLimit {
                requested_digits,
                confirmed_digits,
            } => Self::PrecisionLimit {
                requested_digits: requested_digits.get(),
                confirmed_digits,
            },
            core::IncompleteReason::ComputationLimit { kind } => {
                Self::ComputationLimit { kind: kind.into() }
            }
            core::IncompleteReason::UnsupportedFeature { feature } => Self::UnsupportedFeature {
                feature: feature.into(),
            },
        }
    }
}

impl From<core::SimplificationStatus> for SimplificationStatusDto {
    fn from(value: core::SimplificationStatus) -> Self {
        match value {
            core::SimplificationStatus::FullySimplifiedWithinLimits => {
                Self::FullySimplifiedWithinLimits
            }
            core::SimplificationStatus::PartiallySimplified { reason } => {
                Self::PartiallySimplified {
                    reason: reason.into(),
                }
            }
        }
    }
}

impl From<core::PresentationNode> for PresentationNodeDto {
    fn from(value: core::PresentationNode) -> Self {
        match value {
            core::PresentationNode::Text(text) => Self::Text { text },
            core::PresentationNode::Row(children) => Self::Row {
                children: children.into_iter().map(Into::into).collect(),
            },
            core::PresentationNode::Fraction {
                numerator,
                denominator,
            } => Self::Fraction {
                numerator: Box::new((*numerator).into()),
                denominator: Box::new((*denominator).into()),
            },
            core::PresentationNode::Superscript { base, exponent } => Self::Superscript {
                base: Box::new((*base).into()),
                exponent: Box::new((*exponent).into()),
            },
            core::PresentationNode::Subscript { base, subscript } => Self::Subscript {
                base: Box::new((*base).into()),
                subscript: Box::new((*subscript).into()),
            },
            core::PresentationNode::Radical { index, radicand } => Self::Radical {
                index: index.into(),
                radicand: Box::new((*radicand).into()),
            },
            core::PresentationNode::Function { name, argument } => Self::Function {
                name: name.into(),
                argument: Box::new((*argument).into()),
            },
            core::PresentationNode::Parenthesized(value) => Self::Parenthesized {
                value: Box::new((*value).into()),
            },
        }
    }
}

impl From<core::RadicalIndex> for RadicalIndexDto {
    fn from(value: core::RadicalIndex) -> Self {
        match value {
            core::RadicalIndex::Square => Self::Square,
            core::RadicalIndex::Nth(value) => Self::Nth {
                value: value.inner.to_string(),
            },
        }
    }
}

impl From<core::FunctionName> for FunctionNameDto {
    fn from(value: core::FunctionName) -> Self {
        match value {
            core::FunctionName::Sin => Self::Sin,
            core::FunctionName::Cos => Self::Cos,
            core::FunctionName::Tan => Self::Tan,
            core::FunctionName::Asin => Self::Asin,
            core::FunctionName::Acos => Self::Acos,
            core::FunctionName::Atan => Self::Atan,
            core::FunctionName::Sqrt => Self::Sqrt,
            core::FunctionName::Exp => Self::Exp,
            core::FunctionName::Log => Self::Log,
            core::FunctionName::Ln => Self::Ln,
        }
    }
}

impl From<core::ResultRelation> for ResultRelationDto {
    fn from(value: core::ResultRelation) -> Self {
        match value {
            core::ResultRelation::ExactEqual => Self::ExactEqual,
            core::ResultRelation::ApproximatelyEqual => Self::ApproximatelyEqual,
            core::ResultRelation::ElementOf => Self::ElementOf,
        }
    }
}

impl From<core::ExactRepresentationKind> for ExactRepresentationKindDto {
    fn from(value: core::ExactRepresentationKind) -> Self {
        match value {
            core::ExactRepresentationKind::Integer => Self::Integer,
            core::ExactRepresentationKind::Rational => Self::Rational,
            core::ExactRepresentationKind::FiniteDecimal => Self::FiniteDecimal,
            core::ExactRepresentationKind::RationalPiMultiple => Self::RationalPiMultiple,
            core::ExactRepresentationKind::Radical => Self::Radical,
            core::ExactRepresentationKind::RealAlgebraic => Self::RealAlgebraic,
            core::ExactRepresentationKind::GeneralSymbolic => Self::GeneralSymbolic,
        }
    }
}

impl From<core::MethodTag> for MethodTagDto {
    fn from(value: core::MethodTag) -> Self {
        match value {
            core::MethodTag::RationalReduction => Self::RationalReduction,
            core::MethodTag::RadicalExtraction => Self::RadicalExtraction,
            core::MethodTag::SpecialAngle => Self::SpecialAngle,
            core::MethodTag::CyclotomicReduction => Self::CyclotomicReduction,
            core::MethodTag::AlgebraicMinimalPolynomial => Self::AlgebraicMinimalPolynomial,
            core::MethodTag::AlgebraicRootIsolation => Self::AlgebraicRootIsolation,
            core::MethodTag::SymbolicRetention => Self::SymbolicRetention,
            core::MethodTag::CertifiedIntervalEvaluation => Self::CertifiedIntervalEvaluation,
            core::MethodTag::AdaptivePrecisionRefinement => Self::AdaptivePrecisionRefinement,
        }
    }
}

impl From<core::AssuranceLevel> for AssuranceLevelDto {
    fn from(value: core::AssuranceLevel) -> Self {
        match value {
            core::AssuranceLevel::Exact => Self::Exact,
            core::AssuranceLevel::CertifiedEnclosure => Self::CertifiedEnclosure,
        }
    }
}

impl From<core::CalculatorError> for CalculatorErrorDto {
    fn from(value: core::CalculatorError) -> Self {
        match value {
            core::CalculatorError::Parse(error) => Self::Parse {
                code: error.kind.into(),
                span: error.span.into(),
                expected: error.expected.into_iter().map(Into::into).collect(),
            },
            core::CalculatorError::Domain(error) => Self::Domain {
                code: error.kind.into(),
                span: error.span.into(),
            },
            core::CalculatorError::InputLimit(error) => Self::InputLimit {
                code: error.kind.into(),
            },
            core::CalculatorError::ComputationLimit(error) => Self::ComputationLimit {
                code: error.kind.into(),
            },
            core::CalculatorError::UnsupportedFeature(error) => Self::UnsupportedFeature {
                code: error.feature.into(),
            },
            core::CalculatorError::InternalInvariant(error) => Self::InternalInvariant {
                code: error.code.into(),
            },
        }
    }
}

impl From<core::ByteSpan> for TextSpanDto {
    fn from(value: core::ByteSpan) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<Option<core::ByteSpan>> for OptionalTextSpanDto {
    fn from(value: Option<core::ByteSpan>) -> Self {
        match value {
            Some(value) => Self::Some {
                value: value.into(),
            },
            None => Self::None,
        }
    }
}

impl From<core::ExpectedToken> for ExpectedTokenDto {
    fn from(value: core::ExpectedToken) -> Self {
        Self {
            kind: value.kind.into(),
        }
    }
}

impl From<core::ExpectedTokenKind> for ExpectedTokenKindDto {
    fn from(value: core::ExpectedTokenKind) -> Self {
        match value {
            core::ExpectedTokenKind::Number => Self::Number,
            core::ExpectedTokenKind::Identifier => Self::Identifier,
            core::ExpectedTokenKind::Operator => Self::Operator,
            core::ExpectedTokenKind::OpenParenthesis => Self::OpenParenthesis,
            core::ExpectedTokenKind::CloseParenthesis => Self::CloseParenthesis,
            core::ExpectedTokenKind::Comma => Self::Comma,
            core::ExpectedTokenKind::EndOfInput => Self::EndOfInput,
        }
    }
}

impl From<core::ParseErrorKind> for ParseErrorCodeDto {
    fn from(value: core::ParseErrorKind) -> Self {
        match value {
            core::ParseErrorKind::UnexpectedToken => Self::UnexpectedToken,
            core::ParseErrorKind::UnexpectedEnd => Self::UnexpectedEnd,
            core::ParseErrorKind::UnknownIdentifier => Self::UnknownIdentifier,
            core::ParseErrorKind::InvalidNumberLiteral => Self::InvalidNumberLiteral,
            core::ParseErrorKind::MissingFunctionParenthesis => Self::MissingFunctionParenthesis,
            core::ParseErrorKind::ImplicitMultiplicationDisabled => {
                Self::ImplicitMultiplicationDisabled
            }
            core::ParseErrorKind::PercentRejected => Self::PercentRejected,
        }
    }
}

impl From<core::DomainErrorKind> for DomainErrorCodeDto {
    fn from(value: core::DomainErrorKind) -> Self {
        match value {
            core::DomainErrorKind::DivisionByZero => Self::DivisionByZero,
            core::DomainErrorKind::LogarithmOfNonPositive => Self::LogarithmOfNonPositive,
            core::DomainErrorKind::LogarithmBaseOne => Self::LogarithmBaseOne,
            core::DomainErrorKind::EvenRootOfNegative => Self::EvenRootOfNegative,
            core::DomainErrorKind::InverseTrigonometricOutOfRange => {
                Self::InverseTrigonometricOutOfRange
            }
            core::DomainErrorKind::TangentPole => Self::TangentPole,
            core::DomainErrorKind::ZeroToNegativePower => Self::ZeroToNegativePower,
            core::DomainErrorKind::IndeterminateZeroToZero => Self::IndeterminateZeroToZero,
            core::DomainErrorKind::NonRealPower => Self::NonRealPower,
            core::DomainErrorKind::IntegerFunctionRequiresInteger => {
                Self::IntegerFunctionRequiresInteger
            }
            core::DomainErrorKind::IntegerFunctionRequiresNonNegative => {
                Self::IntegerFunctionRequiresNonNegative
            }
        }
    }
}

impl From<core::InputLimitErrorKind> for InputLimitErrorCodeDto {
    fn from(value: core::InputLimitErrorKind) -> Self {
        match value {
            core::InputLimitErrorKind::InputTooLong => Self::InputTooLong,
            core::InputLimitErrorKind::SourceAstTooDeep => Self::SourceAstTooDeep,
            core::InputLimitErrorKind::SourceAstTooLarge => Self::SourceAstTooLarge,
            core::InputLimitErrorKind::ExpressionTooLarge => Self::ExpressionTooLarge,
            core::InputLimitErrorKind::IntegerTooLarge => Self::IntegerTooLarge,
            core::InputLimitErrorKind::OutputTooLarge => Self::OutputTooLarge,
            core::InputLimitErrorKind::InvalidSignificantDigits => Self::InvalidSignificantDigits,
            core::InputLimitErrorKind::InvalidResourceLimit => Self::InvalidResourceLimit,
        }
    }
}

impl From<core::ComputationLimitKind> for ComputationLimitCodeDto {
    fn from(value: core::ComputationLimitKind) -> Self {
        match value {
            core::ComputationLimitKind::AlgebraicDegree => Self::AlgebraicDegree,
            core::ComputationLimitKind::PolynomialCoefficientBits => {
                Self::PolynomialCoefficientBits
            }
            core::ComputationLimitKind::ResultantDegree => Self::ResultantDegree,
            core::ComputationLimitKind::FactorizationWork => Self::FactorizationWork,
            core::ComputationLimitKind::RootIsolationSteps => Self::RootIsolationSteps,
            core::ComputationLimitKind::RewriteSteps => Self::RewriteSteps,
            core::ComputationLimitKind::PrecisionBits => Self::PrecisionBits,
            core::ComputationLimitKind::RefinementRounds => Self::RefinementRounds,
            core::ComputationLimitKind::LogicalWorkUnits => Self::LogicalWorkUnits,
            core::ComputationLimitKind::PresentationNodes => Self::PresentationNodes,
        }
    }
}

impl From<core::UnsupportedFeature> for UnsupportedFeatureCodeDto {
    fn from(value: core::UnsupportedFeature) -> Self {
        match value {
            core::UnsupportedFeature::ComplexDomain => Self::ComplexDomain,
            core::UnsupportedFeature::PortableProofCertificate => Self::PortableProofCertificate,
            core::UnsupportedFeature::EvaluationEngine => Self::EvaluationEngine,
            core::UnsupportedFeature::ConstantEvaluation => Self::ConstantEvaluation,
            core::UnsupportedFeature::FunctionEvaluation => Self::FunctionEvaluation,
            core::UnsupportedFeature::NonIntegerPower => Self::NonIntegerPower,
        }
    }
}

impl From<core::InternalInvariantCode> for InternalInvariantCodeDto {
    fn from(value: core::InternalInvariantCode) -> Self {
        match value {
            core::InternalInvariantCode::NonCanonicalRational => Self::NonCanonicalRational,
            core::InternalInvariantCode::InvalidAlgebraicIsolation => {
                Self::InvalidAlgebraicIsolation
            }
            core::InternalInvariantCode::InvalidCertifiedInterval => Self::InvalidCertifiedInterval,
            core::InternalInvariantCode::NonDeterministicCacheAccounting => {
                Self::NonDeterministicCacheAccounting
            }
            core::InternalInvariantCode::PresentationWithoutEvaluation => {
                Self::PresentationWithoutEvaluation
            }
            core::InternalInvariantCode::InvalidParsedNumberLiteral => {
                Self::InvalidParsedNumberLiteral
            }
        }
    }
}

impl From<core::ProtocolVersion> for ProtocolVersionDto {
    fn from(value: core::ProtocolVersion) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
        }
    }
}

impl From<core::CalculationRequest> for CalculationRequestDto {
    fn from(value: core::CalculationRequest) -> Self {
        Self {
            parse: value.parse.into(),
            semantics: value.semantics.into(),
            exact_output: value.exact_output.into(),
            scientific_output: value.scientific_output.into(),
            enclosure_output: value.enclosure_output.into(),
            limits: value.limits.into(),
        }
    }
}

impl From<core::ParseSettings> for ParseSettingsDto {
    fn from(value: core::ParseSettings) -> Self {
        Self {
            grammar: value.grammar.into(),
            implicit_multiplication: value.implicit_multiplication.into(),
            unicode_aliases: value.unicode_aliases.into(),
            percent: value.percent.into(),
        }
    }
}

impl From<core::GrammarProfile> for GrammarProfileDto {
    fn from(value: core::GrammarProfile) -> Self {
        match value {
            core::GrammarProfile::Default => Self::Default,
        }
    }
}

impl From<core::ImplicitMultiplicationPolicy> for ImplicitMultiplicationPolicyDto {
    fn from(value: core::ImplicitMultiplicationPolicy) -> Self {
        match value {
            core::ImplicitMultiplicationPolicy::Enabled => Self::Enabled,
            core::ImplicitMultiplicationPolicy::Disabled => Self::Disabled,
        }
    }
}

impl From<core::UnicodeAliasPolicy> for UnicodeAliasPolicyDto {
    fn from(value: core::UnicodeAliasPolicy) -> Self {
        match value {
            core::UnicodeAliasPolicy::MathematicalAliases => Self::MathematicalAliases,
            core::UnicodeAliasPolicy::AsciiOnly => Self::AsciiOnly,
        }
    }
}

impl From<core::PercentParsePolicy> for PercentParsePolicyDto {
    fn from(value: core::PercentParsePolicy) -> Self {
        match value {
            core::PercentParsePolicy::PostfixPercent => Self::PostfixPercent,
            core::PercentParsePolicy::RejectPercent => Self::RejectPercent,
        }
    }
}

impl From<core::ExactOutputRequest> for ExactOutputRequestDto {
    fn from(value: core::ExactOutputRequest) -> Self {
        match value {
            core::ExactOutputRequest::Omit => Self::Omit,
            core::ExactOutputRequest::Include { format } => Self::Include {
                format: format.into(),
            },
        }
    }
}

impl From<core::ExactFormatPreference> for ExactFormatPreferenceDto {
    fn from(value: core::ExactFormatPreference) -> Self {
        match value {
            core::ExactFormatPreference::Auto => Self::Auto,
            core::ExactFormatPreference::Rational => Self::Rational,
            core::ExactFormatPreference::FiniteDecimal => Self::FiniteDecimal,
            core::ExactFormatPreference::MixedFraction => Self::MixedFraction,
            core::ExactFormatPreference::Symbolic => Self::Symbolic,
        }
    }
}

impl From<core::ScientificOutputRequest> for ScientificOutputRequestDto {
    fn from(value: core::ScientificOutputRequest) -> Self {
        match value {
            core::ScientificOutputRequest::Omit => Self::Omit,
            core::ScientificOutputRequest::Include {
                significant_digits,
                rounding_mode,
            } => Self::Include {
                significant_digits: significant_digits.get(),
                rounding_mode: rounding_mode.into(),
            },
        }
    }
}

impl From<core::EnclosureOutputRequest> for EnclosureOutputRequestDto {
    fn from(value: core::EnclosureOutputRequest) -> Self {
        match value {
            core::EnclosureOutputRequest::Omit => Self::Omit,
            core::EnclosureOutputRequest::Include { format } => Self::Include {
                format: format.into(),
            },
        }
    }
}

impl From<core::ResourceLimitRequest> for ResourceLimitRequestDto {
    fn from(value: core::ResourceLimitRequest) -> Self {
        match value {
            core::ResourceLimitRequest::Default => Self::Default,
            core::ResourceLimitRequest::Custom(value) => Self::Custom {
                value: ResourceLimitsDto {
                    max_input_bytes: value.max_input_bytes,
                    max_source_ast_nodes: value.max_source_ast_nodes,
                    max_source_depth: value.max_source_depth,
                    max_expression_nodes: value.max_expression_nodes,
                    max_integer_bits: value.max_integer_bits,
                    max_cyclotomic_order: value.max_cyclotomic_order,
                    max_algebraic_degree: value.max_algebraic_degree,
                    max_polynomial_coefficient_bits: value.max_polynomial_coefficient_bits,
                    max_resultant_degree: value.max_resultant_degree,
                    max_factorization_work: value.max_factorization_work,
                    max_root_isolation_steps: value.max_root_isolation_steps,
                    max_rewrite_steps: value.max_rewrite_steps,
                    max_precision_bits: value.max_precision_bits,
                    max_refinement_rounds: value.max_refinement_rounds,
                    max_logical_work_units: value.max_logical_work_units.to_string(),
                    max_presentation_nodes: value.max_presentation_nodes,
                    max_output_bytes: value.max_output_bytes,
                },
            },
        }
    }
}

impl TryFrom<CalculationOutcomeDto> for core::CalculationOutcome {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculationOutcomeDto) -> Result<Self, Self::Error> {
        match value {
            CalculationOutcomeDto::Complete { calculation } => {
                Ok(Self::Complete(calculation.try_into()?))
            }
            CalculationOutcomeDto::Partial {
                calculation,
                reason,
                certified_enclosure,
            } => Ok(Self::Partial {
                calculation: calculation.try_into()?,
                reason: reason.try_into()?,
                certified_enclosure: certified_enclosure.map(TryInto::try_into).transpose()?,
            }),
        }
    }
}

impl TryFrom<CalculationDto> for core::Calculation {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculationDto) -> Result<Self, Self::Error> {
        Ok(Self {
            exact: value.exact.try_into()?,
            scientific: value.scientific.try_into()?,
            enclosure: value.enclosure.try_into()?,
            metadata: value.metadata.try_into()?,
        })
    }
}

impl TryFrom<ExactOutputDto> for core::ExactOutput {
    type Error = CalculatorErrorDto;

    fn try_from(value: ExactOutputDto) -> Result<Self, Self::Error> {
        match value {
            ExactOutputDto::Omitted => Ok(Self::Omitted),
            ExactOutputDto::Included { value } => Ok(Self::Included(value.try_into()?)),
        }
    }
}

impl TryFrom<ExactPresentationDto> for core::ExactPresentation {
    type Error = CalculatorErrorDto;

    fn try_from(value: ExactPresentationDto) -> Result<Self, Self::Error> {
        Ok(Self {
            relation: value.relation.into(),
            representation: value.representation.into(),
            presentation: value.presentation.try_into()?,
            plain_text: value.plain_text,
        })
    }
}

impl TryFrom<ScientificOutputDto> for core::ScientificOutput {
    type Error = CalculatorErrorDto;

    fn try_from(value: ScientificOutputDto) -> Result<Self, Self::Error> {
        match value {
            ScientificOutputDto::Omitted => Ok(Self::Omitted),
            ScientificOutputDto::Included { value } => Ok(Self::Included(value.try_into()?)),
            ScientificOutputDto::Unavailable { value } => Ok(Self::Unavailable(value.try_into()?)),
        }
    }
}

impl TryFrom<ScientificPresentationDto> for core::ScientificPresentation {
    type Error = CalculatorErrorDto;

    fn try_from(value: ScientificPresentationDto) -> Result<Self, Self::Error> {
        let Some(requested_significant_digits) =
            NonZeroU32::new(value.requested_significant_digits)
        else {
            return Err(input_limit_error(
                InputLimitErrorCodeDto::InvalidSignificantDigits,
            ));
        };
        Ok(Self {
            relation: value.relation.into(),
            significand: value.significand,
            exponent_ten: value.exponent_ten,
            requested_significant_digits,
            confirmed_significant_digits: value.confirmed_significant_digits,
            rounding_mode: value.rounding_mode.into(),
            presentation: value.presentation.try_into()?,
        })
    }
}

impl TryFrom<UnavailableScientificOutputDto> for core::UnavailableScientificOutput {
    type Error = CalculatorErrorDto;

    fn try_from(value: UnavailableScientificOutputDto) -> Result<Self, Self::Error> {
        let Some(requested_significant_digits) =
            NonZeroU32::new(value.requested_significant_digits)
        else {
            return Err(input_limit_error(
                InputLimitErrorCodeDto::InvalidSignificantDigits,
            ));
        };
        Ok(Self {
            requested_significant_digits,
            confirmed_significant_digits: value.confirmed_significant_digits,
            rounding_mode: value.rounding_mode.into(),
            reason: value.reason.try_into()?,
        })
    }
}

impl TryFrom<EnclosureOutputDto> for core::EnclosureOutput {
    type Error = CalculatorErrorDto;

    fn try_from(value: EnclosureOutputDto) -> Result<Self, Self::Error> {
        match value {
            EnclosureOutputDto::Omitted => Ok(Self::Omitted),
            EnclosureOutputDto::Included { value } => Ok(Self::Included(value.try_into()?)),
            EnclosureOutputDto::Unavailable { reason } => {
                Ok(Self::Unavailable(core::UnavailableEnclosureOutput {
                    reason: reason.try_into()?,
                }))
            }
        }
    }
}

impl TryFrom<CertifiedIntervalPresentationDto> for core::CertifiedIntervalPresentation {
    type Error = CalculatorErrorDto;

    fn try_from(value: CertifiedIntervalPresentationDto) -> Result<Self, Self::Error> {
        Ok(Self {
            relation: value.relation.into(),
            bounds: value.bounds.try_into()?,
            presentation: value.presentation.try_into()?,
        })
    }
}

impl TryFrom<CertifiedIntervalBoundsDto> for core::CertifiedIntervalBounds {
    type Error = CalculatorErrorDto;

    fn try_from(value: CertifiedIntervalBoundsDto) -> Result<Self, Self::Error> {
        match value {
            CertifiedIntervalBoundsDto::ExactDyadic { lower, upper } => Ok(Self::ExactDyadic {
                lower: lower.try_into()?,
                upper: upper.try_into()?,
            }),
            CertifiedIntervalBoundsDto::DecimalScientific {
                lower,
                upper,
                requested_significant_digits,
            } => {
                let Some(requested_significant_digits) =
                    NonZeroU32::new(requested_significant_digits)
                else {
                    return Err(input_limit_error(
                        InputLimitErrorCodeDto::InvalidSignificantDigits,
                    ));
                };
                Ok(Self::DecimalScientific {
                    lower: lower.into(),
                    upper: upper.into(),
                    requested_significant_digits,
                })
            }
        }
    }
}

impl From<DecimalScientificBoundDto> for core::DecimalScientificBound {
    fn from(value: DecimalScientificBoundDto) -> Self {
        Self {
            significand: value.significand,
            exponent_ten: value.exponent_ten,
        }
    }
}

impl TryFrom<ExactDyadicDto> for core::ExactDyadic {
    type Error = CalculatorErrorDto;

    fn try_from(value: ExactDyadicDto) -> Result<Self, Self::Error> {
        Ok(Self {
            coefficient: parse_integer(&value.coefficient)?,
            exponent_two: parse_integer(&value.exponent_two)?,
        })
    }
}

impl TryFrom<CalculationMetadataDto> for core::CalculationMetadata {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculationMetadataDto) -> Result<Self, Self::Error> {
        Ok(Self {
            exact_representation: value.exact_representation.into(),
            simplification_status: value.simplification_status.try_into()?,
            semantic_settings: value.semantic_settings.into(),
            methods: value.methods.into_iter().map(Into::into).collect(),
            internal_precision_bits: value.internal_precision_bits,
            refinement_rounds: value.refinement_rounds,
            confirmed_significant_digits: value.confirmed_significant_digits,
            assurance: value.assurance.into(),
            protocol_version: value.protocol_version.into(),
        })
    }
}

impl From<ProtocolVersionDto> for core::ProtocolVersion {
    fn from(value: ProtocolVersionDto) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
        }
    }
}

impl TryFrom<IncompleteReasonDto> for core::IncompleteReason {
    type Error = CalculatorErrorDto;

    fn try_from(value: IncompleteReasonDto) -> Result<Self, Self::Error> {
        match value {
            IncompleteReasonDto::PrecisionLimit {
                requested_digits,
                confirmed_digits,
            } => {
                let Some(requested_digits) = NonZeroU32::new(requested_digits) else {
                    return Err(input_limit_error(
                        InputLimitErrorCodeDto::InvalidSignificantDigits,
                    ));
                };
                Ok(Self::PrecisionLimit {
                    requested_digits,
                    confirmed_digits,
                })
            }
            IncompleteReasonDto::ComputationLimit { kind } => {
                Ok(Self::ComputationLimit { kind: kind.into() })
            }
            IncompleteReasonDto::UnsupportedFeature { feature } => Ok(Self::UnsupportedFeature {
                feature: feature.into(),
            }),
        }
    }
}

impl TryFrom<SimplificationStatusDto> for core::SimplificationStatus {
    type Error = CalculatorErrorDto;

    fn try_from(value: SimplificationStatusDto) -> Result<Self, Self::Error> {
        match value {
            SimplificationStatusDto::FullySimplifiedWithinLimits => {
                Ok(Self::FullySimplifiedWithinLimits)
            }
            SimplificationStatusDto::PartiallySimplified { reason } => {
                Ok(Self::PartiallySimplified {
                    reason: reason.try_into()?,
                })
            }
        }
    }
}

impl TryFrom<PresentationNodeDto> for core::PresentationNode {
    type Error = CalculatorErrorDto;

    fn try_from(value: PresentationNodeDto) -> Result<Self, Self::Error> {
        match value {
            PresentationNodeDto::Text { text } => Ok(Self::Text(text)),
            PresentationNodeDto::Row { children } => Ok(Self::Row(
                children
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            )),
            PresentationNodeDto::Fraction {
                numerator,
                denominator,
            } => Ok(Self::Fraction {
                numerator: Box::new((*numerator).try_into()?),
                denominator: Box::new((*denominator).try_into()?),
            }),
            PresentationNodeDto::Superscript { base, exponent } => Ok(Self::Superscript {
                base: Box::new((*base).try_into()?),
                exponent: Box::new((*exponent).try_into()?),
            }),
            PresentationNodeDto::Subscript { base, subscript } => Ok(Self::Subscript {
                base: Box::new((*base).try_into()?),
                subscript: Box::new((*subscript).try_into()?),
            }),
            PresentationNodeDto::Radical { index, radicand } => Ok(Self::Radical {
                index: index.try_into()?,
                radicand: Box::new((*radicand).try_into()?),
            }),
            PresentationNodeDto::Function { name, argument } => Ok(Self::Function {
                name: name.into(),
                argument: Box::new((*argument).try_into()?),
            }),
            PresentationNodeDto::Parenthesized { value } => {
                Ok(Self::Parenthesized(Box::new((*value).try_into()?)))
            }
        }
    }
}

impl TryFrom<RadicalIndexDto> for core::RadicalIndex {
    type Error = CalculatorErrorDto;

    fn try_from(value: RadicalIndexDto) -> Result<Self, Self::Error> {
        match value {
            RadicalIndexDto::Square => Ok(Self::Square),
            RadicalIndexDto::Nth { value } => {
                let integer = parse_integer(&value)?;
                let Some(value) = core::PositiveInteger::new(integer) else {
                    return Err(input_limit_error(
                        InputLimitErrorCodeDto::InvalidResourceLimit,
                    ));
                };
                Ok(Self::Nth(value))
            }
        }
    }
}

impl From<ResultRelationDto> for core::ResultRelation {
    fn from(value: ResultRelationDto) -> Self {
        match value {
            ResultRelationDto::ExactEqual => Self::ExactEqual,
            ResultRelationDto::ApproximatelyEqual => Self::ApproximatelyEqual,
            ResultRelationDto::ElementOf => Self::ElementOf,
        }
    }
}

impl From<ExactRepresentationKindDto> for core::ExactRepresentationKind {
    fn from(value: ExactRepresentationKindDto) -> Self {
        match value {
            ExactRepresentationKindDto::Integer => Self::Integer,
            ExactRepresentationKindDto::Rational => Self::Rational,
            ExactRepresentationKindDto::FiniteDecimal => Self::FiniteDecimal,
            ExactRepresentationKindDto::RationalPiMultiple => Self::RationalPiMultiple,
            ExactRepresentationKindDto::Radical => Self::Radical,
            ExactRepresentationKindDto::RealAlgebraic => Self::RealAlgebraic,
            ExactRepresentationKindDto::GeneralSymbolic => Self::GeneralSymbolic,
        }
    }
}

impl From<FunctionNameDto> for core::FunctionName {
    fn from(value: FunctionNameDto) -> Self {
        match value {
            FunctionNameDto::Sin => Self::Sin,
            FunctionNameDto::Cos => Self::Cos,
            FunctionNameDto::Tan => Self::Tan,
            FunctionNameDto::Asin => Self::Asin,
            FunctionNameDto::Acos => Self::Acos,
            FunctionNameDto::Atan => Self::Atan,
            FunctionNameDto::Sqrt => Self::Sqrt,
            FunctionNameDto::Exp => Self::Exp,
            FunctionNameDto::Log => Self::Log,
            FunctionNameDto::Ln => Self::Ln,
        }
    }
}

impl From<MethodTagDto> for core::MethodTag {
    fn from(value: MethodTagDto) -> Self {
        match value {
            MethodTagDto::RationalReduction => Self::RationalReduction,
            MethodTagDto::RadicalExtraction => Self::RadicalExtraction,
            MethodTagDto::SpecialAngle => Self::SpecialAngle,
            MethodTagDto::CyclotomicReduction => Self::CyclotomicReduction,
            MethodTagDto::AlgebraicMinimalPolynomial => Self::AlgebraicMinimalPolynomial,
            MethodTagDto::AlgebraicRootIsolation => Self::AlgebraicRootIsolation,
            MethodTagDto::SymbolicRetention => Self::SymbolicRetention,
            MethodTagDto::CertifiedIntervalEvaluation => Self::CertifiedIntervalEvaluation,
            MethodTagDto::AdaptivePrecisionRefinement => Self::AdaptivePrecisionRefinement,
        }
    }
}

impl From<AssuranceLevelDto> for core::AssuranceLevel {
    fn from(value: AssuranceLevelDto) -> Self {
        match value {
            AssuranceLevelDto::Exact => Self::Exact,
            AssuranceLevelDto::CertifiedEnclosure => Self::CertifiedEnclosure,
        }
    }
}

impl From<ComputationLimitCodeDto> for core::ComputationLimitKind {
    fn from(value: ComputationLimitCodeDto) -> Self {
        match value {
            ComputationLimitCodeDto::AlgebraicDegree => Self::AlgebraicDegree,
            ComputationLimitCodeDto::PolynomialCoefficientBits => Self::PolynomialCoefficientBits,
            ComputationLimitCodeDto::ResultantDegree => Self::ResultantDegree,
            ComputationLimitCodeDto::FactorizationWork => Self::FactorizationWork,
            ComputationLimitCodeDto::RootIsolationSteps => Self::RootIsolationSteps,
            ComputationLimitCodeDto::RewriteSteps => Self::RewriteSteps,
            ComputationLimitCodeDto::PrecisionBits => Self::PrecisionBits,
            ComputationLimitCodeDto::RefinementRounds => Self::RefinementRounds,
            ComputationLimitCodeDto::LogicalWorkUnits => Self::LogicalWorkUnits,
            ComputationLimitCodeDto::PresentationNodes => Self::PresentationNodes,
        }
    }
}

impl TryFrom<InputPolicyDto> for core::InputPolicy {
    type Error = CalculatorErrorDto;

    fn try_from(value: InputPolicyDto) -> Result<Self, Self::Error> {
        Ok(Self {
            calculation_request: value.calculation_request.try_into()?,
            percent_policy: value.percent_policy.into(),
        })
    }
}

impl From<PercentPolicyDto> for core::PercentPolicy {
    fn from(value: PercentPolicyDto) -> Self {
        match value {
            PercentPolicyDto::ExpressionPercent => Self::ExpressionPercent,
            PercentPolicyDto::CalculatorPercent => Self::CalculatorPercent,
        }
    }
}

impl From<core::InputState> for SessionStateDto {
    fn from(value: core::InputState) -> Self {
        SessionStateDto::from(&value)
    }
}

impl From<&core::InputState> for SessionStateDto {
    fn from(value: &core::InputState) -> Self {
        Self {
            source: value.source().to_owned(),
            cursor_utf16: utf8_to_utf16_units(value.source(), value.cursor_utf8()),
            selection_utf16: optional_span_to_utf16(value.source(), value.selection_utf8()),
            has_ans: value.has_ans(),
            has_memory: value.has_memory(),
            display: value.display().clone().into(),
        }
    }
}

impl From<core::SessionDisplay> for SessionDisplayDto {
    fn from(value: core::SessionDisplay) -> Self {
        match value {
            core::SessionDisplay::Editing => Self::Editing,
            core::SessionDisplay::Result { calculation } => Self::Result {
                calculation: Box::new((*calculation).into()),
            },
            core::SessionDisplay::Error { error } => Self::Error {
                error: error.into(),
            },
            core::SessionDisplay::Calculating => Self::Calculating,
        }
    }
}

impl From<core::SessionReduction> for SessionDispatchResultDto {
    fn from(value: core::SessionReduction) -> Self {
        let state = SessionStateDto::from(&value.state);
        match value.command {
            core::SessionCommand::None => Self::State { state },
            core::SessionCommand::Calculate { source, request } => Self::Calculate {
                state,
                source,
                request: request.into(),
            },
        }
    }
}

impl From<core::InputError> for InputErrorDto {
    fn from(value: core::InputError) -> Self {
        Self {
            code: value.kind.into(),
        }
    }
}

impl From<core::InputErrorKind> for InputErrorCodeDto {
    fn from(value: core::InputErrorKind) -> Self {
        match value {
            core::InputErrorKind::InvalidDigit => Self::InvalidDigit,
            core::InputErrorKind::InvalidCursor => Self::InvalidCursor,
            core::InputErrorKind::SelectionOutOfBounds => Self::SelectionOutOfBounds,
            core::InputErrorKind::ActionNotAllowedAfterError => Self::ActionNotAllowedAfterError,
            core::InputErrorKind::MemoryEmpty => Self::MemoryEmpty,
        }
    }
}

impl TryFrom<CalculatorErrorDto> for core::CalculatorError {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculatorErrorDto) -> Result<Self, Self::Error> {
        match value {
            CalculatorErrorDto::Parse {
                code,
                span,
                expected,
            } => Ok(Self::Parse(core::ParseError {
                kind: code.into(),
                span: span.into(),
                expected: expected.into_iter().map(Into::into).collect(),
            })),
            CalculatorErrorDto::Domain { code, span } => Ok(Self::Domain(core::DomainError {
                kind: code.into(),
                span: span.into(),
            })),
            CalculatorErrorDto::InputLimit { code } => {
                Ok(Self::InputLimit(core::InputLimitError {
                    kind: code.into(),
                }))
            }
            CalculatorErrorDto::ComputationLimit { code } => {
                Ok(Self::ComputationLimit(core::ComputationLimitError {
                    kind: code.into(),
                }))
            }
            CalculatorErrorDto::UnsupportedFeature { code } => {
                Ok(Self::UnsupportedFeature(core::UnsupportedFeatureError {
                    feature: code.into(),
                }))
            }
            CalculatorErrorDto::InternalInvariant { code } => {
                Ok(Self::InternalInvariant(core::InternalInvariantError {
                    code: code.into(),
                }))
            }
            CalculatorErrorDto::UnsupportedProtocol { .. } => Err(value),
        }
    }
}

impl From<TextSpanDto> for core::ByteSpan {
    fn from(value: TextSpanDto) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<OptionalTextSpanDto> for Option<core::ByteSpan> {
    fn from(value: OptionalTextSpanDto) -> Self {
        match value {
            OptionalTextSpanDto::None => None,
            OptionalTextSpanDto::Some { value } => Some(value.into()),
        }
    }
}

impl From<ExpectedTokenDto> for core::ExpectedToken {
    fn from(value: ExpectedTokenDto) -> Self {
        Self {
            kind: value.kind.into(),
        }
    }
}

impl From<ExpectedTokenKindDto> for core::ExpectedTokenKind {
    fn from(value: ExpectedTokenKindDto) -> Self {
        match value {
            ExpectedTokenKindDto::Number => Self::Number,
            ExpectedTokenKindDto::Identifier => Self::Identifier,
            ExpectedTokenKindDto::Operator => Self::Operator,
            ExpectedTokenKindDto::OpenParenthesis => Self::OpenParenthesis,
            ExpectedTokenKindDto::CloseParenthesis => Self::CloseParenthesis,
            ExpectedTokenKindDto::Comma => Self::Comma,
            ExpectedTokenKindDto::EndOfInput => Self::EndOfInput,
        }
    }
}

impl From<ParseErrorCodeDto> for core::ParseErrorKind {
    fn from(value: ParseErrorCodeDto) -> Self {
        match value {
            ParseErrorCodeDto::UnexpectedToken => Self::UnexpectedToken,
            ParseErrorCodeDto::UnexpectedEnd => Self::UnexpectedEnd,
            ParseErrorCodeDto::UnknownIdentifier => Self::UnknownIdentifier,
            ParseErrorCodeDto::InvalidNumberLiteral => Self::InvalidNumberLiteral,
            ParseErrorCodeDto::MissingFunctionParenthesis => Self::MissingFunctionParenthesis,
            ParseErrorCodeDto::ImplicitMultiplicationDisabled => {
                Self::ImplicitMultiplicationDisabled
            }
            ParseErrorCodeDto::PercentRejected => Self::PercentRejected,
        }
    }
}

impl From<DomainErrorCodeDto> for core::DomainErrorKind {
    fn from(value: DomainErrorCodeDto) -> Self {
        match value {
            DomainErrorCodeDto::DivisionByZero => Self::DivisionByZero,
            DomainErrorCodeDto::LogarithmOfNonPositive => Self::LogarithmOfNonPositive,
            DomainErrorCodeDto::LogarithmBaseOne => Self::LogarithmBaseOne,
            DomainErrorCodeDto::EvenRootOfNegative => Self::EvenRootOfNegative,
            DomainErrorCodeDto::InverseTrigonometricOutOfRange => {
                Self::InverseTrigonometricOutOfRange
            }
            DomainErrorCodeDto::TangentPole => Self::TangentPole,
            DomainErrorCodeDto::ZeroToNegativePower => Self::ZeroToNegativePower,
            DomainErrorCodeDto::IndeterminateZeroToZero => Self::IndeterminateZeroToZero,
            DomainErrorCodeDto::NonRealPower => Self::NonRealPower,
            DomainErrorCodeDto::IntegerFunctionRequiresInteger => {
                Self::IntegerFunctionRequiresInteger
            }
            DomainErrorCodeDto::IntegerFunctionRequiresNonNegative => {
                Self::IntegerFunctionRequiresNonNegative
            }
        }
    }
}

impl From<InputLimitErrorCodeDto> for core::InputLimitErrorKind {
    fn from(value: InputLimitErrorCodeDto) -> Self {
        match value {
            InputLimitErrorCodeDto::InputTooLong => Self::InputTooLong,
            InputLimitErrorCodeDto::SourceAstTooDeep => Self::SourceAstTooDeep,
            InputLimitErrorCodeDto::SourceAstTooLarge => Self::SourceAstTooLarge,
            InputLimitErrorCodeDto::ExpressionTooLarge => Self::ExpressionTooLarge,
            InputLimitErrorCodeDto::IntegerTooLarge => Self::IntegerTooLarge,
            InputLimitErrorCodeDto::OutputTooLarge => Self::OutputTooLarge,
            InputLimitErrorCodeDto::InvalidSignificantDigits => Self::InvalidSignificantDigits,
            InputLimitErrorCodeDto::InvalidResourceLimit => Self::InvalidResourceLimit,
        }
    }
}

impl From<UnsupportedFeatureCodeDto> for core::UnsupportedFeature {
    fn from(value: UnsupportedFeatureCodeDto) -> Self {
        match value {
            UnsupportedFeatureCodeDto::ComplexDomain => Self::ComplexDomain,
            UnsupportedFeatureCodeDto::PortableProofCertificate => Self::PortableProofCertificate,
            UnsupportedFeatureCodeDto::EvaluationEngine => Self::EvaluationEngine,
            UnsupportedFeatureCodeDto::ConstantEvaluation => Self::ConstantEvaluation,
            UnsupportedFeatureCodeDto::FunctionEvaluation => Self::FunctionEvaluation,
            UnsupportedFeatureCodeDto::NonIntegerPower => Self::NonIntegerPower,
        }
    }
}

impl From<InternalInvariantCodeDto> for core::InternalInvariantCode {
    fn from(value: InternalInvariantCodeDto) -> Self {
        match value {
            InternalInvariantCodeDto::NonCanonicalRational => Self::NonCanonicalRational,
            InternalInvariantCodeDto::InvalidAlgebraicIsolation => Self::InvalidAlgebraicIsolation,
            InternalInvariantCodeDto::InvalidCertifiedInterval => Self::InvalidCertifiedInterval,
            InternalInvariantCodeDto::NonDeterministicCacheAccounting => {
                Self::NonDeterministicCacheAccounting
            }
            InternalInvariantCodeDto::PresentationWithoutEvaluation => {
                Self::PresentationWithoutEvaluation
            }
            InternalInvariantCodeDto::InvalidParsedNumberLiteral => {
                Self::InvalidParsedNumberLiteral
            }
        }
    }
}

impl From<InputActionDto> for core::InputAction {
    fn from(value: InputActionDto) -> Self {
        match value {
            InputActionDto::Digit { value } => Self::Digit(u8::try_from(value).unwrap_or(u8::MAX)),
            InputActionDto::DecimalPoint => Self::DecimalPoint,
            InputActionDto::Constant { value } => match value {
                ConstantDto::Pi => Self::Constant(core::Constant::Pi),
                ConstantDto::E => Self::Constant(core::Constant::Euler),
                ConstantDto::Ans => Self::Ans,
                ConstantDto::Memory => Self::MemoryRecall,
            },
            InputActionDto::Function { value } => Self::Function(value.into()),
            InputActionDto::BinaryOperator { value } => Self::BinaryOperator(value.into()),
            InputActionDto::Comma => Self::Comma,
            InputActionDto::Percent => Self::Percent,
            InputActionDto::OpenParenthesis => Self::OpenParenthesis,
            InputActionDto::CloseParenthesis => Self::CloseParenthesis,
            InputActionDto::DeleteBackward => Self::DeleteBackward,
            InputActionDto::ClearEntry => Self::ClearEntry,
            InputActionDto::ClearAll => Self::ClearAll,
            InputActionDto::MemoryClear => Self::MemoryClear,
            InputActionDto::MemoryRecall => Self::MemoryRecall,
            InputActionDto::MemoryAdd => Self::MemoryAdd,
            InputActionDto::MemorySubtract => Self::MemorySubtract,
            InputActionDto::Evaluate => Self::Evaluate,
        }
    }
}

impl From<FunctionDto> for core::Function {
    fn from(value: FunctionDto) -> Self {
        match value {
            FunctionDto::Sin => Self::Sin,
            FunctionDto::Cos => Self::Cos,
            FunctionDto::Tan => Self::Tan,
            FunctionDto::Asin => Self::Asin,
            FunctionDto::Acos => Self::Acos,
            FunctionDto::Atan => Self::Atan,
            FunctionDto::Sqrt => Self::Sqrt,
            FunctionDto::Exp => Self::Exp,
            FunctionDto::Log => Self::Log,
            FunctionDto::Ln => Self::Ln,
        }
    }
}

impl From<BinaryOperatorDto> for core::BinaryOperator {
    fn from(value: BinaryOperatorDto) -> Self {
        match value {
            BinaryOperatorDto::Add => Self::Add,
            BinaryOperatorDto::Subtract => Self::Subtract,
            BinaryOperatorDto::Multiply => Self::Multiply,
            BinaryOperatorDto::Divide => Self::Divide,
            BinaryOperatorDto::Power => Self::Power,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn unsupported_protocol_error(code: UnsupportedProtocolCodeDto) -> CalculatorErrorDto {
    CalculatorErrorDto::UnsupportedProtocol { code }
}

pub fn input_limit_error(code: InputLimitErrorCodeDto) -> CalculatorErrorDto {
    CalculatorErrorDto::InputLimit { code }
}

fn parse_u64(value: &str) -> Result<u64, CalculatorErrorDto> {
    if !is_unsigned_decimal_string(value) {
        return Err(input_limit_error(
            InputLimitErrorCodeDto::InvalidResourceLimit,
        ));
    }
    u64::from_str(value)
        .map_err(|_| input_limit_error(InputLimitErrorCodeDto::InvalidResourceLimit))
}

fn parse_integer(value: &str) -> Result<core::Integer, CalculatorErrorDto> {
    if !is_signed_decimal_string(value) {
        return Err(input_limit_error(
            InputLimitErrorCodeDto::InvalidResourceLimit,
        ));
    }
    let Some(integer) = BigInt::parse_bytes(value.as_bytes(), 10) else {
        return Err(input_limit_error(
            InputLimitErrorCodeDto::InvalidResourceLimit,
        ));
    };
    Ok(core::Integer::from_bigint(integer))
}

fn is_unsigned_decimal_string(value: &str) -> bool {
    let bytes = value.as_bytes();
    match bytes {
        [b'0'] => true,
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

fn is_signed_decimal_string(value: &str) -> bool {
    match value.strip_prefix('-') {
        Some(rest) => rest != "0" && is_unsigned_decimal_string(rest),
        None => is_unsigned_decimal_string(value),
    }
}

fn utf8_to_utf16_units(source: &str, utf8_offset: u32) -> u32 {
    let utf8_offset = utf8_offset as usize;
    source[..utf8_offset].encode_utf16().count() as u32
}

fn optional_span_to_utf16(source: &str, span: &core::OptionalTextSpan) -> OptionalTextSpanDto {
    match span {
        core::OptionalTextSpan::None => OptionalTextSpanDto::None,
        core::OptionalTextSpan::Some(span) => OptionalTextSpanDto::Some {
            value: TextSpanDto {
                start: utf8_to_utf16_units(source, span.start),
                end: utf8_to_utf16_units(source, span.end),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_offsets_convert_to_utf16_code_units() {
        let source = "π𝄞x";
        assert_eq!(utf8_to_utf16_units(source, 0), 0);
        assert_eq!(utf8_to_utf16_units(source, "π".len() as u32), 1);
        assert_eq!(utf8_to_utf16_units(source, "π𝄞".len() as u32), 3);
        assert_eq!(utf8_to_utf16_units(source, source.len() as u32), 4);
    }

    #[test]
    fn optional_span_to_utf16_converts_utf8_byte_span() {
        let span = core::OptionalTextSpan::Some(core::ByteSpan {
            start: "π".len() as u32,
            end: "π𝄞".len() as u32,
        });
        assert_eq!(
            optional_span_to_utf16("π𝄞x", &span),
            OptionalTextSpanDto::Some {
                value: TextSpanDto { start: 1, end: 3 },
            }
        );
    }

    #[test]
    fn unsigned_decimal_parser_rejects_non_canonical_resource_limits() {
        for value in [
            "", "00", "01", "-0", "-1", "+1", "1.0", "1e2", "NaN", "Infinity",
        ] {
            assert_eq!(
                parse_u64(value),
                Err(input_limit_error(
                    InputLimitErrorCodeDto::InvalidResourceLimit
                )),
                "{value} must not be accepted as an unsigned decimal resource limit"
            );
        }
        assert_eq!(parse_u64("0"), Ok(0));
        assert_eq!(parse_u64("123"), Ok(123));
    }

    #[test]
    fn signed_decimal_parser_rejects_non_canonical_integer_strings() {
        for value in [
            "", "00", "01", "-0", "-01", "+1", "1.0", "1e2", "NaN", "Infinity",
        ] {
            assert_eq!(
                parse_integer(value),
                Err(input_limit_error(
                    InputLimitErrorCodeDto::InvalidResourceLimit
                )),
                "{value} must not be accepted as a signed decimal integer"
            );
        }
        assert_eq!(parse_integer("0").unwrap().to_string(), "0");
        assert_eq!(parse_integer("-123").unwrap().to_string(), "-123");
        assert_eq!(parse_integer("123").unwrap().to_string(), "123");
    }
}
