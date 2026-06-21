use std::{num::NonZeroU32, str::FromStr};

use calculator_core as core;

use crate::dto::*;

impl TryFrom<CalculationRequestDto> for core::CalculationRequest {
    type Error = CalculatorErrorDto;

    fn try_from(value: CalculationRequestDto) -> Result<Self, Self::Error> {
        Ok(Self {
            parse: value.parse.into(),
            semantics: value.semantics.into(),
            exact_output: value.exact_output.into(),
            scientific_output: value.scientific_output.try_into()?,
            enclosure_output: value.enclosure_output.into(),
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

impl From<EnclosureOutputRequestDto> for core::EnclosureOutputRequest {
    fn from(value: EnclosureOutputRequestDto) -> Self {
        match value {
            EnclosureOutputRequestDto::Omit => Self::Omit,
            EnclosureOutputRequestDto::Include { format } => Self::Include {
                format: format.into(),
            },
        }
    }
}

impl From<EnclosureFormatDto> for core::EnclosureFormat {
    fn from(value: EnclosureFormatDto) -> Self {
        match value {
            EnclosureFormatDto::ExactDyadic => Self::ExactDyadic,
        }
    }
}

impl From<core::EnclosureFormat> for EnclosureFormatDto {
    fn from(value: core::EnclosureFormat) -> Self {
        match value {
            core::EnclosureFormat::ExactDyadic => Self::ExactDyadic,
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
                    max_logical_work_units: parse_u64(&value.max_logical_work_units)?,
                    ..core::ResourceLimits::default()
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
                certified_enclosure: certified_enclosure.into(),
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
            lower: value.lower.into(),
            upper: value.upper.into(),
            format: value.format.into(),
            presentation: value.presentation.into(),
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
            core::DomainErrorKind::EvenRootOfNegative => Self::EvenRootOfNegative,
            core::DomainErrorKind::InverseTrigonometricOutOfRange => {
                Self::InverseTrigonometricOutOfRange
            }
            core::DomainErrorKind::TangentPole => Self::TangentPole,
            core::DomainErrorKind::ZeroToNegativePower => Self::ZeroToNegativePower,
            core::DomainErrorKind::IndeterminateZeroToZero => Self::IndeterminateZeroToZero,
            core::DomainErrorKind::NonRealPower => Self::NonRealPower,
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

#[cfg(target_arch = "wasm32")]
pub fn unsupported_protocol_error(code: UnsupportedProtocolCodeDto) -> CalculatorErrorDto {
    CalculatorErrorDto::UnsupportedProtocol { code }
}

pub fn input_limit_error(code: InputLimitErrorCodeDto) -> CalculatorErrorDto {
    CalculatorErrorDto::InputLimit { code }
}

fn parse_u64(value: &str) -> Result<u64, CalculatorErrorDto> {
    if value.is_empty() || value.bytes().any(|byte| !byte.is_ascii_digit()) {
        return Err(input_limit_error(
            InputLimitErrorCodeDto::InvalidResourceLimit,
        ));
    }
    u64::from_str(value)
        .map_err(|_| input_limit_error(InputLimitErrorCodeDto::InvalidResourceLimit))
}
