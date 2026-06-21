use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersionDto {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ApiResultDto<T> {
    #[serde(rename = "ok")]
    Ok { value: T },
    #[serde(rename = "error")]
    Error { error: CalculatorErrorDto },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationRequestDto {
    pub parse: ParseSettingsDto,
    pub semantics: SemanticSettingsDto,
    pub exact_output: ExactOutputRequestDto,
    pub scientific_output: ScientificOutputRequestDto,
    pub enclosure_output: EnclosureOutputRequestDto,
    pub limits: ResourceLimitRequestDto,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseSettingsDto {
    pub grammar: GrammarProfileDto,
    pub implicit_multiplication: ImplicitMultiplicationPolicyDto,
    pub unicode_aliases: UnicodeAliasPolicyDto,
    pub percent: PercentParsePolicyDto,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GrammarProfileDto {
    Default,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImplicitMultiplicationPolicyDto {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnicodeAliasPolicyDto {
    MathematicalAliases,
    AsciiOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PercentParsePolicyDto {
    PostfixPercent,
    RejectPercent,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSettingsDto {
    pub domain: EvaluationDomainDto,
    pub angle_unit: AngleUnitDto,
    pub power_semantics: PowerSemanticsDto,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvaluationDomainDto {
    Real,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AngleUnitDto {
    Radian,
    Degree,
    Gradian,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PowerSemanticsDto {
    RealPrincipal,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ExactOutputRequestDto {
    #[serde(rename = "omit")]
    Omit,
    #[serde(rename = "include")]
    Include { format: ExactFormatPreferenceDto },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExactFormatPreferenceDto {
    Auto,
    Rational,
    FiniteDecimal,
    MixedFraction,
    Symbolic,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ScientificOutputRequestDto {
    #[serde(rename = "omit")]
    Omit,
    #[serde(rename = "include")]
    Include {
        significant_digits: u32,
        rounding_mode: DecimalRoundingModeDto,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DecimalRoundingModeDto {
    NearestTiesToEven,
    NearestTiesAwayFromZero,
    TowardPositiveInfinity,
    TowardNegativeInfinity,
    TowardZero,
    AwayFromZero,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum EnclosureOutputRequestDto {
    #[serde(rename = "omit")]
    Omit,
    #[serde(rename = "include")]
    Include { format: EnclosureFormatDto },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnclosureFormatDto {
    ExactDyadic,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ResourceLimitRequestDto {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "custom")]
    Custom { value: ResourceLimitsDto },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimitsDto {
    pub max_logical_work_units: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum CalculationOutcomeDto {
    #[serde(rename = "complete")]
    Complete { calculation: CalculationDto },
    #[serde(rename = "partial")]
    Partial {
        calculation: CalculationDto,
        reason: IncompleteReasonDto,
        certified_enclosure: CertifiedIntervalPresentationDto,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationDto {
    pub exact: ExactOutputDto,
    pub scientific: ScientificOutputDto,
    pub enclosure: EnclosureOutputDto,
    pub metadata: CalculationMetadataDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ExactOutputDto {
    #[serde(rename = "omitted")]
    Omitted,
    #[serde(rename = "included")]
    Included { value: ExactPresentationDto },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum ScientificOutputDto {
    #[serde(rename = "omitted")]
    Omitted,
    #[serde(rename = "included")]
    Included { value: ScientificPresentationDto },
    #[serde(rename = "unavailable")]
    Unavailable {
        value: UnavailableScientificOutputDto,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum EnclosureOutputDto {
    #[serde(rename = "omitted")]
    Omitted,
    #[serde(rename = "included")]
    Included {
        value: CertifiedIntervalPresentationDto,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactPresentationDto {
    pub relation: ResultRelationDto,
    pub representation: ExactRepresentationKindDto,
    pub presentation: PresentationNodeDto,
    pub plain_text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScientificPresentationDto {
    pub relation: ResultRelationDto,
    pub significand: String,
    pub exponent_ten: String,
    pub requested_significant_digits: u32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingModeDto,
    pub presentation: PresentationNodeDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnavailableScientificOutputDto {
    pub requested_significant_digits: u32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingModeDto,
    pub reason: IncompleteReasonDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertifiedIntervalPresentationDto {
    pub relation: ResultRelationDto,
    pub lower: ExactDyadicDto,
    pub upper: ExactDyadicDto,
    pub format: EnclosureFormatDto,
    pub presentation: PresentationNodeDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactDyadicDto {
    pub coefficient: String,
    pub exponent_two: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationMetadataDto {
    pub exact_representation: ExactRepresentationKindDto,
    pub simplification_status: SimplificationStatusDto,
    pub semantic_settings: SemanticSettingsDto,
    pub methods: Vec<MethodTagDto>,
    pub internal_precision_bits: u32,
    pub refinement_rounds: u32,
    pub confirmed_significant_digits: u32,
    pub assurance: AssuranceLevelDto,
    pub protocol_version: ProtocolVersionDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum IncompleteReasonDto {
    #[serde(rename = "precisionLimit")]
    PrecisionLimit {
        requested_digits: u32,
        confirmed_digits: u32,
    },
    #[serde(rename = "computationLimit")]
    ComputationLimit { kind: ComputationLimitCodeDto },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum SimplificationStatusDto {
    #[serde(rename = "fullySimplifiedWithinLimits")]
    FullySimplifiedWithinLimits,
    #[serde(rename = "partiallySimplified")]
    PartiallySimplified { reason: IncompleteReasonDto },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum PresentationNodeDto {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "row")]
    Row { children: Vec<PresentationNodeDto> },
    #[serde(rename = "fraction")]
    Fraction {
        numerator: Box<PresentationNodeDto>,
        denominator: Box<PresentationNodeDto>,
    },
    #[serde(rename = "superscript")]
    Superscript {
        base: Box<PresentationNodeDto>,
        exponent: Box<PresentationNodeDto>,
    },
    #[serde(rename = "radical")]
    Radical {
        index: RadicalIndexDto,
        radicand: Box<PresentationNodeDto>,
    },
    #[serde(rename = "function")]
    Function {
        name: FunctionNameDto,
        argument: Box<PresentationNodeDto>,
    },
    #[serde(rename = "parenthesized")]
    Parenthesized { value: Box<PresentationNodeDto> },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum RadicalIndexDto {
    #[serde(rename = "square")]
    Square,
    #[serde(rename = "nth")]
    Nth { value: String },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FunctionNameDto {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Exp,
    Log,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultRelationDto {
    ExactEqual,
    ApproximatelyEqual,
    ElementOf,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExactRepresentationKindDto {
    Integer,
    Rational,
    FiniteDecimal,
    RationalPiMultiple,
    Radical,
    RealAlgebraic,
    GeneralSymbolic,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MethodTagDto {
    RationalReduction,
    RadicalExtraction,
    SpecialAngle,
    CyclotomicReduction,
    AlgebraicMinimalPolynomial,
    AlgebraicRootIsolation,
    SymbolicRetention,
    CertifiedIntervalEvaluation,
    AdaptivePrecisionRefinement,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssuranceLevelDto {
    Exact,
    CertifiedEnclosure,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum CalculatorErrorDto {
    #[serde(rename = "parse")]
    Parse {
        code: ParseErrorCodeDto,
        span: TextSpanDto,
        expected: Vec<ExpectedTokenDto>,
    },
    #[serde(rename = "domain")]
    Domain {
        code: DomainErrorCodeDto,
        span: OptionalTextSpanDto,
    },
    #[serde(rename = "inputLimit")]
    InputLimit { code: InputLimitErrorCodeDto },
    #[serde(rename = "computationLimit")]
    ComputationLimit { code: ComputationLimitCodeDto },
    #[serde(rename = "unsupportedFeature")]
    UnsupportedFeature { code: UnsupportedFeatureCodeDto },
    #[serde(rename = "internalInvariant")]
    InternalInvariant { code: InternalInvariantCodeDto },
    #[serde(rename = "unsupportedProtocol")]
    UnsupportedProtocol { code: UnsupportedProtocolCodeDto },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpanDto {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tag")]
pub enum OptionalTextSpanDto {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "some")]
    Some { value: TextSpanDto },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedTokenDto {
    pub kind: ExpectedTokenKindDto,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExpectedTokenKindDto {
    Number,
    Identifier,
    Operator,
    OpenParenthesis,
    CloseParenthesis,
    EndOfInput,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParseErrorCodeDto {
    UnexpectedToken,
    UnexpectedEnd,
    UnknownIdentifier,
    InvalidNumberLiteral,
    MissingFunctionParenthesis,
    ImplicitMultiplicationDisabled,
    PercentRejected,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainErrorCodeDto {
    DivisionByZero,
    LogarithmOfNonPositive,
    EvenRootOfNegative,
    InverseTrigonometricOutOfRange,
    TangentPole,
    ZeroToNegativePower,
    IndeterminateZeroToZero,
    NonRealPower,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InputLimitErrorCodeDto {
    InputTooLong,
    SourceAstTooDeep,
    SourceAstTooLarge,
    ExpressionTooLarge,
    IntegerTooLarge,
    OutputTooLarge,
    InvalidSignificantDigits,
    InvalidResourceLimit,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComputationLimitCodeDto {
    AlgebraicDegree,
    PolynomialCoefficientBits,
    ResultantDegree,
    FactorizationWork,
    RootIsolationSteps,
    RewriteSteps,
    PrecisionBits,
    RefinementRounds,
    LogicalWorkUnits,
    PresentationNodes,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedFeatureCodeDto {
    ComplexDomain,
    PortableProofCertificate,
    EvaluationEngine,
    ConstantEvaluation,
    FunctionEvaluation,
    NonIntegerPower,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InternalInvariantCodeDto {
    NonCanonicalRational,
    InvalidAlgebraicIsolation,
    InvalidCertifiedInterval,
    NonDeterministicCacheAccounting,
    PresentationWithoutEvaluation,
    InvalidParsedNumberLiteral,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedProtocolCodeDto {
    UnknownTag,
    UnknownCode,
    UnsupportedMajorVersion,
}
