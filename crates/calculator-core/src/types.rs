use alloc::{boxed::Box, string::String, vec::Vec};
use core::{cmp::Ordering, fmt, num::NonZeroU32};
use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedValue {
    pub exact_expression: ExactExpression,
    pub recognized_exact: RecognizedExact,
    pub certified_enclosure: CertifiedEnclosureState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactExpression {
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertifiedEnclosureState {
    NotRequested,
    Available(CertifiedInterval),
    Unavailable(IncompleteReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecognizedExact {
    Rational(Rational),
    Radical(SimpleRadical),
    RadicalLinearCombination(RadicalLinearCombination),
    RealAlgebraic(RealAlgebraic),
    RationalPiMultiple(Rational),
    GeneralSymbolic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationDomain {
    Real,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleUnit {
    Radian,
    Degree,
    Gradian,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerSemantics {
    RealPrincipal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalRoundingMode {
    NearestTiesToEven,
    NearestTiesAwayFromZero,
    TowardPositiveInfinity,
    TowardNegativeInfinity,
    TowardZero,
    AwayFromZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    Proven,
    Disproven,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignKnowledge {
    Negative,
    Zero,
    Positive,
    NonNegative,
    NonPositive,
    NonZero,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprListId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactValueId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpressionNode {
    Rational(RationalId),
    Exact(ExactValueId),
    Constant(Constant),
    Add(ExprListId),
    Multiply(ExprListId),
    Divide {
        numerator: ExprId,
        denominator: ExprId,
    },
    Power {
        base: ExprId,
        exponent: ExprId,
    },
    LogBase {
        argument: ExprId,
        base: ExprId,
    },
    Function {
        function: Function,
        argument: ExprId,
    },
    BinaryFunction {
        function: Function,
        left: ExprId,
        right: ExprId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constant {
    Pi,
    Euler,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Function {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Root,
    Exp,
    Log,
    Ln,
    Abs,
    Floor,
    Factorial,
    Permutation,
    Combination,
    Modulo,
    Gcd,
    Lcm,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Integer {
    pub(crate) inner: BigInt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigIntBackend {
    pub(crate) inner: BigInt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositiveInteger {
    pub inner: Integer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rational {
    pub numerator: Integer,
    pub denominator: PositiveInteger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleRadical {
    pub coefficient: Rational,
    pub radicand: PositiveInteger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadicalLinearCombination {
    pub rational: Rational,
    pub radicals: Vec<SimpleRadical>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalConstructionError {
    ZeroDenominator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalLiteralError {
    Empty,
    InvalidDigit,
    InvalidExponent,
    ExponentTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalArithmeticError {
    DivisionByZero,
    ZeroToNegativePower,
    ExponentTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialConstructionError {
    ZeroPolynomial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialDivisionError {
    ZeroDivisor,
    NotDivisible,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitivePolynomial {
    pub coefficients_low_to_high: Vec<Integer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveSquareFreeFactor {
    pub factor: PrimitivePolynomial,
    pub multiplicity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitivePolynomialFactor {
    pub factor: PrimitivePolynomial,
    pub multiplicity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitivePolynomialRationalRootFactorization {
    pub factors: Vec<PrimitivePolynomialFactor>,
    pub residual: Option<PrimitivePolynomial>,
    pub incomplete_reason: Option<PrimitivePolynomialFactorizationIncompleteReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitivePolynomialFactorization {
    pub factors: Vec<PrimitivePolynomialFactor>,
    pub residual: Option<PrimitivePolynomial>,
    pub incomplete_reason: Option<PrimitivePolynomialFactorizationIncompleteReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialFactorizationIncompleteReason {
    WorkLimitExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialSign {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedPrimitivePolynomial {
    pub sign: PrimitivePolynomialSign,
    pub polynomial: PrimitivePolynomial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialRootCountingError {
    ZeroPolynomial,
    InvalidInterval,
    CountOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialRootIsolationError {
    ZeroPolynomial,
    StepLimitExceeded,
    CountOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealAlgebraicConstructionError {
    ConstantPolynomial,
    InvalidInterval,
    EndpointRoot,
    NonIsolatingInterval,
    PolynomialConstruction(PrimitivePolynomialConstructionError),
    PolynomialResultant(PrimitivePolynomialResultantError),
    NoMatchingPolynomialFactor,
    RootIndexOverflow,
    RootIndexNotFound,
    RootCounting(PrimitivePolynomialRootCountingError),
    RootIsolation(PrimitivePolynomialRootIsolationError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePolynomialResultantError {
    ZeroPolynomial,
    ConstantPolynomial,
    DegreeOverflow,
    DegreeLimitExceeded,
    NonIntegralInterpolation,
}

#[derive(Clone, Debug)]
pub struct RealAlgebraic {
    pub(crate) minimal_polynomial: PrimitivePolynomial,
    pub(crate) real_root_index: u32,
    pub(crate) isolating_interval: RationalInterval,
}

impl PartialEq for RealAlgebraic {
    fn eq(&self, rhs: &Self) -> bool {
        self.minimal_polynomial == rhs.minimal_polynomial
            && self.real_root_index == rhs.real_root_index
    }
}

impl Eq for RealAlgebraic {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalInterval {
    pub lower: Rational,
    pub upper: Rational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalPiMultiple {
    pub coefficient: Rational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertifiedInterval {
    pub lower: ExactDyadic,
    pub upper: ExactDyadic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactDyadic {
    pub coefficient: Integer,
    pub exponent_two: Integer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalculationOutcome {
    Complete(Calculation),
    Partial {
        calculation: Calculation,
        reason: IncompleteReason,
        certified_enclosure: Option<CertifiedIntervalPresentation>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IncompleteReason {
    PrecisionLimit {
        requested_digits: NonZeroU32,
        confirmed_digits: u32,
    },
    ComputationLimit {
        kind: ComputationLimitKind,
    },
    UnsupportedFeature {
        feature: UnsupportedFeature,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationNode {
    Text(String),
    Row(Vec<PresentationNode>),
    Fraction {
        numerator: Box<PresentationNode>,
        denominator: Box<PresentationNode>,
    },
    Superscript {
        base: Box<PresentationNode>,
        exponent: Box<PresentationNode>,
    },
    Subscript {
        base: Box<PresentationNode>,
        subscript: Box<PresentationNode>,
    },
    Radical {
        index: RadicalIndex,
        radicand: Box<PresentationNode>,
    },
    Function {
        name: FunctionName,
        argument: Box<PresentationNode>,
    },
    Parenthesized(Box<PresentationNode>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadicalIndex {
    Square,
    Nth(PositiveInteger),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionName {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Exp,
    Log,
    Ln,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultRelation {
    ExactEqual,
    ApproximatelyEqual,
    ElementOf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedExpression {
    pub(crate) source: String,
    pub(crate) settings: ParseSettings,
    pub(crate) root: crate::syntax::SourceExpr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvaluationContext {
    cache_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutcome {
    pub value: EvaluatedValue,
    pub metadata: EvaluationMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationMetadata {
    pub semantic_settings: SemanticSettings,
    pub methods: Vec<MethodTag>,
    pub internal_precision_bits: u32,
    pub refinement_rounds: u32,
    pub incomplete_reason: Option<IncompleteReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    Domain(DomainError),
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    UnsupportedFeature(UnsupportedFeatureError),
    InternalInvariant(InternalInvariantError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationError {
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    InternalInvariant(InternalInvariantError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationRequest {
    pub parse: ParseSettings,
    pub semantics: SemanticSettings,
    pub exact_output: ExactOutputRequest,
    pub scientific_output: ScientificOutputRequest,
    pub enclosure_output: EnclosureOutputRequest,
    pub limits: ResourceLimitRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseSettings {
    pub grammar: GrammarProfile,
    pub implicit_multiplication: ImplicitMultiplicationPolicy,
    pub unicode_aliases: UnicodeAliasPolicy,
    pub percent: PercentParsePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrammarProfile {
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImplicitMultiplicationPolicy {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeAliasPolicy {
    MathematicalAliases,
    AsciiOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PercentParsePolicy {
    PostfixPercent,
    RejectPercent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticSettings {
    pub domain: EvaluationDomain,
    pub angle_unit: AngleUnit,
    pub power_semantics: PowerSemantics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactOutputRequest {
    Omit,
    Include { format: ExactFormatPreference },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScientificOutputRequest {
    Omit,
    Include {
        significant_digits: NonZeroU32,
        rounding_mode: DecimalRoundingMode,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnclosureOutputRequest {
    Omit,
    Include { format: EnclosureFormat },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceLimitRequest {
    Default,
    Custom(ResourceLimits),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationRequest {
    pub semantics: SemanticSettings,
    pub limits: ResourceLimitRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationRequest {
    pub exact_output: ExactOutputRequest,
    pub scientific_output: ScientificOutputRequest,
    pub enclosure_output: EnclosureOutputRequest,
    pub limits: ResourceLimitRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactFormatPreference {
    Auto,
    Rational,
    FiniteDecimal,
    MixedFraction,
    Symbolic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnclosureFormat {
    ExactDyadic,
    DecimalScientific { significant_digits: NonZeroU32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calculation {
    pub exact: ExactOutput,
    pub scientific: ScientificOutput,
    pub enclosure: EnclosureOutput,
    pub metadata: CalculationMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactOutput {
    Omitted,
    Included(ExactPresentation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScientificOutput {
    Omitted,
    Included(ScientificPresentation),
    Unavailable(UnavailableScientificOutput),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnclosureOutput {
    Omitted,
    Included(CertifiedIntervalPresentation),
    Unavailable(UnavailableEnclosureOutput),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnavailableEnclosureOutput {
    pub reason: IncompleteReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPresentation {
    pub relation: ResultRelation,
    pub representation: ExactRepresentationKind,
    pub presentation: PresentationNode,
    pub plain_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScientificPresentation {
    pub relation: ResultRelation,
    pub significand: String,
    pub exponent_ten: String,
    pub requested_significant_digits: NonZeroU32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingMode,
    pub presentation: PresentationNode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnavailableScientificOutput {
    pub requested_significant_digits: NonZeroU32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingMode,
    pub reason: IncompleteReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertifiedIntervalPresentation {
    pub relation: ResultRelation,
    pub bounds: CertifiedIntervalBounds,
    pub presentation: PresentationNode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertifiedIntervalBounds {
    ExactDyadic {
        lower: ExactDyadic,
        upper: ExactDyadic,
    },
    DecimalScientific {
        lower: DecimalScientificBound,
        upper: DecimalScientificBound,
        requested_significant_digits: NonZeroU32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecimalScientificBound {
    pub significand: String,
    pub exponent_ten: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationMetadata {
    pub exact_representation: ExactRepresentationKind,
    pub simplification_status: SimplificationStatus,
    pub semantic_settings: SemanticSettings,
    pub methods: Vec<MethodTag>,
    pub internal_precision_bits: u32,
    pub refinement_rounds: u32,
    pub confirmed_significant_digits: u32,
    pub assurance: AssuranceLevel,
    pub protocol_version: ProtocolVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactRepresentationKind {
    Integer,
    Rational,
    FiniteDecimal,
    RationalPiMultiple,
    Radical,
    RealAlgebraic,
    GeneralSymbolic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimplificationStatus {
    FullySimplifiedWithinLimits,
    PartiallySimplified { reason: IncompleteReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodTag {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssuranceLevel {
    Exact,
    CertifiedEnclosure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableCertificate {
    PolynomialRootIsolation(PolynomialRootCertificate),
    SeriesRemainderBound(SeriesCertificate),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialRootCertificate {
    _private: (),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesCertificate {
    _private: (),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalculatorError {
    Parse(ParseError),
    Domain(DomainError),
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    UnsupportedFeature(UnsupportedFeatureError),
    InternalInvariant(InternalInvariantError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainError {
    pub kind: DomainErrorKind,
    pub span: Option<ByteSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainErrorKind {
    DivisionByZero,
    LogarithmOfNonPositive,
    LogarithmBaseOne,
    EvenRootOfNegative,
    InverseTrigonometricOutOfRange,
    TangentPole,
    ZeroToNegativePower,
    IndeterminateZeroToZero,
    NonRealPower,
    IntegerFunctionRequiresInteger,
    IntegerFunctionRequiresNonNegative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken,
    UnexpectedEnd,
    UnknownIdentifier,
    InvalidNumberLiteral,
    MissingFunctionParenthesis,
    ImplicitMultiplicationDisabled,
    PercentRejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: ByteSpan,
    pub expected: Vec<ExpectedToken>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedToken {
    pub kind: ExpectedTokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedTokenKind {
    Number,
    Identifier,
    Operator,
    OpenParenthesis,
    CloseParenthesis,
    Comma,
    EndOfInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputLimitError {
    pub kind: InputLimitErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputLimitErrorKind {
    InputTooLong,
    SourceAstTooDeep,
    SourceAstTooLarge,
    ExpressionTooLarge,
    IntegerTooLarge,
    OutputTooLarge,
    InvalidSignificantDigits,
    InvalidResourceLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputationLimitError {
    pub kind: ComputationLimitKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputationLimitKind {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedFeatureError {
    pub feature: UnsupportedFeature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedFeature {
    ComplexDomain,
    PortableProofCertificate,
    EvaluationEngine,
    ConstantEvaluation,
    FunctionEvaluation,
    NonIntegerPower,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InternalInvariantError {
    pub code: InternalInvariantCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InternalInvariantCode {
    NonCanonicalRational,
    InvalidAlgebraicIsolation,
    InvalidCertifiedInterval,
    NonDeterministicCacheAccounting,
    PresentationWithoutEvaluation,
    InvalidParsedNumberLiteral,
    UnprovenDomainObligation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_input_bytes: u32,
    pub max_source_ast_nodes: u32,
    pub max_source_depth: u32,
    pub max_expression_nodes: u32,
    pub max_integer_bits: u32,
    pub max_cyclotomic_order: u32,
    pub max_algebraic_degree: u32,
    pub max_polynomial_coefficient_bits: u32,
    pub max_resultant_degree: u32,
    pub max_factorization_work: u32,
    pub max_root_isolation_steps: u32,
    pub max_rewrite_steps: u32,
    pub max_precision_bits: u32,
    pub max_refinement_rounds: u32,
    pub max_logical_work_units: u64,
    pub max_presentation_nodes: u32,
    pub max_output_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputAction {
    Digit(u8),
    DecimalPoint,
    Constant(Constant),
    Ans,
    Function(Function),
    BinaryOperator(BinaryOperator),
    Comma,
    Percent,
    OpenParenthesis,
    CloseParenthesis,
    DeleteBackward,
    ClearEntry,
    ClearAll,
    MemoryClear,
    MemoryRecall,
    MemoryAdd,
    MemorySubtract,
    Evaluate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputState {
    pub(crate) source: String,
    pub(crate) cursor_utf8: u32,
    pub(crate) selection_utf8: OptionalTextSpan,
    pub(crate) has_ans: bool,
    pub(crate) has_memory: bool,
    pub(crate) display: SessionDisplay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputError {
    pub kind: InputErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputErrorKind {
    InvalidDigit,
    InvalidCursor,
    SelectionOutOfBounds,
    ActionNotAllowedAfterError,
    MemoryEmpty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionReduction {
    pub state: InputState,
    pub command: SessionCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionCommand {
    None,
    Calculate {
        source: String,
        request: CalculationRequest,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPolicy {
    pub calculation_request: CalculationRequest,
    pub percent_policy: PercentPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PercentPolicy {
    ExpressionPercent,
    CalculatorPercent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionalTextSpan {
    None,
    Some(ByteSpan),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionDisplay {
    Editing,
    Result { calculation: Box<Calculation> },
    Error { error: CalculatorError },
    Calculating,
}

impl Integer {
    pub fn zero() -> Self {
        Self {
            inner: BigInt::zero(),
        }
    }

    pub fn one() -> Self {
        Self {
            inner: BigInt::one(),
        }
    }

    pub fn from_bigint(inner: BigInt) -> Self {
        Self { inner }
    }

    pub fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    pub fn abs(&self) -> Self {
        Self {
            inner: self.inner.abs(),
        }
    }

    pub fn sign(&self) -> Sign {
        self.inner.sign()
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self {
            inner: BigInt::from(value),
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.inner)
    }
}

impl PositiveInteger {
    pub fn new(value: Integer) -> Option<Self> {
        if value.inner.sign() == Sign::Plus {
            Some(Self { inner: value })
        } else {
            None
        }
    }

    pub fn one() -> Self {
        Self {
            inner: Integer::one(),
        }
    }
}

impl Rational {
    pub fn new(
        numerator: Integer,
        denominator: Integer,
    ) -> Result<Self, RationalConstructionError> {
        if denominator.is_zero() {
            return Err(RationalConstructionError::ZeroDenominator);
        }

        let mut numerator = numerator.inner;
        let mut denominator = denominator.inner;
        if denominator.sign() == Sign::Minus {
            numerator = -numerator;
            denominator = -denominator;
        }

        if numerator.is_zero() {
            return Ok(Self {
                numerator: Integer::zero(),
                denominator: PositiveInteger::one(),
            });
        }

        let divisor = numerator.gcd(&denominator);
        numerator /= &divisor;
        denominator /= divisor;

        Ok(Self {
            numerator: Integer::from_bigint(numerator),
            denominator: PositiveInteger {
                inner: Integer::from_bigint(denominator),
            },
        })
    }

    pub fn from_integer(value: Integer) -> Self {
        Self {
            numerator: value,
            denominator: PositiveInteger::one(),
        }
    }

    pub fn zero() -> Self {
        Self::from_integer(Integer::zero())
    }

    pub fn one() -> Self {
        Self::from_integer(Integer::one())
    }

    pub fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.numerator.inner.sign() == Sign::Minus
    }

    pub fn is_integer(&self) -> bool {
        self.denominator.inner.inner.is_one()
    }

    pub fn as_i64_if_integer(&self) -> Option<i64> {
        if self.is_integer() {
            self.numerator.inner.to_i64()
        } else {
            None
        }
    }

    pub fn negate(&self) -> Self {
        Self {
            numerator: Integer::from_bigint(-self.numerator.inner.clone()),
            denominator: self.denominator.clone(),
        }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        if self.is_zero() {
            return rhs.clone();
        }
        if rhs.is_zero() {
            return self.clone();
        }
        if self.is_integer() && rhs.is_integer() {
            return Self::from_integer(Integer::from_bigint(
                &self.numerator.inner + &rhs.numerator.inner,
            ));
        }
        if rhs.is_integer() {
            return Self {
                numerator: Integer::from_bigint(
                    &self.numerator.inner + &rhs.numerator.inner * &self.denominator.inner.inner,
                ),
                denominator: self.denominator.clone(),
            };
        }
        if self.is_integer() {
            return Self {
                numerator: Integer::from_bigint(
                    &self.numerator.inner * &rhs.denominator.inner.inner + &rhs.numerator.inner,
                ),
                denominator: rhs.denominator.clone(),
            };
        }
        let numerator = (&self.numerator.inner * &rhs.denominator.inner.inner)
            + (&rhs.numerator.inner * &self.denominator.inner.inner);
        let denominator = &self.denominator.inner.inner * &rhs.denominator.inner.inner;
        Self::new(
            Integer::from_bigint(numerator),
            Integer::from_bigint(denominator),
        )
        .expect("multiplying positive denominators cannot produce zero")
    }

    pub fn subtract(&self, rhs: &Self) -> Self {
        self.add(&rhs.negate())
    }

    pub(crate) fn compare(&self, rhs: &Self) -> Ordering {
        if rhs.is_zero() {
            return match self.numerator.inner.sign() {
                Sign::Minus => Ordering::Less,
                Sign::NoSign => Ordering::Equal,
                Sign::Plus => Ordering::Greater,
            };
        }
        if self.is_zero() {
            return match rhs.numerator.inner.sign() {
                Sign::Minus => Ordering::Greater,
                Sign::NoSign => Ordering::Equal,
                Sign::Plus => Ordering::Less,
            };
        }
        if self.is_integer() && rhs.is_integer() {
            return self.numerator.inner.cmp(&rhs.numerator.inner);
        }
        if rhs.is_integer() {
            return self
                .numerator
                .inner
                .cmp(&(&rhs.numerator.inner * &self.denominator.inner.inner));
        }
        if self.is_integer() {
            return (&self.numerator.inner * &rhs.denominator.inner.inner)
                .cmp(&rhs.numerator.inner);
        }
        (&self.numerator.inner * &rhs.denominator.inner.inner)
            .cmp(&(&rhs.numerator.inner * &self.denominator.inner.inner))
    }

    pub fn multiply(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() {
            return Self::zero();
        }
        if self.is_integer() && self.numerator.inner.is_one() {
            return rhs.clone();
        }
        if rhs.is_integer() && rhs.numerator.inner.is_one() {
            return self.clone();
        }
        if self.is_integer() && rhs.is_integer() {
            return Self::from_integer(Integer::from_bigint(
                &self.numerator.inner * &rhs.numerator.inner,
            ));
        }
        let numerator = &self.numerator.inner * &rhs.numerator.inner;
        let denominator = &self.denominator.inner.inner * &rhs.denominator.inner.inner;
        Self::new(
            Integer::from_bigint(numerator),
            Integer::from_bigint(denominator),
        )
        .expect("multiplying positive denominators cannot produce zero")
    }

    pub fn divide(&self, rhs: &Self) -> Result<Self, RationalArithmeticError> {
        if rhs.is_zero() {
            return Err(RationalArithmeticError::DivisionByZero);
        }
        let numerator = &self.numerator.inner * &rhs.denominator.inner.inner;
        let denominator = &self.denominator.inner.inner * &rhs.numerator.inner;
        Self::new(
            Integer::from_bigint(numerator),
            Integer::from_bigint(denominator),
        )
        .map_err(|_| RationalArithmeticError::DivisionByZero)
    }

    pub fn percent(&self) -> Self {
        self.divide(&Self::from_integer(Integer::from(100)))
            .expect("100 is non-zero")
    }

    pub(crate) fn modulo_integer(&self, period: u32) -> Self {
        debug_assert!(period > 0);
        let modulus = &self.denominator.inner.inner * BigInt::from(period);
        let remainder = self.numerator.inner.mod_floor(&modulus);
        Self::new(
            Integer::from_bigint(remainder),
            self.denominator.inner.clone(),
        )
        .expect("positive period modulo preserves a non-zero denominator")
    }

    pub(crate) fn sqrt_if_rational(&self) -> Option<Self> {
        if self.is_negative() {
            return None;
        }
        let numerator = floor_sqrt_nonnegative(&self.numerator.inner);
        if &numerator * &numerator != self.numerator.inner {
            return None;
        }
        let denominator = floor_sqrt_nonnegative(&self.denominator.inner.inner);
        if &denominator * &denominator != self.denominator.inner.inner {
            return None;
        }
        Some(
            Self::new(
                Integer::from_bigint(numerator),
                Integer::from_bigint(denominator),
            )
            .expect("square root of a canonical positive denominator is canonical"),
        )
    }

    pub(crate) fn nth_root_if_rational(&self, index: u32) -> Option<Self> {
        debug_assert!(index > 0);
        if index == 1 {
            return Some(self.clone());
        }
        if self.is_negative() && index.is_multiple_of(2) {
            return None;
        }

        let numerator_magnitude = self.numerator.inner.abs();
        let numerator_root = floor_nth_root_nonnegative(&numerator_magnitude, index);
        if numerator_root.pow(index) != numerator_magnitude {
            return None;
        }

        let denominator_root = floor_nth_root_nonnegative(&self.denominator.inner.inner, index);
        if denominator_root.pow(index) != self.denominator.inner.inner {
            return None;
        }

        let numerator = if self.is_negative() {
            -numerator_root
        } else {
            numerator_root
        };
        Some(
            Self::new(
                Integer::from_bigint(numerator),
                Integer::from_bigint(denominator_root),
            )
            .expect("root of a canonical rational preserves a non-zero denominator"),
        )
    }

    pub(crate) fn sqrt_as_simple_radical(&self) -> Option<SimpleRadical> {
        if self.is_negative() || self.is_zero() {
            return None;
        }

        let combined = &self.numerator.inner * &self.denominator.inner.inner;
        let (outside, radicand) = extract_square_factor(combined);
        if radicand.is_one() {
            return None;
        }

        let coefficient = Rational::new(
            Integer::from_bigint(outside),
            self.denominator.inner.clone(),
        )
        .expect("canonical rational denominator is non-zero");
        Some(SimpleRadical {
            coefficient,
            radicand: PositiveInteger::new(Integer::from_bigint(radicand))
                .expect("positive radicand remains positive after square extraction"),
        })
    }

    pub fn pow_i64(&self, exponent: i64) -> Result<Self, RationalArithmeticError> {
        if exponent == 0 {
            return Ok(Self::one());
        }
        if self.is_zero() && exponent < 0 {
            return Err(RationalArithmeticError::ZeroToNegativePower);
        }

        let magnitude = exponent
            .checked_abs()
            .ok_or(RationalArithmeticError::ExponentTooLarge)?;
        let magnitude =
            u32::try_from(magnitude).map_err(|_| RationalArithmeticError::ExponentTooLarge)?;

        let numerator = self.numerator.inner.pow(magnitude);
        let denominator = self.denominator.inner.inner.pow(magnitude);
        if exponent > 0 {
            Self::new(
                Integer::from_bigint(numerator),
                Integer::from_bigint(denominator),
            )
            .map_err(|_| RationalArithmeticError::DivisionByZero)
        } else {
            Self::new(
                Integer::from_bigint(denominator),
                Integer::from_bigint(numerator),
            )
            .map_err(|_| RationalArithmeticError::DivisionByZero)
        }
    }

    pub fn from_decimal_literal(literal: &str) -> Result<Self, DecimalLiteralError> {
        if literal.is_empty() {
            return Err(DecimalLiteralError::Empty);
        }

        let (sign, unsigned) = match literal.as_bytes()[0] {
            b'-' => (Sign::Minus, &literal[1..]),
            b'+' => (Sign::Plus, &literal[1..]),
            _ => (Sign::Plus, literal),
        };
        if unsigned.is_empty() {
            return Err(DecimalLiteralError::Empty);
        }

        if unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
            let mut integer = parse_bigint_decimal(unsigned)?;
            if sign == Sign::Minus {
                integer = -integer;
            }
            return Ok(Self::from_integer(Integer::from_bigint(integer)));
        }

        let (mantissa, exponent) = split_exponent(unsigned)?;
        let mut digits = String::new();
        let mut fractional_digits = 0_i64;
        let mut saw_digit = false;
        let mut saw_dot = false;

        for ch in mantissa.chars() {
            if ch == '.' {
                if saw_dot {
                    return Err(DecimalLiteralError::InvalidDigit);
                }
                saw_dot = true;
                continue;
            }
            if !ch.is_ascii_digit() {
                return Err(DecimalLiteralError::InvalidDigit);
            }
            saw_digit = true;
            digits.push(ch);
            if saw_dot {
                fractional_digits = fractional_digits
                    .checked_add(1)
                    .ok_or(DecimalLiteralError::ExponentTooLarge)?;
            }
        }

        if !saw_digit {
            return Err(DecimalLiteralError::Empty);
        }

        let scale = fractional_digits
            .checked_sub(exponent)
            .ok_or(DecimalLiteralError::ExponentTooLarge)?;
        let mut numerator = parse_bigint_decimal(&digits)?;
        if sign == Sign::Minus {
            numerator = -numerator;
        }

        if scale >= 0 {
            let denominator = pow10(scale)?;
            Rational::new(
                Integer::from_bigint(numerator),
                Integer::from_bigint(denominator),
            )
            .map_err(|_| DecimalLiteralError::InvalidDigit)
        } else {
            let multiplier = pow10(
                scale
                    .checked_neg()
                    .ok_or(DecimalLiteralError::ExponentTooLarge)?,
            )?;
            Rational::new(Integer::from_bigint(numerator * multiplier), Integer::one())
                .map_err(|_| DecimalLiteralError::InvalidDigit)
        }
    }
}

const SMALL_PRIME_SQUARE_FACTORS: [u32; 16] =
    [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
const MAX_TRIAL_SQUARE_FACTOR: u32 = 4096;

fn extract_square_factor(mut value: BigInt) -> (BigInt, BigInt) {
    debug_assert!(value.sign() == Sign::Plus);
    let mut outside = BigInt::one();
    for prime in SMALL_PRIME_SQUARE_FACTORS {
        if !extract_repeated_square_factor(&mut value, &mut outside, prime) {
            break;
        }
    }

    let mut factor = SMALL_PRIME_SQUARE_FACTORS[SMALL_PRIME_SQUARE_FACTORS.len() - 1] + 2;
    while factor <= MAX_TRIAL_SQUARE_FACTOR && !value.is_one() {
        if !extract_repeated_square_factor(&mut value, &mut outside, factor) {
            break;
        }
        factor += 2;
    }

    let root = floor_sqrt_nonnegative(&value);
    if &root * &root == value {
        outside *= root;
        value = BigInt::one();
    }

    (outside, value)
}

fn extract_repeated_square_factor(value: &mut BigInt, outside: &mut BigInt, factor: u32) -> bool {
    let factor = BigInt::from(factor);
    let square = &factor * &factor;
    if square > *value {
        return false;
    }
    while (&*value % &square).is_zero() {
        *value /= &square;
        *outside *= &factor;
    }
    true
}

pub(crate) fn floor_sqrt_nonnegative(value: &BigInt) -> BigInt {
    debug_assert!(value.sign() != Sign::Minus);
    if value.is_zero() {
        return BigInt::zero();
    }

    let mut high = BigInt::one();
    while &high * &high <= *value {
        high <<= 1_u8;
    }
    let mut low = &high >> 1_u8;
    while &low + 1_u8 < high {
        let mid = (&low + &high) >> 1_u8;
        if &mid * &mid <= *value {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

pub(crate) fn floor_nth_root_nonnegative(value: &BigInt, index: u32) -> BigInt {
    debug_assert!(value.sign() != Sign::Minus);
    debug_assert!(index > 0);
    if index == 1 {
        return value.clone();
    }
    if value.is_zero() {
        return BigInt::zero();
    }

    let mut high = BigInt::one();
    while high.pow(index) <= *value {
        high <<= 1_u8;
    }
    let mut low = &high >> 1_u8;
    while &low + 1_u8 < high {
        let mid = (&low + &high) >> 1_u8;
        if mid.pow(index) <= *value {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

#[cfg(test)]
pub(crate) fn ceil_nth_root_nonnegative(value: &BigInt, index: u32) -> BigInt {
    let floor = floor_nth_root_nonnegative(value, index);
    if floor.pow(index) == *value {
        floor
    } else {
        floor + 1_u8
    }
}

pub(crate) fn ceil_sqrt_nonnegative(value: &BigInt) -> BigInt {
    let floor = floor_sqrt_nonnegative(value);
    if &floor * &floor == *value {
        floor
    } else {
        floor + 1_u8
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator.inner.inner.is_one() {
            write!(formatter, "{}", self.numerator)
        } else {
            write!(formatter, "{}/{}", self.numerator, self.denominator.inner)
        }
    }
}

fn split_exponent(literal: &str) -> Result<(&str, i64), DecimalLiteralError> {
    let Some(index) = literal.find(['e', 'E']) else {
        return Ok((literal, 0));
    };
    let mantissa = &literal[..index];
    let exponent_text = &literal[index + 1..];
    if exponent_text.is_empty() {
        return Err(DecimalLiteralError::InvalidExponent);
    }
    let exponent = exponent_text
        .parse::<i64>()
        .map_err(|_| DecimalLiteralError::InvalidExponent)?;
    Ok((mantissa, exponent))
}

fn parse_bigint_decimal(digits: &str) -> Result<BigInt, DecimalLiteralError> {
    if digits.bytes().any(|byte| !byte.is_ascii_digit()) {
        return Err(DecimalLiteralError::InvalidDigit);
    }
    BigInt::parse_bytes(digits.as_bytes(), 10).ok_or(DecimalLiteralError::InvalidDigit)
}

fn pow10(exponent: i64) -> Result<BigInt, DecimalLiteralError> {
    let exponent = u32::try_from(exponent).map_err(|_| DecimalLiteralError::ExponentTooLarge)?;
    Ok(BigInt::from(10_u8).pow(exponent))
}

impl Default for ParseSettings {
    fn default() -> Self {
        Self {
            grammar: GrammarProfile::Default,
            implicit_multiplication: ImplicitMultiplicationPolicy::Enabled,
            unicode_aliases: UnicodeAliasPolicy::MathematicalAliases,
            percent: PercentParsePolicy::PostfixPercent,
        }
    }
}

impl Default for SemanticSettings {
    fn default() -> Self {
        Self {
            domain: EvaluationDomain::Real,
            angle_unit: AngleUnit::Radian,
            power_semantics: PowerSemantics::RealPrincipal,
        }
    }
}

impl Default for CalculationRequest {
    fn default() -> Self {
        Self {
            parse: ParseSettings::default(),
            semantics: SemanticSettings::default(),
            exact_output: ExactOutputRequest::Include {
                format: ExactFormatPreference::Auto,
            },
            scientific_output: ScientificOutputRequest::Include {
                significant_digits: nonzero_u32(5),
                rounding_mode: DecimalRoundingMode::NearestTiesToEven,
            },
            enclosure_output: EnclosureOutputRequest::Include {
                format: EnclosureFormat::DecimalScientific {
                    significant_digits: nonzero_u32(5),
                },
            },
            limits: ResourceLimitRequest::Default,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024,
            max_source_ast_nodes: 16 * 1024,
            max_source_depth: 512,
            max_expression_nodes: 64 * 1024,
            max_integer_bits: 1_000_000,
            max_cyclotomic_order: 128,
            max_algebraic_degree: 64,
            max_polynomial_coefficient_bits: 1_000_000,
            max_resultant_degree: 128,
            max_factorization_work: 100_000,
            max_root_isolation_steps: 100_000,
            max_rewrite_steps: 100_000,
            max_precision_bits: 1_000_000,
            max_refinement_rounds: 256,
            max_logical_work_units: 10_000_000,
            max_presentation_nodes: 64 * 1024,
            max_output_bytes: 1024 * 1024,
        }
    }
}

impl Default for InputPolicy {
    fn default() -> Self {
        Self {
            calculation_request: CalculationRequest::default(),
            percent_policy: PercentPolicy::ExpressionPercent,
        }
    }
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 4, minor: 1 };
}

const fn nonzero_u32(value: u32) -> NonZeroU32 {
    match NonZeroU32::new(value) {
        Some(value) => value,
        None => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString};

    use super::*;

    #[test]
    fn default_request_matches_phase_zero_contract() {
        let request = CalculationRequest::default();
        assert_eq!(request.parse.grammar, GrammarProfile::Default);
        assert_eq!(request.semantics.domain, EvaluationDomain::Real);
        assert_eq!(request.semantics.angle_unit, AngleUnit::Radian);
        assert_eq!(
            request.semantics.power_semantics,
            PowerSemantics::RealPrincipal
        );
        assert_eq!(
            request.scientific_output,
            ScientificOutputRequest::Include {
                significant_digits: nonzero_u32(5),
                rounding_mode: DecimalRoundingMode::NearestTiesToEven,
            }
        );
        assert_eq!(
            request.enclosure_output,
            EnclosureOutputRequest::Include {
                format: EnclosureFormat::DecimalScientific {
                    significant_digits: nonzero_u32(5),
                },
            }
        );
        assert_eq!(request.limits, ResourceLimitRequest::Default);
    }

    #[test]
    fn protocol_version_is_current_public_contract() {
        assert_eq!(
            ProtocolVersion::CURRENT,
            ProtocolVersion { major: 4, minor: 1 }
        );
    }

    #[test]
    fn rational_is_reduced_and_denominator_is_positive() {
        let rational = Rational::new(Integer::from(6), Integer::from(-8)).unwrap();
        assert_eq!(rational.numerator.to_string(), "-3");
        assert_eq!(rational.denominator.inner.to_string(), "4");
    }

    #[test]
    fn rational_integer_predicate_uses_canonical_denominator() {
        let large = Rational::from_decimal_literal(&format!("-{}", "9".repeat(2_048))).unwrap();
        let fractional = Rational::new(Integer::from(6), Integer::from(-14)).unwrap();
        let large_denominator = Rational::new(
            Integer::from(1),
            Integer::from_bigint(BigInt::from(10_u8).pow(1_024) + BigInt::one()),
        )
        .unwrap();

        assert!(Rational::zero().is_integer());
        assert!(Rational::one().is_integer());
        assert!(large.is_integer());
        assert!(!fractional.is_integer());
        assert!(!large_denominator.is_integer());
    }

    #[test]
    fn rational_zero_has_unique_denominator() {
        let rational = Rational::new(Integer::zero(), Integer::from(-99)).unwrap();
        assert_eq!(rational.numerator.to_string(), "0");
        assert_eq!(rational.denominator.inner.to_string(), "1");
    }

    #[test]
    fn rational_rejects_zero_denominator() {
        assert_eq!(
            Rational::new(Integer::one(), Integer::zero()),
            Err(RationalConstructionError::ZeroDenominator)
        );
    }

    #[test]
    fn decimal_literal_is_exact_rational() {
        let rational = Rational::from_decimal_literal("12.3400e-3").unwrap();
        assert_eq!(rational.numerator.to_string(), "617");
        assert_eq!(rational.denominator.inner.to_string(), "50000");

        let rational = Rational::from_decimal_literal("1.2e-3").unwrap();
        assert_eq!(rational.numerator.to_string(), "3");
        assert_eq!(rational.denominator.inner.to_string(), "2500");
    }

    #[test]
    fn integer_decimal_literals_construct_canonical_integers_directly() {
        for literal in [
            "0",
            "+0",
            "-0",
            "000000",
            "+00042",
            "-00042",
            &"9".repeat(4_096),
        ] {
            let actual = Rational::from_decimal_literal(literal).unwrap();
            let expected_integer = BigInt::parse_bytes(literal.as_bytes(), 10).unwrap();
            let expected =
                Rational::new(Integer::from_bigint(expected_integer), Integer::one()).unwrap();
            assert_eq!(actual, expected, "{literal}");
            assert!(actual.is_integer());
            if actual.is_zero() {
                assert_eq!(actual.denominator.inner, Integer::one());
            }
        }

        for (literal, error) in [
            ("+", DecimalLiteralError::Empty),
            ("-", DecimalLiteralError::Empty),
            ("12x", DecimalLiteralError::InvalidDigit),
            ("1e", DecimalLiteralError::InvalidExponent),
            (
                "1e999999999999999999999999999999",
                DecimalLiteralError::InvalidExponent,
            ),
        ] {
            assert_eq!(Rational::from_decimal_literal(literal), Err(error));
        }

        for literal in ["12.3400e-3", "1.2e-3", "42e3", "42e-3"] {
            let actual = Rational::from_decimal_literal(literal).unwrap();
            let (numerator, denominator) = match literal {
                "12.3400e-3" => (617, 50_000),
                "1.2e-3" => (3, 2_500),
                "42e3" => (42_000, 1),
                "42e-3" => (21, 500),
                _ => unreachable!(),
            };
            assert_eq!(
                actual,
                Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
            );
        }
    }

    #[test]
    fn rational_arithmetic_preserves_canonical_form() {
        let one_third = Rational::new(Integer::one(), Integer::from(3)).unwrap();
        let one_sixth = Rational::new(Integer::one(), Integer::from(6)).unwrap();
        let sum = one_third.add(&one_sixth);
        assert_eq!(sum.numerator.to_string(), "1");
        assert_eq!(sum.denominator.inner.to_string(), "2");

        let product = Rational::new(Integer::from(2), Integer::from(3))
            .unwrap()
            .multiply(&Rational::new(Integer::from(9), Integer::from(4)).unwrap());
        assert_eq!(product.numerator.to_string(), "3");
        assert_eq!(product.denominator.inner.to_string(), "2");
    }

    #[test]
    fn rational_negation_preserves_canonical_form() {
        for value in [
            Rational::zero(),
            Rational::from_integer(Integer::from(7)),
            Rational::from_decimal_literal(&"9".repeat(2_048)).unwrap(),
            Rational::new(Integer::from(-5), Integer::from(14)).unwrap(),
            Rational::new(Integer::from(22), Integer::from(7)).unwrap(),
        ] {
            let expected = Rational::new(
                Integer::from_bigint(-value.numerator.inner.clone()),
                value.denominator.inner.clone(),
            )
            .unwrap();
            let negated = value.negate();
            assert_eq!(negated, expected);
            assert_eq!(negated.negate(), value);
            assert!(negated.denominator.inner.inner.sign() == Sign::Plus);
        }
    }

    #[test]
    fn rational_integer_addition_matches_canonical_constructor() {
        let large = BigInt::one() << 4096_usize;
        for (left, right) in [
            (BigInt::zero(), BigInt::zero()),
            (BigInt::zero(), large.clone()),
            (large.clone(), BigInt::zero()),
            (large.clone(), -large.clone()),
            (large.clone() - 1_u8, BigInt::one()),
            (-large.clone(), -BigInt::one()),
        ] {
            let actual = Rational::from_integer(Integer::from_bigint(left.clone()))
                .add(&Rational::from_integer(Integer::from_bigint(right.clone())));
            let expected =
                Rational::new(Integer::from_bigint(left + right), Integer::one()).unwrap();
            assert_eq!(actual, expected);
            assert!(actual.is_integer());
        }
    }

    #[test]
    fn rational_integer_multiplication_matches_canonical_constructor() {
        let large = BigInt::one() << 4096_usize;
        for (left, right) in [
            (BigInt::zero(), large.clone()),
            (large.clone(), BigInt::zero()),
            (BigInt::one(), large.clone()),
            (large.clone(), BigInt::one()),
            (-BigInt::one(), large.clone()),
            (large.clone() - 1_u8, large.clone() + 1_u8),
            (-large.clone(), -large.clone()),
        ] {
            let actual = Rational::from_integer(Integer::from_bigint(left.clone()))
                .multiply(&Rational::from_integer(Integer::from_bigint(right.clone())));
            let expected =
                Rational::new(Integer::from_bigint(left * right), Integer::one()).unwrap();
            assert_eq!(actual, expected);
            assert!(actual.is_integer());
            if actual.is_zero() {
                assert_eq!(actual.denominator.inner, Integer::one());
            }
        }
    }

    #[test]
    fn rational_multiplication_identities_preserve_fractional_canonical_form() {
        let fraction = Rational::new(Integer::from(-5), Integer::from(14)).unwrap();
        assert_eq!(Rational::zero().multiply(&fraction), Rational::zero());
        assert_eq!(fraction.multiply(&Rational::zero()), Rational::zero());
        assert_eq!(Rational::one().multiply(&fraction), fraction);
        assert_eq!(fraction.multiply(&Rational::one()), fraction);

        let integer = Rational::from_integer(Integer::from(6));
        let expected = Rational::new(Integer::from(-15), Integer::from(7)).unwrap();
        assert_eq!(integer.multiply(&fraction), expected);
        assert_eq!(fraction.multiply(&integer), expected);

        for (left, right) in [
            (
                Rational::from_integer(Integer::from(-1)),
                Rational::new(Integer::from(5), Integer::from(14)).unwrap(),
            ),
            (
                Rational::new(Integer::from(5), Integer::from(14)).unwrap(),
                Rational::from_integer(Integer::from(-6)),
            ),
            (
                Rational::new(Integer::from(2), Integer::from(3)).unwrap(),
                Rational::new(Integer::from(-9), Integer::from(4)).unwrap(),
            ),
        ] {
            let expected = Rational::new(
                Integer::from_bigint(&left.numerator.inner * &right.numerator.inner),
                Integer::from_bigint(
                    &left.denominator.inner.inner * &right.denominator.inner.inner,
                ),
            )
            .unwrap();
            assert_eq!(left.multiply(&right), expected);
        }
    }

    #[test]
    fn rational_mixed_addition_still_uses_canonical_fraction_result() {
        let integer = Rational::from_integer(Integer::from(7));
        let fraction = Rational::new(Integer::from(-5), Integer::from(6)).unwrap();
        for (left, right) in [
            (integer.clone(), fraction.clone()),
            (fraction.clone(), integer.clone()),
            (Rational::zero(), fraction.clone()),
            (fraction.clone(), Rational::zero()),
            (
                Rational::new(Integer::from(1), Integer::from(6)).unwrap(),
                Rational::new(Integer::from(1), Integer::from(3)).unwrap(),
            ),
        ] {
            let expected = Rational::new(
                Integer::from_bigint(
                    &left.numerator.inner * &right.denominator.inner.inner
                        + &right.numerator.inner * &left.denominator.inner.inner,
                ),
                Integer::from_bigint(
                    &left.denominator.inner.inner * &right.denominator.inner.inner,
                ),
            )
            .unwrap();
            assert_eq!(left.add(&right), expected);
        }

        let sum = integer.add(&fraction);
        assert_eq!(sum.numerator.to_string(), "37");
        assert_eq!(sum.denominator.inner.to_string(), "6");
        assert_eq!(fraction.subtract(&integer).to_string(), "-47/6");
    }

    #[test]
    fn rational_integer_comparison_matches_cross_product_oracle() {
        let large = BigInt::one() << 4096_usize;
        let values = [
            Rational::from_integer(Integer::from_bigint(-large.clone())),
            Rational::from_integer(Integer::from(-1)),
            Rational::zero(),
            Rational::from_integer(Integer::one()),
            Rational::from_integer(Integer::from_bigint(large)),
            Rational::new(Integer::from(-7), Integer::from(3)).unwrap(),
            Rational::new(Integer::from(-1), Integer::from(2)).unwrap(),
            Rational::new(Integer::from(1), Integer::from(2)).unwrap(),
            Rational::new(Integer::from(7), Integer::from(3)).unwrap(),
        ];

        for left in &values {
            for right in &values {
                let expected = (&left.numerator.inner * &right.denominator.inner.inner)
                    .cmp(&(&right.numerator.inner * &left.denominator.inner.inner));
                assert_eq!(left.compare(right), expected, "{left} versus {right}");
                assert_eq!(right.compare(left), expected.reverse());
            }
        }
    }

    #[test]
    fn rational_comparison_uses_exact_cross_products() {
        assert_eq!(
            Rational::new(Integer::from(2), Integer::from(3))
                .unwrap()
                .compare(&Rational::new(Integer::from(3), Integer::from(4)).unwrap()),
            Ordering::Less
        );
        assert_eq!(
            Rational::new(Integer::from(-2), Integer::from(3))
                .unwrap()
                .compare(&Rational::new(Integer::from(-3), Integer::from(4)).unwrap()),
            Ordering::Greater
        );
    }

    #[test]
    fn rational_division_rejects_zero() {
        assert_eq!(
            Rational::one().divide(&Rational::zero()),
            Err(RationalArithmeticError::DivisionByZero)
        );
    }

    #[test]
    fn rational_integer_power_handles_negative_exponents() {
        let value = Rational::new(Integer::from(2), Integer::from(3)).unwrap();
        let squared = value.pow_i64(2).unwrap();
        assert_eq!(squared.numerator.to_string(), "4");
        assert_eq!(squared.denominator.inner.to_string(), "9");

        let reciprocal = value.pow_i64(-2).unwrap();
        assert_eq!(reciprocal.numerator.to_string(), "9");
        assert_eq!(reciprocal.denominator.inner.to_string(), "4");
    }

    #[test]
    fn rational_nth_root_requires_exact_rational_root() {
        let value = Rational::new(Integer::from(-27), Integer::from(8)).unwrap();
        let root = value.nth_root_if_rational(3).unwrap();
        assert_eq!(root.numerator.to_string(), "-3");
        assert_eq!(root.denominator.inner.to_string(), "2");

        let value = Rational::new(Integer::from(16), Integer::from(81)).unwrap();
        let root = value.nth_root_if_rational(4).unwrap();
        assert_eq!(root.numerator.to_string(), "2");
        assert_eq!(root.denominator.inner.to_string(), "3");

        assert!(Rational::from_integer(Integer::from(2))
            .nth_root_if_rational(2)
            .is_none());
        assert!(Rational::from_integer(Integer::from(-8))
            .nth_root_if_rational(2)
            .is_none());
    }

    #[test]
    fn rational_square_root_extracts_simple_radical() {
        let radical = Rational::from_integer(Integer::from(72))
            .sqrt_as_simple_radical()
            .unwrap();
        assert_eq!(radical.coefficient.to_string(), "6");
        assert_eq!(radical.radicand.inner.to_string(), "2");

        let radical = Rational::new(Integer::from(1), Integer::from(2))
            .unwrap()
            .sqrt_as_simple_radical()
            .unwrap();
        assert_eq!(radical.coefficient.to_string(), "1/2");
        assert_eq!(radical.radicand.inner.to_string(), "2");

        let radical = Rational::from_integer(Integer::from(6962))
            .sqrt_as_simple_radical()
            .unwrap();
        assert_eq!(radical.coefficient.to_string(), "59");
        assert_eq!(radical.radicand.inner.to_string(), "2");

        let radical = Rational::new(Integer::from(1), Integer::from(6962))
            .unwrap()
            .sqrt_as_simple_radical()
            .unwrap();
        assert_eq!(radical.coefficient.to_string(), "1/118");
        assert_eq!(radical.radicand.inner.to_string(), "2");

        assert!(Rational::from_integer(Integer::from(4))
            .sqrt_as_simple_radical()
            .is_none());
        assert!(Rational::from_integer(Integer::from(-2))
            .sqrt_as_simple_radical()
            .is_none());
    }

    #[test]
    fn square_factor_trial_stops_above_remaining_value() {
        let mut value = BigInt::from(2_u8);
        let mut outside = BigInt::one();
        assert!(!extract_repeated_square_factor(&mut value, &mut outside, 2));
        assert_eq!(value, BigInt::from(2_u8));
        assert_eq!(outside, BigInt::one());

        assert_eq!(
            extract_square_factor(BigInt::from(59_u32).pow(2) * BigInt::from(2_u8)),
            (BigInt::from(59_u8), BigInt::from(2_u8))
        );

        for (value, expected) in [
            (
                BigInt::from(59_u32).pow(4) * BigInt::from(2_u8),
                (BigInt::from(59_u32).pow(2), BigInt::from(2_u8)),
            ),
            (
                BigInt::from(4093_u32).pow(2) * BigInt::from(2_u8),
                (BigInt::from(4093_u32), BigInt::from(2_u8)),
            ),
            (
                BigInt::from(4099_u32).pow(2) * BigInt::from(2_u8),
                (
                    BigInt::one(),
                    BigInt::from(4099_u32).pow(2) * BigInt::from(2_u8),
                ),
            ),
        ] {
            assert_eq!(extract_square_factor(value), expected);
        }
    }

    #[test]
    fn rational_percent_is_exact() {
        let value = Rational::from_integer(Integer::from(50)).percent();
        assert_eq!(value.numerator.to_string(), "1");
        assert_eq!(value.denominator.inner.to_string(), "2");
    }

    #[test]
    fn rational_modulo_integer_uses_nonnegative_remainder() {
        let value = Rational::new(Integer::from(-11), Integer::from(6)).unwrap();
        assert_eq!(value.modulo_integer(2).to_string(), "1/6");

        let value = Rational::new(Integer::from(7), Integer::from(4)).unwrap();
        assert_eq!(value.modulo_integer(1).to_string(), "3/4");
    }
}
