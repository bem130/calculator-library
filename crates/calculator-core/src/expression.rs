use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::{
    interval::{self, IntervalError},
    syntax::{SourceExpr, UnaryOperator},
    types::*,
};

const EVALUATION_INTERVAL_PRECISION_BITS: u32 = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactExpressionDag {
    root: ExprId,
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
    exact_values: Vec<StoredExactValue>,
    normalization: ExactNormalization,
    semantics: SemanticSettings,
    canonical_rewrite_steps_used: u32,
    canonical_logical_work_used: u64,
    domain_obligations: Vec<DomainObligation>,
    structural_sizes: Vec<usize>,
}

impl ExactExpressionDag {
    pub(crate) fn root(&self) -> ExprId {
        self.root
    }

    pub(crate) fn semantics(&self) -> SemanticSettings {
        self.semantics
    }

    pub(crate) fn node(&self, id: ExprId) -> &ExpressionNode {
        &self.nodes[id.0 as usize]
    }

    pub(crate) fn nodes(&self) -> &[ExpressionNode] {
        &self.nodes
    }

    pub(crate) fn list(&self, id: ExprListId) -> &[ExprId] {
        &self.lists[id.0 as usize]
    }

    pub(crate) fn rational(&self, id: RationalId) -> &Rational {
        &self.rationals[id.0 as usize]
    }

    pub(crate) fn exact_value(&self, id: ExactValueId) -> &ExactReduction {
        &self.exact_values[id.0 as usize].reduction
    }

    pub(crate) fn exact_presentation(&self, id: ExactValueId) -> &ExpressionNode {
        &self.exact_values[id.0 as usize].presentation
    }

    pub(crate) fn semantic_node(&self, id: ExprId) -> &ExpressionNode {
        match self.node(id) {
            ExpressionNode::Exact(value)
                if matches!(self.exact_value(*value), ExactReduction::Symbolic) =>
            {
                self.exact_presentation(*value)
            }
            node @ (ExpressionNode::Rational(_)
            | ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. }) => node,
        }
    }

    pub(crate) fn symbolic_rewrites_allowed(&self) -> bool {
        self.normalization.limit_reached().is_none()
    }

    fn push_list(&mut self, values: Vec<ExprId>) -> ExprListId {
        let id = ExprListId(self.lists.len() as u32);
        self.lists.push(values);
        id
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExactNormalization {
    used_special_angle: bool,
    used_radical_reduction: bool,
    used_algebraic_reduction: bool,
    used_cyclotomic_reduction: bool,
    limit_reached: Option<ComputationLimitKind>,
}

impl ExactNormalization {
    pub(crate) fn used_special_angle(self) -> bool {
        self.used_special_angle
    }

    pub(crate) fn used_radical_reduction(self) -> bool {
        self.used_radical_reduction
    }

    pub(crate) fn used_algebraic_reduction(self) -> bool {
        self.used_algebraic_reduction
    }

    pub(crate) fn used_cyclotomic_reduction(self) -> bool {
        self.used_cyclotomic_reduction
    }

    pub(crate) fn limit_reached(self) -> Option<ComputationLimitKind> {
        self.limit_reached
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RationalEvaluation {
    value: Rational,
    used_special_angle: bool,
}

impl RationalEvaluation {
    pub(crate) fn direct(value: Rational) -> Self {
        Self {
            value,
            used_special_angle: false,
        }
    }

    fn special_angle(value: Rational) -> Self {
        Self {
            value,
            used_special_angle: true,
        }
    }

    fn with_origin(value: Rational, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &Rational {
        &self.value
    }

    pub(crate) fn into_value(self) -> Rational {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PiCoefficientEvaluation {
    coefficient: Rational,
    used_special_angle: bool,
}

impl PiCoefficientEvaluation {
    fn direct(coefficient: Rational) -> Self {
        Self {
            coefficient,
            used_special_angle: false,
        }
    }

    fn special_angle(coefficient: Rational) -> Self {
        Self {
            coefficient,
            used_special_angle: true,
        }
    }

    fn with_origin(coefficient: Rational, used_special_angle: bool) -> Self {
        Self {
            coefficient,
            used_special_angle,
        }
    }

    pub(crate) fn coefficient(&self) -> &Rational {
        &self.coefficient
    }

    pub(crate) fn into_coefficient(self) -> Rational {
        self.coefficient
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RadicalEvaluation {
    value: SimpleRadical,
    used_special_angle: bool,
}

impl RadicalEvaluation {
    fn with_origin(value: SimpleRadical, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &SimpleRadical {
        &self.value
    }

    pub(crate) fn into_value(self) -> SimpleRadical {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RadicalLinearCombinationEvaluation {
    value: RadicalLinearCombination,
    used_special_angle: bool,
}

impl RadicalLinearCombinationEvaluation {
    fn with_origin(value: RadicalLinearCombination, used_special_angle: bool) -> Self {
        Self {
            value,
            used_special_angle,
        }
    }

    pub(crate) fn value(&self) -> &RadicalLinearCombination {
        &self.value
    }

    pub(crate) fn into_value(self) -> RadicalLinearCombination {
        self.value
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        self.used_special_angle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RadicalReduction {
    Rational(RationalEvaluation),
    Radical(RadicalEvaluation),
    LinearCombination(RadicalLinearCombinationEvaluation),
}

impl RadicalReduction {
    fn rational(value: Rational, used_special_angle: bool) -> Self {
        Self::Rational(RationalEvaluation::with_origin(value, used_special_angle))
    }

    fn radical(value: SimpleRadical, used_special_angle: bool) -> Self {
        Self::Radical(RadicalEvaluation::with_origin(value, used_special_angle))
    }

    fn linear_combination(value: RadicalLinearCombination, used_special_angle: bool) -> Self {
        Self::LinearCombination(RadicalLinearCombinationEvaluation::with_origin(
            value,
            used_special_angle,
        ))
    }

    pub(crate) fn used_special_angle(&self) -> bool {
        match self {
            Self::Rational(value) => value.used_special_angle(),
            Self::Radical(value) => value.used_special_angle(),
            Self::LinearCombination(value) => value.used_special_angle(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RealAlgebraicEvaluation {
    Rational(RationalEvaluation),
    Algebraic(RealAlgebraic),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExactReduction {
    PiMultiple(PiCoefficientEvaluation),
    Radical(RadicalReduction),
    RealAlgebraic(RealAlgebraicEvaluation),
    Symbolic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredExactValue {
    reduction: ExactReduction,
    presentation: ExpressionNode,
    canonical_polynomial: Option<BuilderPolynomial>,
}

impl RealAlgebraicEvaluation {
    fn rational(value: Rational) -> Self {
        Self::Rational(RationalEvaluation::direct(value))
    }

    fn from_algebraic(value: RealAlgebraic) -> Self {
        if let Some(rational) = value.rational_value() {
            Self::rational(rational)
        } else {
            Self::Algebraic(value)
        }
    }

    #[cfg(test)]
    fn into_algebraic(self) -> Option<RealAlgebraic> {
        match self {
            Self::Rational(_) => None,
            Self::Algebraic(value) => Some(value),
        }
    }
}

#[derive(Default)]
struct DagBuilder {
    nodes: Vec<ExpressionNode>,
    lists: Vec<Vec<ExprId>>,
    rationals: Vec<Rational>,
    node_index: BTreeMap<u64, Vec<ExprId>>,
    list_index: BTreeMap<u64, Vec<ExprListId>>,
    rational_index: BTreeMap<u64, Vec<RationalId>>,
    semantics: SemanticSettings,
    canonical_budget: CanonicalBudget,
    canonical_term_limit: usize,
    canonical_limit_reached: Option<ComputationLimitKind>,
    additive_literal_fold_limit_reached: bool,
    multiplicative_literal_fold_limit_reached: bool,
    domain_obligations: Vec<DomainObligation>,
    structural_sizes: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DomainObligation {
    Defined(ExprId),
    NonZero(ExprId),
}

struct BuilderLinearTerm {
    expression: ExprId,
    coefficient: Rational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BuilderMonomial {
    coefficient: Rational,
    factors: Vec<(ExprId, i64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BuilderPolynomial {
    terms: Vec<BuilderMonomial>,
}

impl BuilderPolynomial {
    fn one() -> Self {
        Self {
            terms: vec![BuilderMonomial {
                coefficient: Rational::one(),
                factors: Vec::new(),
            }],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct CanonicalBudget {
    rewrite_steps_remaining: u32,
    logical_work_remaining: u64,
}

impl CanonicalBudget {
    fn reserve(
        &mut self,
        rewrite_steps: usize,
        logical_work: usize,
    ) -> Result<(), ComputationLimitKind> {
        let rewrite_steps =
            u32::try_from(rewrite_steps).map_err(|_| ComputationLimitKind::RewriteSteps)?;
        let logical_work =
            u64::try_from(logical_work).map_err(|_| ComputationLimitKind::LogicalWorkUnits)?;
        if rewrite_steps > self.rewrite_steps_remaining {
            return Err(ComputationLimitKind::RewriteSteps);
        }
        if logical_work > self.logical_work_remaining {
            return Err(ComputationLimitKind::LogicalWorkUnits);
        }
        self.rewrite_steps_remaining -= rewrite_steps;
        self.logical_work_remaining -= logical_work;
        Ok(())
    }
}

pub(crate) fn lower_source_expression(
    expression: &SourceExpr,
    semantics: SemanticSettings,
    limits: &ResourceLimits,
) -> Result<ExactExpressionDag, EvaluationError> {
    let mut builder = DagBuilder {
        semantics,
        canonical_budget: CanonicalBudget {
            rewrite_steps_remaining: limits.max_rewrite_steps,
            logical_work_remaining: limits.max_logical_work_units,
        },
        canonical_term_limit: usize::try_from(limits.max_expression_nodes).unwrap_or(usize::MAX),
        ..DagBuilder::default()
    };
    let root = builder.lower(expression)?;
    let canonical_rewrite_steps_used = limits
        .max_rewrite_steps
        .saturating_sub(builder.canonical_budget.rewrite_steps_remaining);
    let canonical_logical_work_used = limits
        .max_logical_work_units
        .saturating_sub(builder.canonical_budget.logical_work_remaining);
    Ok(ExactExpressionDag {
        root,
        nodes: builder.nodes,
        lists: builder.lists,
        rationals: builder.rationals,
        exact_values: Vec::new(),
        normalization: ExactNormalization {
            limit_reached: builder.canonical_limit_reached,
            ..ExactNormalization::default()
        },
        semantics,
        canonical_rewrite_steps_used,
        canonical_logical_work_used,
        domain_obligations: builder.domain_obligations,
        structural_sizes: builder.structural_sizes,
    })
}

pub(crate) fn normalize_exact_subexpressions(
    dag: ExactExpressionDag,
    limits: &ResourceLimits,
) -> Result<(ExactExpressionDag, ExactNormalization), EvaluationError> {
    if !dag.exact_values.is_empty() {
        let normalization = dag.normalization;
        return Ok((dag, normalization));
    }
    let mut reduced = ExactExpressionDag {
        root: dag.root,
        nodes: Vec::with_capacity(dag.nodes.len()),
        lists: Vec::with_capacity(dag.lists.len()),
        rationals: Vec::with_capacity(dag.rationals.len()),
        exact_values: Vec::new(),
        normalization: ExactNormalization::default(),
        semantics: dag.semantics,
        canonical_rewrite_steps_used: dag.canonical_rewrite_steps_used,
        canonical_logical_work_used: dag.canonical_logical_work_used,
        domain_obligations: dag.domain_obligations.clone(),
        structural_sizes: Vec::with_capacity(dag.structural_sizes.len()),
    };
    let mut normalization = dag.normalization;
    let mut budget = ReductionBudget::new(
        limits,
        dag.canonical_rewrite_steps_used,
        dag.canonical_logical_work_used,
        normalization.limit_reached,
    );

    for obligation in &dag.domain_obligations {
        match obligation {
            DomainObligation::Defined(id) => validate_obvious_domain(&dag, *id)?,
            DomainObligation::NonZero(id) => {
                validate_obvious_domain(&dag, *id)?;
                if !prove_expression_nonzero(&dag, *id)? {
                    return Err(EvaluationError::InternalInvariant(InternalInvariantError {
                        code: InternalInvariantCode::UnprovenDomainObligation,
                    }));
                }
            }
        }
    }

    // Lowering assigns children before parents. Each source node is rebuilt and reduced
    // exactly once, so parents always observe memoized exact children.
    for index in 0..dag.nodes.len() {
        let id = ExprId(index as u32);
        let presentation = rebuild_node(&dag, id, &mut reduced);
        debug_assert_eq!(reduced.nodes.len(), index);
        let presentation_size = expression_node_structural_size(
            &presentation,
            &reduced.structural_sizes,
            &reduced.lists,
            &reduced.rationals,
        );
        reduced.nodes.push(presentation.clone());
        reduced.structural_sizes.push(presentation_size);

        if matches!(presentation, ExpressionNode::Rational(_)) {
            continue;
        }

        let value = recognize_exact_subexpression(&reduced, id, limits, &mut budget)?
            .unwrap_or(RecognizedExactSubexpression::Symbolic);
        reduced.nodes[index] =
            store_exact_subexpression(&mut reduced, value, presentation, &mut normalization);
        reduced.structural_sizes[index] = match reduced.nodes[index] {
            ExpressionNode::Exact(value) => {
                presentation_size.max(exact_reduction_structural_size(reduced.exact_value(value)))
            }
            ExpressionNode::Rational(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => presentation_size,
        };
    }

    materialize_stored_canonical_polynomials(&mut reduced, limits, &mut budget);

    normalization.limit_reached = budget.limit_reached;

    reduced.normalization = normalization;
    Ok((reduced, normalization))
}

struct ReductionBudget {
    rewrite_steps_remaining: u32,
    logical_work_remaining: u64,
    limit_reached: Option<ComputationLimitKind>,
}

impl ReductionBudget {
    fn new(
        limits: &ResourceLimits,
        rewrite_steps_used: u32,
        logical_work_used: u64,
        limit_reached: Option<ComputationLimitKind>,
    ) -> Self {
        if limit_reached.is_some() {
            return Self {
                rewrite_steps_remaining: 0,
                logical_work_remaining: 0,
                limit_reached,
            };
        }
        Self {
            rewrite_steps_remaining: limits.max_rewrite_steps.saturating_sub(rewrite_steps_used),
            logical_work_remaining: limits
                .max_logical_work_units
                .saturating_sub(logical_work_used),
            limit_reached,
        }
    }

    fn consume_node(&mut self) -> bool {
        if self.rewrite_steps_remaining == 0 {
            self.limit_reached
                .get_or_insert(ComputationLimitKind::RewriteSteps);
            return false;
        }
        if self.logical_work_remaining == 0 {
            self.limit_reached
                .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
            return false;
        }
        self.rewrite_steps_remaining -= 1;
        self.logical_work_remaining -= 1;
        true
    }

    fn consume_work(&mut self, units: u64) -> bool {
        let Some(remaining) = self.logical_work_remaining.checked_sub(units) else {
            self.limit_reached
                .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
            return false;
        };
        self.logical_work_remaining = remaining;
        true
    }

    fn consume_rewrites(&mut self, units: u32) -> bool {
        let Some(remaining) = self.rewrite_steps_remaining.checked_sub(units) else {
            self.limit_reached
                .get_or_insert(ComputationLimitKind::RewriteSteps);
            return false;
        };
        self.rewrite_steps_remaining = remaining;
        true
    }
}

fn rebuild_node(
    source: &ExactExpressionDag,
    id: ExprId,
    target: &mut ExactExpressionDag,
) -> ExpressionNode {
    match source.node(id) {
        ExpressionNode::Rational(value) => {
            let rational = RationalId(target.rationals.len() as u32);
            target.rationals.push(source.rational(*value).clone());
            ExpressionNode::Rational(rational)
        }
        ExpressionNode::Exact(_) => unreachable!("lowered source DAG cannot contain exact values"),
        ExpressionNode::Constant(value) => ExpressionNode::Constant(*value),
        ExpressionNode::Add(values) => {
            let values = target.push_list(source.list(*values).to_vec());
            ExpressionNode::Add(values)
        }
        ExpressionNode::Multiply(values) => {
            let values = target.push_list(source.list(*values).to_vec());
            ExpressionNode::Multiply(values)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => ExpressionNode::Divide {
            numerator: *numerator,
            denominator: *denominator,
        },
        ExpressionNode::Power { base, exponent } => ExpressionNode::Power {
            base: *base,
            exponent: *exponent,
        },
        ExpressionNode::LogBase { argument, base } => ExpressionNode::LogBase {
            argument: *argument,
            base: *base,
        },
        ExpressionNode::Function { function, argument } => ExpressionNode::Function {
            function: *function,
            argument: *argument,
        },
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => ExpressionNode::BinaryFunction {
            function: *function,
            left: *left,
            right: *right,
        },
    }
}

enum RecognizedExactSubexpression {
    Rational(RationalEvaluation),
    PiMultiple(PiCoefficientEvaluation),
    Radical(RadicalReduction),
    RealAlgebraic {
        value: RealAlgebraicEvaluation,
        canonical_polynomial: Option<BuilderPolynomial>,
    },
    CanonicalPolynomial(BuilderPolynomial),
    Symbolic,
}

fn recognize_exact_subexpression(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
    budget: &mut ReductionBudget,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    if !budget.consume_node() {
        validate_obvious_domain(dag, id)?;
        return Ok(None);
    }
    let Some(logarithm_work) = reserved_logarithm_identity_work(dag, id, budget) else {
        validate_obvious_domain(dag, id)?;
        return Ok(None);
    };
    if !budget
        .consume_work(estimated_additional_work(dag, id, limits)?.saturating_add(logarithm_work))
    {
        validate_obvious_domain(dag, id)?;
        return Ok(None);
    }
    if zero_product_has_unproven_factor_domain(dag, id)? {
        return Ok(None);
    }
    if let Some(value) = recognize_exact_operation(dag, id, limits)? {
        return Ok(Some(value));
    }
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(Some(RecognizedExactSubexpression::Rational(value))),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    if let Some(value) = evaluate_pi_coefficient(dag, id)? {
        return Ok(Some(RecognizedExactSubexpression::PiMultiple(value)));
    }
    if let Some(value) = evaluate_radical_node(dag, id)? {
        return Ok(Some(RecognizedExactSubexpression::Radical(value)));
    }
    if let Some(value) = evaluate_real_algebraic_node(dag, id, limits)? {
        let canonical_polynomial = recognize_canonical_polynomial(dag, id, limits, budget)?;
        return Ok(Some(RecognizedExactSubexpression::RealAlgebraic {
            value,
            canonical_polynomial,
        }));
    }
    if let Some(polynomial) = recognize_canonical_polynomial(dag, id, limits, budget)? {
        return Ok(Some(match polynomial.terms.as_slice() {
            [] => {
                RecognizedExactSubexpression::Rational(RationalEvaluation::direct(Rational::zero()))
            }
            [term] if term.factors.is_empty() => RecognizedExactSubexpression::Rational(
                RationalEvaluation::direct(term.coefficient.clone()),
            ),
            [_] | [_, ..] => RecognizedExactSubexpression::CanonicalPolynomial(polynomial),
        }));
    }
    if budget.limit_reached.is_some() {
        validate_obvious_domain(dag, id)?;
    }
    Ok(None)
}

fn recognize_canonical_polynomial(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
    budget: &mut ReductionBudget,
) -> Result<Option<BuilderPolynomial>, EvaluationError> {
    if !matches!(
        dag.semantic_node(id),
        ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
    ) || !expression_domain_known_well_defined(dag, id)?
    {
        return Ok(None);
    }
    let term_limit = usize::try_from(limits.max_expression_nodes).unwrap_or(usize::MAX);
    let Some(polynomial) = exact_polynomial_from_expression(dag, id, budget, term_limit)? else {
        return Ok(None);
    };
    if polynomial_is_single_atom(dag, id, &polynomial) {
        Ok(None)
    } else {
        Ok(Some(polynomial))
    }
}

fn polynomial_is_single_atom(
    dag: &ExactExpressionDag,
    id: ExprId,
    polynomial: &BuilderPolynomial,
) -> bool {
    let [term] = polynomial.terms.as_slice() else {
        return false;
    };
    term.coefficient == Rational::one()
        && matches!(term.factors.as_slice(), [(base, 1)] if structurally_equal_expressions(dag, id, *base))
}

fn exact_polynomial_from_expression(
    dag: &ExactExpressionDag,
    id: ExprId,
    budget: &mut ReductionBudget,
    term_limit: usize,
) -> Result<Option<BuilderPolynomial>, EvaluationError> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(value) => Ok(Some(BuilderPolynomial {
            terms: vec![BuilderMonomial {
                coefficient: dag.rational(*value).clone(),
                factors: Vec::new(),
            }],
        })),
        ExpressionNode::Add(values) => {
            let mut polynomial = BuilderPolynomial { terms: Vec::new() };
            for child in dag.list(*values) {
                let Some(child) =
                    exact_polynomial_from_expression(dag, *child, budget, term_limit)?
                else {
                    return Ok(None);
                };
                let Some(sum) = exact_add_polynomials(dag, polynomial, child, budget, term_limit)
                else {
                    return Ok(None);
                };
                polynomial = sum;
            }
            Ok(Some(polynomial))
        }
        ExpressionNode::Multiply(values) => {
            let mut polynomial = BuilderPolynomial::one();
            for child in dag.list(*values) {
                let Some(child) =
                    exact_polynomial_from_expression(dag, *child, budget, term_limit)?
                else {
                    return Ok(None);
                };
                let Some(product) =
                    exact_multiply_polynomials(dag, polynomial, child, budget, term_limit)?
                else {
                    return Ok(None);
                };
                polynomial = product;
            }
            Ok(Some(polynomial))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            if !prove_expression_nonzero(dag, *denominator)? {
                return Ok(None);
            }
            let Some(numerator) =
                exact_polynomial_from_expression(dag, *numerator, budget, term_limit)?
            else {
                return Ok(None);
            };
            let Some(denominator) =
                exact_polynomial_from_expression(dag, *denominator, budget, term_limit)?
            else {
                return Ok(None);
            };
            let [denominator] = denominator.terms.as_slice() else {
                return Ok(None);
            };
            if denominator.coefficient.is_zero() {
                return Ok(None);
            }
            for (base, _) in &denominator.factors {
                if !prove_expression_nonzero(dag, *base)? {
                    return Ok(None);
                }
            }
            let coefficient = Rational::one()
                .divide(&denominator.coefficient)
                .map_err(arithmetic_error)?;
            let mut factors = Vec::with_capacity(denominator.factors.len());
            for (base, exponent) in &denominator.factors {
                let Some(exponent) = exponent.checked_neg() else {
                    return Ok(None);
                };
                factors.push((*base, exponent));
            }
            exact_multiply_polynomials(
                dag,
                numerator,
                BuilderPolynomial {
                    terms: vec![BuilderMonomial {
                        coefficient,
                        factors,
                    }],
                },
                budget,
                term_limit,
            )
        }
        ExpressionNode::Power { base, exponent } => {
            let Ok(exponent) = evaluate_node(dag, *exponent) else {
                return Ok(Some(exact_atomic_polynomial(id)));
            };
            let Some(exponent) = exponent.value().as_i64_if_integer() else {
                return Ok(Some(exact_atomic_polynomial(id)));
            };
            if exponent < 0 && !prove_expression_nonzero(dag, *base)? {
                return Ok(Some(exact_atomic_polynomial(id)));
            }
            let Some(base) = exact_polynomial_from_expression(dag, *base, budget, term_limit)?
            else {
                return Ok(None);
            };
            if exponent < 0 && base.terms.len() != 1 {
                return Ok(Some(exact_atomic_polynomial(id)));
            }
            exact_power_polynomial(dag, base, exponent, budget, term_limit)
        }
        ExpressionNode::Exact(value) => {
            let stored = &dag.exact_values[value.0 as usize];
            let scaled_monomial = stored
                .canonical_polynomial
                .as_ref()
                .filter(|polynomial| {
                    matches!(
                        polynomial.terms.as_slice(),
                        [term] if term.coefficient != Rational::one()
                    )
                })
                .cloned();
            Ok(Some(
                scaled_monomial.unwrap_or_else(|| exact_atomic_polynomial(id)),
            ))
        }
        ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(Some(exact_atomic_polynomial(id))),
    }
}

fn exact_atomic_polynomial(id: ExprId) -> BuilderPolynomial {
    BuilderPolynomial {
        terms: vec![BuilderMonomial {
            coefficient: Rational::one(),
            factors: vec![(id, 1)],
        }],
    }
}

fn exact_add_polynomials(
    dag: &ExactExpressionDag,
    mut left: BuilderPolynomial,
    right: BuilderPolynomial,
    budget: &mut ReductionBudget,
    term_limit: usize,
) -> Option<BuilderPolynomial> {
    let work = left.terms.len().saturating_add(right.terms.len());
    if work > term_limit || !budget.consume_work(u64::try_from(work).unwrap_or(u64::MAX)) {
        return None;
    }
    left.terms.extend(right.terms);
    exact_normalize_polynomial_terms(dag, left, budget, term_limit)
}

fn exact_multiply_polynomials(
    dag: &ExactExpressionDag,
    left: BuilderPolynomial,
    right: BuilderPolynomial,
    budget: &mut ReductionBudget,
    term_limit: usize,
) -> Result<Option<BuilderPolynomial>, EvaluationError> {
    let work = left.terms.len().saturating_mul(right.terms.len());
    if work > term_limit || !budget.consume_work(u64::try_from(work).unwrap_or(u64::MAX)) {
        return Ok(None);
    }
    let factor_work =
        polynomial_factor_product_work(&left, &right, |id| dag.structural_sizes[id.0 as usize]);
    if !budget.consume_work(u64::try_from(factor_work).unwrap_or(u64::MAX)) {
        return Ok(None);
    }
    let mut terms = Vec::with_capacity(work);
    for left in &left.terms {
        for right in &right.terms {
            let Some(product) = exact_multiply_monomials(dag, left, right)? else {
                return Ok(None);
            };
            terms.push(product);
        }
    }
    Ok(exact_normalize_polynomial_terms(
        dag,
        BuilderPolynomial { terms },
        budget,
        term_limit,
    ))
}

fn exact_multiply_monomials(
    dag: &ExactExpressionDag,
    left: &BuilderMonomial,
    right: &BuilderMonomial,
) -> Result<Option<BuilderMonomial>, EvaluationError> {
    let mut factors = Vec::with_capacity(left.factors.len().saturating_add(right.factors.len()));
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.factors.len() && right_index < right.factors.len() {
        let left_factor = left.factors[left_index];
        let right_factor = right.factors[right_index];
        match compare_exact_expressions(dag, left_factor.0, right_factor.0) {
            Ordering::Less => {
                factors.push(left_factor);
                left_index += 1;
            }
            Ordering::Greater => {
                factors.push(right_factor);
                right_index += 1;
            }
            Ordering::Equal => {
                let Some(exponent) = left_factor.1.checked_add(right_factor.1) else {
                    return Ok(None);
                };
                if exponent != 0 {
                    factors.push((left_factor.0, exponent));
                } else if !prove_expression_nonzero(dag, left_factor.0)? {
                    return Ok(None);
                }
                left_index += 1;
                right_index += 1;
            }
        }
    }
    factors.extend_from_slice(&left.factors[left_index..]);
    factors.extend_from_slice(&right.factors[right_index..]);
    Ok(Some(BuilderMonomial {
        coefficient: left.coefficient.multiply(&right.coefficient),
        factors,
    }))
}

fn exact_power_polynomial(
    dag: &ExactExpressionDag,
    base: BuilderPolynomial,
    exponent: i64,
    budget: &mut ReductionBudget,
    term_limit: usize,
) -> Result<Option<BuilderPolynomial>, EvaluationError> {
    if exponent == 0 {
        return Ok(Some(BuilderPolynomial::one()));
    }
    if exponent < 0 {
        let [term] = base.terms.as_slice() else {
            return Ok(None);
        };
        let Some(magnitude) = exponent.checked_abs() else {
            return Ok(None);
        };
        let coefficient = term
            .coefficient
            .pow_i64(exponent)
            .map_err(arithmetic_error)?;
        let mut factors = Vec::with_capacity(term.factors.len());
        for (base, factor_exponent) in &term.factors {
            let Some(exponent) = factor_exponent.checked_mul(-magnitude) else {
                return Ok(None);
            };
            factors.push((*base, exponent));
        }
        return Ok(Some(BuilderPolynomial {
            terms: vec![BuilderMonomial {
                coefficient,
                factors,
            }],
        }));
    }

    let mut exponent = u64::try_from(exponent).map_err(|_| {
        EvaluationError::ComputationLimit(ComputationLimitError {
            kind: ComputationLimitKind::LogicalWorkUnits,
        })
    })?;
    let mut factor = base;
    let mut result = BuilderPolynomial::one();
    while exponent > 0 {
        if exponent & 1 == 1 {
            let Some(product) =
                exact_multiply_polynomials(dag, result, factor.clone(), budget, term_limit)?
            else {
                return Ok(None);
            };
            result = product;
        }
        exponent >>= 1;
        if exponent > 0 {
            let Some(square) =
                exact_multiply_polynomials(dag, factor.clone(), factor, budget, term_limit)?
            else {
                return Ok(None);
            };
            factor = square;
        }
    }
    Ok(Some(result))
}

fn exact_normalize_polynomial_terms(
    dag: &ExactExpressionDag,
    mut polynomial: BuilderPolynomial,
    budget: &mut ReductionBudget,
    term_limit: usize,
) -> Option<BuilderPolynomial> {
    let merge_work =
        polynomial_term_merge_work(&polynomial, |id| dag.structural_sizes[id.0 as usize]);
    if !budget.consume_work(u64::try_from(merge_work).unwrap_or(u64::MAX)) {
        return None;
    }
    // Every monomial constructor preserves the strict factor order: atoms contain one
    // factor, powers preserve order, and products use a two-pointer canonical merge.
    debug_assert!(polynomial.terms.iter().all(|term| {
        term.factors.iter().all(|(_, exponent)| *exponent != 0)
            && term.factors.windows(2).all(|factors| {
                compare_exact_expressions(dag, factors[0].0, factors[1].0) == Ordering::Less
            })
    }));
    polynomial
        .terms
        .sort_by(|left, right| compare_exact_monomials(dag, left, right));

    let mut terms = Vec::<BuilderMonomial>::new();
    for term in polynomial.terms {
        if term.coefficient.is_zero() {
            continue;
        }
        if let Some(existing) = terms.last_mut() {
            if exact_monomials_equal(dag, existing, &term) {
                existing.coefficient = existing.coefficient.add(&term.coefficient);
                continue;
            }
        }
        terms.push(term);
        if terms.len() > term_limit {
            return None;
        }
    }
    terms.retain(|term| !term.coefficient.is_zero());
    let (complement_rewrites, complement_work) = polynomial_trig_complement_plan(
        &terms,
        |id| dag.structural_sizes[id.0 as usize],
        |id, exponent| trig_square_candidate_kind(dag.semantic_node(id), exponent),
        |left, right| trig_complement_monomials_match(dag, left, right),
    );
    if !budget.consume_rewrites(u32::try_from(complement_rewrites).unwrap_or(u32::MAX))
        || !budget.consume_work(u64::try_from(complement_work).unwrap_or(u64::MAX))
    {
        return None;
    }
    reduce_trig_square_complements(dag, &mut terms);
    Some(BuilderPolynomial { terms })
}

fn polynomial_trig_complement_plan(
    terms: &[BuilderMonomial],
    factor_weight: impl Fn(ExprId) -> usize + Copy,
    candidate_kind: impl Fn(ExprId, i64) -> u8,
    compatible: impl Fn(&BuilderMonomial, &BuilderMonomial) -> bool,
) -> (usize, usize) {
    let (sines, cosines) = terms.iter().flat_map(|term| &term.factors).fold(
        (0usize, 0usize),
        |(sines, cosines), (id, exponent)| match candidate_kind(*id, *exponent) {
            1 => (sines.saturating_add(1), cosines),
            2 => (sines, cosines.saturating_add(1)),
            _ => (sines, cosines),
        },
    );
    if sines == 0 || cosines == 0 {
        return (0, 0);
    }
    let compatible_pairs = terms
        .iter()
        .enumerate()
        .map(|(index, left)| {
            terms[index + 1..]
                .iter()
                .filter(|right| compatible(left, right))
                .count()
        })
        .sum::<usize>();
    let candidate_count = sines.saturating_add(cosines);
    let rewrite_bound = if compatible_pairs == 0 {
        0
    } else {
        candidate_count.saturating_sub(1)
    };
    let scan_work = terms
        .iter()
        .enumerate()
        .fold(0usize, |work, (index, left)| {
            terms[index + 1..].iter().fold(work, |work, right| {
                work.saturating_add(
                    monomial_structural_weight(left, factor_weight)
                        .saturating_mul(monomial_structural_weight(right, factor_weight)),
                )
            })
        });
    let merge_work = polynomial_term_merge_work(
        &BuilderPolynomial {
            terms: terms.to_vec(),
        },
        factor_weight,
    );
    let coefficient_work = terms
        .iter()
        .map(|term| rational_structural_size(&term.coefficient))
        .sum::<usize>()
        .saturating_mul(8);
    let iteration_work = scan_work
        .saturating_add(merge_work)
        .saturating_add(coefficient_work);
    (
        rewrite_bound,
        rewrite_bound
            .saturating_add(1)
            .saturating_mul(iteration_work),
    )
}

fn trig_square_candidate_kind(node: &ExpressionNode, exponent: i64) -> u8 {
    if exponent != 2 {
        return 0;
    }
    match node {
        ExpressionNode::Function {
            function: Function::Sin,
            ..
        } => 1,
        ExpressionNode::Function {
            function: Function::Cos,
            ..
        } => 2,
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => 0,
    }
}

fn reduce_trig_square_complements(dag: &ExactExpressionDag, terms: &mut Vec<BuilderMonomial>) {
    loop {
        let mut match_indices = None;
        'pairs: for left_index in 0..terms.len() {
            for right_index in left_index + 1..terms.len() {
                for left_factor in 0..terms[left_index].factors.len() {
                    let Some((left_function, left_argument)) =
                        trig_square_factor(dag, &terms[left_index], left_factor)
                    else {
                        continue;
                    };
                    for right_factor in 0..terms[right_index].factors.len() {
                        let Some((right_function, right_argument)) =
                            trig_square_factor(dag, &terms[right_index], right_factor)
                        else {
                            continue;
                        };
                        if !trig_functions_are_complements(left_function, right_function)
                            || compare_exact_expressions(dag, left_argument, right_argument)
                                != Ordering::Equal
                            || !monomial_factors_equal_without(
                                dag,
                                &terms[left_index],
                                left_factor,
                                &terms[right_index],
                                right_factor,
                            )
                        {
                            continue;
                        }
                        let Some(coefficient) = common_signed_coefficient(
                            &terms[left_index].coefficient,
                            &terms[right_index].coefficient,
                        ) else {
                            continue;
                        };
                        match_indices = Some((left_index, right_index, left_factor, coefficient));
                        break 'pairs;
                    }
                }
            }
        }

        let Some((left_index, right_index, left_factor, coefficient)) = match_indices else {
            break;
        };
        let mut common_factors = terms[left_index].factors.clone();
        common_factors.remove(left_factor);
        terms[left_index].coefficient = terms[left_index].coefficient.subtract(&coefficient);
        terms[right_index].coefficient = terms[right_index].coefficient.subtract(&coefficient);
        terms.push(BuilderMonomial {
            coefficient,
            factors: common_factors,
        });
        terms.retain(|term| !term.coefficient.is_zero());
        terms.sort_by(|left, right| compare_exact_monomials(dag, left, right));
        let mut merged = Vec::<BuilderMonomial>::new();
        for term in terms.drain(..) {
            if let Some(existing) = merged.last_mut() {
                if exact_monomials_equal(dag, existing, &term) {
                    existing.coefficient = existing.coefficient.add(&term.coefficient);
                    continue;
                }
            }
            merged.push(term);
        }
        merged.retain(|term| !term.coefficient.is_zero());
        *terms = merged;
    }
}

fn trig_complement_monomials_match(
    dag: &ExactExpressionDag,
    left: &BuilderMonomial,
    right: &BuilderMonomial,
) -> bool {
    for left_factor in 0..left.factors.len() {
        let Some((left_function, left_argument)) = trig_square_factor(dag, left, left_factor)
        else {
            continue;
        };
        for right_factor in 0..right.factors.len() {
            let Some((right_function, right_argument)) =
                trig_square_factor(dag, right, right_factor)
            else {
                continue;
            };
            if trig_functions_are_complements(left_function, right_function)
                && compare_exact_expressions(dag, left_argument, right_argument) == Ordering::Equal
                && monomial_factors_equal_without(dag, left, left_factor, right, right_factor)
                && common_signed_coefficient(&left.coefficient, &right.coefficient).is_some()
            {
                return true;
            }
        }
    }
    false
}

fn trig_square_factor(
    dag: &ExactExpressionDag,
    term: &BuilderMonomial,
    factor_index: usize,
) -> Option<(Function, ExprId)> {
    let (base, exponent) = term.factors[factor_index];
    if exponent != 2 {
        return None;
    }
    let ExpressionNode::Function { function, argument } = dag.semantic_node(base) else {
        return None;
    };
    matches!(function, Function::Sin | Function::Cos).then_some((*function, *argument))
}

fn trig_functions_are_complements(left: Function, right: Function) -> bool {
    matches!(
        (left, right),
        (Function::Sin, Function::Cos) | (Function::Cos, Function::Sin)
    )
}

fn monomial_factors_equal_without(
    dag: &ExactExpressionDag,
    left: &BuilderMonomial,
    left_skip: usize,
    right: &BuilderMonomial,
    right_skip: usize,
) -> bool {
    if left.factors.len() != right.factors.len() {
        return false;
    }
    left.factors
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != left_skip)
        .zip(
            right
                .factors
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != right_skip),
        )
        .all(|((_, left), (_, right))| {
            left.1 == right.1 && compare_exact_expressions(dag, left.0, right.0) == Ordering::Equal
        })
}

fn common_signed_coefficient(left: &Rational, right: &Rational) -> Option<Rational> {
    if left.is_negative() && right.is_negative() {
        Some(if left.compare(right) == Ordering::Greater {
            left.clone()
        } else {
            right.clone()
        })
    } else if !left.is_negative() && !right.is_negative() {
        Some(if left.compare(right) == Ordering::Less {
            left.clone()
        } else {
            right.clone()
        })
    } else {
        None
    }
}

fn polynomial_factor_product_work(
    left: &BuilderPolynomial,
    right: &BuilderPolynomial,
    factor_weight: impl Fn(ExprId) -> usize + Copy,
) -> usize {
    left.terms.iter().fold(0usize, |work, left| {
        right.terms.iter().fold(work, |work, right| {
            work.saturating_add(
                monomial_structural_weight(left, factor_weight)
                    .saturating_mul(monomial_structural_weight(right, factor_weight)),
            )
        })
    })
}

fn polynomial_term_merge_work(
    polynomial: &BuilderPolynomial,
    factor_weight: impl Fn(ExprId) -> usize + Copy,
) -> usize {
    let term_count = polynomial.terms.len();
    let max_term_weight = polynomial
        .terms
        .iter()
        .map(|term| monomial_structural_weight(term, factor_weight))
        .max()
        .unwrap_or(1);
    let pairwise = term_count
        .saturating_mul(term_count.saturating_sub(1))
        .saturating_div(2)
        .saturating_mul(max_term_weight);
    let sort_levels = if term_count <= 1 {
        0
    } else {
        usize::BITS as usize - term_count.saturating_sub(1).leading_zeros() as usize
    };
    pairwise.saturating_add(
        term_count
            .saturating_mul(sort_levels)
            .saturating_mul(max_term_weight),
    )
}

fn monomial_structural_weight(
    monomial: &BuilderMonomial,
    factor_weight: impl Fn(ExprId) -> usize,
) -> usize {
    monomial.factors.iter().fold(1usize, |weight, (base, _)| {
        weight.saturating_add(factor_weight(*base))
    })
}

fn exact_monomials_equal(
    dag: &ExactExpressionDag,
    left: &BuilderMonomial,
    right: &BuilderMonomial,
) -> bool {
    left.factors.len() == right.factors.len()
        && left.factors.iter().zip(&right.factors).all(
            |((left_base, left_exponent), (right_base, right_exponent))| {
                left_exponent == right_exponent
                    && compare_exact_expressions(dag, *left_base, *right_base) == Ordering::Equal
            },
        )
}

fn compare_exact_monomials(
    dag: &ExactExpressionDag,
    left: &BuilderMonomial,
    right: &BuilderMonomial,
) -> Ordering {
    let category = |term: &BuilderMonomial| {
        if term.factors.is_empty() {
            1
        } else if term.coefficient.is_negative() {
            2
        } else {
            0
        }
    };
    category(left).cmp(&category(right)).then_with(|| {
        for ((left_base, left_exponent), (right_base, right_exponent)) in
            left.factors.iter().zip(&right.factors)
        {
            let order = compare_exact_expressions(dag, *left_base, *right_base)
                .then_with(|| left_exponent.cmp(right_exponent));
            if order != Ordering::Equal {
                return order;
            }
        }
        left.factors.len().cmp(&right.factors.len())
    })
}

fn compare_exact_expressions(dag: &ExactExpressionDag, left: ExprId, right: ExprId) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    let left_node = dag.semantic_node(left);
    let right_node = dag.semantic_node(right);
    let rank_order = expression_node_rank(left_node).cmp(&expression_node_rank(right_node));
    if rank_order != Ordering::Equal {
        return rank_order;
    }
    match (left_node, right_node) {
        (ExpressionNode::Rational(left), ExpressionNode::Rational(right)) => {
            dag.rational(*left).compare(dag.rational(*right))
        }
        (ExpressionNode::Exact(left), ExpressionNode::Exact(right)) => {
            compare_exact_reductions(dag.exact_value(*left), dag.exact_value(*right))
        }
        (ExpressionNode::Constant(left), ExpressionNode::Constant(right)) => {
            constant_rank(*left).cmp(&constant_rank(*right))
        }
        (ExpressionNode::Add(left), ExpressionNode::Add(right))
        | (ExpressionNode::Multiply(left), ExpressionNode::Multiply(right)) => {
            compare_exact_expression_lists(dag, *left, *right)
        }
        (
            ExpressionNode::Divide {
                numerator: left_numerator,
                denominator: left_denominator,
            },
            ExpressionNode::Divide {
                numerator: right_numerator,
                denominator: right_denominator,
            },
        ) => compare_exact_expressions(dag, *left_numerator, *right_numerator)
            .then_with(|| compare_exact_expressions(dag, *left_denominator, *right_denominator)),
        (
            ExpressionNode::Power {
                base: left_base,
                exponent: left_exponent,
            },
            ExpressionNode::Power {
                base: right_base,
                exponent: right_exponent,
            },
        ) => compare_exact_expressions(dag, *left_base, *right_base)
            .then_with(|| compare_exact_expressions(dag, *left_exponent, *right_exponent)),
        (
            ExpressionNode::LogBase {
                argument: left_argument,
                base: left_base,
            },
            ExpressionNode::LogBase {
                argument: right_argument,
                base: right_base,
            },
        ) => compare_exact_expressions(dag, *left_argument, *right_argument)
            .then_with(|| compare_exact_expressions(dag, *left_base, *right_base)),
        (
            ExpressionNode::Function {
                function: left_function,
                argument: left_argument,
            },
            ExpressionNode::Function {
                function: right_function,
                argument: right_argument,
            },
        ) => function_rank(*left_function)
            .cmp(&function_rank(*right_function))
            .then_with(|| compare_exact_expressions(dag, *left_argument, *right_argument)),
        (
            ExpressionNode::BinaryFunction {
                function: left_function,
                left: left_left,
                right: left_right,
            },
            ExpressionNode::BinaryFunction {
                function: right_function,
                left: right_left,
                right: right_right,
            },
        ) => function_rank(*left_function)
            .cmp(&function_rank(*right_function))
            .then_with(|| compare_exact_expressions(dag, *left_left, *right_left))
            .then_with(|| compare_exact_expressions(dag, *left_right, *right_right)),
        (
            ExpressionNode::Rational(_)
            | ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. },
            ExpressionNode::Rational(_)
            | ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. },
        ) => unreachable!("different expression variants have distinct structural ranks"),
    }
}

fn expression_node_rank(node: &ExpressionNode) -> u8 {
    match node {
        ExpressionNode::Rational(_) => 0,
        ExpressionNode::Constant(_) => 1,
        ExpressionNode::Power { .. } => 2,
        ExpressionNode::Function { .. } => 3,
        ExpressionNode::LogBase { .. } => 4,
        ExpressionNode::Multiply(_) => 5,
        ExpressionNode::Divide { .. } => 6,
        ExpressionNode::Add(_) => 7,
        ExpressionNode::BinaryFunction { .. } => 8,
        ExpressionNode::Exact(_) => 9,
    }
}

fn compare_exact_expression_lists(
    dag: &ExactExpressionDag,
    left: ExprListId,
    right: ExprListId,
) -> Ordering {
    let left = dag.list(left);
    let right = dag.list(right);
    for (left, right) in left.iter().zip(right) {
        let order = compare_exact_expressions(dag, *left, *right);
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_exact_reductions(left: &ExactReduction, right: &ExactReduction) -> Ordering {
    let rank = |value: &ExactReduction| match value {
        ExactReduction::PiMultiple(_) => 0,
        ExactReduction::Radical(_) => 1,
        ExactReduction::RealAlgebraic(_) => 2,
        ExactReduction::Symbolic => 3,
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| match (left, right) {
            (ExactReduction::PiMultiple(left), ExactReduction::PiMultiple(right)) => {
                left.coefficient().compare(right.coefficient())
            }
            (ExactReduction::Radical(left), ExactReduction::Radical(right)) => {
                compare_radical_reductions(left, right)
            }
            (ExactReduction::RealAlgebraic(left), ExactReduction::RealAlgebraic(right)) => {
                compare_real_algebraic_evaluations(left, right)
            }
            (ExactReduction::Symbolic, ExactReduction::Symbolic) => Ordering::Equal,
            (
                ExactReduction::PiMultiple(_)
                | ExactReduction::Radical(_)
                | ExactReduction::RealAlgebraic(_)
                | ExactReduction::Symbolic,
                ExactReduction::PiMultiple(_)
                | ExactReduction::Radical(_)
                | ExactReduction::RealAlgebraic(_)
                | ExactReduction::Symbolic,
            ) => unreachable!("different exact reduction variants have distinct ranks"),
        })
}

fn compare_radical_reductions(left: &RadicalReduction, right: &RadicalReduction) -> Ordering {
    let rank = |value: &RadicalReduction| match value {
        RadicalReduction::Rational(_) => 0,
        RadicalReduction::Radical(_) => 1,
        RadicalReduction::LinearCombination(_) => 2,
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| match (left, right) {
            (RadicalReduction::Rational(left), RadicalReduction::Rational(right)) => {
                left.value().compare(right.value())
            }
            (RadicalReduction::Radical(left), RadicalReduction::Radical(right)) => {
                compare_simple_radicals(left.value(), right.value())
            }
            (
                RadicalReduction::LinearCombination(left),
                RadicalReduction::LinearCombination(right),
            ) => compare_radical_linear_combinations(left.value(), right.value()),
            (
                RadicalReduction::Rational(_)
                | RadicalReduction::Radical(_)
                | RadicalReduction::LinearCombination(_),
                RadicalReduction::Rational(_)
                | RadicalReduction::Radical(_)
                | RadicalReduction::LinearCombination(_),
            ) => unreachable!("different radical variants have distinct ranks"),
        })
}

fn compare_simple_radicals(left: &SimpleRadical, right: &SimpleRadical) -> Ordering {
    left.coefficient
        .compare(&right.coefficient)
        .then_with(|| left.radicand.inner.inner.cmp(&right.radicand.inner.inner))
}

fn compare_radical_linear_combinations(
    left: &RadicalLinearCombination,
    right: &RadicalLinearCombination,
) -> Ordering {
    left.rational.compare(&right.rational).then_with(|| {
        for (left, right) in left.radicals.iter().zip(&right.radicals) {
            let order = compare_simple_radicals(left, right);
            if order != Ordering::Equal {
                return order;
            }
        }
        left.radicals.len().cmp(&right.radicals.len())
    })
}

fn compare_real_algebraic_evaluations(
    left: &RealAlgebraicEvaluation,
    right: &RealAlgebraicEvaluation,
) -> Ordering {
    match (left, right) {
        (RealAlgebraicEvaluation::Rational(left), RealAlgebraicEvaluation::Rational(right)) => {
            left.value().compare(right.value())
        }
        (RealAlgebraicEvaluation::Rational(_), RealAlgebraicEvaluation::Algebraic(_)) => {
            Ordering::Less
        }
        (RealAlgebraicEvaluation::Algebraic(_), RealAlgebraicEvaluation::Rational(_)) => {
            Ordering::Greater
        }
        (RealAlgebraicEvaluation::Algebraic(left), RealAlgebraicEvaluation::Algebraic(right)) => {
            compare_primitive_polynomials(&left.minimal_polynomial, &right.minimal_polynomial)
                .then_with(|| left.real_root_index.cmp(&right.real_root_index))
        }
    }
}

fn compare_primitive_polynomials(
    left: &PrimitivePolynomial,
    right: &PrimitivePolynomial,
) -> Ordering {
    for (left, right) in left
        .coefficients_low_to_high
        .iter()
        .zip(&right.coefficients_low_to_high)
    {
        let order = left.inner.cmp(&right.inner);
        if order != Ordering::Equal {
            return order;
        }
    }
    left.coefficients_low_to_high
        .len()
        .cmp(&right.coefficients_low_to_high.len())
}

fn zero_product_has_unproven_factor_domain(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    let ExpressionNode::Multiply(values) = dag.semantic_node(id) else {
        return Ok(false);
    };
    let factors = dag.list(*values);
    let has_zero = factors
        .iter()
        .any(|factor| matches!(evaluate_node(dag, *factor), Ok(value) if value.value().is_zero()));
    if !has_zero {
        return Ok(false);
    }
    for factor in factors {
        if !expression_domain_known_well_defined(dag, *factor)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn recognize_exact_operation(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    match dag.semantic_node(id) {
        ExpressionNode::Add(list) => {
            if let Some(value) = recognize_logarithm_sum(dag, *list)? {
                Ok(Some(value))
            } else {
                recognize_structural_zero_sum(dag, *list)
            }
        }
        ExpressionNode::Multiply(list) => recognize_logarithm_chain_product(dag, *list),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => recognize_logarithm_change_of_base(dag, *numerator, *denominator),
        ExpressionNode::Function {
            function: Function::Abs,
            argument,
        } => recognize_exact_absolute_value(dag, *argument, limits),
        ExpressionNode::Function {
            function: Function::Floor,
            argument,
        } => recognize_exact_floor(dag, *argument),
        ExpressionNode::Function {
            function: Function::Factorial,
            argument,
        } if exact_node_is_proven_noninteger(dag, *argument) => Err(domain_error(
            DomainErrorKind::IntegerFunctionRequiresInteger,
        )),
        ExpressionNode::BinaryFunction {
            function:
                Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm,
            left,
            right,
        } if exact_node_is_proven_noninteger(dag, *left)
            || exact_node_is_proven_noninteger(dag, *right) =>
        {
            Err(domain_error(
                DomainErrorKind::IntegerFunctionRequiresInteger,
            ))
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn recognize_logarithm_sum(
    dag: &ExactExpressionDag,
    list: ExprListId,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let Some(reduction) = logarithm_sum_reduction(dag, dag.list(list))? else {
        return Ok(None);
    };
    let mut argument = Rational::one();
    let mut used_special_angle = reduction.scale.used_special_angle();
    for argument_id in &reduction.numerator_arguments {
        let value = match evaluate_node(dag, *argument_id) {
            Ok(value) => value,
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        argument = argument.multiply(value.value());
        used_special_angle |= value.used_special_angle();
    }
    for argument_id in &reduction.denominator_arguments {
        let value = match evaluate_node(dag, *argument_id) {
            Ok(value) => value,
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        argument = argument.divide(value.value()).map_err(arithmetic_error)?;
        used_special_angle |= value.used_special_angle();
    }
    let base = match evaluate_node(dag, reduction.base) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match finish_rational_log_base(
        RationalEvaluation::with_origin(argument, used_special_angle),
        base,
    ) {
        Ok(value) => Ok(Some(RecognizedExactSubexpression::Rational(
            RationalEvaluation::with_origin(
                value
                    .value()
                    .multiply(reduction.scale.value())
                    .add(reduction.offset.value()),
                value.used_special_angle()
                    || reduction.scale.used_special_angle()
                    || reduction.offset.used_special_angle(),
            ),
        ))),
        Err(error) if is_unsupported_exact_expression(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

fn recognize_logarithm_change_of_base(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let Some((argument, base, scale)) = logarithm_quotient_identity(dag, numerator, denominator)?
    else {
        return Ok(None);
    };
    match evaluate_log_base_function(dag, argument, base) {
        Ok(value) => Ok(Some(RecognizedExactSubexpression::Rational(
            RationalEvaluation::with_origin(
                value.value().multiply(scale.value()),
                value.used_special_angle() || scale.used_special_angle(),
            ),
        ))),
        Err(error) if is_unsupported_exact_expression(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

fn recognize_logarithm_chain_product(
    dag: &ExactExpressionDag,
    list: ExprListId,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let factors = dag.list(list);
    let Some(reduction) = logarithm_product_reduction(dag, factors)? else {
        return Ok(None);
    };
    let value = if let Some((argument, base)) = reduction.logarithm {
        match evaluate_log_base_function(dag, argument, base) {
            Ok(value) => RationalEvaluation::with_origin(
                value.value().multiply(reduction.scale.value()),
                value.used_special_angle() || reduction.scale.used_special_angle(),
            ),
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
            Err(error) => return Err(error),
        }
    } else {
        reduction.scale
    };
    Ok(Some(RecognizedExactSubexpression::Rational(value)))
}

fn validate_obvious_domain(dag: &ExactExpressionDag, id: ExprId) -> Result<(), EvaluationError> {
    match dag.semantic_node(id) {
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => {
            if semantic_sign_for_domain(dag, *argument)? == Some(Ordering::Less) {
                return Err(domain_error(DomainErrorKind::EvenRootOfNegative));
            }
        }
        ExpressionNode::Divide { denominator, .. } => {
            if semantic_sign_for_domain(dag, *denominator)? == Some(Ordering::Equal) {
                return Err(domain_error(DomainErrorKind::DivisionByZero));
            }
        }
        ExpressionNode::Power { base, exponent } => {
            let base_sign = semantic_sign_for_domain(dag, *base)?;
            let exponent_value = semantic_rational_for_domain(dag, *exponent);
            if base_sign == Some(Ordering::Equal) {
                if exponent_value.as_ref().is_some_and(Rational::is_zero) {
                    return Err(domain_error(DomainErrorKind::IndeterminateZeroToZero));
                }
                if exponent_value.as_ref().is_some_and(Rational::is_negative) {
                    return Err(domain_error(DomainErrorKind::ZeroToNegativePower));
                }
            }
            if base_sign == Some(Ordering::Less) && prove_noninteger_for_domain(dag, *exponent)? {
                return Err(domain_error(DomainErrorKind::NonRealPower));
            }
        }
        ExpressionNode::LogBase { argument, base } => {
            if matches!(
                semantic_sign_for_domain(dag, *argument)?,
                Some(Ordering::Less | Ordering::Equal)
            ) {
                return Err(logarithm_of_non_positive_error());
            }
            if matches!(
                semantic_sign_for_domain(dag, *base)?,
                Some(Ordering::Less | Ordering::Equal)
            ) {
                return Err(logarithm_of_non_positive_error());
            }
            if prove_expression_one_for_domain(dag, *base) {
                return Err(domain_error(DomainErrorKind::LogarithmBaseOne));
            }
        }
        ExpressionNode::Function {
            function: Function::Log | Function::Ln,
            argument,
        } => {
            if matches!(
                semantic_sign_for_domain(dag, *argument)?,
                Some(Ordering::Less | Ordering::Equal)
            ) {
                return Err(logarithm_of_non_positive_error());
            }
        }
        ExpressionNode::Function {
            function: Function::Tan,
            argument,
        } => {
            if let Some(coefficient) = semantic_rational_pi_coefficient(dag, *argument) {
                let phase = coefficient.modulo_integer(1);
                if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
                    return Err(domain_error(DomainErrorKind::TangentPole));
                }
            }
        }
        ExpressionNode::Function {
            function: Function::Asin | Function::Acos,
            argument,
        } => {
            if let Some(argument) = semantic_rational_for_domain(dag, *argument) {
                if argument.compare(&rational_integer(-1)) == Ordering::Less
                    || argument.compare(&Rational::one()) == Ordering::Greater
                {
                    return Err(domain_error(
                        DomainErrorKind::InverseTrigonometricOutOfRange,
                    ));
                }
            }
        }
        ExpressionNode::Function {
            function: Function::Factorial,
            argument,
        } => {
            if prove_noninteger_for_domain(dag, *argument)? {
                return Err(domain_error(
                    DomainErrorKind::IntegerFunctionRequiresInteger,
                ));
            }
            if let Some(argument) = semantic_rational_for_domain(dag, *argument) {
                if argument.is_negative() {
                    return Err(domain_error(
                        DomainErrorKind::IntegerFunctionRequiresNonNegative,
                    ));
                }
            }
        }
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => {
            let left_value = semantic_rational_for_domain(dag, *left);
            let right_value = semantic_rational_for_domain(dag, *right);
            if prove_noninteger_for_domain(dag, *left)? || prove_noninteger_for_domain(dag, *right)?
            {
                return Err(domain_error(
                    DomainErrorKind::IntegerFunctionRequiresInteger,
                ));
            }
            if matches!(function, Function::Permutation | Function::Combination)
                && [left_value.as_ref(), right_value.as_ref()]
                    .into_iter()
                    .flatten()
                    .any(Rational::is_negative)
            {
                return Err(domain_error(
                    DomainErrorKind::IntegerFunctionRequiresNonNegative,
                ));
            }
            if matches!(function, Function::Modulo)
                && right_value.as_ref().is_some_and(Rational::is_zero)
            {
                return Err(domain_error(DomainErrorKind::DivisionByZero));
            }
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Function { .. } => {}
    }
    Ok(())
}

fn semantic_sign_for_domain(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<Ordering>, EvaluationError> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(value) => Ok(Some(rational_sign(dag.rational(*value)))),
        ExpressionNode::Exact(value) => Ok(exact_reduction_sign(dag.exact_value(*value))),
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(Some(Ordering::Greater)),
        ExpressionNode::Add(values) => {
            let mut saw_negative = false;
            let mut saw_positive = false;
            for child in dag.list(*values) {
                match semantic_sign_for_domain(dag, *child)? {
                    Some(Ordering::Less) => saw_negative = true,
                    Some(Ordering::Greater) => saw_positive = true,
                    Some(Ordering::Equal) => {}
                    None => return Ok(None),
                }
                if saw_negative && saw_positive {
                    return Ok(None);
                }
            }
            Ok(Some(match (saw_negative, saw_positive) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => Ordering::Equal,
                (true, true) => unreachable!(),
            }))
        }
        ExpressionNode::Multiply(values) => {
            let mut negative = false;
            for child in dag.list(*values) {
                match semantic_sign_for_domain(dag, *child)? {
                    Some(Ordering::Less) => negative = !negative,
                    Some(Ordering::Equal) => return Ok(Some(Ordering::Equal)),
                    Some(Ordering::Greater) => {}
                    None => return Ok(None),
                }
            }
            Ok(Some(if negative {
                Ordering::Less
            } else {
                Ordering::Greater
            }))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => match (
            semantic_sign_for_domain(dag, *numerator)?,
            semantic_sign_for_domain(dag, *denominator)?,
        ) {
            (Some(Ordering::Equal), Some(Ordering::Less | Ordering::Greater)) => {
                Ok(Some(Ordering::Equal))
            }
            (Some(numerator), Some(denominator))
                if numerator != Ordering::Equal && denominator != Ordering::Equal =>
            {
                Ok(Some(if numerator == denominator {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }))
            }
            _ => Ok(None),
        },
        ExpressionNode::Power { base, exponent } => {
            let Some(exponent) = semantic_rational_for_domain(dag, *exponent) else {
                return Ok(None);
            };
            let Some(exponent) = exponent.as_i64_if_integer() else {
                return Ok(None);
            };
            let base = semantic_sign_for_domain(dag, *base)?;
            if exponent == 0 {
                return Ok(base
                    .filter(|sign| *sign != Ordering::Equal)
                    .map(|_| Ordering::Greater));
            }
            if exponent % 2 == 0 {
                Ok(base.map(|sign| {
                    if sign == Ordering::Equal {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                }))
            } else {
                Ok(base)
            }
        }
        ExpressionNode::Function { function, argument } => {
            match function {
                Function::Exp => Ok(expression_domain_known_well_defined(dag, *argument)?
                    .then_some(Ordering::Greater)),
                Function::Sin => {
                    let Some(argument) = semantic_rational_for_domain(dag, *argument) else {
                        return Ok(None);
                    };
                    let sign = rational_sign(&argument);
                    let magnitude = if sign == Ordering::Less {
                        argument.negate()
                    } else {
                        argument
                    };
                    Ok((magnitude.compare(&rational_integer(3)) == Ordering::Less).then_some(sign))
                }
                Function::Sqrt | Function::Abs => match semantic_sign_for_domain(dag, *argument)? {
                    Some(Ordering::Equal) => Ok(Some(Ordering::Equal)),
                    Some(Ordering::Less | Ordering::Greater) => Ok(Some(Ordering::Greater)),
                    None => Ok(None),
                },
                Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Root
                | Function::Log
                | Function::Ln
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh => Ok(None),
            }
        }
        ExpressionNode::LogBase { .. } | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn semantic_rational_pi_coefficient(dag: &ExactExpressionDag, id: ExprId) -> Option<Rational> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(value) => dag.rational(*value).is_zero().then(Rational::zero),
        ExpressionNode::Constant(Constant::Pi) => Some(Rational::one()),
        ExpressionNode::Add(values) => {
            let mut total = Rational::zero();
            for child in dag.list(*values) {
                total = total.add(&semantic_rational_pi_coefficient(dag, *child)?);
            }
            Some(total)
        }
        ExpressionNode::Multiply(values) => {
            let mut scalar = Rational::one();
            let mut coefficient = None;
            for child in dag.list(*values) {
                if let ExpressionNode::Rational(value) = dag.semantic_node(*child) {
                    scalar = scalar.multiply(dag.rational(*value));
                    continue;
                }
                let child = semantic_rational_pi_coefficient(dag, *child)?;
                if coefficient.replace(child).is_some() {
                    return None;
                }
            }
            coefficient.map(|coefficient| scalar.multiply(&coefficient))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let numerator = semantic_rational_pi_coefficient(dag, *numerator)?;
            let ExpressionNode::Rational(denominator) = dag.semantic_node(*denominator) else {
                return None;
            };
            numerator.divide(dag.rational(*denominator)).ok()
        }
        ExpressionNode::Exact(_)
        | ExpressionNode::Constant(Constant::Euler)
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn semantic_rational_for_domain(dag: &ExactExpressionDag, id: ExprId) -> Option<Rational> {
    match dag.semantic_node(id) {
        ExpressionNode::Rational(value) => Some(dag.rational(*value).clone()),
        ExpressionNode::Add(values) => {
            let mut total = Rational::zero();
            for child in dag.list(*values) {
                total = total.add(&semantic_rational_for_domain(dag, *child)?);
            }
            Some(total)
        }
        ExpressionNode::Multiply(values) => {
            let mut product = Rational::one();
            for child in dag.list(*values) {
                product = product.multiply(&semantic_rational_for_domain(dag, *child)?);
            }
            Some(product)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => semantic_rational_for_domain(dag, *numerator)?
            .divide(&semantic_rational_for_domain(dag, *denominator)?)
            .ok(),
        ExpressionNode::Power { base, exponent } => {
            let base = semantic_rational_for_domain(dag, *base)?;
            let exponent = semantic_rational_for_domain(dag, *exponent)?;
            let exponent = exponent.as_i64_if_integer()?;
            (exponent.unsigned_abs() <= 1024)
                .then(|| base.pow_i64(exponent).ok())
                .flatten()
        }
        ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn prove_noninteger_for_domain(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    if let Some(value) = semantic_rational_for_domain(dag, id) {
        return Ok(!value.is_integer());
    }
    match dag.semantic_node(id) {
        ExpressionNode::Exact(value) => {
            Ok(!matches!(dag.exact_value(*value), ExactReduction::Symbolic))
        }
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => {
            let Some(argument) = semantic_rational_for_domain(dag, *argument) else {
                return Ok(false);
            };
            if argument.is_negative() {
                return Err(domain_error(DomainErrorKind::EvenRootOfNegative));
            }
            Ok(argument.sqrt_if_rational().is_none())
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn prove_expression_one_for_domain(dag: &ExactExpressionDag, id: ExprId) -> bool {
    if semantic_rational_for_domain(dag, id).is_some_and(|value| value == Rational::one()) {
        return true;
    }
    matches!(
        dag.semantic_node(id),
        ExpressionNode::Function {
            function: Function::Exp,
            argument,
        } if semantic_sign_for_domain(dag, *argument).ok().flatten() == Some(Ordering::Equal)
    )
}

fn estimated_additional_work(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<u64, EvaluationError> {
    let integer_work = |argument: ExprId| -> Result<u64, EvaluationError> {
        let value = match evaluate_node(dag, argument) {
            Ok(value) => value,
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(0),
            Err(error) => return Err(error),
        };
        if !value.value().is_integer() || value.value().is_negative() {
            return Ok(0);
        }
        Ok(value.value().numerator.inner.to_u64().unwrap_or(u64::MAX))
    };

    let integer_work = match dag.semantic_node(id) {
        ExpressionNode::Function {
            function: Function::Factorial,
            argument,
        } => integer_work(*argument),
        ExpressionNode::BinaryFunction {
            function: Function::Permutation | Function::Combination,
            left,
            ..
        } => integer_work(*left),
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(0),
    }?;
    let large_exp_interval_work = match dag.semantic_node(id) {
        ExpressionNode::Function {
            function: Function::Exp,
            argument,
        } => match dag.semantic_node(*argument) {
            ExpressionNode::Rational(value) => {
                let value = dag.rational(*value);
                let numerator = value.numerator.inner.magnitude();
                let denominator = value.denominator.inner.inner.magnitude();
                let numerator_bits = numerator.bits();
                let denominator_bits = denominator.bits();
                let uses_binary_scaling = if numerator_bits != denominator_bits.saturating_add(6) {
                    numerator_bits > denominator_bits.saturating_add(6)
                } else {
                    numerator > &(denominator << 6_u32)
                };
                if !uses_binary_scaling {
                    0
                } else {
                    4_u64.saturating_mul(130_u64.saturating_add(numerator_bits))
                }
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => {
                // Interval evaluation can prove a non-rational endpoint large enough to
                // select binary scaling (for example, 100*pi). Reserve a fixed
                // conservative guard rather than letting that work bypass the shared
                // normalization budget. Known rational arguments retain the exact
                // |x| > 64 path predicate above, so ordinary exp(+-2) is unchanged.
                4_u64.saturating_mul(130_u64.saturating_add(64))
            }
        },
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => 0,
    };
    Ok(integer_work
        .saturating_add(large_exp_interval_work)
        .saturating_add(reserved_algebraic_work(dag, id, limits)))
}

fn reserved_logarithm_identity_work(
    dag: &ExactExpressionDag,
    id: ExprId,
    budget: &mut ReductionBudget,
) -> Option<u64> {
    match dag.semantic_node(id) {
        ExpressionNode::Add(terms) | ExpressionNode::Multiply(terms) => {
            let mut work = 0u64;
            let mut logarithms = 0u64;
            for term in dag.list(*terms) {
                let (term_work, term_logarithms) = logarithm_factor_work(dag, *term, budget)?;
                work = work.saturating_add(term_work);
                logarithms = logarithms.saturating_add(term_logarithms);
            }
            if logarithms < 2 {
                Some(0)
            } else {
                Some(
                    work.saturating_add(logarithms.saturating_mul(logarithms))
                        .saturating_mul(2),
                )
            }
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let (numerator_work, numerator_logarithms) =
                logarithm_factor_work(dag, *numerator, budget)?;
            let (denominator_work, denominator_logarithms) =
                logarithm_factor_work(dag, *denominator, budget)?;
            if numerator_logarithms == 0 || denominator_logarithms == 0 {
                Some(0)
            } else {
                Some(
                    numerator_work
                        .saturating_add(denominator_work)
                        .saturating_mul(2),
                )
            }
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Some(0),
    }
}

fn logarithm_factor_work(
    dag: &ExactExpressionDag,
    id: ExprId,
    budget: &mut ReductionBudget,
) -> Option<(u64, u64)> {
    if !budget.consume_work(1) {
        return None;
    }
    match dag.semantic_node(id) {
        ExpressionNode::Function {
            function: Function::Log | Function::Ln,
            ..
        }
        | ExpressionNode::LogBase { .. } => Some((expression_structural_work(dag, id, budget)?, 1)),
        ExpressionNode::Add(factors) | ExpressionNode::Multiply(factors) => {
            let mut work = 1u64;
            let mut logarithms = 0u64;
            for factor in dag.list(*factors) {
                let (factor_work, factor_logarithms) = logarithm_factor_work(dag, *factor, budget)?;
                work = work.saturating_add(factor_work);
                logarithms = logarithms.saturating_add(factor_logarithms);
            }
            Some((work, logarithms))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let (numerator_work, numerator_logarithms) =
                logarithm_factor_work(dag, *numerator, budget)?;
            let (denominator_work, denominator_logarithms) =
                logarithm_factor_work(dag, *denominator, budget)?;
            Some((
                1u64.saturating_add(numerator_work)
                    .saturating_add(denominator_work),
                numerator_logarithms.saturating_add(denominator_logarithms),
            ))
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Power { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => {
            Some((expression_structural_work(dag, id, budget)?, 0))
        }
    }
}

fn expression_structural_work(
    dag: &ExactExpressionDag,
    id: ExprId,
    budget: &mut ReductionBudget,
) -> Option<u64> {
    if !budget.consume_work(1) {
        return None;
    }
    match dag.semantic_node(id) {
        ExpressionNode::Rational(_) | ExpressionNode::Exact(_) | ExpressionNode::Constant(_) => {
            Some(1)
        }
        ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => {
            let mut work = 1u64;
            for value in dag.list(*values) {
                work = work.saturating_add(expression_structural_work(dag, *value, budget)?);
            }
            Some(work)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        }
        | ExpressionNode::Power {
            base: numerator,
            exponent: denominator,
        }
        | ExpressionNode::LogBase {
            argument: numerator,
            base: denominator,
        }
        | ExpressionNode::BinaryFunction {
            left: numerator,
            right: denominator,
            ..
        } => Some(
            1u64.saturating_add(expression_structural_work(dag, *numerator, budget)?)
                .saturating_add(expression_structural_work(dag, *denominator, budget)?),
        ),
        ExpressionNode::Function { argument, .. } => {
            Some(1u64.saturating_add(expression_structural_work(dag, *argument, budget)?))
        }
    }
}

// Algebraic algorithms have operation-local limits, but normalization needs one
// cumulative limit for the whole expression. Reserve a deterministic upper bound
// before entering an algebraic operation; unused work is intentionally not
// refunded, so actual work can never exceed the shared logical-work budget.
fn reserved_algebraic_work(dag: &ExactExpressionDag, id: ExprId, limits: &ResourceLimits) -> u64 {
    let pipeline = |resultant_work: u64| {
        resultant_work
            .saturating_add(u64::from(limits.max_factorization_work))
            .saturating_add(u64::from(limits.max_root_isolation_steps))
    };
    let binary_pipeline = |lhs_degree: u64, rhs_degree: u64| {
        let matrix_degree = lhs_degree.saturating_add(rhs_degree);
        let interpolation_points = lhs_degree.saturating_mul(rhs_degree).saturating_add(1);
        pipeline(interpolation_points.saturating_mul(matrix_degree.saturating_pow(3)))
    };
    let power_pipeline = |base_degree: u64, exponent: i64| {
        let magnitude = exponent.unsigned_abs();
        if magnitude == 0 {
            return 0;
        }
        let maximum_degree = u64::from(limits.max_algebraic_degree);
        let mut remaining = magnitude;
        let mut factor_degree = base_degree;
        let mut result_degree = None;
        let mut work = 0u64;
        while remaining > 0 {
            if remaining & 1 == 1 {
                if let Some(current_degree) = result_degree {
                    work = work.saturating_add(binary_pipeline(current_degree, factor_degree));
                    result_degree = Some(
                        current_degree
                            .saturating_mul(factor_degree)
                            .min(maximum_degree),
                    );
                } else {
                    result_degree = Some(factor_degree);
                }
            }
            remaining >>= 1;
            if remaining > 0 {
                work = work.saturating_add(binary_pipeline(factor_degree, factor_degree));
                factor_degree = factor_degree
                    .saturating_mul(factor_degree)
                    .min(maximum_degree);
            }
        }
        if exponent < 0 {
            work.saturating_add(pipeline(base_degree))
        } else {
            work
        }
    };

    match dag.semantic_node(id) {
        ExpressionNode::Power { base, exponent } => {
            let Ok(exponent) = evaluate_node(dag, *exponent) else {
                return 0;
            };
            if exponent.value().is_integer() {
                let Some(base_degree) = exact_algebraic_degree(dag, *base) else {
                    return 0;
                };
                let Some(exponent) = exponent.value().as_i64_if_integer() else {
                    return u64::MAX;
                };
                power_pipeline(base_degree, exponent)
            } else {
                let root_index = exponent
                    .value()
                    .denominator
                    .inner
                    .inner
                    .to_u64()
                    .unwrap_or(u64::MAX);
                let algebraic_degree = exact_algebraic_degree(dag, *base);
                let integer_power = algebraic_degree
                    .and_then(|degree| {
                        exponent
                            .value()
                            .numerator
                            .inner
                            .to_i64()
                            .map(|numerator| power_pipeline(degree, numerator))
                    })
                    .unwrap_or(0);
                let sign_proofs = algebraic_degree
                    .map(|_| u64::from(limits.max_root_isolation_steps).saturating_mul(2))
                    .unwrap_or(0);
                integer_power
                    .saturating_add(sign_proofs)
                    .saturating_add(pipeline(root_index))
            }
        }
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => {
            if exact_algebraic_degree(dag, *argument).is_some()
                || evaluate_node(dag, *argument).is_ok()
            {
                pipeline(2)
            } else {
                0
            }
        }
        ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => {
            let degrees = dag
                .list(*values)
                .iter()
                .filter_map(|child| exact_algebraic_degree(dag, *child))
                .collect::<Vec<_>>();
            match degrees.as_slice() {
                [] => 0,
                [degree] => pipeline(*degree),
                [first, rest @ ..] => {
                    let (work, result_degree) =
                        rest.iter()
                            .fold((0u64, *first), |(work, accumulated_degree), degree| {
                                let operation = binary_pipeline(accumulated_degree, *degree);
                                let result_degree = accumulated_degree
                                    .saturating_mul(*degree)
                                    .min(u64::from(limits.max_algebraic_degree));
                                (work.saturating_add(operation), result_degree)
                            });
                    work.saturating_add(pipeline(result_degree))
                }
            }
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => match (
            exact_algebraic_degree(dag, *numerator),
            exact_algebraic_degree(dag, *denominator),
        ) {
            (Some(lhs), Some(rhs)) => pipeline(rhs).saturating_add(binary_pipeline(lhs, rhs)),
            (Some(degree), None) | (None, Some(degree)) => pipeline(degree),
            (None, None) => 0,
        },
        ExpressionNode::Function {
            function: Function::Sin | Function::Cos | Function::Tan,
            ..
        } => pipeline(u64::from(limits.max_cyclotomic_order)),
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => 0,
    }
}

fn exact_algebraic_degree(dag: &ExactExpressionDag, id: ExprId) -> Option<u64> {
    let ExpressionNode::Exact(value) = dag.node(id) else {
        return None;
    };
    match dag.exact_value(*value) {
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) => value
            .minimal_polynomial()
            .degree()
            .and_then(|degree| u64::try_from(degree).ok()),
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Rational(_)) => Some(1),
        ExactReduction::PiMultiple(_) | ExactReduction::Radical(_) | ExactReduction::Symbolic => {
            None
        }
    }
}

fn exact_node_is_proven_noninteger(dag: &ExactExpressionDag, id: ExprId) -> bool {
    let ExpressionNode::Exact(value) = dag.node(id) else {
        return false;
    };
    !matches!(dag.exact_value(*value), ExactReduction::Symbolic)
}

fn recognize_structural_zero_sum(
    dag: &ExactExpressionDag,
    list: ExprListId,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let terms = dag.list(list);
    if terms.len() != 2 {
        return Ok(None);
    }
    for (positive, negative) in [(terms[0], terms[1]), (terms[1], terms[0])] {
        let ExpressionNode::Multiply(factors) = dag.semantic_node(negative) else {
            continue;
        };
        let factors = dag.list(*factors);
        if factors.len() != 2 {
            continue;
        }
        let value = if matches!(
            evaluate_node(dag, factors[0]),
            Ok(value) if value.value() == &rational_integer(-1)
        ) {
            factors[1]
        } else if matches!(
            evaluate_node(dag, factors[1]),
            Ok(value) if value.value() == &rational_integer(-1)
        ) {
            factors[0]
        } else {
            continue;
        };
        if structurally_equal_expressions(dag, positive, value)
            && expression_domain_known_well_defined(dag, positive)?
        {
            return Ok(Some(RecognizedExactSubexpression::Rational(
                RationalEvaluation::direct(Rational::zero()),
            )));
        }
    }
    Ok(None)
}

fn recognize_exact_absolute_value(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let ExpressionNode::Exact(value) = dag.node(argument) else {
        return Ok(None);
    };
    let reduction = dag.exact_value(*value);
    let sign = exact_reduction_sign_bounded(reduction, limits)?;
    match sign {
        Some(Ordering::Greater | Ordering::Equal) => {
            Ok(Some(recognized_from_exact_reduction(reduction.clone())))
        }
        Some(Ordering::Less) => {
            Ok(negate_exact_reduction(reduction, limits)?.map(recognized_from_exact_reduction))
        }
        None => Ok(None),
    }
}

fn recognize_exact_floor(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RecognizedExactSubexpression>, EvaluationError> {
    let ExpressionNode::Exact(_value) = dag.node(argument) else {
        return Ok(None);
    };
    let interval =
        evaluate_interval_node(dag, argument, 128).map_err(evaluation_error_from_interval)?;
    Ok(interval::unique_floor(&interval)
        .map_err(evaluation_error_from_interval)?
        .map(|value| RecognizedExactSubexpression::Rational(RationalEvaluation::direct(value))))
}

fn recognized_from_exact_reduction(value: ExactReduction) -> RecognizedExactSubexpression {
    match value {
        ExactReduction::PiMultiple(value) => RecognizedExactSubexpression::PiMultiple(value),
        ExactReduction::Radical(value) => RecognizedExactSubexpression::Radical(value),
        ExactReduction::RealAlgebraic(value) => RecognizedExactSubexpression::RealAlgebraic {
            value,
            canonical_polynomial: None,
        },
        ExactReduction::Symbolic => RecognizedExactSubexpression::Symbolic,
    }
}

fn store_exact_subexpression(
    dag: &mut ExactExpressionDag,
    value: RecognizedExactSubexpression,
    presentation: ExpressionNode,
    normalization: &mut ExactNormalization,
) -> ExpressionNode {
    let mut canonical_polynomial = None;
    let reduction = match value {
        RecognizedExactSubexpression::Rational(value) => {
            normalization.used_special_angle |= value.used_special_angle();
            return store_rational_node(dag, value.into_value());
        }
        RecognizedExactSubexpression::PiMultiple(value) => {
            normalization.used_special_angle |= value.used_special_angle();
            if value.coefficient().is_zero() {
                return store_rational_node(dag, value.into_coefficient());
            }
            ExactReduction::PiMultiple(value)
        }
        RecognizedExactSubexpression::Radical(value) => {
            normalization.used_special_angle |= value.used_special_angle();
            normalization.used_radical_reduction = true;
            match value {
                RadicalReduction::Rational(value) => {
                    return store_rational_node(dag, value.into_value());
                }
                value @ (RadicalReduction::Radical(_) | RadicalReduction::LinearCombination(_)) => {
                    ExactReduction::Radical(value)
                }
            }
        }
        RecognizedExactSubexpression::RealAlgebraic {
            value,
            canonical_polynomial: polynomial,
        } => {
            canonical_polynomial = polynomial;
            normalization.used_algebraic_reduction = true;
            normalization.used_cyclotomic_reduction |= matches!(
                &presentation,
                ExpressionNode::Function {
                    function: Function::Sin | Function::Cos | Function::Tan,
                    ..
                }
            );
            match value {
                RealAlgebraicEvaluation::Rational(value) => {
                    normalization.used_special_angle |= value.used_special_angle();
                    return store_rational_node(dag, value.into_value());
                }
                value => ExactReduction::RealAlgebraic(value),
            }
        }
        RecognizedExactSubexpression::CanonicalPolynomial(polynomial) => {
            canonical_polynomial = Some(polynomial);
            ExactReduction::Symbolic
        }
        RecognizedExactSubexpression::Symbolic => ExactReduction::Symbolic,
    };
    let presentation = canonical_exact_presentation(dag, &presentation, &reduction);
    let value = ExactValueId(dag.exact_values.len() as u32);
    dag.exact_values.push(StoredExactValue {
        reduction,
        presentation,
        canonical_polynomial,
    });
    ExpressionNode::Exact(value)
}

fn canonical_exact_presentation(
    dag: &ExactExpressionDag,
    presentation: &ExpressionNode,
    reduction: &ExactReduction,
) -> ExpressionNode {
    if !matches!(reduction, ExactReduction::RealAlgebraic(_)) {
        return presentation.clone();
    }
    let ExpressionNode::Function {
        function: Function::Abs,
        argument,
    } = presentation
    else {
        return presentation.clone();
    };
    let ExpressionNode::Exact(value) = dag.node(*argument) else {
        return presentation.clone();
    };
    if exact_reduction_sign(dag.exact_value(*value)) == Some(Ordering::Greater) {
        dag.exact_presentation(*value).clone()
    } else {
        presentation.clone()
    }
}

fn store_rational_node(dag: &mut ExactExpressionDag, value: Rational) -> ExpressionNode {
    let rational = RationalId(dag.rationals.len() as u32);
    dag.rationals.push(value);
    ExpressionNode::Rational(rational)
}

fn materialize_stored_canonical_polynomials(
    dag: &mut ExactExpressionDag,
    limits: &ResourceLimits,
    budget: &mut ReductionBudget,
) {
    for index in 0..dag.exact_values.len() {
        let Some(polynomial) = dag.exact_values[index].canonical_polynomial.take() else {
            continue;
        };
        let estimated_nodes = canonical_polynomial_node_estimate(&polynomial);
        let fits_expression_limit = dag
            .nodes
            .len()
            .checked_add(estimated_nodes)
            .is_some_and(|nodes| nodes <= limits.max_expression_nodes as usize);
        if !fits_expression_limit {
            budget
                .limit_reached
                .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
            continue;
        }
        let mut reserved = true;
        for _ in 0..estimated_nodes {
            if !budget.consume_node() {
                reserved = false;
                break;
            }
        }
        if !reserved {
            continue;
        }
        dag.exact_values[index].presentation = materialize_exact_polynomial(dag, polynomial);
    }
}

fn canonical_polynomial_node_estimate(polynomial: &BuilderPolynomial) -> usize {
    polynomial
        .terms
        .iter()
        .map(|term| {
            let powers = term
                .factors
                .iter()
                .filter(|(_, exponent)| exponent.unsigned_abs() > 1)
                .count()
                .saturating_mul(2);
            let coefficient =
                usize::from(term.coefficient != Rational::one() || term.factors.is_empty());
            powers
                .saturating_add(coefficient)
                .saturating_add(term.factors.len())
                .saturating_add(3)
        })
        .sum::<usize>()
        .saturating_add(1)
}

fn materialize_exact_polynomial(
    dag: &mut ExactExpressionDag,
    polynomial: BuilderPolynomial,
) -> ExpressionNode {
    let terms = polynomial
        .terms
        .into_iter()
        .map(|term| materialize_exact_monomial(dag, term))
        .collect::<Vec<_>>();
    match terms.as_slice() {
        [] => append_rational_node(dag, Rational::zero()),
        [term] => dag.node(*term).clone(),
        [_, ..] => {
            let list = dag.push_list(terms);
            ExpressionNode::Add(list)
        }
    }
}

fn materialize_exact_monomial(dag: &mut ExactExpressionDag, monomial: BuilderMonomial) -> ExprId {
    let mut numerator = Vec::new();
    let mut denominator = Vec::new();
    for (base, exponent) in monomial.factors {
        let (target, magnitude) = if exponent < 0 {
            (&mut denominator, exponent.unsigned_abs())
        } else {
            (&mut numerator, exponent as u64)
        };
        if magnitude == 0 {
            continue;
        }
        let factor = if magnitude == 1 {
            base
        } else {
            let exponent = append_rational_expr(
                dag,
                rational_integer(
                    i64::try_from(magnitude)
                        .expect("canonical factor exponent magnitude fits in i64"),
                ),
            );
            append_expression_node(dag, ExpressionNode::Power { base, exponent })
        };
        target.push(factor);
    }
    if monomial.coefficient != Rational::one() || numerator.is_empty() {
        numerator.insert(0, append_rational_expr(dag, monomial.coefficient));
    }
    let numerator = append_product(dag, numerator);
    if denominator.is_empty() {
        numerator
    } else {
        let denominator = append_product(dag, denominator);
        append_expression_node(
            dag,
            ExpressionNode::Divide {
                numerator,
                denominator,
            },
        )
    }
}

fn append_product(dag: &mut ExactExpressionDag, factors: Vec<ExprId>) -> ExprId {
    match factors.as_slice() {
        [factor] => *factor,
        [_, ..] => {
            let list = dag.push_list(factors);
            append_expression_node(dag, ExpressionNode::Multiply(list))
        }
        [] => append_rational_expr(dag, Rational::one()),
    }
}

fn append_rational_node(dag: &mut ExactExpressionDag, value: Rational) -> ExpressionNode {
    let rational = RationalId(dag.rationals.len() as u32);
    dag.rationals.push(value);
    ExpressionNode::Rational(rational)
}

fn append_rational_expr(dag: &mut ExactExpressionDag, value: Rational) -> ExprId {
    let node = append_rational_node(dag, value);
    append_expression_node(dag, node)
}

fn append_expression_node(dag: &mut ExactExpressionDag, node: ExpressionNode) -> ExprId {
    let id = ExprId(dag.nodes.len() as u32);
    let structural_size = match &node {
        ExpressionNode::Exact(value) => exact_reduction_structural_size(dag.exact_value(*value)),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => expression_node_structural_size(
            &node,
            &dag.structural_sizes,
            &dag.lists,
            &dag.rationals,
        ),
    };
    dag.nodes.push(node);
    dag.structural_sizes.push(structural_size);
    id
}

fn expression_node_structural_size(
    node: &ExpressionNode,
    structural_sizes: &[usize],
    lists: &[Vec<ExprId>],
    rationals: &[Rational],
) -> usize {
    let child_size = |id: ExprId| structural_sizes[id.0 as usize];
    match node {
        ExpressionNode::Rational(value) => rational_structural_size(&rationals[value.0 as usize]),
        ExpressionNode::Exact(_) | ExpressionNode::Constant(_) => 1,
        ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => {
            lists[values.0 as usize].iter().fold(1usize, |size, child| {
                size.saturating_add(child_size(*child))
            })
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => 1usize
            .saturating_add(child_size(*numerator))
            .saturating_add(child_size(*denominator)),
        ExpressionNode::Power { base, exponent }
        | ExpressionNode::LogBase {
            argument: base,
            base: exponent,
        } => 1usize
            .saturating_add(child_size(*base))
            .saturating_add(child_size(*exponent)),
        ExpressionNode::Function { argument, .. } => 1usize.saturating_add(child_size(*argument)),
        ExpressionNode::BinaryFunction { left, right, .. } => 1usize
            .saturating_add(child_size(*left))
            .saturating_add(child_size(*right)),
    }
}

fn exact_reduction_structural_size(value: &ExactReduction) -> usize {
    match value {
        ExactReduction::PiMultiple(value) => {
            1usize.saturating_add(rational_structural_size(value.coefficient()))
        }
        ExactReduction::Radical(value) => match value {
            RadicalReduction::Rational(value) => {
                1usize.saturating_add(rational_structural_size(value.value()))
            }
            RadicalReduction::Radical(value) => {
                1usize.saturating_add(simple_radical_structural_size(value.value()))
            }
            RadicalReduction::LinearCombination(value) => {
                let value = value.value();
                value.radicals.iter().fold(
                    1usize.saturating_add(rational_structural_size(&value.rational)),
                    |size, radical| size.saturating_add(simple_radical_structural_size(radical)),
                )
            }
        },
        ExactReduction::RealAlgebraic(value) => match value {
            RealAlgebraicEvaluation::Rational(value) => {
                1usize.saturating_add(rational_structural_size(value.value()))
            }
            RealAlgebraicEvaluation::Algebraic(value) => value
                .minimal_polynomial
                .coefficients_low_to_high
                .iter()
                .fold(2usize, |size, coefficient| {
                    size.saturating_add(integer_structural_size(coefficient))
                }),
        },
        ExactReduction::Symbolic => 1,
    }
}

fn simple_radical_structural_size(value: &SimpleRadical) -> usize {
    1usize
        .saturating_add(rational_structural_size(&value.coefficient))
        .saturating_add(integer_structural_size(&value.radicand.inner))
}

fn rational_structural_size(value: &Rational) -> usize {
    1usize
        .saturating_add(integer_structural_size(&value.numerator))
        .saturating_add(integer_structural_size(&value.denominator.inner))
}

fn integer_structural_size(value: &Integer) -> usize {
    let magnitude_bits = value.inner.bits();
    let negative_power_of_two = value.inner.sign() == Sign::Minus
        && value.inner.trailing_zeros() == magnitude_bits.checked_sub(1);
    let signed_bits = if negative_power_of_two {
        magnitude_bits
    } else {
        magnitude_bits.saturating_add(1)
    };
    let limb_bits = u64::from(u64::BITS);
    let limbs = signed_bits.saturating_add(limb_bits - 1) / limb_bits;
    usize::try_from(limbs).unwrap_or(usize::MAX).max(1)
}

#[cfg(test)]
pub(crate) fn evaluate_rational_dag(dag: &ExactExpressionDag) -> Result<Rational, EvaluationError> {
    Ok(evaluate_rational_evaluation_dag(dag)?.into_value())
}

pub(crate) fn evaluate_rational_evaluation_dag(
    dag: &ExactExpressionDag,
) -> Result<RationalEvaluation, EvaluationError> {
    evaluate_node(dag, dag.root())
}

pub(crate) fn evaluate_interval_dag(
    dag: &ExactExpressionDag,
) -> Result<CertifiedInterval, IntervalError> {
    evaluate_interval_node(dag, dag.root(), EVALUATION_INTERVAL_PRECISION_BITS)
}

pub(crate) fn evaluate_rational_pi_multiple_dag(
    dag: &ExactExpressionDag,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    evaluate_pi_coefficient(dag, dag.root())
}

pub(crate) fn evaluate_radical_dag(
    dag: &ExactExpressionDag,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    evaluate_radical_node(dag, dag.root())
}

pub(crate) fn evaluate_real_algebraic_dag(
    dag: &ExactExpressionDag,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    evaluate_real_algebraic_node(dag, dag.root(), limits)
}

pub(crate) fn structurally_equal_expressions(
    dag: &ExactExpressionDag,
    left: ExprId,
    right: ExprId,
) -> bool {
    if left == right {
        return true;
    }

    match (dag.semantic_node(left), dag.semantic_node(right)) {
        (ExpressionNode::Rational(left), ExpressionNode::Rational(right)) => {
            dag.rational(*left) == dag.rational(*right)
        }
        (ExpressionNode::Exact(left), ExpressionNode::Exact(right)) => {
            compare_exact_reductions(dag.exact_value(*left), dag.exact_value(*right))
                == Ordering::Equal
        }
        (ExpressionNode::Constant(left), ExpressionNode::Constant(right)) => left == right,
        (ExpressionNode::Add(left), ExpressionNode::Add(right))
        | (ExpressionNode::Multiply(left), ExpressionNode::Multiply(right)) => {
            structurally_equal_expression_lists(dag, *left, *right)
        }
        (
            ExpressionNode::Divide {
                numerator: left_numerator,
                denominator: left_denominator,
            },
            ExpressionNode::Divide {
                numerator: right_numerator,
                denominator: right_denominator,
            },
        ) => {
            structurally_equal_expressions(dag, *left_numerator, *right_numerator)
                && structurally_equal_expressions(dag, *left_denominator, *right_denominator)
        }
        (
            ExpressionNode::Power {
                base: left_base,
                exponent: left_exponent,
            },
            ExpressionNode::Power {
                base: right_base,
                exponent: right_exponent,
            },
        ) => {
            structurally_equal_expressions(dag, *left_base, *right_base)
                && structurally_equal_expressions(dag, *left_exponent, *right_exponent)
        }
        (
            ExpressionNode::LogBase {
                argument: left_argument,
                base: left_base,
            },
            ExpressionNode::LogBase {
                argument: right_argument,
                base: right_base,
            },
        ) => {
            structurally_equal_expressions(dag, *left_argument, *right_argument)
                && structurally_equal_expressions(dag, *left_base, *right_base)
        }
        (
            ExpressionNode::Function {
                function: left_function,
                argument: left_argument,
            },
            ExpressionNode::Function {
                function: right_function,
                argument: right_argument,
            },
        ) => {
            left_function == right_function
                && structurally_equal_expressions(dag, *left_argument, *right_argument)
        }
        (
            ExpressionNode::BinaryFunction {
                function: left_function,
                left: left_left,
                right: left_right,
            },
            ExpressionNode::BinaryFunction {
                function: right_function,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_function == right_function
                && structurally_equal_expressions(dag, *left_left, *right_left)
                && structurally_equal_expressions(dag, *left_right, *right_right)
        }
        _ => false,
    }
}

fn structurally_equal_expression_lists(
    dag: &ExactExpressionDag,
    left: ExprListId,
    right: ExprListId,
) -> bool {
    let left = dag.list(left);
    let right = dag.list(right);
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| structurally_equal_expressions(dag, *left, *right))
}

fn prove_expression_nonzero(dag: &ExactExpressionDag, id: ExprId) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_zero()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.semantic_node(id) {
        ExpressionNode::Rational(_) => Ok(false),
        ExpressionNode::Exact(value) => Ok(exact_reduction_sign(dag.exact_value(*value)).is_some()),
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Add(_) => Ok(false),
        ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !prove_expression_nonzero(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(prove_expression_nonzero(dag, *numerator)?
            && prove_expression_nonzero(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_nonzero(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { argument, base } => {
            prove_log_base_domain(dag, *argument, *base)?;
            Ok(prove_log_argument_not_one(dag, *argument)?)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_positive(dag, *argument),
            Function::Exp => expression_domain_known_well_defined(dag, *argument),
            Function::Log | Function::Ln => {
                prove_log_argument_positive(dag, *argument)?;
                prove_log_argument_not_one(dag, *argument)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(false),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn expression_domain_known_well_defined(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(_) => return Ok(true),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.semantic_node(id) {
        ExpressionNode::Rational(_) | ExpressionNode::Exact(_) | ExpressionNode::Constant(_) => {
            Ok(true)
        }
        ExpressionNode::Add(list_id) | ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !expression_domain_known_well_defined(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(expression_domain_known_well_defined(dag, *numerator)?
            && prove_expression_nonzero(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_positive(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { argument, base } => {
            match prove_log_base_domain(dag, *argument, *base) {
                Ok(()) => Ok(true),
                Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
                Err(error) => Err(error),
            }
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_nonnegative(dag, *argument),
            Function::Exp | Function::Sin | Function::Cos | Function::Atan => {
                expression_domain_known_well_defined(dag, *argument)
            }
            Function::Log | Function::Ln => match prove_log_argument_positive(dag, *argument) {
                Ok(()) => Ok(true),
                Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
                Err(error) => Err(error),
            },
            Function::Tan | Function::Asin | Function::Acos => Ok(false),
            Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => expression_domain_known_well_defined(dag, *argument),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn prove_expression_positive(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_negative() && !value.value().is_zero()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.semantic_node(id) {
        ExpressionNode::Rational(_) => Ok(false),
        ExpressionNode::Exact(value) => {
            Ok(exact_reduction_sign(dag.exact_value(*value)) == Some(Ordering::Greater))
        }
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Add(list_id) => {
            let mut saw_positive = false;
            for child in dag.list(*list_id) {
                if prove_expression_positive(dag, *child)? {
                    saw_positive = true;
                } else if !prove_expression_nonnegative(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(saw_positive)
        }
        ExpressionNode::Multiply(list_id) => {
            for child in dag.list(*list_id) {
                if !prove_expression_positive(dag, *child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => Ok(prove_expression_positive(dag, *numerator)?
            && prove_expression_positive(dag, *denominator)?),
        ExpressionNode::Power { base, exponent } => Ok(prove_expression_positive(dag, *base)?
            && expression_domain_known_well_defined(dag, *exponent)?),
        ExpressionNode::LogBase { .. } => Ok(false),
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => prove_expression_positive(dag, *argument),
            Function::Exp => expression_domain_known_well_defined(dag, *argument),
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh
            | Function::Log
            | Function::Ln => Ok(false),
        },
        ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn prove_expression_nonnegative(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(!value.value().is_negative()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    match dag.semantic_node(id) {
        ExpressionNode::Exact(value) => Ok(matches!(
            exact_reduction_sign(dag.exact_value(*value)),
            Some(Ordering::Equal | Ordering::Greater)
        )),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => prove_expression_nonnegative(dag, *argument),
        ExpressionNode::Function {
            function: Function::Exp,
            argument,
        } => expression_domain_known_well_defined(dag, *argument),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => prove_expression_positive(dag, id),
    }
}

fn prove_log_base_domain(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<(), EvaluationError> {
    prove_log_argument_positive(dag, argument)?;
    let base = match evaluate_node(dag, base) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Err(error),
        Err(error) => return Err(error),
    };
    ensure_log_base_domain(base.value())
}

fn prove_log_argument_positive(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<(), EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => {
            if value.value().is_negative() || value.value().is_zero() {
                Err(logarithm_of_non_positive_error())
            } else {
                Ok(())
            }
        }
        Err(error) if is_unsupported_exact_expression(&error) => Err(error),
        Err(error) => Err(error),
    }
}

fn prove_log_argument_not_one(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(value.value() != &Rational::one()),
        Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn evaluate_node(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => Ok(RationalEvaluation::direct(dag.rational(*id).clone())),
        ExpressionNode::Exact(_) => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::ConstantEvaluation,
            },
        )),
        ExpressionNode::Constant(_) => Err(EvaluationError::UnsupportedFeature(
            UnsupportedFeatureError {
                feature: UnsupportedFeature::ConstantEvaluation,
            },
        )),
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let child = evaluate_node(dag, *child)?;
                used_special_angle |= child.used_special_angle();
                total = total.add(child.value());
            }
            Ok(RationalEvaluation::with_origin(total, used_special_angle))
        }
        ExpressionNode::Multiply(list_id) => {
            let mut product = Rational::one();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let child = evaluate_node(dag, *child)?;
                used_special_angle |= child.used_special_angle();
                product = product.multiply(child.value());
            }
            Ok(RationalEvaluation::with_origin(product, used_special_angle))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let denominator_evaluation = match evaluate_node(dag, *denominator) {
                Ok(value) => {
                    if value.value().is_zero() {
                        return Err(domain_error(DomainErrorKind::DivisionByZero));
                    }
                    Ok(value)
                }
                Err(error) if is_unsupported_exact_expression(&error) => Err(error),
                Err(error) => return Err(error),
            };
            let numerator = match evaluate_node(dag, *numerator) {
                Ok(value) => value,
                Err(error)
                    if is_unsupported_exact_expression(&error)
                        && structurally_equal_expressions(dag, *numerator, *denominator)
                        && prove_expression_nonzero(dag, *numerator)? =>
                {
                    return Ok(RationalEvaluation::direct(Rational::one()));
                }
                Err(error) => return Err(error),
            };
            let denominator = denominator_evaluation?;
            let used_special_angle =
                numerator.used_special_angle() || denominator.used_special_angle();
            let value = numerator
                .value()
                .divide(denominator.value())
                .map_err(arithmetic_error)?;
            Ok(RationalEvaluation::with_origin(value, used_special_angle))
        }
        ExpressionNode::Power { base, exponent } => evaluate_power(dag, *base, *exponent),
        ExpressionNode::LogBase { argument, base } => {
            evaluate_log_base_function(dag, *argument, *base)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Abs => {
                let argument = evaluate_node(dag, *argument)?;
                let value = if argument.value().is_negative() {
                    argument.value().negate()
                } else {
                    argument.value().clone()
                };
                Ok(RationalEvaluation::with_origin(
                    value,
                    argument.used_special_angle(),
                ))
            }
            Function::Floor => {
                let argument = evaluate_node(dag, *argument)?;
                Ok(RationalEvaluation::with_origin(
                    rational_floor(argument.value()),
                    argument.used_special_angle(),
                ))
            }
            Function::Factorial => {
                let argument = evaluate_node(dag, *argument)?;
                evaluate_factorial(argument.value()).map(|value| {
                    RationalEvaluation::with_origin(value, argument.used_special_angle())
                })
            }
            Function::Sqrt => {
                let argument = evaluate_node(dag, *argument)?;
                if argument.value().is_negative() {
                    return Err(EvaluationError::Domain(DomainError {
                        kind: DomainErrorKind::EvenRootOfNegative,
                        span: None,
                    }));
                }
                argument
                    .value()
                    .sqrt_if_rational()
                    .map(|value| {
                        RationalEvaluation::with_origin(value, argument.used_special_angle())
                    })
                    .ok_or_else(unsupported_function_evaluation)
            }
            Function::Exp => evaluate_exp_function(dag, *argument),
            Function::Log | Function::Ln => evaluate_log_function(dag, *argument),
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin | Function::Acos | Function::Atan => {
                evaluate_inverse_trigonometric_function(dag, *function, *argument)
            }
            Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Err(unsupported_function_evaluation()),
        },
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => evaluate_binary_function(dag, *function, *left, *right),
    }
}

const MAX_EXACT_INTEGER_FUNCTION_ORDER: u32 = 4096;

fn evaluate_binary_function(
    dag: &ExactExpressionDag,
    function: Function,
    left: ExprId,
    right: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let left = evaluate_node(dag, left)?;
    let right = evaluate_node(dag, right)?;
    let used_special_angle = left.used_special_angle() || right.used_special_angle();
    let value = match function {
        Function::Gcd => evaluate_gcd(left.value(), right.value())?,
        Function::Lcm => evaluate_lcm(left.value(), right.value())?,
        Function::Modulo => evaluate_modulo(left.value(), right.value())?,
        Function::Permutation => evaluate_permutation(left.value(), right.value())?,
        Function::Combination => evaluate_combination(left.value(), right.value())?,
        Function::Root
        | Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => return Err(unsupported_function_evaluation()),
    };
    Ok(RationalEvaluation::with_origin(value, used_special_angle))
}

fn rational_floor(value: &Rational) -> Rational {
    Rational::from_integer(Integer::from_bigint(
        value
            .numerator
            .inner
            .div_floor(&value.denominator.inner.inner),
    ))
}

fn evaluate_factorial(value: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(value)?;
    ensure_integer_function_order(n)?;
    Ok(Rational::from_integer(Integer::from_bigint(
        factorial_bigint(n),
    )))
}

fn evaluate_gcd(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?.abs();
    let right = integer_bigint(right)?.abs();
    Ok(Rational::from_integer(Integer::from_bigint(
        left.gcd(&right),
    )))
}

fn evaluate_lcm(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?;
    let right = integer_bigint(right)?;
    if left.is_zero() || right.is_zero() {
        return Ok(Rational::zero());
    }
    let gcd = left.gcd(&right);
    Ok(Rational::from_integer(Integer::from_bigint(
        (left / gcd * right).abs(),
    )))
}

fn evaluate_modulo(left: &Rational, right: &Rational) -> Result<Rational, EvaluationError> {
    let left = integer_bigint(left)?;
    let right = integer_bigint(right)?;
    if right.is_zero() {
        return Err(domain_error(DomainErrorKind::DivisionByZero));
    }
    Ok(Rational::from_integer(Integer::from_bigint(
        left.mod_floor(&right),
    )))
}

fn evaluate_permutation(n: &Rational, r: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(n)?;
    let r = nonnegative_integer_u32(r)?;
    ensure_integer_function_order(n)?;
    if r > n {
        return Ok(Rational::zero());
    }
    let mut value = BigInt::one();
    for factor in (n - r + 1)..=n {
        value *= BigInt::from(factor);
    }
    Ok(Rational::from_integer(Integer::from_bigint(value)))
}

fn evaluate_combination(n: &Rational, r: &Rational) -> Result<Rational, EvaluationError> {
    let n = nonnegative_integer_u32(n)?;
    let mut r = nonnegative_integer_u32(r)?;
    ensure_integer_function_order(n)?;
    if r > n {
        return Ok(Rational::zero());
    }
    r = r.min(n - r);
    let mut value = BigInt::one();
    for i in 1..=r {
        value *= BigInt::from(n - r + i);
        value /= BigInt::from(i);
    }
    Ok(Rational::from_integer(Integer::from_bigint(value)))
}

fn integer_bigint(value: &Rational) -> Result<BigInt, EvaluationError> {
    if value.is_integer() {
        Ok(value.numerator.inner.clone())
    } else {
        Err(domain_error(
            DomainErrorKind::IntegerFunctionRequiresInteger,
        ))
    }
}

fn nonnegative_integer_u32(value: &Rational) -> Result<u32, EvaluationError> {
    let integer = integer_bigint(value)?;
    if integer.is_negative() {
        return Err(domain_error(
            DomainErrorKind::IntegerFunctionRequiresNonNegative,
        ));
    }
    integer.to_u32().ok_or_else(logical_work_limit_error)
}

fn ensure_integer_function_order(value: u32) -> Result<(), EvaluationError> {
    if value > MAX_EXACT_INTEGER_FUNCTION_ORDER {
        Err(logical_work_limit_error())
    } else {
        Ok(())
    }
}

fn factorial_bigint(value: u32) -> BigInt {
    let mut product = BigInt::one();
    for factor in 2..=value {
        product *= BigInt::from(factor);
    }
    product
}

fn evaluate_pi_coefficient(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Rational(id) => {
            let value = dag.rational(*id);
            Ok(value
                .is_zero()
                .then(|| PiCoefficientEvaluation::direct(Rational::zero())))
        }
        ExpressionNode::Constant(Constant::Pi) => {
            Ok(Some(PiCoefficientEvaluation::direct(Rational::one())))
        }
        ExpressionNode::Exact(value) => match dag.exact_value(*value) {
            ExactReduction::PiMultiple(value) => Ok(Some(value.clone())),
            ExactReduction::Radical(_)
            | ExactReduction::RealAlgebraic(_)
            | ExactReduction::Symbolic => Ok(None),
        },
        ExpressionNode::Constant(Constant::Euler) => Ok(None),
        ExpressionNode::Add(list_id) => {
            let mut total = Rational::zero();
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
                    return Ok(None);
                };
                used_special_angle |= coefficient.used_special_angle();
                total = total.add(coefficient.coefficient());
            }
            Ok(Some(PiCoefficientEvaluation::with_origin(
                total,
                used_special_angle,
            )))
        }
        ExpressionNode::Multiply(list_id) => {
            let mut scalar = Rational::one();
            let mut pi_coefficient = None;
            let mut used_special_angle = false;
            for child in dag.list(*list_id) {
                match evaluate_node(dag, *child) {
                    Ok(value) => {
                        if value.value().is_zero() {
                            return Ok(Some(PiCoefficientEvaluation::with_origin(
                                Rational::zero(),
                                value.used_special_angle(),
                            )));
                        }
                        used_special_angle |= value.used_special_angle();
                        scalar = scalar.multiply(value.value());
                    }
                    Err(error) if is_unsupported_exact_expression(&error) => {
                        let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
                            return Ok(None);
                        };
                        if pi_coefficient.is_some() {
                            return Ok(None);
                        }
                        used_special_angle |= coefficient.used_special_angle();
                        pi_coefficient = Some(coefficient);
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(pi_coefficient.map(|coefficient| {
                PiCoefficientEvaluation::with_origin(
                    scalar.multiply(coefficient.coefficient()),
                    used_special_angle,
                )
            }))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let Some(numerator) = evaluate_pi_coefficient(dag, *numerator)? else {
                return Ok(None);
            };
            let denominator = match evaluate_node(dag, *denominator) {
                Ok(value) => value.into_value(),
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            let used_special_angle = numerator.used_special_angle();
            numerator
                .coefficient()
                .divide(&denominator)
                .map(|coefficient| {
                    Some(PiCoefficientEvaluation::with_origin(
                        coefficient,
                        used_special_angle,
                    ))
                })
                .map_err(arithmetic_error)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Asin | Function::Acos | Function::Atan
                if dag.semantics().angle_unit == AngleUnit::Radian =>
            {
                evaluate_inverse_trig_pi_coefficient(dag, *function, *argument)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Power { .. } => Ok(None),
        ExpressionNode::LogBase { .. } => Ok(None),
        ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_radical_node(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Exact(value) => match dag.exact_value(*value) {
            ExactReduction::Radical(value) => Ok(Some(value.clone())),
            ExactReduction::PiMultiple(_)
            | ExactReduction::RealAlgebraic(_)
            | ExactReduction::Symbolic => Ok(None),
        },
        ExpressionNode::Add(list_id) => evaluate_radical_sum(dag, *list_id),
        ExpressionNode::Multiply(list_id) => evaluate_radical_product(dag, *list_id),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let Some(numerator) = evaluate_radical_reduction(dag, *numerator)? else {
                return Ok(None);
            };
            let Some(denominator) = evaluate_radical_reduction(dag, *denominator)? else {
                return Ok(None);
            };
            reduce_radical_quotient(
                &numerator,
                &denominator,
                numerator.used_special_angle() || denominator.used_special_angle(),
            )
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => {
                let Some(argument) = evaluate_radical_reduction(dag, *argument)? else {
                    return Ok(None);
                };
                reduce_square_root(&argument, DomainErrorKind::EvenRootOfNegative)
            }
            Function::Exp => evaluate_radical_exp_function(dag, *argument),
            Function::Log | Function::Ln => evaluate_radical_log_function(dag, *argument),
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_radical_trigonometric_function(dag, *function, *argument)
            }
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Power { base, exponent } => {
            let exponent = match evaluate_node(dag, *exponent) {
                Ok(value) => value,
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            if exponent.value() != &rational(1, 2) {
                if !exponent.value().is_integer() {
                    return Ok(None);
                }
                let Some(exponent_value) = exponent.value().as_i64_if_integer() else {
                    return Err(exponent_too_large_error());
                };
                let Some(base) = evaluate_radical_reduction(dag, *base)? else {
                    return Ok(None);
                };
                return reduce_radical_integer_power(
                    &base,
                    exponent_value,
                    base.used_special_angle() || exponent.used_special_angle(),
                );
            }
            let Some(base) = evaluate_radical_reduction(dag, *base)? else {
                return Ok(None);
            };
            let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
            reduce_square_root(&base, DomainErrorKind::NonRealPower).map(|value| {
                value.map(|value| radical_reduction_with_origin(value, used_special_angle))
            })
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_radical_reduction(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    Ok(evaluate_radical_reduction_with_origin(dag, id)?.map(|(value, _)| value))
}

fn evaluate_radical_reduction_with_origin(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<(RadicalReduction, bool)>, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(Some((RadicalReduction::Rational(value), true))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            Ok(evaluate_radical_node(dag, id)?.map(|value| (value, false)))
        }
        Err(error) => Err(error),
    }
}

fn evaluate_real_algebraic_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Exact(value) => match dag.exact_value(*value) {
            ExactReduction::RealAlgebraic(value) => Ok(Some(value.clone())),
            ExactReduction::Radical(_) => {
                evaluate_real_algebraic_expression_node(dag, dag.exact_presentation(*value), limits)
            }
            ExactReduction::PiMultiple(_) => Ok(None),
            ExactReduction::Symbolic => Ok(None),
        },
        node @ (ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. }) => {
            evaluate_real_algebraic_expression_node(dag, node, limits)
        }
    }
}

fn evaluate_real_algebraic_expression_node(
    dag: &ExactExpressionDag,
    node: &ExpressionNode,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match node {
        ExpressionNode::Exact(_) => Ok(None),
        ExpressionNode::Power { base, exponent } => {
            evaluate_real_algebraic_power(dag, *base, *exponent, limits)
        }
        ExpressionNode::Add(list_id) => evaluate_real_algebraic_sum(dag, *list_id, limits),
        ExpressionNode::Multiply(list_id) => evaluate_real_algebraic_product(dag, *list_id, limits),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => evaluate_real_algebraic_quotient(dag, *numerator, *denominator, limits),
        ExpressionNode::Function { function, argument } => match function {
            Function::Sin | Function::Cos | Function::Tan => {
                evaluate_real_algebraic_trigonometric_function(dag, *function, *argument, limits)
            }
            Function::Exp => evaluate_real_algebraic_exp_function(dag, *argument, limits),
            Function::Log | Function::Ln => {
                evaluate_real_algebraic_log_function(dag, *argument, limits)
            }
            Function::Sqrt => evaluate_real_algebraic_sqrt_function(dag, *argument, limits),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Root
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => Ok(None),
        },
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn evaluate_real_algebraic_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let exponent = match evaluate_node(dag, exponent) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match evaluate_node(dag, base) {
        Ok(base) => rational_power_algebraic(base.value(), exponent.value(), limits),
        Err(error) if is_unsupported_exact_expression(&error) => {
            let Some(base) = evaluate_real_algebraic_node(dag, base, limits)? else {
                return Ok(None);
            };
            match base {
                RealAlgebraicEvaluation::Rational(base) => {
                    evaluate_collapsed_rational_power(base, exponent, limits)
                }
                RealAlgebraicEvaluation::Algebraic(base) => {
                    if !exponent.value().is_integer() {
                        return real_algebraic_rational_power(
                            base,
                            exponent.value(),
                            DomainErrorKind::NonRealPower,
                            limits,
                        );
                    }
                    let Some(exponent) = exponent.value().as_i64_if_integer() else {
                        return Err(exponent_too_large_error());
                    };
                    real_algebraic_integer_power(base, exponent, limits)
                }
            }
        }
        Err(error) => Err(error),
    }
}

fn evaluate_collapsed_rational_power(
    base: RationalEvaluation,
    exponent: RationalEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
    match evaluate_rational_power(base.value(), exponent.value()) {
        Ok(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(value, used_special_angle),
        ))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            rational_power_algebraic(base.value(), exponent.value(), limits)
        }
        Err(error) => Err(error),
    }
}

fn evaluate_real_algebraic_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let mut rational = Rational::zero();
    let mut algebraic = None;
    let mut used_algebraic_path = false;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.add(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                used_algebraic_path = true;
                match value {
                    RealAlgebraicEvaluation::Rational(value) => {
                        rational = rational.add(value.value());
                    }
                    RealAlgebraicEvaluation::Algebraic(value) => {
                        algebraic = match algebraic {
                            Some(current) => {
                                let Some(sum) = add_real_algebraics(current, value, limits)? else {
                                    return Ok(None);
                                };
                                match sum {
                                    RealAlgebraicEvaluation::Rational(value) => {
                                        rational = rational.add(value.value());
                                        None
                                    }
                                    RealAlgebraicEvaluation::Algebraic(value) => Some(value),
                                }
                            }
                            None => Some(value),
                        };
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(used_algebraic_path.then(|| RealAlgebraicEvaluation::rational(rational)));
    };
    if rational.is_zero() {
        return Ok(Some(RealAlgebraicEvaluation::from_algebraic(algebraic)));
    }
    match algebraic.add_rational_bounded(
        &rational,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn add_real_algebraics(
    lhs: RealAlgebraic,
    rhs: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match lhs.add_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn evaluate_real_algebraic_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let mut rational = Rational::one();
    let mut algebraic = None;
    let mut used_algebraic_path = false;
    for child in dag.list(list_id) {
        match evaluate_node(dag, *child) {
            Ok(value) => {
                rational = rational.multiply(value.value());
            }
            Err(error) if is_unsupported_exact_expression(&error) => {
                let Some(value) = evaluate_real_algebraic_node(dag, *child, limits)? else {
                    return Ok(None);
                };
                used_algebraic_path = true;
                match value {
                    RealAlgebraicEvaluation::Rational(value) => {
                        rational = rational.multiply(value.value());
                    }
                    RealAlgebraicEvaluation::Algebraic(value) => {
                        algebraic = match algebraic {
                            Some(current) => {
                                let Some(product) =
                                    multiply_real_algebraics(current, value, limits)?
                                else {
                                    return Ok(None);
                                };
                                match product {
                                    RealAlgebraicEvaluation::Rational(value) => {
                                        rational = rational.multiply(value.value());
                                        None
                                    }
                                    RealAlgebraicEvaluation::Algebraic(value) => Some(value),
                                }
                            }
                            None => Some(value),
                        };
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }

    let Some(algebraic) = algebraic else {
        return Ok(used_algebraic_path.then(|| RealAlgebraicEvaluation::rational(rational)));
    };
    scale_real_algebraic_by_rational(algebraic, &rational, limits)
}

fn multiply_real_algebraics(
    lhs: RealAlgebraic,
    rhs: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match lhs.multiply_bounded(
        &rhs,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn evaluate_real_algebraic_quotient(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(denominator) = evaluate_rational_or_real_algebraic_node(dag, denominator, limits)?
    else {
        return Ok(None);
    };
    let Some(numerator) = evaluate_rational_or_real_algebraic_node(dag, numerator, limits)? else {
        return Ok(None);
    };
    divide_real_algebraic_evaluations(numerator, denominator, limits)
}

fn evaluate_rational_or_real_algebraic_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => Ok(Some(RealAlgebraicEvaluation::Rational(value))),
        Err(error) if is_unsupported_exact_expression(&error) => {
            evaluate_real_algebraic_node(dag, id, limits)
        }
        Err(error) => Err(error),
    }
}

fn divide_real_algebraic_evaluations(
    numerator: RealAlgebraicEvaluation,
    denominator: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match denominator {
        RealAlgebraicEvaluation::Rational(denominator) => {
            let reciprocal = Rational::one()
                .divide(denominator.value())
                .map_err(arithmetic_error)?;
            multiply_real_algebraic_evaluation_by_rational(numerator, &reciprocal, limits)
        }
        RealAlgebraicEvaluation::Algebraic(denominator) => {
            let Some(reciprocal) = reciprocal_real_algebraic(denominator, limits)? else {
                return Ok(None);
            };
            multiply_real_algebraic_evaluations(numerator, reciprocal, limits)
        }
    }
}

fn multiply_real_algebraic_evaluations(
    lhs: RealAlgebraicEvaluation,
    rhs: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match (lhs, rhs) {
        (RealAlgebraicEvaluation::Rational(lhs), RealAlgebraicEvaluation::Rational(rhs)) => {
            let used_special_angle = lhs.used_special_angle() || rhs.used_special_angle();
            Ok(Some(RealAlgebraicEvaluation::Rational(
                RationalEvaluation::with_origin(
                    lhs.value().multiply(rhs.value()),
                    used_special_angle,
                ),
            )))
        }
        (RealAlgebraicEvaluation::Algebraic(lhs), RealAlgebraicEvaluation::Rational(rhs))
        | (RealAlgebraicEvaluation::Rational(rhs), RealAlgebraicEvaluation::Algebraic(lhs)) => {
            multiply_real_algebraic_evaluation_by_rational(
                RealAlgebraicEvaluation::Algebraic(lhs),
                rhs.value(),
                limits,
            )
        }
        (RealAlgebraicEvaluation::Algebraic(lhs), RealAlgebraicEvaluation::Algebraic(rhs)) => {
            multiply_real_algebraics(lhs, rhs, limits)
        }
    }
}

fn multiply_real_algebraic_evaluation_by_rational(
    value: RealAlgebraicEvaluation,
    scalar: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(
                value.value().multiply(scalar),
                value.used_special_angle(),
            ),
        ))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            scale_real_algebraic_by_rational(value, scalar, limits)
        }
    }
}

fn scale_real_algebraic_by_rational(
    algebraic: RealAlgebraic,
    scalar: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if scalar.is_zero() {
        return Ok(Some(RealAlgebraicEvaluation::rational(Rational::zero())));
    }
    if scalar == &Rational::one() {
        return Ok(Some(RealAlgebraicEvaluation::from_algebraic(algebraic)));
    }
    match algebraic.scale_rational_bounded(
        scalar,
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn reciprocal_real_algebraic(
    algebraic: RealAlgebraic,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match algebraic.reciprocal_bounded(
        limits.max_polynomial_coefficient_bits,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn real_algebraic_integer_power(
    base: RealAlgebraic,
    exponent: i64,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if exponent == 0 {
        return Ok(Some(RealAlgebraicEvaluation::rational(Rational::one())));
    }
    let magnitude = exponent
        .checked_abs()
        .ok_or_else(exponent_too_large_error)?;
    let magnitude = u32::try_from(magnitude).map_err(|_| exponent_too_large_error())?;
    let base = if exponent < 0 {
        let Some(reciprocal) = reciprocal_real_algebraic(base, limits)? else {
            return Ok(None);
        };
        reciprocal
    } else {
        RealAlgebraicEvaluation::Algebraic(base)
    };
    real_algebraic_positive_integer_power(base, magnitude, limits)
}

fn real_algebraic_positive_integer_power(
    base: RealAlgebraicEvaluation,
    mut exponent: u32,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    debug_assert!(exponent > 0);
    let mut result = None;
    let mut factor = base;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(current) => {
                    let Some(product) =
                        multiply_real_algebraic_evaluations(current, factor.clone(), limits)?
                    else {
                        return Ok(None);
                    };
                    product
                }
                None => factor.clone(),
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            let Some(product) =
                multiply_real_algebraic_evaluations(factor.clone(), factor, limits)?
            else {
                return Ok(None);
            };
            factor = product;
        }
    }
    Ok(result)
}

fn evaluate_real_algebraic_sqrt_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(value) = evaluate_rational_or_real_algebraic_node(dag, argument, limits)? else {
        return Ok(None);
    };
    match value {
        RealAlgebraicEvaluation::Rational(value) => evaluate_collapsed_rational_power(
            value,
            RationalEvaluation::direct(rational(1, 2)),
            limits,
        ),
        RealAlgebraicEvaluation::Algebraic(value) => {
            real_algebraic_nth_root(value, 2, DomainErrorKind::EvenRootOfNegative, limits)
        }
    }
}

fn real_algebraic_rational_power(
    value: RealAlgebraic,
    exponent: &Rational,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if exponent.is_integer() {
        let Some(exponent) = exponent.as_i64_if_integer() else {
            return Err(exponent_too_large_error());
        };
        return real_algebraic_integer_power(value, exponent, limits);
    }

    let Some(root_index) = exponent.denominator.inner.inner.to_u32() else {
        return Ok(None);
    };
    if root_index <= 1 || root_index > limits.max_resultant_degree {
        return Ok(None);
    }
    let Some(exponent_numerator) = exponent.numerator.inner.to_i64() else {
        return Ok(None);
    };

    match value.sign_bounded(limits.max_root_isolation_steps) {
        Ok(Some(Ordering::Equal)) => {
            return evaluate_zero_power(exponent)
                .map(RealAlgebraicEvaluation::rational)
                .map(Some);
        }
        Ok(Some(Ordering::Less)) if root_index.is_multiple_of(2) => {
            return Err(domain_error(negative_even_root_error));
        }
        Ok(Some(Ordering::Greater | Ordering::Less)) => {}
        Ok(None)
        | Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    }

    let Some(powered) = real_algebraic_integer_power(value, exponent_numerator, limits)? else {
        return Ok(None);
    };
    nth_root_real_algebraic_evaluation(powered, root_index, negative_even_root_error, limits)
}

fn real_algebraic_nth_root(
    value: RealAlgebraic,
    root_index: u32,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if root_index <= 1 {
        return Ok(Some(RealAlgebraicEvaluation::Algebraic(value)));
    }
    match value.sign_bounded(limits.max_root_isolation_steps) {
        Ok(Some(Ordering::Greater)) => {}
        Ok(Some(Ordering::Equal)) => {
            return Ok(Some(RealAlgebraicEvaluation::rational(Rational::zero())));
        }
        Ok(Some(Ordering::Less)) if root_index.is_multiple_of(2) => {
            return Err(domain_error(negative_even_root_error));
        }
        Ok(Some(Ordering::Less)) => {}
        Ok(None)
        | Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    }

    match value.principal_nth_root_bounded(
        root_index,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn nth_root_real_algebraic_evaluation(
    value: RealAlgebraicEvaluation,
    root_index: u32,
    negative_even_root_error: DomainErrorKind,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => evaluate_collapsed_rational_power(
            value,
            RationalEvaluation::direct(rational(1, i64::from(root_index))),
            limits,
        ),
        RealAlgebraicEvaluation::Algebraic(value) => {
            real_algebraic_nth_root(value, root_index, negative_even_root_error, limits)
        }
    }
}

fn evaluate_real_algebraic_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Log,
        argument,
    } = dag.semantic_node(argument)
    else {
        return Ok(None);
    };
    let Some(value) = evaluate_rational_or_real_algebraic_node(dag, *argument, limits)? else {
        return Ok(None);
    };
    match real_algebraic_log_domain_proof(&value, limits)? {
        LogDomainProof::Positive => Ok(Some(value)),
        LogDomainProof::NonPositive => Err(logarithm_of_non_positive_error()),
        LogDomainProof::Unknown => Ok(None),
    }
}

fn evaluate_real_algebraic_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.semantic_node(argument)
    else {
        return Ok(None);
    };
    evaluate_rational_or_real_algebraic_node(dag, *argument, limits)
}

fn evaluate_real_algebraic_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if let Some(value) =
        evaluate_real_algebraic_trigonometric_special_angle(dag, function, argument, limits)?
    {
        return Ok(Some(value));
    }
    evaluate_real_algebraic_inverse_trig_composition(dag, function, argument, limits)
}

fn evaluate_real_algebraic_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(coefficient) = evaluate_pi_coefficient(dag, argument)? else {
        return Ok(None);
    };
    match function {
        Function::Sin => Ok(
            cyclotomic_sine_algebraic(coefficient.coefficient(), limits)?
                .map(RealAlgebraicEvaluation::from_algebraic),
        ),
        Function::Cos => Ok(
            cyclotomic_cosine_algebraic(coefficient.coefficient(), limits)?
                .map(RealAlgebraicEvaluation::from_algebraic),
        ),
        Function::Tan => {
            let phase = coefficient.coefficient().modulo_integer(1);
            if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
                return Err(domain_error(DomainErrorKind::TangentPole));
            }
            let Some(sine) = cyclotomic_sine_algebraic(coefficient.coefficient(), limits)? else {
                return Ok(None);
            };
            let Some(cosine) = cyclotomic_cosine_algebraic(coefficient.coefficient(), limits)?
            else {
                return Ok(None);
            };
            let Some(reciprocal_cosine) = reciprocal_real_algebraic(cosine, limits)? else {
                return Ok(None);
            };
            multiply_real_algebraic_evaluations(
                RealAlgebraicEvaluation::from_algebraic(sine),
                reciprocal_cosine,
                limits,
            )
        }
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => Ok(None),
    }
}

fn cyclotomic_sine_algebraic(
    coefficient: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let cosine_coefficient = rational(1, 2).subtract(coefficient);
    cyclotomic_cosine_algebraic(&cosine_coefficient, limits)
}

fn cyclotomic_cosine_algebraic(
    coefficient: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraic>, EvaluationError> {
    let phase = coefficient.modulo_integer(2);
    let Some((numerator, denominator)) = cyclotomic_phase_parts(&phase) else {
        return Ok(None);
    };
    if denominator > limits.max_cyclotomic_order {
        return Ok(None);
    }

    let candidate_polynomial = chebyshev_cosine_candidate_polynomial(numerator, denominator)?;
    let Some(root_index) = cosine_candidate_root_index(numerator, denominator) else {
        return Ok(None);
    };
    let intervals = match candidate_polynomial.isolate_real_roots(limits.max_root_isolation_steps) {
        Ok(intervals) => intervals,
        Err(PrimitivePolynomialRootIsolationError::StepLimitExceeded) => return Ok(None),
        Err(error) => {
            return Err(real_algebraic_construction_error(
                RealAlgebraicConstructionError::RootIsolation(error),
            ));
        }
    };
    let Some(isolating_interval) = intervals.get(root_index).cloned() else {
        return Err(invalid_algebraic_isolation_error());
    };

    match RealAlgebraic::from_candidate_polynomial_bounded(
        candidate_polynomial,
        isolating_interval,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(value) => Ok(value),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn cyclotomic_phase_parts(phase: &Rational) -> Option<(u32, u32)> {
    let numerator = phase.numerator.inner.to_u32()?;
    let denominator = phase.denominator.inner.inner.to_u32()?;
    Some((numerator, denominator))
}

fn cosine_candidate_root_index(numerator: u32, denominator: u32) -> Option<usize> {
    let denominator = u64::from(denominator);
    let period = denominator.checked_mul(2)?;
    let phase_numerator = u64::from(numerator) % period;
    let target_angle_numerator = if phase_numerator > denominator {
        period - phase_numerator
    } else {
        phase_numerator
    };
    let parity = u64::from(numerator % 2);
    let index = ((target_angle_numerator + 1)..=denominator)
        .filter(|candidate| candidate % 2 == parity)
        .count();
    Some(index)
}

fn chebyshev_cosine_candidate_polynomial(
    numerator: u32,
    denominator: u32,
) -> Result<PrimitivePolynomial, EvaluationError> {
    let mut polynomial = chebyshev_t_polynomial(denominator)?;
    let target = if numerator.is_multiple_of(2) { 1 } else { -1 };
    polynomial.coefficients_low_to_high[0].inner -= target;
    PrimitivePolynomial::new(polynomial.coefficients_low_to_high)
        .map_err(|_| invalid_algebraic_isolation_error())
}

fn chebyshev_t_polynomial(order: u32) -> Result<PrimitivePolynomial, EvaluationError> {
    let mut previous = vec![Integer::one()];
    if order == 0 {
        return PrimitivePolynomial::new(previous).map_err(|_| invalid_algebraic_isolation_error());
    }

    let mut current = vec![Integer::zero(), Integer::one()];
    for _ in 1..order {
        let mut next = vec![Integer::zero(); current.len() + 1];
        for (index, coefficient) in current.iter().enumerate() {
            next[index + 1].inner += &coefficient.inner * 2;
        }
        for (index, coefficient) in previous.iter().enumerate() {
            next[index].inner -= &coefficient.inner;
        }
        previous = current;
        current = next;
    }
    PrimitivePolynomial::new(current).map_err(|_| invalid_algebraic_isolation_error())
}

fn rational_power_algebraic(
    base: &Rational,
    exponent: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    if base.is_zero() || exponent.is_integer() {
        return Ok(None);
    }
    let Some(root_index) = exponent.denominator.inner.inner.to_u32() else {
        return Ok(None);
    };
    if root_index <= 1 || root_index > limits.max_resultant_degree {
        return Ok(None);
    }
    if base.is_negative() && root_index.is_multiple_of(2) {
        return Err(domain_error(DomainErrorKind::NonRealPower));
    }
    let Some(exponent_numerator) = exponent.numerator.inner.to_i64() else {
        return Ok(None);
    };
    let powered_base = match base.pow_i64(exponent_numerator) {
        Ok(value) => value,
        Err(RationalArithmeticError::ExponentTooLarge) => return Ok(None),
        Err(error) => return Err(arithmetic_error(error)),
    };

    let root = match RealAlgebraic::nth_root_of_rational_bounded(
        &powered_base,
        root_index,
        limits.max_algebraic_degree,
        limits.max_polynomial_coefficient_bits,
        limits.max_resultant_degree,
        limits.max_factorization_work,
        limits.max_root_isolation_steps,
    ) {
        Ok(Some(root)) => RealAlgebraicEvaluation::from_algebraic(root),
        Ok(None) => return Ok(None),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => return Ok(None),
        Err(error) => return Err(real_algebraic_construction_error(error)),
    };
    Ok(Some(root))
}

fn evaluate_radical_sum(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let mut rational = Rational::zero();
    let mut radicals = Vec::new();
    let mut used_special_angle = false;
    let mut saw_radical = false;

    for child in dag.list(list_id) {
        let Some((value, direct_rational)) = evaluate_radical_reduction_with_origin(dag, *child)?
        else {
            return Ok(None);
        };
        saw_radical |= !direct_rational;
        match value {
            RadicalReduction::Rational(value) => {
                used_special_angle |= value.used_special_angle();
                rational = rational.add(value.value());
            }
            RadicalReduction::Radical(radical) => {
                used_special_angle |= radical.used_special_angle();
                add_radical_term(&mut radicals, radical.value());
            }
            RadicalReduction::LinearCombination(value) => {
                used_special_angle |= value.used_special_angle();
                rational = rational.add(&value.value().rational);
                for radical in &value.value().radicals {
                    add_radical_term(&mut radicals, radical);
                }
            }
        }
    }

    if !saw_radical {
        return Ok(None);
    }
    Ok(Some(radical_linear_combination_reduction(
        rational,
        radicals,
        used_special_angle,
    )))
}

fn evaluate_radical_product(
    dag: &ExactExpressionDag,
    list_id: ExprListId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let mut product = RadicalReduction::rational(Rational::one(), false);
    let mut saw_non_rational = false;

    for child in dag.list(list_id) {
        let Some((value, direct_rational)) = evaluate_radical_reduction_with_origin(dag, *child)?
        else {
            return Ok(None);
        };
        saw_non_rational |= !direct_rational;
        let used_special_angle = product.used_special_angle() || value.used_special_angle();
        let Some(next_product) = multiply_radical_reductions(&product, &value, used_special_angle)?
        else {
            return Ok(None);
        };
        product = next_product;
    }

    if !saw_non_rational {
        return Ok(None);
    }
    Ok(Some(product))
}

fn evaluate_radical_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if let Some(value) = evaluate_radical_trigonometric_special_angle(dag, function, argument)? {
        return Ok(Some(value));
    }
    evaluate_radical_inverse_trig_composition(dag, function, argument)
}

fn evaluate_radical_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(coefficient) = evaluate_pi_coefficient(dag, argument)? else {
        return Ok(None);
    };
    let value = match function {
        Function::Sin => sine_radical_special_angle(coefficient.coefficient()),
        Function::Cos => cosine_radical_special_angle(coefficient.coefficient()),
        Function::Tan => tangent_radical_special_angle(coefficient.coefficient())?,
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    };
    Ok(value)
}

fn evaluate_radical_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Log,
        argument,
    } = dag.semantic_node(argument)
    else {
        return Ok(None);
    };
    let Some(value) = evaluate_radical_reduction(dag, *argument)? else {
        return Ok(None);
    };
    match radical_log_domain_proof(&value) {
        LogDomainProof::Positive => Ok(Some(value)),
        LogDomainProof::NonPositive => Err(logarithm_of_non_positive_error()),
        LogDomainProof::Unknown => Ok(None),
    }
}

fn evaluate_radical_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.semantic_node(argument)
    else {
        return Ok(None);
    };
    evaluate_radical_reduction(dag, *argument)
}

fn reduce_square_root(
    value: &RadicalReduction,
    negative_kind: DomainErrorKind,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    match value {
        RadicalReduction::Rational(value) => {
            if value.value().is_negative() {
                return Err(domain_error(negative_kind));
            }
            Ok(reduce_rational_square_root(
                value.value(),
                value.used_special_angle(),
            ))
        }
        RadicalReduction::Radical(value) => {
            if value.value().coefficient.is_negative() {
                return Err(domain_error(negative_kind));
            }
            Ok(None)
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn reduce_radical_integer_power(
    base: &RadicalReduction,
    exponent: i64,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if exponent == 0 {
        return Ok(Some(RadicalReduction::rational(
            Rational::one(),
            used_special_angle,
        )));
    }
    let magnitude = exponent
        .checked_abs()
        .ok_or_else(exponent_too_large_error)?;
    let magnitude = u32::try_from(magnitude).map_err(|_| exponent_too_large_error())?;
    if exponent < 0 {
        let Some(denominator) =
            reduce_radical_positive_integer_power(base, magnitude, used_special_angle)?
        else {
            return Ok(None);
        };
        let numerator = RadicalReduction::rational(Rational::one(), used_special_angle);
        return reduce_radical_quotient(
            &numerator,
            &denominator,
            used_special_angle || denominator.used_special_angle(),
        );
    }
    reduce_radical_positive_integer_power(base, magnitude, used_special_angle)
}

fn reduce_radical_positive_integer_power(
    base: &RadicalReduction,
    mut exponent: u32,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    debug_assert!(exponent > 0);
    let mut result = None;
    let mut factor = radical_reduction_with_origin(base.clone(), used_special_angle);
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(current) => {
                    let Some(product) =
                        multiply_radical_reductions(&current, &factor, used_special_angle)?
                    else {
                        return Ok(None);
                    };
                    product
                }
                None => factor.clone(),
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            let Some(product) = multiply_radical_reductions(&factor, &factor, used_special_angle)?
            else {
                return Ok(None);
            };
            factor = product;
        }
    }
    Ok(result)
}

fn multiply_radical_reductions(
    lhs: &RadicalReduction,
    rhs: &RadicalReduction,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let used_special_angle =
        used_special_angle || lhs.used_special_angle() || rhs.used_special_angle();
    let lhs = radical_reduction_as_linear_combination(lhs);
    let rhs = radical_reduction_as_linear_combination(rhs);
    let mut rational = lhs.rational.multiply(&rhs.rational);
    let mut radicals = Vec::new();

    for radical in &lhs.radicals {
        add_scaled_radical_term(&mut radicals, radical, &rhs.rational);
    }
    for radical in &rhs.radicals {
        add_scaled_radical_term(&mut radicals, radical, &lhs.rational);
    }
    for lhs_radical in &lhs.radicals {
        for rhs_radical in &rhs.radicals {
            let coefficient = lhs_radical.coefficient.multiply(&rhs_radical.coefficient);
            let radicand = rational_from_positive_integer(&lhs_radical.radicand)
                .multiply(&rational_from_positive_integer(&rhs_radical.radicand));
            let Some(product) = reduce_radical_product(coefficient, radicand, used_special_angle)?
            else {
                return Ok(None);
            };
            add_radical_reduction_terms(&mut rational, &mut radicals, &product);
        }
    }

    Ok(Some(radical_linear_combination_reduction(
        rational,
        radicals,
        used_special_angle,
    )))
}

fn radical_reduction_as_linear_combination(value: &RadicalReduction) -> RadicalLinearCombination {
    match value {
        RadicalReduction::Rational(value) => RadicalLinearCombination {
            rational: value.value().clone(),
            radicals: Vec::new(),
        },
        RadicalReduction::Radical(value) => RadicalLinearCombination {
            rational: Rational::zero(),
            radicals: vec![value.value().clone()],
        },
        RadicalReduction::LinearCombination(value) => value.value().clone(),
    }
}

fn radical_reduction_with_origin(
    value: RadicalReduction,
    used_special_angle: bool,
) -> RadicalReduction {
    let used_special_angle = used_special_angle || value.used_special_angle();
    match value {
        RadicalReduction::Rational(value) => {
            RadicalReduction::rational(value.into_value(), used_special_angle)
        }
        RadicalReduction::Radical(value) => {
            RadicalReduction::radical(value.into_value(), used_special_angle)
        }
        RadicalReduction::LinearCombination(value) => {
            RadicalReduction::linear_combination(value.into_value(), used_special_angle)
        }
    }
}

fn add_scaled_radical_term(
    radicals: &mut Vec<SimpleRadical>,
    radical: &SimpleRadical,
    scalar: &Rational,
) {
    if scalar.is_zero() {
        return;
    }
    add_radical_term(
        radicals,
        &SimpleRadical {
            coefficient: radical.coefficient.multiply(scalar),
            radicand: radical.radicand.clone(),
        },
    );
}

fn add_radical_reduction_terms(
    rational: &mut Rational,
    radicals: &mut Vec<SimpleRadical>,
    value: &RadicalReduction,
) {
    match value {
        RadicalReduction::Rational(value) => {
            *rational = rational.add(value.value());
        }
        RadicalReduction::Radical(value) => add_radical_term(radicals, value.value()),
        RadicalReduction::LinearCombination(value) => {
            *rational = rational.add(&value.value().rational);
            for radical in &value.value().radicals {
                add_radical_term(radicals, radical);
            }
        }
    }
}

fn reduce_rational_square_root(
    value: &Rational,
    used_special_angle: bool,
) -> Option<RadicalReduction> {
    if let Some(root) = value.sqrt_if_rational() {
        return Some(RadicalReduction::rational(root, used_special_angle));
    }
    value
        .sqrt_as_simple_radical()
        .map(|value| RadicalReduction::radical(value, used_special_angle))
}

fn reduce_radical_quotient(
    numerator: &RadicalReduction,
    denominator: &RadicalReduction,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if let RadicalReduction::LinearCombination(numerator) = numerator {
        let Some(denominator) = radical_components(denominator) else {
            return Ok(None);
        };
        if denominator.radicand != Rational::one() {
            return Ok(None);
        }
        let scalar = Rational::one()
            .divide(&denominator.coefficient)
            .map_err(arithmetic_error)?;
        return Ok(Some(scale_radical_linear_combination(
            numerator.value(),
            &scalar,
            used_special_angle,
        )));
    }
    let Some(numerator) = radical_components(numerator) else {
        return Ok(None);
    };
    let Some(denominator) = radical_components(denominator) else {
        return Ok(None);
    };
    let coefficient = numerator
        .coefficient
        .divide(&denominator.coefficient)
        .map_err(arithmetic_error)?;
    let radicand = numerator
        .radicand
        .divide(&denominator.radicand)
        .map_err(arithmetic_error)?;
    reduce_radical_product(coefficient, radicand, used_special_angle)
}

fn reduce_radical_product(
    coefficient: Rational,
    radicand: Rational,
    used_special_angle: bool,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    if coefficient.is_zero() {
        return Ok(Some(RadicalReduction::rational(
            Rational::zero(),
            used_special_angle,
        )));
    }
    if radicand.is_negative() || radicand.is_zero() {
        return Err(domain_error(DomainErrorKind::EvenRootOfNegative));
    }
    if let Some(root) = radicand.sqrt_if_rational() {
        return Ok(Some(RadicalReduction::rational(
            coefficient.multiply(&root),
            used_special_angle,
        )));
    }
    let Some(radical) = radicand.sqrt_as_simple_radical() else {
        return Ok(None);
    };
    Ok(Some(RadicalReduction::radical(
        SimpleRadical {
            coefficient: coefficient.multiply(&radical.coefficient),
            radicand: radical.radicand,
        },
        used_special_angle,
    )))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RadicalComponents {
    coefficient: Rational,
    radicand: Rational,
}

fn radical_components(value: &RadicalReduction) -> Option<RadicalComponents> {
    match value {
        RadicalReduction::Rational(value) => Some(RadicalComponents {
            coefficient: value.value().clone(),
            radicand: Rational::one(),
        }),
        RadicalReduction::Radical(value) => Some(RadicalComponents {
            coefficient: value.value().coefficient.clone(),
            radicand: rational_from_positive_integer(&value.value().radicand),
        }),
        RadicalReduction::LinearCombination(_) => None,
    }
}

fn rational_from_positive_integer(value: &PositiveInteger) -> Rational {
    Rational::from_integer(value.inner.clone())
}

fn add_radical_term(radicals: &mut Vec<SimpleRadical>, radical: &SimpleRadical) {
    if radical.coefficient.is_zero() {
        return;
    }
    if let Some(existing) = radicals
        .iter_mut()
        .find(|existing| existing.radicand == radical.radicand)
    {
        existing.coefficient = existing.coefficient.add(&radical.coefficient);
        return;
    }
    radicals.push(radical.clone());
}

fn radical_linear_combination_reduction(
    rational: Rational,
    mut radicals: Vec<SimpleRadical>,
    used_special_angle: bool,
) -> RadicalReduction {
    radicals.retain(|radical| !radical.coefficient.is_zero());
    radicals.sort_by(|left, right| {
        left.coefficient
            .is_negative()
            .cmp(&right.coefficient.is_negative())
            .then_with(|| left.radicand.inner.inner.cmp(&right.radicand.inner.inner))
    });

    match (rational.is_zero(), radicals.len()) {
        (_, 0) => RadicalReduction::rational(rational, used_special_angle),
        (true, 1) => RadicalReduction::radical(
            radicals
                .pop()
                .expect("radical length was checked before popping"),
            used_special_angle,
        ),
        _ => RadicalReduction::linear_combination(
            RadicalLinearCombination { rational, radicals },
            used_special_angle,
        ),
    }
}

fn scale_radical_linear_combination(
    value: &RadicalLinearCombination,
    scalar: &Rational,
    used_special_angle: bool,
) -> RadicalReduction {
    let rational = value.rational.multiply(scalar);
    let radicals = value
        .radicals
        .iter()
        .map(|radical| SimpleRadical {
            coefficient: radical.coefficient.multiply(scalar),
            radicand: radical.radicand.clone(),
        })
        .collect();
    radical_linear_combination_reduction(rational, radicals, used_special_angle)
}

const MAX_RADICAL_SIGN_PROOF_DEPTH: usize = 8;
const MAX_RADICAL_SIGN_PROOF_TERMS: usize = 8;

fn radical_reduction_sign(value: &RadicalReduction) -> Option<Ordering> {
    radical_reduction_sign_with_depth(value, 0)
}

fn radical_reduction_sign_with_depth(value: &RadicalReduction, depth: usize) -> Option<Ordering> {
    match value {
        RadicalReduction::Rational(value) => Some(rational_sign(value.value())),
        RadicalReduction::Radical(value) => Some(rational_sign(&value.value().coefficient)),
        RadicalReduction::LinearCombination(value) => {
            radical_linear_combination_sign(value.value(), depth)
        }
    }
}

fn radical_linear_combination_sign(
    value: &RadicalLinearCombination,
    depth: usize,
) -> Option<Ordering> {
    if let Some(sign) = radical_linear_combination_trivial_sign(value) {
        return Some(sign);
    }
    if depth >= MAX_RADICAL_SIGN_PROOF_DEPTH
        || radical_linear_combination_term_count(value) > MAX_RADICAL_SIGN_PROOF_TERMS
    {
        return None;
    }

    let (positive, negative) = split_radical_linear_combination_by_sign(value);
    compare_nonnegative_radical_reductions(&positive, &negative, depth + 1)
}

fn radical_linear_combination_trivial_sign(value: &RadicalLinearCombination) -> Option<Ordering> {
    let mut saw_positive = false;
    let mut saw_negative = false;
    update_component_sign(
        rational_sign(&value.rational),
        &mut saw_positive,
        &mut saw_negative,
    );
    for radical in &value.radicals {
        update_component_sign(
            rational_sign(&radical.coefficient),
            &mut saw_positive,
            &mut saw_negative,
        );
    }

    match (saw_positive, saw_negative) {
        (false, false) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Greater),
        (false, true) => Some(Ordering::Less),
        (true, true) => None,
    }
}

fn update_component_sign(sign: Ordering, saw_positive: &mut bool, saw_negative: &mut bool) {
    match sign {
        Ordering::Less => *saw_negative = true,
        Ordering::Equal => {}
        Ordering::Greater => *saw_positive = true,
    }
}

fn radical_linear_combination_term_count(value: &RadicalLinearCombination) -> usize {
    usize::from(!value.rational.is_zero()) + value.radicals.len()
}

fn split_radical_linear_combination_by_sign(
    value: &RadicalLinearCombination,
) -> (RadicalReduction, RadicalReduction) {
    let mut positive_rational = Rational::zero();
    let mut negative_rational = Rational::zero();
    if value.rational.is_negative() {
        negative_rational = value.rational.negate();
    } else {
        positive_rational = value.rational.clone();
    }

    let mut positive_radicals = Vec::new();
    let mut negative_radicals = Vec::new();
    for radical in &value.radicals {
        if radical.coefficient.is_negative() {
            negative_radicals.push(SimpleRadical {
                coefficient: radical.coefficient.negate(),
                radicand: radical.radicand.clone(),
            });
        } else {
            positive_radicals.push(radical.clone());
        }
    }

    (
        radical_linear_combination_reduction(positive_rational, positive_radicals, false),
        radical_linear_combination_reduction(negative_rational, negative_radicals, false),
    )
}

fn compare_nonnegative_radical_reductions(
    left: &RadicalReduction,
    right: &RadicalReduction,
    depth: usize,
) -> Option<Ordering> {
    if depth >= MAX_RADICAL_SIGN_PROOF_DEPTH {
        return None;
    }
    let left_terms =
        radical_linear_combination_term_count(&radical_reduction_as_linear_combination(left));
    let right_terms =
        radical_linear_combination_term_count(&radical_reduction_as_linear_combination(right));
    if left_terms + right_terms > MAX_RADICAL_SIGN_PROOF_TERMS {
        return None;
    }

    let left_squared = multiply_radical_reductions(left, left, false).ok()??;
    let right_squared = multiply_radical_reductions(right, right, false).ok()??;
    let difference = subtract_radical_reductions(&left_squared, &right_squared);
    radical_reduction_sign_with_depth(&difference, depth + 1)
}

fn subtract_radical_reductions(lhs: &RadicalReduction, rhs: &RadicalReduction) -> RadicalReduction {
    let lhs = radical_reduction_as_linear_combination(lhs);
    let rhs = radical_reduction_as_linear_combination(rhs);
    let mut radicals = lhs.radicals;
    for radical in rhs.radicals {
        add_radical_term(
            &mut radicals,
            &SimpleRadical {
                coefficient: radical.coefficient.negate(),
                radicand: radical.radicand,
            },
        );
    }
    radical_linear_combination_reduction(lhs.rational.subtract(&rhs.rational), radicals, false)
}

fn rational_sign(value: &Rational) -> Ordering {
    if value.is_negative() {
        Ordering::Less
    } else if value.is_zero() {
        Ordering::Equal
    } else {
        Ordering::Greater
    }
}

fn stored_real_algebraic_sign(value: &RealAlgebraic) -> Option<Ordering> {
    let zero = Rational::zero();
    let interval = value.isolating_interval();
    if interval.lower.compare(&zero) != Ordering::Less {
        Some(Ordering::Greater)
    } else if interval.upper.compare(&zero) != Ordering::Greater {
        Some(Ordering::Less)
    } else {
        None
    }
}

fn exact_reduction_sign(value: &ExactReduction) -> Option<Ordering> {
    match value {
        ExactReduction::PiMultiple(value) => Some(rational_sign(value.coefficient())),
        ExactReduction::Radical(value) => radical_reduction_sign(value),
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Rational(value)) => {
            Some(rational_sign(value.value()))
        }
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) => {
            stored_real_algebraic_sign(value)
        }
        ExactReduction::Symbolic => None,
    }
}

pub(crate) fn node_is_exact_zero(dag: &ExactExpressionDag, id: ExprId) -> bool {
    match dag.node(id) {
        ExpressionNode::Rational(value) => dag.rational(*value).is_zero(),
        ExpressionNode::Exact(value) => {
            exact_reduction_sign(dag.exact_value(*value)) == Some(Ordering::Equal)
        }
        ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => false,
    }
}

fn exact_reduction_sign_bounded(
    value: &ExactReduction,
    limits: &ResourceLimits,
) -> Result<Option<Ordering>, EvaluationError> {
    if let Some(sign) = exact_reduction_sign(value) {
        return Ok(Some(sign));
    }
    let ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) = value else {
        return Ok(None);
    };
    match value.sign_bounded(limits.max_root_isolation_steps) {
        Ok(sign) => Ok(sign),
        Err(RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        )) => Ok(None),
        Err(error) => Err(real_algebraic_construction_error(error)),
    }
}

fn negate_exact_reduction(
    value: &ExactReduction,
    limits: &ResourceLimits,
) -> Result<Option<ExactReduction>, EvaluationError> {
    Ok(match value {
        ExactReduction::PiMultiple(value) => Some(ExactReduction::PiMultiple(
            PiCoefficientEvaluation::with_origin(
                value.coefficient().negate(),
                value.used_special_angle(),
            ),
        )),
        ExactReduction::Radical(RadicalReduction::Rational(value)) => {
            Some(ExactReduction::Radical(RadicalReduction::Rational(
                RationalEvaluation::with_origin(value.value().negate(), value.used_special_angle()),
            )))
        }
        ExactReduction::Radical(RadicalReduction::Radical(value)) => {
            let mut radical = value.value().clone();
            radical.coefficient = radical.coefficient.negate();
            Some(ExactReduction::Radical(RadicalReduction::radical(
                radical,
                value.used_special_angle(),
            )))
        }
        ExactReduction::Radical(RadicalReduction::LinearCombination(value)) => {
            let mut combination = value.value().clone();
            combination.rational = combination.rational.negate();
            for radical in &mut combination.radicals {
                radical.coefficient = radical.coefficient.negate();
            }
            Some(ExactReduction::Radical(
                RadicalReduction::linear_combination(combination, value.used_special_angle()),
            ))
        }
        ExactReduction::RealAlgebraic(value) => multiply_real_algebraic_evaluation_by_rational(
            value.clone(),
            &rational_integer(-1),
            limits,
        )?
        .map(ExactReduction::RealAlgebraic),
        ExactReduction::Symbolic => None,
    })
}

fn evaluate_exact_reduction_interval(
    value: &ExactReduction,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match value {
        ExactReduction::PiMultiple(value) => interval::multiply(
            &interval::from_rational(value.coefficient(), precision_bits),
            &interval::constant(Constant::Pi, precision_bits)?,
        ),
        ExactReduction::Radical(value) => radical_reduction_interval(value, precision_bits),
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Rational(value)) => {
            Ok(interval::from_rational(value.value(), precision_bits))
        }
        ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) => {
            interval::from_rational_bounds(
                &value.isolating_interval().lower,
                &value.isolating_interval().upper,
                precision_bits,
            )
        }
        ExactReduction::Symbolic => Err(IntervalError::UnsupportedExpression),
    }
}

fn radical_reduction_interval(
    value: &RadicalReduction,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match value {
        RadicalReduction::Rational(value) => {
            Ok(interval::from_rational(value.value(), precision_bits))
        }
        RadicalReduction::Radical(value) => simple_radical_interval(value.value(), precision_bits),
        RadicalReduction::LinearCombination(value) => {
            let value = value.value();
            let mut result = interval::from_rational(&value.rational, precision_bits);
            for radical in &value.radicals {
                result =
                    interval::add(&result, &simple_radical_interval(radical, precision_bits)?)?;
            }
            Ok(result)
        }
    }
}

fn simple_radical_interval(
    value: &SimpleRadical,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let radicand = Rational::from_integer(value.radicand.inner.clone());
    interval::multiply(
        &interval::from_rational(&value.coefficient, precision_bits),
        &interval::sqrt(
            &interval::from_rational(&radicand, precision_bits),
            precision_bits,
        )?,
    )
}

fn evaluate_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let base = evaluate_node(dag, base)?;
    let exponent = match evaluate_node(dag, exponent) {
        Ok(exponent) => exponent,
        Err(error) if base.value().is_negative() && is_unsupported_exact_expression(&error) => {
            return Err(domain_error(DomainErrorKind::NonRealPower));
        }
        Err(error) => return Err(error),
    };
    let used_special_angle = base.used_special_angle() || exponent.used_special_angle();
    evaluate_rational_power(base.value(), exponent.value())
        .map(|value| RationalEvaluation::with_origin(value, used_special_angle))
}

fn is_unsupported_exact_expression(error: &EvaluationError) -> bool {
    matches!(error, EvaluationError::UnsupportedFeature(_))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogDomainProof {
    Positive,
    NonPositive,
    Unknown,
}

fn rational_log_domain_proof(value: &Rational) -> LogDomainProof {
    if value.is_negative() || value.is_zero() {
        LogDomainProof::NonPositive
    } else {
        LogDomainProof::Positive
    }
}

fn radical_log_domain_proof(value: &RadicalReduction) -> LogDomainProof {
    match radical_reduction_sign(value) {
        Some(Ordering::Greater) => LogDomainProof::Positive,
        Some(Ordering::Equal | Ordering::Less) => LogDomainProof::NonPositive,
        None => LogDomainProof::Unknown,
    }
}

fn real_algebraic_log_domain_proof(
    value: &RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<LogDomainProof, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(rational_log_domain_proof(value.value())),
        RealAlgebraicEvaluation::Algebraic(value) => {
            match value.sign_bounded(limits.max_root_isolation_steps) {
                Ok(Some(Ordering::Greater)) => Ok(LogDomainProof::Positive),
                Ok(Some(Ordering::Equal | Ordering::Less)) => Ok(LogDomainProof::NonPositive),
                Ok(None)
                | Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(LogDomainProof::Unknown),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitIntervalDomainProof {
    InRange,
    OutOfRange,
    Unknown,
}

fn rational_unit_interval_domain_proof(value: &Rational) -> UnitIntervalDomainProof {
    if value.compare(&rational_integer(-1)) == Ordering::Less
        || value.compare(&Rational::one()) == Ordering::Greater
    {
        UnitIntervalDomainProof::OutOfRange
    } else {
        UnitIntervalDomainProof::InRange
    }
}

fn radical_unit_interval_domain_proof(value: &RadicalReduction) -> UnitIntervalDomainProof {
    let minus_one = RadicalReduction::rational(rational_integer(-1), false);
    let one = RadicalReduction::rational(Rational::one(), false);
    let lower_margin = subtract_radical_reductions(value, &minus_one);
    let upper_margin = subtract_radical_reductions(&one, value);
    let lower_sign = radical_reduction_sign(&lower_margin);
    let upper_sign = radical_reduction_sign(&upper_margin);

    if matches!(lower_sign, Some(Ordering::Less)) || matches!(upper_sign, Some(Ordering::Less)) {
        UnitIntervalDomainProof::OutOfRange
    } else if lower_sign.is_some() && upper_sign.is_some() {
        UnitIntervalDomainProof::InRange
    } else {
        UnitIntervalDomainProof::Unknown
    }
}

fn real_algebraic_unit_interval_domain_proof(
    value: &RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<UnitIntervalDomainProof, EvaluationError> {
    let lower_comparison =
        compare_real_algebraic_evaluation_to_rational(value, &rational_integer(-1), limits)?;
    let upper_comparison =
        compare_real_algebraic_evaluation_to_rational(value, &Rational::one(), limits)?;

    if matches!(lower_comparison, Some(Ordering::Less))
        || matches!(upper_comparison, Some(Ordering::Greater))
    {
        Ok(UnitIntervalDomainProof::OutOfRange)
    } else if lower_comparison.is_some() && upper_comparison.is_some() {
        Ok(UnitIntervalDomainProof::InRange)
    } else {
        Ok(UnitIntervalDomainProof::Unknown)
    }
}

fn compare_real_algebraic_evaluation_to_rational(
    value: &RealAlgebraicEvaluation,
    rhs: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<Ordering>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(value.value().compare(rhs))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            let difference = match value.add_rational_bounded(
                &rhs.negate(),
                limits.max_polynomial_coefficient_bits,
                limits.max_root_isolation_steps,
            ) {
                Ok(Some(value)) => value,
                Ok(None)
                | Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => return Ok(None),
                Err(error) => return Err(real_algebraic_construction_error(error)),
            };
            match difference.sign_bounded(limits.max_root_isolation_steps) {
                Ok(sign) => Ok(sign),
                Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(None),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

fn evaluate_rational_power(
    base: &Rational,
    exponent: &Rational,
) -> Result<Rational, EvaluationError> {
    if base.is_zero() {
        return evaluate_zero_power(exponent);
    }

    let exponent_numerator = exponent
        .numerator
        .inner
        .to_i64()
        .ok_or_else(exponent_too_large_error)?;
    if exponent.is_integer() {
        return base.pow_i64(exponent_numerator).map_err(arithmetic_error);
    }

    let root_index = exponent
        .denominator
        .inner
        .inner
        .to_u32()
        .ok_or_else(exponent_too_large_error)?;
    if base.is_negative() && root_index.is_multiple_of(2) {
        return Err(domain_error(DomainErrorKind::NonRealPower));
    }

    let root = base
        .nth_root_if_rational(root_index)
        .ok_or_else(non_integer_power_error)?;
    root.pow_i64(exponent_numerator).map_err(arithmetic_error)
}

fn evaluate_zero_power(exponent: &Rational) -> Result<Rational, EvaluationError> {
    if exponent.is_zero() {
        Err(domain_error(DomainErrorKind::IndeterminateZeroToZero))
    } else if exponent.is_negative() {
        Err(domain_error(DomainErrorKind::ZeroToNegativePower))
    } else {
        Ok(Rational::zero())
    }
}

fn evaluate_exp_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_exp_log_identity(dag, argument)? {
        return Ok(value);
    }
    let argument = evaluate_node(dag, argument)?;
    if argument.value().is_zero() {
        Ok(RationalEvaluation::with_origin(
            Rational::one(),
            argument.used_special_angle(),
        ))
    } else {
        Err(unsupported_function_evaluation())
    }
}

fn evaluate_log_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_log_exp_identity(dag, argument)? {
        return Ok(value);
    }
    if matches!(
        dag.semantic_node(argument),
        ExpressionNode::Constant(Constant::Euler)
    ) {
        return Ok(RationalEvaluation::direct(Rational::one()));
    }
    let argument = evaluate_node(dag, argument)?;
    if argument.value().is_negative() || argument.value().is_zero() {
        return Err(logarithm_of_non_positive_error());
    }
    if argument.value() == &Rational::one() {
        Ok(RationalEvaluation::with_origin(
            Rational::zero(),
            argument.used_special_angle(),
        ))
    } else {
        Err(unsupported_function_evaluation())
    }
}

const MAX_EXACT_LOG_BASE_ABS_EXPONENT: i64 = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalPowerPattern {
    basis: RationalEvaluation,
    exponent: RationalEvaluation,
}

#[derive(Clone, Copy)]
struct LogarithmRatio {
    argument: ExprId,
    base: Option<ExprId>,
}

#[derive(Clone)]
struct ScaledLogarithmRatio {
    logarithm: LogarithmRatio,
    scale: RationalEvaluation,
}

fn logarithm_ratio(dag: &ExactExpressionDag, id: ExprId) -> Option<LogarithmRatio> {
    match dag.semantic_node(id) {
        ExpressionNode::Function {
            function: Function::Log | Function::Ln,
            argument,
        } => Some(LogarithmRatio {
            argument: *argument,
            base: None,
        }),
        ExpressionNode::LogBase { argument, base } => Some(LogarithmRatio {
            argument: *argument,
            base: Some(*base),
        }),
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => None,
    }
}

fn scaled_logarithm_ratio(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<ScaledLogarithmRatio>, EvaluationError> {
    if let Some(logarithm) = logarithm_ratio(dag, id) {
        return Ok(Some(ScaledLogarithmRatio {
            logarithm,
            scale: RationalEvaluation::direct(Rational::one()),
        }));
    }
    match dag.semantic_node(id) {
        ExpressionNode::Multiply(list) => {
            let mut logarithm = None;
            let mut scale = Rational::one();
            let mut used_special_angle = false;
            for factor in dag.list(*list) {
                if let Some(value) = scaled_logarithm_ratio(dag, *factor)? {
                    if logarithm.replace(value.logarithm).is_some() {
                        return Ok(None);
                    }
                    scale = scale.multiply(value.scale.value());
                    used_special_angle |= value.scale.used_special_angle();
                    continue;
                }
                match evaluate_node(dag, *factor) {
                    Ok(value) => {
                        scale = scale.multiply(value.value());
                        used_special_angle |= value.used_special_angle();
                    }
                    Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                    Err(error) => return Err(error),
                }
            }
            Ok(logarithm.map(|logarithm| ScaledLogarithmRatio {
                logarithm,
                scale: RationalEvaluation::with_origin(scale, used_special_angle),
            }))
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let Some(mut numerator) = scaled_logarithm_ratio(dag, *numerator)? else {
                return Ok(None);
            };
            let denominator = match evaluate_node(dag, *denominator) {
                Ok(value) => value,
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            let scale = numerator
                .scale
                .value()
                .divide(denominator.value())
                .map_err(arithmetic_error)?;
            numerator.scale = RationalEvaluation::with_origin(
                scale,
                numerator.scale.used_special_angle() || denominator.used_special_angle(),
            );
            Ok(Some(numerator))
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(None),
    }
}

fn logarithm_ratio_domain_is_proven(
    dag: &ExactExpressionDag,
    logarithm: LogarithmRatio,
) -> Result<bool, EvaluationError> {
    if !prove_expression_positive(dag, logarithm.argument)? {
        return Ok(false);
    }
    let Some(base) = logarithm.base else {
        return Ok(true);
    };
    Ok(prove_expression_positive(dag, base)? && prove_expression_not_one(dag, base)?)
}

fn prove_expression_not_one(dag: &ExactExpressionDag, id: ExprId) -> Result<bool, EvaluationError> {
    match evaluate_node(dag, id) {
        Ok(value) => return Ok(value.value() != &Rational::one()),
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }
    match dag.node(id) {
        ExpressionNode::Exact(value) => {
            Ok(!matches!(dag.exact_value(*value), ExactReduction::Symbolic))
        }
        ExpressionNode::Constant(Constant::Pi | Constant::Euler) => Ok(true),
        ExpressionNode::Rational(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => Ok(false),
    }
}

fn logarithm_expression_equal(dag: &ExactExpressionDag, left: ExprId, right: ExprId) -> bool {
    if left == right {
        return true;
    }
    let is_non_symbolic_exact = |id| {
        matches!(
            dag.node(id),
            ExpressionNode::Exact(value)
                if !matches!(dag.exact_value(*value), ExactReduction::Symbolic)
        )
    };
    if is_non_symbolic_exact(left) || is_non_symbolic_exact(right) {
        return false;
    }
    structurally_equal_expressions(dag, left, right)
}

pub(crate) fn logarithm_quotient_identity(
    dag: &ExactExpressionDag,
    numerator: ExprId,
    denominator: ExprId,
) -> Result<Option<(ExprId, ExprId, RationalEvaluation)>, EvaluationError> {
    let Some(numerator) = scaled_logarithm_ratio(dag, numerator)? else {
        return Ok(None);
    };
    let Some(denominator) = scaled_logarithm_ratio(dag, denominator)? else {
        return Ok(None);
    };
    if !logarithm_ratio_domain_is_proven(dag, numerator.logarithm)?
        || !logarithm_ratio_domain_is_proven(dag, denominator.logarithm)?
        || !prove_expression_not_one(dag, denominator.logarithm.argument)?
    {
        return Ok(None);
    }

    let scale = RationalEvaluation::with_origin(
        numerator
            .scale
            .value()
            .divide(denominator.scale.value())
            .map_err(arithmetic_error)?,
        numerator.scale.used_special_angle() || denominator.scale.used_special_angle(),
    );

    let common_base = match (numerator.logarithm.base, denominator.logarithm.base) {
        (None, None) => true,
        (Some(left), Some(right)) => logarithm_expression_equal(dag, left, right),
        _ => false,
    };
    if common_base {
        return Ok(Some((
            numerator.logarithm.argument,
            denominator.logarithm.argument,
            scale,
        )));
    }
    if logarithm_expression_equal(
        dag,
        numerator.logarithm.argument,
        denominator.logarithm.argument,
    ) {
        if let (Some(numerator_base), Some(denominator_base)) =
            (numerator.logarithm.base, denominator.logarithm.base)
        {
            return Ok(Some((denominator_base, numerator_base, scale)));
        }
    }
    Ok(None)
}

pub(crate) struct LogarithmSumReduction {
    pub(crate) numerator_arguments: Vec<ExprId>,
    pub(crate) denominator_arguments: Vec<ExprId>,
    pub(crate) base: ExprId,
    pub(crate) scale: RationalEvaluation,
    pub(crate) offset: RationalEvaluation,
}

pub(crate) fn logarithm_sum_reduction(
    dag: &ExactExpressionDag,
    terms: &[ExprId],
) -> Result<Option<LogarithmSumReduction>, EvaluationError> {
    let mut logarithms = Vec::new();
    let mut offset = Rational::zero();
    let mut offset_used_special_angle = false;
    for term in terms {
        if !collect_logarithm_sum_term(
            dag,
            *term,
            &mut logarithms,
            &mut offset,
            &mut offset_used_special_angle,
        )? {
            return Ok(None);
        }
    }
    if logarithms.len() < 2 {
        return Ok(None);
    }
    let Some(base) = logarithms[0].logarithm.base else {
        return Ok(None);
    };
    let scale_magnitude = rational_magnitude(logarithms[0].scale.value());
    if scale_magnitude.is_zero() {
        return Ok(None);
    }
    let mut used_special_angle = logarithms[0].scale.used_special_angle();
    let mut numerator_arguments = Vec::new();
    let mut denominator_arguments = Vec::new();
    for logarithm in logarithms {
        let Some(logarithm_base) = logarithm.logarithm.base else {
            return Ok(None);
        };
        if !logarithm_expression_equal(dag, base, logarithm_base)
            || !logarithm_ratio_domain_is_proven(dag, logarithm.logarithm)?
            || rational_magnitude(logarithm.scale.value()) != scale_magnitude
        {
            return Ok(None);
        }
        used_special_angle |= logarithm.scale.used_special_angle();
        if logarithm.scale.value().is_negative() {
            denominator_arguments.push(logarithm.logarithm.argument);
        } else {
            numerator_arguments.push(logarithm.logarithm.argument);
        }
    }
    let scale = if numerator_arguments.is_empty() {
        core::mem::swap(&mut numerator_arguments, &mut denominator_arguments);
        scale_magnitude.negate()
    } else {
        scale_magnitude
    };
    Ok(Some(LogarithmSumReduction {
        numerator_arguments,
        denominator_arguments,
        base,
        scale: RationalEvaluation::with_origin(scale, used_special_angle),
        offset: RationalEvaluation::with_origin(offset, offset_used_special_angle),
    }))
}

fn rational_magnitude(value: &Rational) -> Rational {
    if value.is_negative() {
        value.negate()
    } else {
        value.clone()
    }
}

fn collect_logarithm_sum_term(
    dag: &ExactExpressionDag,
    id: ExprId,
    logarithms: &mut Vec<ScaledLogarithmRatio>,
    offset: &mut Rational,
    offset_used_special_angle: &mut bool,
) -> Result<bool, EvaluationError> {
    if let ExpressionNode::Add(terms) = dag.semantic_node(id) {
        for term in dag.list(*terms) {
            if !collect_logarithm_sum_term(
                dag,
                *term,
                logarithms,
                offset,
                offset_used_special_angle,
            )? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if let ExpressionNode::Multiply(factors) = dag.semantic_node(id) {
        let mut nested_sum = None;
        for factor in dag.list(*factors) {
            if let ExpressionNode::Add(terms) = dag.semantic_node(*factor) {
                if nested_sum.replace(*terms).is_some() {
                    return Ok(false);
                }
            }
        }
        let Some(nested_sum) = nested_sum else {
            if let Some(logarithm) = scaled_logarithm_ratio(dag, id)? {
                logarithms.push(logarithm);
                return Ok(true);
            }
            return Ok(false);
        };
        let mut scale = Rational::one();
        let mut scale_used_special_angle = false;
        for factor in dag.list(*factors) {
            if matches!(dag.semantic_node(*factor), ExpressionNode::Add(_)) {
                continue;
            }
            match evaluate_node(dag, *factor) {
                Ok(value) => {
                    scale = scale.multiply(value.value());
                    scale_used_special_angle |= value.used_special_angle();
                }
                Err(error) if is_unsupported_exact_expression(&error) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        let mut nested_logarithms = Vec::new();
        let mut nested_offset = Rational::zero();
        let mut nested_offset_used_special_angle = false;
        for term in dag.list(nested_sum) {
            if !collect_logarithm_sum_term(
                dag,
                *term,
                &mut nested_logarithms,
                &mut nested_offset,
                &mut nested_offset_used_special_angle,
            )? {
                return Ok(false);
            }
        }
        for logarithm in &mut nested_logarithms {
            logarithm.scale = RationalEvaluation::with_origin(
                logarithm.scale.value().multiply(&scale),
                logarithm.scale.used_special_angle() || scale_used_special_angle,
            );
        }
        logarithms.extend(nested_logarithms);
        *offset = offset.add(&nested_offset.multiply(&scale));
        *offset_used_special_angle |= nested_offset_used_special_angle || scale_used_special_angle;
        return Ok(true);
    }
    if let Some(logarithm) = scaled_logarithm_ratio(dag, id)? {
        logarithms.push(logarithm);
        return Ok(true);
    }
    match evaluate_node(dag, id) {
        Ok(value) => {
            *offset = offset.add(value.value());
            *offset_used_special_angle |= value.used_special_angle();
            Ok(true)
        }
        Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) struct LogarithmProductReduction {
    pub(crate) logarithm: Option<(ExprId, ExprId)>,
    pub(crate) scale: RationalEvaluation,
}

pub(crate) fn logarithm_product_reduction(
    dag: &ExactExpressionDag,
    factors: &[ExprId],
) -> Result<Option<LogarithmProductReduction>, EvaluationError> {
    let mut numerators = Vec::new();
    let mut denominators = Vec::new();
    let mut scale = Rational::one();
    let mut used_special_angle = false;
    for factor in factors {
        if !collect_logarithm_product_factor(
            dag,
            *factor,
            &mut numerators,
            &mut denominators,
            &mut scale,
            &mut used_special_angle,
        )? {
            return Ok(None);
        }
    }
    if numerators.is_empty() {
        return Ok(None);
    }

    let mut numerator_index = 0;
    while numerator_index < numerators.len() {
        let Some(denominator_index) = denominators.iter().position(|denominator| {
            logarithm_expression_equal(dag, numerators[numerator_index], *denominator)
        }) else {
            numerator_index += 1;
            continue;
        };
        numerators.remove(numerator_index);
        denominators.remove(denominator_index);
    }

    let logarithm = match (numerators.as_slice(), denominators.as_slice()) {
        ([], []) => None,
        ([argument], [base]) => Some((*argument, *base)),
        _ => return Ok(None),
    };
    Ok(Some(LogarithmProductReduction {
        logarithm,
        scale: RationalEvaluation::with_origin(scale, used_special_angle),
    }))
}

fn collect_logarithm_product_factor(
    dag: &ExactExpressionDag,
    id: ExprId,
    numerators: &mut Vec<ExprId>,
    denominators: &mut Vec<ExprId>,
    scale: &mut Rational,
    used_special_angle: &mut bool,
) -> Result<bool, EvaluationError> {
    if let Some(logarithm) = logarithm_ratio(dag, id) {
        let Some(base) = logarithm.base else {
            return Ok(false);
        };
        if !logarithm_ratio_domain_is_proven(dag, logarithm)? {
            return Ok(false);
        }
        numerators.push(logarithm.argument);
        denominators.push(base);
        return Ok(true);
    }
    if let ExpressionNode::Multiply(list) = dag.semantic_node(id) {
        for factor in dag.list(*list) {
            if !collect_logarithm_product_factor(
                dag,
                *factor,
                numerators,
                denominators,
                scale,
                used_special_angle,
            )? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    match evaluate_node(dag, id) {
        Ok(value) => {
            *scale = scale.multiply(value.value());
            *used_special_angle |= value.used_special_angle();
            Ok(true)
        }
        Err(error) if is_unsupported_exact_expression(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

impl RationalPowerPattern {
    fn used_special_angle(&self) -> bool {
        self.basis.used_special_angle() || self.exponent.used_special_angle()
    }
}

fn evaluate_log_base_function(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_log_base_common_power_identity(dag, argument, base)? {
        return Ok(value);
    }

    let base = evaluate_node(dag, base)?;
    let argument = evaluate_node(dag, argument)?;
    finish_rational_log_base(argument, base)
}

fn finish_rational_log_base(
    argument: RationalEvaluation,
    base: RationalEvaluation,
) -> Result<RationalEvaluation, EvaluationError> {
    ensure_log_base_domain(base.value())?;
    if argument.value().is_negative() || argument.value().is_zero() {
        return Err(logarithm_of_non_positive_error());
    }

    let used_special_angle = argument.used_special_angle() || base.used_special_angle();
    if argument.value() == &Rational::one() {
        return Ok(RationalEvaluation::with_origin(
            Rational::zero(),
            used_special_angle,
        ));
    }
    if argument.value() == base.value() {
        return Ok(RationalEvaluation::with_origin(
            Rational::one(),
            used_special_angle,
        ));
    }

    if let Some(exponent) = integer_log_exponent(argument.value(), base.value())? {
        return Ok(RationalEvaluation::with_origin(
            Rational::from_integer(Integer::from(exponent)),
            used_special_angle,
        ));
    }

    Err(unsupported_function_evaluation())
}

fn evaluate_log_base_common_power_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let Some(base_pattern) = evaluate_positive_rational_power_pattern(dag, base)? else {
        return Ok(None);
    };
    ensure_log_base_pattern_domain(&base_pattern)?;

    match evaluate_node(dag, argument) {
        Ok(value) => {
            if value.value().is_negative() || value.value().is_zero() {
                return Err(logarithm_of_non_positive_error());
            }
            if value.value() == &Rational::one() {
                return Ok(Some(RationalEvaluation::with_origin(
                    Rational::zero(),
                    value.used_special_angle() || base_pattern.used_special_angle(),
                )));
            }
        }
        Err(error) if is_unsupported_exact_expression(&error) => {}
        Err(error) => return Err(error),
    }

    let Some(argument_pattern) = evaluate_positive_rational_power_pattern(dag, argument)? else {
        return Ok(None);
    };
    finish_common_power_log_identity(argument_pattern, base_pattern)
}

fn finish_common_power_log_identity(
    argument: RationalPowerPattern,
    base: RationalPowerPattern,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    if argument.basis.value() == base.basis.value() {
        return log_power_exponent_quotient(argument, 1, base, 1);
    }

    if let Some(argument_basis_exponent) =
        integer_log_exponent(argument.basis.value(), base.basis.value())?
    {
        return log_power_exponent_quotient(argument, argument_basis_exponent, base, 1);
    }

    if let Some(base_basis_exponent) =
        integer_log_exponent(base.basis.value(), argument.basis.value())?
    {
        return log_power_exponent_quotient(argument, 1, base, base_basis_exponent);
    }

    Ok(None)
}

fn log_power_exponent_quotient(
    argument: RationalPowerPattern,
    argument_basis_exponent: i64,
    base: RationalPowerPattern,
    base_basis_exponent: i64,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let numerator = argument
        .exponent
        .value()
        .multiply(&Rational::from_integer(Integer::from(
            argument_basis_exponent,
        )));
    let denominator = base
        .exponent
        .value()
        .multiply(&Rational::from_integer(Integer::from(base_basis_exponent)));
    let value = numerator.divide(&denominator).map_err(arithmetic_error)?;
    Ok(Some(RationalEvaluation::with_origin(
        value,
        argument.used_special_angle() || base.used_special_angle(),
    )))
}

fn evaluate_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    match dag.node(id) {
        ExpressionNode::Exact(value) => {
            evaluate_positive_rational_power_pattern_node(dag, dag.exact_presentation(*value), None)
        }
        node @ (ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. }) => {
            evaluate_positive_rational_power_pattern_node(dag, node, Some(id))
        }
    }
}

fn evaluate_positive_rational_power_pattern_node(
    dag: &ExactExpressionDag,
    node: &ExpressionNode,
    id: Option<ExprId>,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    match node {
        ExpressionNode::Exact(_) => Ok(None),
        ExpressionNode::Power {
            base: power_base,
            exponent,
        } => evaluate_explicit_positive_rational_power_pattern(dag, *power_base, *exponent),
        ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        } => evaluate_square_root_positive_rational_power_pattern(dag, *argument),
        ExpressionNode::Rational(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Multiply(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function {
            function:
                Function::Sin
                | Function::Cos
                | Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Exp
                | Function::Log
                | Function::Ln
                | Function::Root
                | Function::Abs
                | Function::Floor
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Sinh
                | Function::Cosh
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh,
            ..
        }
        | ExpressionNode::BinaryFunction { .. } => {
            let Some(id) = id else {
                return Ok(None);
            };
            evaluate_direct_positive_rational_power_pattern(dag, id)
        }
    }
}

fn evaluate_explicit_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    power_base: ExprId,
    exponent: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let power_base = match evaluate_node(dag, power_base) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if power_base.value().is_negative() || power_base.value().is_zero() {
        return Ok(None);
    }

    let exponent = match evaluate_node(dag, exponent) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Some(RationalPowerPattern {
        basis: power_base,
        exponent,
    }))
}

fn evaluate_square_root_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    radicand: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let radicand = match evaluate_node(dag, radicand) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if radicand.value().is_negative() || radicand.value().is_zero() {
        return Ok(None);
    }
    Ok(Some(RationalPowerPattern {
        basis: radicand,
        exponent: RationalEvaluation::direct(rational(1, 2)),
    }))
}

fn evaluate_direct_positive_rational_power_pattern(
    dag: &ExactExpressionDag,
    id: ExprId,
) -> Result<Option<RationalPowerPattern>, EvaluationError> {
    let value = match evaluate_node(dag, id) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if value.value().is_negative() || value.value().is_zero() {
        return Ok(None);
    }
    Ok(Some(RationalPowerPattern {
        basis: value,
        exponent: RationalEvaluation::direct(Rational::one()),
    }))
}

fn ensure_log_base_pattern_domain(base: &RationalPowerPattern) -> Result<(), EvaluationError> {
    if base.basis.value() == &Rational::one() || base.exponent.value().is_zero() {
        return Err(domain_error(DomainErrorKind::LogarithmBaseOne));
    }
    Ok(())
}

fn integer_log_exponent(
    argument: &Rational,
    base: &Rational,
) -> Result<Option<i64>, EvaluationError> {
    if argument.is_negative()
        || argument.is_zero()
        || base.is_negative()
        || base.is_zero()
        || base == &Rational::one()
    {
        return Ok(None);
    }
    if argument == &Rational::one() {
        return Ok(Some(0));
    }
    if argument == base {
        return Ok(Some(1));
    }
    for exponent in 2..=MAX_EXACT_LOG_BASE_ABS_EXPONENT {
        if base.pow_i64(exponent).map_err(arithmetic_error)? == *argument {
            return Ok(Some(exponent));
        }
        if base.pow_i64(-exponent).map_err(arithmetic_error)? == *argument {
            return Ok(Some(-exponent));
        }
    }
    Ok(None)
}

fn ensure_log_base_domain(base: &Rational) -> Result<(), EvaluationError> {
    if base.is_negative() || base.is_zero() {
        return Err(logarithm_of_non_positive_error());
    }
    if base == &Rational::one() {
        return Err(domain_error(DomainErrorKind::LogarithmBaseOne));
    }
    Ok(())
}

fn evaluate_exp_log_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let (scale, argument) = match dag.semantic_node(argument) {
        ExpressionNode::Function {
            function: Function::Log,
            argument,
        } => (RationalEvaluation::direct(Rational::one()), *argument),
        ExpressionNode::Multiply(list) => {
            let mut scale = Rational::one();
            let mut used_special_angle = false;
            let mut logarithm_argument = None;
            for factor in dag.list(*list) {
                if let ExpressionNode::Function {
                    function: Function::Log,
                    argument,
                } = dag.semantic_node(*factor)
                {
                    if logarithm_argument.replace(*argument).is_some() {
                        return Ok(None);
                    }
                    continue;
                }
                let factor = match evaluate_node(dag, *factor) {
                    Ok(value) => value,
                    Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
                    Err(error) => return Err(error),
                };
                scale = scale.multiply(factor.value());
                used_special_angle |= factor.used_special_angle();
            }
            let Some(argument) = logarithm_argument else {
                return Ok(None);
            };
            (
                RationalEvaluation::with_origin(scale, used_special_angle),
                argument,
            )
        }
        ExpressionNode::Rational(_)
        | ExpressionNode::Exact(_)
        | ExpressionNode::Constant(_)
        | ExpressionNode::Add(_)
        | ExpressionNode::Divide { .. }
        | ExpressionNode::Power { .. }
        | ExpressionNode::LogBase { .. }
        | ExpressionNode::Function { .. }
        | ExpressionNode::BinaryFunction { .. } => return Ok(None),
    };
    let value = evaluate_node(dag, argument)?;
    if value.value().is_negative() || value.value().is_zero() {
        Err(logarithm_of_non_positive_error())
    } else {
        let result = evaluate_rational_power(value.value(), scale.value())?;
        Ok(Some(RationalEvaluation::with_origin(
            result,
            value.used_special_angle() || scale.used_special_angle(),
        )))
    }
}

fn evaluate_log_exp_identity(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let ExpressionNode::Function {
        function: Function::Exp,
        argument,
    } = dag.semantic_node(argument)
    else {
        return Ok(None);
    };
    Ok(Some(evaluate_node(dag, *argument)?))
}

fn evaluate_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    if let Some(value) = evaluate_rational_trigonometric_special_angle(dag, function, argument)? {
        return Ok(value);
    }
    if let Some(value) = evaluate_rational_inverse_trig_composition(dag, function, argument)? {
        return Ok(value);
    }
    Err(unsupported_function_evaluation())
}

fn evaluate_rational_trigonometric_special_angle(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let coefficient = match evaluate_pi_coefficient(dag, argument)? {
        Some(coefficient) => coefficient,
        None => match rational_zero_as_pi_coefficient(dag, argument) {
            Ok(coefficient) => PiCoefficientEvaluation::direct(coefficient),
            Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
            Err(error) => return Err(error),
        },
    };
    let value = match function {
        Function::Sin => sine_special_angle(coefficient.coefficient()),
        Function::Cos => cosine_special_angle(coefficient.coefficient()),
        Function::Tan => tangent_special_angle(coefficient.coefficient())?,
        Function::Asin
        | Function::Acos
        | Function::Atan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    }
    .map(RationalEvaluation::special_angle);
    Ok(value)
}

fn evaluate_rational_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RationalEvaluation>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    if composition.kind != InverseTrigCompositionKind::Identity {
        return Ok(None);
    }
    let value = match evaluate_node(dag, composition.inner_argument) {
        Ok(value) => value,
        Err(error) if is_unsupported_exact_expression(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match rational_unit_interval_domain_proof(value.value()) {
                UnitIntervalDomainProof::InRange => Ok(Some(value)),
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => Ok(Some(value)),
    }
}

fn evaluate_radical_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    let Some(value) = evaluate_radical_reduction(dag, composition.inner_argument)? else {
        return Ok(None);
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match radical_unit_interval_domain_proof(&value) {
                UnitIntervalDomainProof::InRange => match composition.kind {
                    InverseTrigCompositionKind::Identity => Ok(Some(value)),
                    InverseTrigCompositionKind::UnitCofunction => {
                        radical_inverse_trig_cofunction_value(&value)
                    }
                },
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => match composition.kind {
            InverseTrigCompositionKind::Identity => Ok(Some(value)),
            InverseTrigCompositionKind::UnitCofunction => Ok(None),
        },
    }
}

fn radical_inverse_trig_cofunction_value(
    value: &RadicalReduction,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let Some(square) = multiply_radical_reductions(value, value, value.used_special_angle())?
    else {
        return Ok(None);
    };
    let one = RadicalReduction::rational(Rational::one(), value.used_special_angle());
    let complement = subtract_radical_reductions(&one, &square);
    reduce_square_root(&complement, DomainErrorKind::EvenRootOfNegative)
}

fn real_algebraic_inverse_trig_cofunction_value(
    value: RealAlgebraicEvaluation,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(square) = multiply_real_algebraic_evaluations(value.clone(), value, limits)? else {
        return Ok(None);
    };
    let Some(negative_square) =
        multiply_real_algebraic_evaluation_by_rational(square, &rational_integer(-1), limits)?
    else {
        return Ok(None);
    };
    let Some(complement) =
        add_rational_to_real_algebraic_evaluation(negative_square, &Rational::one(), limits)?
    else {
        return Ok(None);
    };
    nth_root_real_algebraic_evaluation(complement, 2, DomainErrorKind::EvenRootOfNegative, limits)
}

fn add_rational_to_real_algebraic_evaluation(
    value: RealAlgebraicEvaluation,
    rhs: &Rational,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    match value {
        RealAlgebraicEvaluation::Rational(value) => Ok(Some(RealAlgebraicEvaluation::Rational(
            RationalEvaluation::with_origin(value.value().add(rhs), value.used_special_angle()),
        ))),
        RealAlgebraicEvaluation::Algebraic(value) => {
            match value.add_rational_bounded(
                rhs,
                limits.max_polynomial_coefficient_bits,
                limits.max_root_isolation_steps,
            ) {
                Ok(value) => Ok(value.map(RealAlgebraicEvaluation::from_algebraic)),
                Err(RealAlgebraicConstructionError::RootIsolation(
                    PrimitivePolynomialRootIsolationError::StepLimitExceeded,
                )) => Ok(None),
                Err(error) => Err(real_algebraic_construction_error(error)),
            }
        }
    }
}

fn evaluate_real_algebraic_inverse_trig_composition(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
    limits: &ResourceLimits,
) -> Result<Option<RealAlgebraicEvaluation>, EvaluationError> {
    let Some(composition) = inverse_trig_composition(dag, function, argument)? else {
        return Ok(None);
    };
    let Some(value) =
        evaluate_rational_or_real_algebraic_node(dag, composition.inner_argument, limits)?
    else {
        return Ok(None);
    };
    match composition.domain {
        InverseTrigCompositionDomain::UnitInterval => {
            match real_algebraic_unit_interval_domain_proof(&value, limits)? {
                UnitIntervalDomainProof::InRange => match composition.kind {
                    InverseTrigCompositionKind::Identity => Ok(Some(value)),
                    InverseTrigCompositionKind::UnitCofunction => {
                        real_algebraic_inverse_trig_cofunction_value(value, limits)
                    }
                },
                UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
                UnitIntervalDomainProof::Unknown => Ok(None),
            }
        }
        InverseTrigCompositionDomain::RealLine => match composition.kind {
            InverseTrigCompositionKind::Identity => Ok(Some(value)),
            InverseTrigCompositionKind::UnitCofunction => Ok(None),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InverseTrigComposition {
    domain: InverseTrigCompositionDomain,
    kind: InverseTrigCompositionKind,
    inner_argument: ExprId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InverseTrigCompositionKind {
    Identity,
    UnitCofunction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InverseTrigCompositionDomain {
    UnitInterval,
    RealLine,
}

fn inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
) -> Result<Option<InverseTrigComposition>, EvaluationError> {
    match dag.semantics().angle_unit {
        AngleUnit::Radian => Ok(direct_inverse_trig_composition(
            dag,
            outer_function,
            argument,
        )),
        AngleUnit::Degree => scaled_inverse_trig_composition(dag, outer_function, argument, 180),
        AngleUnit::Gradian => scaled_inverse_trig_composition(dag, outer_function, argument, 200),
    }
}

fn direct_inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
) -> Option<InverseTrigComposition> {
    let ExpressionNode::Function {
        function: inner_function,
        argument: inner_argument,
    } = dag.semantic_node(argument)
    else {
        return None;
    };
    inverse_trig_composition_rule(outer_function, *inner_function).map(|(domain, kind)| {
        InverseTrigComposition {
            domain,
            kind,
            inner_argument: *inner_argument,
        }
    })
}

fn scaled_inverse_trig_composition(
    dag: &ExactExpressionDag,
    outer_function: Function,
    argument: ExprId,
    unit_denominator: i64,
) -> Result<Option<InverseTrigComposition>, EvaluationError> {
    let ExpressionNode::Multiply(list_id) = dag.semantic_node(argument) else {
        return Ok(None);
    };
    let mut composition = None;
    let mut scale = Rational::one();
    for child in dag.list(*list_id) {
        if let ExpressionNode::Function {
            function: inner_function,
            argument: inner_argument,
        } = dag.semantic_node(*child)
        {
            if let Some((domain, kind)) =
                inverse_trig_composition_rule(outer_function, *inner_function)
            {
                if composition
                    .replace(InverseTrigComposition {
                        domain,
                        kind,
                        inner_argument: *inner_argument,
                    })
                    .is_some()
                {
                    return Ok(None);
                }
                continue;
            }
        }

        let Some(coefficient) = evaluate_pi_coefficient(dag, *child)? else {
            return Ok(None);
        };
        scale = scale.multiply(coefficient.coefficient());
    }

    if scale == rational(1, unit_denominator) {
        Ok(composition)
    } else {
        Ok(None)
    }
}

fn inverse_trig_composition_rule(
    outer_function: Function,
    inner_function: Function,
) -> Option<(InverseTrigCompositionDomain, InverseTrigCompositionKind)> {
    match (outer_function, inner_function) {
        (Function::Sin, Function::Asin) | (Function::Cos, Function::Acos) => Some((
            InverseTrigCompositionDomain::UnitInterval,
            InverseTrigCompositionKind::Identity,
        )),
        (Function::Cos, Function::Asin) | (Function::Sin, Function::Acos) => Some((
            InverseTrigCompositionDomain::UnitInterval,
            InverseTrigCompositionKind::UnitCofunction,
        )),
        (Function::Tan, Function::Atan) => Some((
            InverseTrigCompositionDomain::RealLine,
            InverseTrigCompositionKind::Identity,
        )),
        _ => None,
    }
}

fn rational_zero_as_pi_coefficient(
    dag: &ExactExpressionDag,
    argument: ExprId,
) -> Result<Rational, EvaluationError> {
    match evaluate_node(dag, argument) {
        Ok(value) if value.value().is_zero() => Ok(Rational::zero()),
        Ok(_) => Err(unsupported_function_evaluation()),
        Err(error) if is_unsupported_exact_expression(&error) => {
            Err(unsupported_function_evaluation())
        }
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy)]
struct RationalParts {
    numerator: i64,
    denominator: i64,
}

#[derive(Clone, Copy)]
struct RationalSpecialAngleEntry {
    phase: RationalParts,
    value: RationalParts,
}

#[derive(Clone, Copy)]
struct RadicalTermParts {
    coefficient: RationalParts,
    radicand: i64,
}

#[derive(Clone, Copy)]
struct RadicalSpecialAngleEntry {
    phase: RationalParts,
    rational: RationalParts,
    radicals: &'static [RadicalTermParts],
}

const SQRT_2_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(1, 2, 2)];
const NEG_SQRT_2_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(-1, 2, 2)];
const SQRT_3_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(1, 2, 3)];
const NEG_SQRT_3_OVER_2: [RadicalTermParts; 1] = [radical_term_parts(-1, 2, 3)];
const SQRT_3: [RadicalTermParts; 1] = [radical_term_parts(1, 1, 3)];
const NEG_SQRT_3: [RadicalTermParts; 1] = [radical_term_parts(-1, 1, 3)];
const SQRT_3_OVER_3: [RadicalTermParts; 1] = [radical_term_parts(1, 3, 3)];
const NEG_SQRT_3_OVER_3: [RadicalTermParts; 1] = [radical_term_parts(-1, 3, 3)];
const SQRT_6_MINUS_SQRT_2_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(-1, 4, 2), radical_term_parts(1, 4, 6)];
const SQRT_2_PLUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(1, 4, 2), radical_term_parts(1, 4, 6)];
const SQRT_2_MINUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(1, 4, 2), radical_term_parts(-1, 4, 6)];
const NEG_SQRT_2_MINUS_SQRT_6_OVER_4: [RadicalTermParts; 2] =
    [radical_term_parts(-1, 4, 2), radical_term_parts(-1, 4, 6)];

const SINE_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 0, 1),
    rational_angle_entry(1, 1, 0, 1),
    rational_angle_entry(1, 2, 1, 1),
    rational_angle_entry(3, 2, -1, 1),
    rational_angle_entry(1, 6, 1, 2),
    rational_angle_entry(5, 6, 1, 2),
    rational_angle_entry(7, 6, -1, 2),
    rational_angle_entry(11, 6, -1, 2),
];

const COSINE_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 1, 1),
    rational_angle_entry(1, 1, -1, 1),
    rational_angle_entry(1, 2, 0, 1),
    rational_angle_entry(3, 2, 0, 1),
    rational_angle_entry(1, 3, 1, 2),
    rational_angle_entry(5, 3, 1, 2),
    rational_angle_entry(2, 3, -1, 2),
    rational_angle_entry(4, 3, -1, 2),
];

const TANGENT_RATIONAL_SPECIAL_ANGLES: &[RationalSpecialAngleEntry] = &[
    rational_angle_entry(0, 1, 0, 1),
    rational_angle_entry(1, 4, 1, 1),
    rational_angle_entry(3, 4, -1, 1),
];

const TANGENT_POLE_PHASES: &[RationalParts] = &[rational_parts(1, 2)];

const SINE_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(3, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(5, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(7, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(1, 3, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(2, 3, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(4, 3, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(5, 3, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(1, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(11, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(5, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(7, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(13, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(23, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(17, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(19, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
];

const COSINE_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(7, 4, 0, 1, &SQRT_2_OVER_2),
    radical_angle_entry(3, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(5, 4, 0, 1, &NEG_SQRT_2_OVER_2),
    radical_angle_entry(1, 6, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(11, 6, 0, 1, &SQRT_3_OVER_2),
    radical_angle_entry(5, 6, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(7, 6, 0, 1, &NEG_SQRT_3_OVER_2),
    radical_angle_entry(1, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(23, 12, 0, 1, &SQRT_2_PLUS_SQRT_6_OVER_4),
    radical_angle_entry(5, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(19, 12, 0, 1, &SQRT_6_MINUS_SQRT_2_OVER_4),
    radical_angle_entry(7, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(17, 12, 0, 1, &SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(11, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
    radical_angle_entry(13, 12, 0, 1, &NEG_SQRT_2_MINUS_SQRT_6_OVER_4),
];

const TANGENT_RADICAL_SPECIAL_ANGLES: &[RadicalSpecialAngleEntry] = &[
    radical_angle_entry(1, 3, 0, 1, &SQRT_3),
    radical_angle_entry(2, 3, 0, 1, &NEG_SQRT_3),
    radical_angle_entry(1, 6, 0, 1, &SQRT_3_OVER_3),
    radical_angle_entry(5, 6, 0, 1, &NEG_SQRT_3_OVER_3),
    radical_angle_entry(1, 12, 2, 1, &NEG_SQRT_3),
    radical_angle_entry(5, 12, 2, 1, &SQRT_3),
    radical_angle_entry(7, 12, -2, 1, &NEG_SQRT_3),
    radical_angle_entry(11, 12, -2, 1, &SQRT_3),
];

const fn rational_parts(numerator: i64, denominator: i64) -> RationalParts {
    RationalParts {
        numerator,
        denominator,
    }
}

const fn rational_angle_entry(
    phase_numerator: i64,
    phase_denominator: i64,
    value_numerator: i64,
    value_denominator: i64,
) -> RationalSpecialAngleEntry {
    RationalSpecialAngleEntry {
        phase: rational_parts(phase_numerator, phase_denominator),
        value: rational_parts(value_numerator, value_denominator),
    }
}

const fn radical_term_parts(
    coefficient_numerator: i64,
    coefficient_denominator: i64,
    radicand: i64,
) -> RadicalTermParts {
    RadicalTermParts {
        coefficient: rational_parts(coefficient_numerator, coefficient_denominator),
        radicand,
    }
}

const fn radical_angle_entry(
    phase_numerator: i64,
    phase_denominator: i64,
    rational_numerator: i64,
    rational_denominator: i64,
    radicals: &'static [RadicalTermParts],
) -> RadicalSpecialAngleEntry {
    RadicalSpecialAngleEntry {
        phase: rational_parts(phase_numerator, phase_denominator),
        rational: rational_parts(rational_numerator, rational_denominator),
        radicals,
    }
}

fn sine_special_angle(coefficient: &Rational) -> Option<Rational> {
    lookup_rational_special_angle(coefficient, 2, SINE_RATIONAL_SPECIAL_ANGLES)
}

fn cosine_special_angle(coefficient: &Rational) -> Option<Rational> {
    lookup_rational_special_angle(coefficient, 2, COSINE_RATIONAL_SPECIAL_ANGLES)
}

fn tangent_special_angle(coefficient: &Rational) -> Result<Option<Rational>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else {
        Ok(lookup_rational_phase(
            &phase,
            TANGENT_RATIONAL_SPECIAL_ANGLES,
        ))
    }
}

fn sine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    lookup_radical_special_angle(coefficient, 2, SINE_RADICAL_SPECIAL_ANGLES)
}

fn cosine_radical_special_angle(coefficient: &Rational) -> Option<RadicalReduction> {
    lookup_radical_special_angle(coefficient, 2, COSINE_RADICAL_SPECIAL_ANGLES)
}

fn tangent_radical_special_angle(
    coefficient: &Rational,
) -> Result<Option<RadicalReduction>, EvaluationError> {
    let phase = coefficient.modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        Err(domain_error(DomainErrorKind::TangentPole))
    } else {
        Ok(lookup_radical_phase(&phase, TANGENT_RADICAL_SPECIAL_ANGLES))
    }
}

fn lookup_rational_special_angle(
    coefficient: &Rational,
    period: u32,
    entries: &[RationalSpecialAngleEntry],
) -> Option<Rational> {
    let phase = coefficient.modulo_integer(period);
    lookup_rational_phase(&phase, entries)
}

fn lookup_rational_phase(
    phase: &Rational,
    entries: &[RationalSpecialAngleEntry],
) -> Option<Rational> {
    entries
        .iter()
        .find(|entry| phase_matches(phase, entry.phase))
        .map(|entry| rational_from_parts(entry.value))
}

fn lookup_radical_special_angle(
    coefficient: &Rational,
    period: u32,
    entries: &[RadicalSpecialAngleEntry],
) -> Option<RadicalReduction> {
    let phase = coefficient.modulo_integer(period);
    lookup_radical_phase(&phase, entries)
}

fn lookup_radical_phase(
    phase: &Rational,
    entries: &[RadicalSpecialAngleEntry],
) -> Option<RadicalReduction> {
    entries
        .iter()
        .find(|entry| phase_matches(phase, entry.phase))
        .map(radical_reduction_from_parts)
}

fn phase_matches_any(phase: &Rational, entries: &[RationalParts]) -> bool {
    entries.iter().any(|entry| phase_matches(phase, *entry))
}

fn phase_matches(phase: &Rational, expected: RationalParts) -> bool {
    phase == &rational_from_parts(expected)
}

fn rational_from_parts(parts: RationalParts) -> Rational {
    rational(parts.numerator, parts.denominator)
}

fn radical_reduction_from_parts(entry: &RadicalSpecialAngleEntry) -> RadicalReduction {
    let radicals = entry
        .radicals
        .iter()
        .map(|radical| simple_radical(rational_from_parts(radical.coefficient), radical.radicand))
        .collect();
    special_angle_linear_combination(rational_from_parts(entry.rational), radicals)
}

fn special_angle_linear_combination(
    rational: Rational,
    radicals: Vec<SimpleRadical>,
) -> RadicalReduction {
    radical_linear_combination_reduction(rational, radicals, true)
}

fn evaluate_inverse_trigonometric_function(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<RationalEvaluation, EvaluationError> {
    let Some(coefficient) = evaluate_inverse_trig_pi_coefficient(dag, function, argument)? else {
        return Err(unsupported_function_evaluation());
    };
    match dag.semantics().angle_unit {
        AngleUnit::Radian if coefficient.coefficient().is_zero() => {
            Ok(RationalEvaluation::special_angle(Rational::zero()))
        }
        AngleUnit::Radian => Err(unsupported_function_evaluation()),
        AngleUnit::Degree => Ok(RationalEvaluation::special_angle(
            coefficient.coefficient().multiply(&rational_integer(180)),
        )),
        AngleUnit::Gradian => Ok(RationalEvaluation::special_angle(
            coefficient.coefficient().multiply(&rational_integer(200)),
        )),
    }
}

fn evaluate_inverse_trig_pi_coefficient(
    dag: &ExactExpressionDag,
    function: Function,
    argument: ExprId,
) -> Result<Option<PiCoefficientEvaluation>, EvaluationError> {
    let reduction = match evaluate_node(dag, argument) {
        Ok(value) => RadicalReduction::Rational(value),
        Err(error) if is_unsupported_exact_expression(&error) => {
            let Some(reduction) = evaluate_radical_node(dag, argument)? else {
                return Ok(None);
            };
            reduction
        }
        Err(error) => return Err(error),
    };
    let coefficient = match function {
        Function::Asin => asin_known_pi_coefficient(&reduction)?,
        Function::Acos => acos_known_pi_coefficient(&reduction)?,
        Function::Atan => atan_known_pi_coefficient(&reduction),
        Function::Sin
        | Function::Cos
        | Function::Tan
        | Function::Sqrt
        | Function::Root
        | Function::Exp
        | Function::Log
        | Function::Ln
        | Function::Abs
        | Function::Floor
        | Function::Factorial
        | Function::Permutation
        | Function::Combination
        | Function::Modulo
        | Function::Gcd
        | Function::Lcm
        | Function::Sinh
        | Function::Cosh
        | Function::Tanh
        | Function::Asinh
        | Function::Acosh
        | Function::Atanh => {
            return Ok(None);
        }
    };
    Ok(coefficient.map(PiCoefficientEvaluation::special_angle))
}

fn asin_known_pi_coefficient(
    argument: &RadicalReduction,
) -> Result<Option<Rational>, EvaluationError> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            ensure_inverse_sine_cosine_rational_domain(argument)?;
            if *argument == rational_integer(-1) {
                Ok(Some(rational(-1, 2)))
            } else if *argument == rational(-1, 2) {
                Ok(Some(rational(-1, 6)))
            } else if argument.is_zero() {
                Ok(Some(Rational::zero()))
            } else if *argument == rational(1, 2) {
                Ok(Some(rational(1, 6)))
            } else if *argument == Rational::one() {
                Ok(Some(rational(1, 2)))
            } else {
                Ok(None)
            }
        }
        RadicalReduction::Radical(argument) => {
            ensure_inverse_sine_cosine_radical_domain(argument.value())?;
            Ok(asin_radical_pi_coefficient(argument.value()))
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn acos_known_pi_coefficient(
    argument: &RadicalReduction,
) -> Result<Option<Rational>, EvaluationError> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            ensure_inverse_sine_cosine_rational_domain(argument)?;
            if *argument == rational_integer(-1) {
                Ok(Some(Rational::one()))
            } else if *argument == rational(-1, 2) {
                Ok(Some(rational(2, 3)))
            } else if argument.is_zero() {
                Ok(Some(rational(1, 2)))
            } else if *argument == rational(1, 2) {
                Ok(Some(rational(1, 3)))
            } else if *argument == Rational::one() {
                Ok(Some(Rational::zero()))
            } else {
                Ok(None)
            }
        }
        RadicalReduction::Radical(argument) => {
            ensure_inverse_sine_cosine_radical_domain(argument.value())?;
            Ok(acos_radical_pi_coefficient(argument.value()))
        }
        RadicalReduction::LinearCombination(_) => Ok(None),
    }
}

fn atan_known_pi_coefficient(argument: &RadicalReduction) -> Option<Rational> {
    match argument {
        RadicalReduction::Rational(argument) => {
            let argument = argument.value();
            if *argument == rational_integer(-1) {
                Some(rational(-1, 4))
            } else if argument.is_zero() {
                Some(Rational::zero())
            } else if *argument == Rational::one() {
                Some(rational(1, 4))
            } else {
                None
            }
        }
        RadicalReduction::Radical(argument) => atan_radical_pi_coefficient(argument.value()),
        RadicalReduction::LinearCombination(_) => None,
    }
}

fn asin_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if is_simple_radical(argument, rational(1, 2), 2) {
        Some(rational(1, 4))
    } else if is_simple_radical(argument, rational(-1, 2), 2) {
        Some(rational(-1, 4))
    } else if is_simple_radical(argument, rational(1, 2), 3) {
        Some(rational(1, 3))
    } else if is_simple_radical(argument, rational(-1, 2), 3) {
        Some(rational(-1, 3))
    } else {
        None
    }
}

fn acos_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if is_simple_radical(argument, rational(1, 2), 3) {
        Some(rational(1, 6))
    } else if is_simple_radical(argument, rational(-1, 2), 3) {
        Some(rational(5, 6))
    } else if is_simple_radical(argument, rational(1, 2), 2) {
        Some(rational(1, 4))
    } else if is_simple_radical(argument, rational(-1, 2), 2) {
        Some(rational(3, 4))
    } else {
        None
    }
}

fn atan_radical_pi_coefficient(argument: &SimpleRadical) -> Option<Rational> {
    if !has_radicand(argument, 3) {
        return None;
    }
    if argument.coefficient == Rational::one() {
        Some(rational(1, 3))
    } else if argument.coefficient == rational_integer(-1) {
        Some(rational(-1, 3))
    } else if argument.coefficient == rational(1, 3) {
        Some(rational(1, 6))
    } else if argument.coefficient == rational(-1, 3) {
        Some(rational(-1, 6))
    } else {
        None
    }
}

fn is_simple_radical(argument: &SimpleRadical, coefficient: Rational, radicand: i64) -> bool {
    argument.coefficient == coefficient && has_radicand(argument, radicand)
}

fn has_radicand(argument: &SimpleRadical, radicand: i64) -> bool {
    argument.radicand.inner == Integer::from(radicand)
}

fn ensure_inverse_sine_cosine_rational_domain(argument: &Rational) -> Result<(), EvaluationError> {
    match rational_unit_interval_domain_proof(argument) {
        UnitIntervalDomainProof::InRange => Ok(()),
        UnitIntervalDomainProof::OutOfRange => Err(inverse_trig_out_of_range_error()),
        UnitIntervalDomainProof::Unknown => Ok(()),
    }
}

fn ensure_inverse_sine_cosine_radical_domain(
    argument: &SimpleRadical,
) -> Result<(), EvaluationError> {
    let squared = argument
        .coefficient
        .multiply(&argument.coefficient)
        .multiply(&rational_from_positive_integer(&argument.radicand));
    if squared.compare(&Rational::one()) == Ordering::Greater {
        Err(domain_error(
            DomainErrorKind::InverseTrigonometricOutOfRange,
        ))
    } else {
        Ok(())
    }
}

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("hard-coded rational constants have non-zero denominators")
}

fn rational_integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn simple_radical(coefficient: Rational, radicand: i64) -> SimpleRadical {
    SimpleRadical {
        coefficient,
        radicand: PositiveInteger::new(Integer::from(radicand))
            .expect("hard-coded radical radicands are positive"),
    }
}

fn unsupported_function_evaluation() -> EvaluationError {
    EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
        feature: UnsupportedFeature::FunctionEvaluation,
    })
}

fn non_integer_power_error() -> EvaluationError {
    EvaluationError::UnsupportedFeature(UnsupportedFeatureError {
        feature: UnsupportedFeature::NonIntegerPower,
    })
}

fn exponent_too_large_error() -> EvaluationError {
    logical_work_limit_error()
}

fn logical_work_limit_error() -> EvaluationError {
    EvaluationError::ComputationLimit(ComputationLimitError {
        kind: ComputationLimitKind::LogicalWorkUnits,
    })
}

fn logarithm_of_non_positive_error() -> EvaluationError {
    domain_error(DomainErrorKind::LogarithmOfNonPositive)
}

fn inverse_trig_out_of_range_error() -> EvaluationError {
    domain_error(DomainErrorKind::InverseTrigonometricOutOfRange)
}

fn domain_error(kind: DomainErrorKind) -> EvaluationError {
    EvaluationError::Domain(DomainError { kind, span: None })
}

fn invalid_algebraic_isolation_error() -> EvaluationError {
    EvaluationError::InternalInvariant(InternalInvariantError {
        code: InternalInvariantCode::InvalidAlgebraicIsolation,
    })
}

fn real_algebraic_construction_error(error: RealAlgebraicConstructionError) -> EvaluationError {
    match error {
        RealAlgebraicConstructionError::RootCounting(
            PrimitivePolynomialRootCountingError::CountOverflow,
        )
        | RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::CountOverflow,
        ) => EvaluationError::ComputationLimit(ComputationLimitError {
            kind: ComputationLimitKind::RootIsolationSteps,
        }),
        RealAlgebraicConstructionError::ConstantPolynomial
        | RealAlgebraicConstructionError::InvalidInterval
        | RealAlgebraicConstructionError::EndpointRoot
        | RealAlgebraicConstructionError::NonIsolatingInterval
        | RealAlgebraicConstructionError::PolynomialConstruction(_)
        | RealAlgebraicConstructionError::PolynomialResultant(_)
        | RealAlgebraicConstructionError::NoMatchingPolynomialFactor
        | RealAlgebraicConstructionError::RootIndexOverflow
        | RealAlgebraicConstructionError::RootIndexNotFound
        | RealAlgebraicConstructionError::RootCounting(
            PrimitivePolynomialRootCountingError::ZeroPolynomial
            | PrimitivePolynomialRootCountingError::InvalidInterval,
        )
        | RealAlgebraicConstructionError::RootIsolation(
            PrimitivePolynomialRootIsolationError::ZeroPolynomial
            | PrimitivePolynomialRootIsolationError::StepLimitExceeded,
        ) => invalid_algebraic_isolation_error(),
    }
}

fn evaluate_interval_node(
    dag: &ExactExpressionDag,
    id: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    evaluate_interval_expression_node(dag, dag.node(id), Some(id), precision_bits)
}

fn evaluate_interval_expression_node(
    dag: &ExactExpressionDag,
    node: &ExpressionNode,
    id: Option<ExprId>,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match node {
        ExpressionNode::Rational(id) => {
            Ok(interval::from_rational(dag.rational(*id), precision_bits))
        }
        ExpressionNode::Exact(value) => {
            let reduction = dag.exact_value(*value);
            if matches!(reduction, ExactReduction::Symbolic) {
                return evaluate_interval_expression_node(
                    dag,
                    dag.exact_presentation(*value),
                    None,
                    precision_bits,
                );
            }
            let exact = evaluate_exact_reduction_interval(reduction, precision_bits)?;
            if !matches!(
                reduction,
                ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(_))
            ) {
                return Ok(exact);
            }
            let presentation = match evaluate_interval_expression_node(
                dag,
                dag.exact_presentation(*value),
                None,
                precision_bits,
            ) {
                Ok(value) => value,
                Err(IntervalError::UnsupportedExpression) => return Ok(exact),
                Err(error) => return Err(error),
            };
            interval::intersect(&exact, &presentation, precision_bits)
        }
        ExpressionNode::Constant(value) => interval::constant(*value, precision_bits),
        ExpressionNode::Add(list_id) => {
            let mut children = dag.list(*list_id).iter();
            let Some(first) = children.next() else {
                return Ok(interval::from_rational(&Rational::zero(), precision_bits));
            };
            let mut total = evaluate_interval_node(dag, *first, precision_bits)?;
            for child in children {
                total = interval::add(
                    &total,
                    &evaluate_interval_node(dag, *child, precision_bits)?,
                )?;
            }
            Ok(total)
        }
        ExpressionNode::Multiply(list_id) => {
            let mut children = dag.list(*list_id).iter();
            let Some(first) = children.next() else {
                return Ok(interval::from_rational(&Rational::one(), precision_bits));
            };
            let mut product = evaluate_interval_node(dag, *first, precision_bits)?;
            for child in children {
                product = interval::multiply(
                    &product,
                    &evaluate_interval_node(dag, *child, precision_bits)?,
                )?;
            }
            Ok(product)
        }
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => interval::divide(
            &evaluate_interval_node(dag, *numerator, precision_bits)?,
            &evaluate_interval_node(dag, *denominator, precision_bits)?,
            precision_bits,
        ),
        ExpressionNode::Power { base, exponent } => {
            evaluate_interval_power(dag, *base, *exponent, precision_bits)
        }
        ExpressionNode::LogBase { argument, base } => {
            evaluate_interval_log_base(dag, *argument, *base, precision_bits)
        }
        ExpressionNode::Function { function, argument } => match function {
            Function::Sqrt => interval::sqrt(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Exp => interval::exp(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Log | Function::Ln => interval::log(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Sin => sine_interval(dag, *argument, precision_bits),
            Function::Cos => cosine_interval(dag, *argument, precision_bits),
            Function::Tan => tangent_interval(dag, *argument, precision_bits),
            Function::Atan => interval::atan(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Asin => interval::asin(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Acos => interval::acos(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Abs => interval::absolute(
                &evaluate_interval_node(dag, *argument, precision_bits)?,
                precision_bits,
            ),
            Function::Floor
            | Function::Factorial
            | Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => {
                let Some(id) = id else {
                    return Err(IntervalError::UnsupportedExpression);
                };
                evaluate_node(dag, id)
                    .map(|value| interval::from_rational(value.value(), precision_bits))
                    .map_err(evaluation_error_to_interval_error)
            }
        },
        ExpressionNode::BinaryFunction { .. } => {
            let Some(id) = id else {
                return Err(IntervalError::UnsupportedExpression);
            };
            evaluate_node(dag, id)
                .map(|value| interval::from_rational(value.value(), precision_bits))
                .map_err(evaluation_error_to_interval_error)
        }
    }
}

fn evaluate_interval_log_base(
    dag: &ExactExpressionDag,
    argument: ExprId,
    base: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    if let Ok(base) = evaluate_node(dag, base) {
        ensure_log_base_domain(base.value()).map_err(|error| match error {
            EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
            EvaluationError::InputLimit(_)
            | EvaluationError::ComputationLimit(_)
            | EvaluationError::UnsupportedFeature(_)
            | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
        })?;
    }
    let base = evaluate_interval_node(dag, base, precision_bits)?;
    if base == interval::from_rational(&Rational::one(), precision_bits) {
        return Err(IntervalError::Domain(DomainErrorKind::LogarithmBaseOne));
    }
    interval::divide(
        &interval::log(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        )?,
        &interval::log(&base, precision_bits)?,
        precision_bits,
    )
}

fn evaluate_interval_power(
    dag: &ExactExpressionDag,
    base: ExprId,
    exponent: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let exponent = match evaluate_node(dag, exponent) {
        Ok(exponent) => exponent,
        Err(error) if is_unsupported_exact_expression(&error) => {
            return interval::pow_positive_base(
                &evaluate_interval_node(dag, base, precision_bits)?,
                &evaluate_interval_node(dag, exponent, precision_bits)?,
                precision_bits,
            );
        }
        Err(error) => return Err(evaluation_error_to_interval_error(error)),
    };

    if let Some(exponent_integer) = exponent.value().as_i64_if_integer() {
        return interval::pow_i64(
            &evaluate_interval_node(dag, base, precision_bits)?,
            exponent_integer,
            precision_bits,
        );
    }

    match evaluate_node(dag, base) {
        Ok(base) => interval::pow_rational(base.value(), exponent.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::pow_positive_base(
            &evaluate_interval_node(dag, base, precision_bits)?,
            &interval::from_rational(exponent.value(), precision_bits),
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn tangent_rational_pi_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    let coefficient = evaluate_pi_coefficient(dag, argument).map_err(|error| match error {
        EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
        EvaluationError::InputLimit(_)
        | EvaluationError::ComputationLimit(_)
        | EvaluationError::UnsupportedFeature(_)
        | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
    })?;
    let Some(coefficient) = coefficient else {
        return Err(IntervalError::UnsupportedExpression);
    };

    let phase = coefficient.coefficient().modulo_integer(1);
    if phase_matches_any(&phase, TANGENT_POLE_PHASES) {
        return Err(IntervalError::Domain(DomainErrorKind::TangentPole));
    }

    let mut distance_to_pole = phase.subtract(&rational(1, 2));
    if distance_to_pole.is_negative() {
        distance_to_pole = distance_to_pole.negate();
    }
    if distance_to_pole.is_zero() {
        return Err(IntervalError::Domain(DomainErrorKind::TangentPole));
    }
    let denominator = distance_to_pole.multiply(&rational_integer(2));
    let bound = Rational::one()
        .divide(&denominator)
        .map_err(|_| IntervalError::DivisionByIntervalContainingZero)?;
    interval::from_rational_bounds(&bound.negate(), &bound, precision_bits)
}

fn tangent_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match tangent_rational_pi_interval(dag, argument, precision_bits) {
        Ok(interval) => Ok(interval),
        Err(IntervalError::UnsupportedExpression) => match evaluate_node(dag, argument) {
            Ok(argument) => interval::tan_rational(argument.value(), precision_bits),
            Err(error) if is_unsupported_exact_expression(&error) => interval::tan(
                &evaluate_interval_node(dag, argument, precision_bits)?,
                precision_bits,
            ),
            Err(error) => Err(evaluation_error_to_interval_error(error)),
        },
        Err(error) => Err(error),
    }
}

fn sine_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match evaluate_node(dag, argument) {
        Ok(argument) => interval::sin_rational(argument.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::sin(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn cosine_interval(
    dag: &ExactExpressionDag,
    argument: ExprId,
    precision_bits: u32,
) -> Result<CertifiedInterval, IntervalError> {
    match evaluate_node(dag, argument) {
        Ok(argument) => interval::cos_rational(argument.value(), precision_bits),
        Err(error) if is_unsupported_exact_expression(&error) => interval::cos(
            &evaluate_interval_node(dag, argument, precision_bits)?,
            precision_bits,
        ),
        Err(error) => Err(evaluation_error_to_interval_error(error)),
    }
}

fn evaluation_error_to_interval_error(error: EvaluationError) -> IntervalError {
    match error {
        EvaluationError::Domain(DomainError { kind, .. }) => IntervalError::Domain(kind),
        EvaluationError::ComputationLimit(_)
        | EvaluationError::InputLimit(_)
        | EvaluationError::UnsupportedFeature(_)
        | EvaluationError::InternalInvariant(_) => IntervalError::UnsupportedExpression,
    }
}

fn evaluation_error_from_interval(error: IntervalError) -> EvaluationError {
    match error {
        IntervalError::Domain(kind) => domain_error(kind),
        IntervalError::ExponentTooLarge => exponent_too_large_error(),
        IntervalError::InvalidBounds
        | IntervalError::UnsupportedExpression
        | IntervalError::DivisionByIntervalContainingZero => unsupported_function_evaluation(),
    }
}

fn arithmetic_error(error: RationalArithmeticError) -> EvaluationError {
    match error {
        RationalArithmeticError::DivisionByZero => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::DivisionByZero,
            span: None,
        }),
        RationalArithmeticError::ZeroToNegativePower => EvaluationError::Domain(DomainError {
            kind: DomainErrorKind::ZeroToNegativePower,
            span: None,
        }),
        RationalArithmeticError::ExponentTooLarge => {
            EvaluationError::ComputationLimit(ComputationLimitError {
                kind: ComputationLimitKind::LogicalWorkUnits,
            })
        }
    }
}

impl DagBuilder {
    fn lower(&mut self, expression: &SourceExpr) -> Result<ExprId, EvaluationError> {
        match expression {
            SourceExpr::Number { literal, .. } => {
                let rational = Rational::from_decimal_literal(literal)
                    .map_err(decimal_literal_error_to_evaluation_error)?;
                Ok(self.push_rational(rational))
            }
            SourceExpr::Constant { constant, .. } => {
                Ok(self.push_node(ExpressionNode::Constant(*constant)))
            }
            SourceExpr::Unary { op, expr, .. } => match op {
                UnaryOperator::Plus => self.lower(expr),
                UnaryOperator::Negate => {
                    let value = self.lower(expr)?;
                    Ok(self.negate(value))
                }
            },
            SourceExpr::Binary {
                op, left, right, ..
            } => {
                if matches!(op, BinaryOperator::Add | BinaryOperator::Subtract) {
                    let mut terms = Vec::new();
                    let mut constant = Rational::zero();
                    self.lower_additive_terms(expression, false, &mut terms, &mut constant)?;
                    return Ok(self.add_many_with_constant(terms, constant));
                }
                if *op == BinaryOperator::Multiply {
                    let mut factors = Vec::new();
                    let mut coefficient = Rational::one();
                    self.lower_multiplicative_factors(expression, &mut factors, &mut coefficient)?;
                    return Ok(self.multiply_many_with_coefficient(factors, coefficient));
                }
                let left = self.lower(left)?;
                let right = self.lower(right)?;
                Ok(match op {
                    BinaryOperator::Multiply => self.multiply(left, right),
                    BinaryOperator::Divide => self.divide(left, right),
                    BinaryOperator::Power => self.lower_power(left, right),
                    BinaryOperator::Add | BinaryOperator::Subtract => unreachable!(),
                })
            }
            SourceExpr::Percent { expr, .. } => {
                let numerator = self.lower(expr)?;
                let denominator = self.push_rational(Rational::from_integer(Integer::from(100)));
                Ok(self.divide(numerator, denominator))
            }
            SourceExpr::Function {
                function,
                argument,
                base,
                ..
            } => {
                let argument = self.lower(argument)?;
                let argument = self.lower_function_argument(*function, argument);
                if let Some(base) = base {
                    let base = self.lower(base)?;
                    return self.lower_binary_function(*function, argument, base);
                }
                let function = match function {
                    Function::Ln => Function::Log,
                    function @ (Function::Sin
                    | Function::Cos
                    | Function::Tan
                    | Function::Asin
                    | Function::Acos
                    | Function::Atan
                    | Function::Sqrt
                    | Function::Abs
                    | Function::Floor
                    | Function::Factorial
                    | Function::Exp
                    | Function::Log) => *function,
                    Function::Sinh
                    | Function::Cosh
                    | Function::Tanh
                    | Function::Asinh
                    | Function::Acosh
                    | Function::Atanh => {
                        return Ok(self.lower_hyperbolic_function(*function, argument));
                    }
                    Function::Root
                    | Function::Permutation
                    | Function::Combination
                    | Function::Modulo
                    | Function::Gcd
                    | Function::Lcm => *function,
                };
                Ok(if function == Function::Sqrt {
                    self.sqrt(argument)
                } else {
                    self.push_node(ExpressionNode::Function { function, argument })
                })
            }
        }
    }

    fn lower_multiplicative_factors(
        &mut self,
        expression: &SourceExpr,
        factors: &mut Vec<ExprId>,
        coefficient: &mut Rational,
    ) -> Result<(), EvaluationError> {
        if let Some((literal, negative)) = additive_numeric_literal(expression, false) {
            let value = Rational::from_decimal_literal(literal)
                .map_err(decimal_literal_error_to_evaluation_error)?;
            let value = if negative { value.negate() } else { value };
            let work = rational_multiply_work(coefficient, &value);
            if self.canonical_limit_reached.is_none()
                && self.canonical_budget.reserve(0, work).is_ok()
            {
                *coefficient = coefficient.multiply(&value);
            } else {
                if self.canonical_limit_reached.is_none() {
                    self.canonical_limit_reached
                        .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
                }
                self.multiplicative_literal_fold_limit_reached = true;
                factors.push(self.push_rational(value));
            }
            return Ok(());
        }
        match expression {
            SourceExpr::Binary {
                op: BinaryOperator::Multiply,
                left,
                right,
                ..
            } => {
                self.lower_multiplicative_factors(left, factors, coefficient)?;
                self.lower_multiplicative_factors(right, factors, coefficient)
            }
            SourceExpr::Number { .. }
            | SourceExpr::Constant { .. }
            | SourceExpr::Unary { .. }
            | SourceExpr::Binary { .. }
            | SourceExpr::Percent { .. }
            | SourceExpr::Function { .. } => {
                factors.push(self.lower(expression)?);
                Ok(())
            }
        }
    }

    fn lower_additive_terms(
        &mut self,
        expression: &SourceExpr,
        negative: bool,
        terms: &mut Vec<ExprId>,
        constant: &mut Rational,
    ) -> Result<(), EvaluationError> {
        if let Some((literal, negative)) = additive_numeric_literal(expression, negative) {
            let value = Rational::from_decimal_literal(literal)
                .map_err(decimal_literal_error_to_evaluation_error)?;
            let value = if negative { value.negate() } else { value };
            let work = rational_add_work(constant, &value);
            if self.canonical_limit_reached.is_none()
                && self.canonical_budget.reserve(0, work).is_ok()
            {
                *constant = constant.add(&value);
            } else {
                if self.canonical_limit_reached.is_none() {
                    self.canonical_limit_reached
                        .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
                }
                self.additive_literal_fold_limit_reached = true;
                terms.push(self.push_rational(value));
            }
            return Ok(());
        }
        match expression {
            SourceExpr::Binary {
                op: BinaryOperator::Add,
                left,
                right,
                ..
            } => {
                self.lower_additive_terms(left, negative, terms, constant)?;
                self.lower_additive_terms(right, negative, terms, constant)
            }
            SourceExpr::Binary {
                op: BinaryOperator::Subtract,
                left,
                right,
                ..
            } => {
                self.lower_additive_terms(left, negative, terms, constant)?;
                self.lower_additive_terms(right, !negative, terms, constant)
            }
            SourceExpr::Number { .. }
            | SourceExpr::Constant { .. }
            | SourceExpr::Unary { .. }
            | SourceExpr::Binary { .. }
            | SourceExpr::Percent { .. }
            | SourceExpr::Function { .. } => {
                let term = self.lower(expression)?;
                terms.push(if negative { self.negate(term) } else { term });
                Ok(())
            }
        }
    }

    fn lower_binary_function(
        &mut self,
        function: Function,
        argument: ExprId,
        base: ExprId,
    ) -> Result<ExprId, EvaluationError> {
        match function {
            Function::Exp if self.is_euler_constant(base) => {
                Ok(self.push_node(ExpressionNode::Function {
                    function: Function::Exp,
                    argument,
                }))
            }
            Function::Exp => Ok(self.power(base, argument)),
            Function::Log if self.is_euler_constant(base) => {
                Ok(self.push_node(ExpressionNode::Function {
                    function: Function::Log,
                    argument,
                }))
            }
            Function::Log => Ok(self.push_node(ExpressionNode::LogBase { argument, base })),
            Function::Root => {
                let one = self.push_rational(Rational::one());
                let exponent = self.divide(one, base);
                Ok(self.power(argument, exponent))
            }
            Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm => Ok(self.push_node(ExpressionNode::BinaryFunction {
                function,
                left: argument,
                right: base,
            })),
            Function::Ln
            | Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => {
                Ok(self.push_node(ExpressionNode::Function { function, argument }))
            }
        }
    }

    fn lower_power(&mut self, base: ExprId, exponent: ExprId) -> ExprId {
        if self.is_euler_constant(base) {
            self.exp(exponent)
        } else {
            self.power(base, exponent)
        }
    }

    fn lower_hyperbolic_function(&mut self, function: Function, argument: ExprId) -> ExprId {
        match function {
            Function::Sinh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.subtract(exp_positive, exp_negative);
                let two = self.integer(2);
                self.divide(numerator, two)
            }
            Function::Cosh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.add(exp_positive, exp_negative);
                let two = self.integer(2);
                self.divide(numerator, two)
            }
            Function::Tanh => {
                let exp_positive = self.exp(argument);
                let negative_argument = self.negate(argument);
                let exp_negative = self.exp(negative_argument);
                let numerator = self.subtract(exp_positive, exp_negative);
                let denominator = self.add(exp_positive, exp_negative);
                self.divide(numerator, denominator)
            }
            Function::Asinh => {
                let two = self.integer(2);
                let square = self.power(argument, two);
                let one = self.integer(1);
                let under_root = self.add(square, one);
                let root = self.sqrt(under_root);
                let logarithm_argument = self.add(argument, root);
                self.log(logarithm_argument)
            }
            Function::Acosh => {
                let two = self.integer(2);
                let square = self.power(argument, two);
                let one = self.integer(1);
                let under_root = self.subtract(square, one);
                let root = self.sqrt(under_root);
                let logarithm_argument = self.add(argument, root);
                self.log(logarithm_argument)
            }
            Function::Atanh => {
                let numerator_one = self.integer(1);
                let numerator = self.add(numerator_one, argument);
                let denominator_one = self.integer(1);
                let denominator = self.subtract(denominator_one, argument);
                let quotient = self.divide(numerator, denominator);
                let logarithm = self.log(quotient);
                let two = self.integer(2);
                self.divide(logarithm, two)
            }
            Function::Sin
            | Function::Cos
            | Function::Tan
            | Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm => self.push_node(ExpressionNode::Function { function, argument }),
        }
    }

    fn is_euler_constant(&self, id: ExprId) -> bool {
        matches!(
            self.nodes[id.0 as usize],
            ExpressionNode::Constant(Constant::Euler)
        )
    }

    fn lower_function_argument(&mut self, function: Function, argument: ExprId) -> ExprId {
        match function {
            Function::Sin | Function::Cos | Function::Tan => self.lower_angle_to_radians(argument),
            Function::Asin
            | Function::Acos
            | Function::Atan
            | Function::Sqrt
            | Function::Root
            | Function::Exp
            | Function::Log
            | Function::Ln
            | Function::Abs
            | Function::Floor
            | Function::Factorial
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
            | Function::Sinh
            | Function::Cosh
            | Function::Tanh
            | Function::Asinh
            | Function::Acosh
            | Function::Atanh => argument,
        }
    }

    fn lower_angle_to_radians(&mut self, argument: ExprId) -> ExprId {
        match self.semantics.angle_unit {
            AngleUnit::Radian => argument,
            AngleUnit::Degree => self.multiply_by_pi_over_integer(argument, 180),
            AngleUnit::Gradian => self.multiply_by_pi_over_integer(argument, 200),
        }
    }

    fn multiply_by_pi_over_integer(&mut self, argument: ExprId, denominator: i64) -> ExprId {
        let pi = self.push_node(ExpressionNode::Constant(Constant::Pi));
        let denominator = self.push_rational(Rational::from_integer(Integer::from(denominator)));
        let scale = self.divide(pi, denominator);
        self.multiply(argument, scale)
    }

    fn negate(&mut self, value: ExprId) -> ExprId {
        if let ExpressionNode::Rational(rational) = self.nodes[value.0 as usize] {
            return self.push_rational(self.rationals[rational.0 as usize].negate());
        }
        let minus_one = self.push_rational(Rational::from_integer(Integer::from(-1)));
        self.multiply(minus_one, value)
    }

    fn integer(&mut self, value: i64) -> ExprId {
        self.push_rational(Rational::from_integer(Integer::from(value)))
    }

    fn add(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.add_many(vec![left, right])
    }

    fn subtract(&mut self, left: ExprId, right: ExprId) -> ExprId {
        let negative_right = self.negate(right);
        self.add(left, negative_right)
    }

    fn multiply(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.multiply_many(vec![left, right])
    }

    fn divide(&mut self, numerator: ExprId, denominator: ExprId) -> ExprId {
        let expression = self.divide_raw(numerator, denominator);
        self.normalize_arithmetic_expression(expression)
    }

    fn divide_raw(&mut self, numerator: ExprId, denominator: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Divide {
            numerator,
            denominator,
        })
    }

    fn power(&mut self, base: ExprId, exponent: ExprId) -> ExprId {
        let expression = self.power_raw(base, exponent);
        self.normalize_arithmetic_expression(expression)
    }

    fn power_raw(&mut self, base: ExprId, exponent: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Power { base, exponent })
    }

    fn add_many(&mut self, values: Vec<ExprId>) -> ExprId {
        self.add_many_with_constant(values, Rational::zero())
    }

    fn add_many_with_constant(&mut self, values: Vec<ExprId>, constant: Rational) -> ExprId {
        if values.is_empty() {
            return self.push_rational(constant);
        }
        if self.additive_literal_fold_limit_reached {
            let mut values = values;
            if !constant.is_zero() {
                values.push(self.push_rational(constant));
            }
            return match values.as_slice() {
                [value] => *value,
                [_, ..] => {
                    let list = self.push_list(values);
                    self.push_node(ExpressionNode::Add(list))
                }
                [] => self.push_rational(Rational::zero()),
            };
        }
        let expression = self.add_many_linear(values, constant);
        self.normalize_arithmetic_expression(expression)
    }

    fn add_many_linear(&mut self, values: Vec<ExprId>, mut constant: Rational) -> ExprId {
        let mut terms = Vec::<BuilderLinearTerm>::new();
        let mut values = values.into_iter();
        if let Some(first) = values.next() {
            if let ExpressionNode::Add(list) = self.nodes[first.0 as usize] {
                for term in self.lists[list.0 as usize].clone() {
                    self.seed_canonical_linear_term(term, &mut terms, &mut constant);
                }
            } else {
                self.collect_linear_terms(first, &Rational::one(), &mut terms, &mut constant);
            }
        }
        for value in values {
            self.collect_linear_terms(value, &Rational::one(), &mut terms, &mut constant);
        }
        terms
            .retain(|term| !term.coefficient.is_zero() || !self.is_proven_defined(term.expression));

        let mut expressions = Vec::with_capacity(terms.len() + usize::from(!constant.is_zero()));
        for term in terms {
            if term.coefficient == Rational::one() {
                expressions.push(term.expression);
            } else {
                let coefficient = self.push_rational(term.coefficient);
                expressions.push(self.multiply_many_factors(vec![coefficient, term.expression]));
            }
        }
        if !constant.is_zero() {
            expressions.push(self.push_rational(constant));
        }
        expressions.sort_by(|left, right| self.compare_add_terms(*left, *right));
        match expressions.as_slice() {
            [] => self.push_rational(Rational::zero()),
            [expression] => *expression,
            _ => {
                let list = self.push_list(expressions);
                self.push_node(ExpressionNode::Add(list))
            }
        }
    }

    fn seed_canonical_linear_term(
        &mut self,
        id: ExprId,
        terms: &mut Vec<BuilderLinearTerm>,
        constant: &mut Rational,
    ) {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => {
                *constant = constant.add(&self.rationals[value.0 as usize]);
            }
            ExpressionNode::Multiply(_) => {
                let (coefficient, expression) = self.split_product_coefficient(id);
                if let Some(expression) = expression {
                    terms.push(BuilderLinearTerm {
                        expression,
                        coefficient,
                    });
                } else {
                    *constant = constant.add(&coefficient);
                }
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => terms.push(BuilderLinearTerm {
                expression: id,
                coefficient: Rational::one(),
            }),
        }
    }

    fn collect_linear_terms(
        &mut self,
        id: ExprId,
        scale: &Rational,
        terms: &mut Vec<BuilderLinearTerm>,
        constant: &mut Rational,
    ) {
        match self.nodes[id.0 as usize].clone() {
            ExpressionNode::Rational(value) => {
                *constant = constant.add(&scale.multiply(&self.rationals[value.0 as usize]));
            }
            ExpressionNode::Add(values) => {
                for child in self.lists[values.0 as usize].clone() {
                    self.collect_linear_terms(child, scale, terms, constant);
                }
            }
            ExpressionNode::Multiply(_) => {
                let (coefficient, expression) = self.split_product_coefficient(id);
                let coefficient = scale.multiply(&coefficient);
                let Some(expression) = expression else {
                    *constant = constant.add(&coefficient);
                    return;
                };
                if expression == id
                    || matches!(
                        self.nodes[expression.0 as usize],
                        ExpressionNode::Multiply(_)
                    )
                {
                    self.add_builder_linear_term(expression, coefficient, terms);
                } else {
                    self.collect_linear_terms(expression, &coefficient, terms, constant);
                }
            }
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                let denominator_value = match self.nodes[denominator.0 as usize] {
                    ExpressionNode::Rational(value) => {
                        Some(self.rationals[value.0 as usize].clone())
                    }
                    ExpressionNode::Exact(_)
                    | ExpressionNode::Constant(_)
                    | ExpressionNode::Add(_)
                    | ExpressionNode::Multiply(_)
                    | ExpressionNode::Divide { .. }
                    | ExpressionNode::Power { .. }
                    | ExpressionNode::LogBase { .. }
                    | ExpressionNode::Function { .. }
                    | ExpressionNode::BinaryFunction { .. } => None,
                };
                if let Some(denominator_value) = denominator_value {
                    if let Ok(quotient) = scale.divide(&denominator_value) {
                        self.collect_linear_terms(numerator, &quotient, terms, constant);
                        return;
                    }
                }
                self.add_builder_linear_term(id, scale.clone(), terms);
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => {
                self.add_builder_linear_term(id, scale.clone(), terms);
            }
        }
    }

    fn split_product_coefficient(&mut self, id: ExprId) -> (Rational, Option<ExprId>) {
        match self.nodes[id.0 as usize].clone() {
            ExpressionNode::Rational(value) => (self.rationals[value.0 as usize].clone(), None),
            ExpressionNode::Multiply(values) => {
                let original = self.lists[values.0 as usize].clone();
                let mut coefficient = Rational::one();
                let mut factors = Vec::new();
                for factor in &original {
                    let (factor_coefficient, factor) = self.split_product_coefficient(*factor);
                    coefficient = coefficient.multiply(&factor_coefficient);
                    if let Some(factor) = factor {
                        factors.push(factor);
                    }
                }
                let expression = match factors.as_slice() {
                    [] => None,
                    [factor] => Some(*factor),
                    _ if factors == original => Some(id),
                    _ => {
                        factors.sort_by(|left, right| self.compare_expressions(*left, *right));
                        let values = self.push_list(factors);
                        Some(self.push_node(ExpressionNode::Multiply(values)))
                    }
                };
                (coefficient, expression)
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => (Rational::one(), Some(id)),
        }
    }

    fn add_builder_linear_term(
        &self,
        expression: ExprId,
        coefficient: Rational,
        terms: &mut Vec<BuilderLinearTerm>,
    ) {
        if self.is_proven_defined(expression) {
            if let Some(term) = terms
                .iter_mut()
                .find(|term| self.builder_terms_equal(term.expression, expression))
            {
                term.coefficient = term.coefficient.add(&coefficient);
                return;
            }
        }
        terms.push(BuilderLinearTerm {
            expression,
            coefficient,
        });
    }

    fn builder_terms_equal(&self, left: ExprId, right: ExprId) -> bool {
        if left == right {
            return true;
        }
        if !matches!(self.nodes[left.0 as usize], ExpressionNode::Multiply(_))
            || !matches!(self.nodes[right.0 as usize], ExpressionNode::Multiply(_))
        {
            return false;
        }
        let mut left_factors = Vec::new();
        let mut right_factors = Vec::new();
        self.collect_flat_product_ids(left, &mut left_factors);
        self.collect_flat_product_ids(right, &mut right_factors);
        left_factors.sort_by(|left, right| self.compare_expressions(*left, *right));
        right_factors.sort_by(|left, right| self.compare_expressions(*left, *right));
        left_factors == right_factors
    }

    fn collect_flat_product_ids(&self, id: ExprId, factors: &mut Vec<ExprId>) {
        if let ExpressionNode::Multiply(values) = self.nodes[id.0 as usize] {
            for factor in &self.lists[values.0 as usize] {
                self.collect_flat_product_ids(*factor, factors);
            }
        } else {
            factors.push(id);
        }
    }

    fn multiply_many(&mut self, values: Vec<ExprId>) -> ExprId {
        let expression = self.multiply_many_factors(values);
        self.normalize_arithmetic_expression(expression)
    }

    fn multiply_many_with_coefficient(
        &mut self,
        mut values: Vec<ExprId>,
        coefficient: Rational,
    ) -> ExprId {
        if self.multiplicative_literal_fold_limit_reached {
            if coefficient != Rational::one() || values.is_empty() {
                values.push(self.push_rational(coefficient));
            }
            return match values.as_slice() {
                [value] => *value,
                [_, ..] => {
                    let list = self.push_list(values);
                    self.push_node(ExpressionNode::Multiply(list))
                }
                [] => unreachable!("an empty product receives its coefficient"),
            };
        }
        let expression = self.multiply_many_factors_with_coefficient(values, coefficient);
        self.normalize_arithmetic_expression(expression)
    }

    fn multiply_many_factors(&mut self, values: Vec<ExprId>) -> ExprId {
        self.multiply_many_factors_with_coefficient(values, Rational::one())
    }

    fn multiply_many_factors_with_coefficient(
        &mut self,
        values: Vec<ExprId>,
        mut coefficient: Rational,
    ) -> ExprId {
        let mut factors = Vec::new();
        for value in values {
            self.collect_product_factors(value, &mut coefficient, &mut factors);
        }
        factors.sort_by(|left, right| self.compare_expressions(*left, *right));
        if coefficient.is_zero() && factors.iter().all(|factor| self.is_proven_defined(*factor)) {
            return self.push_rational(coefficient);
        }
        if coefficient != Rational::one() || factors.is_empty() {
            factors.insert(0, self.push_rational(coefficient));
        }
        match factors.as_slice() {
            [factor] => *factor,
            _ => {
                let list = self.push_list(factors);
                self.push_node(ExpressionNode::Multiply(list))
            }
        }
    }

    fn collect_product_factors(
        &self,
        id: ExprId,
        coefficient: &mut Rational,
        factors: &mut Vec<ExprId>,
    ) {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => {
                *coefficient = coefficient.multiply(&self.rationals[value.0 as usize]);
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => factors.push(id),
        }
    }

    fn normalize_arithmetic_expression(&mut self, id: ExprId) -> ExprId {
        if self.canonical_limit_reached.is_some() || !self.is_proven_defined(id) {
            return id;
        }

        let mut budget = self.canonical_budget;
        let polynomial = match self.polynomial_from_expression(id, &mut budget) {
            Ok(polynomial) => polynomial,
            Err(kind) => {
                self.canonical_limit_reached.get_or_insert(kind);
                return id;
            }
        };
        let estimated_nodes = polynomial
            .terms
            .iter()
            .map(|term| term.factors.len().saturating_mul(2).saturating_add(3))
            .sum::<usize>();
        if polynomial.terms.len() > self.canonical_term_limit
            || self.nodes.len().saturating_add(estimated_nodes) > self.canonical_term_limit
        {
            self.canonical_limit_reached
                .get_or_insert(ComputationLimitKind::LogicalWorkUnits);
            return id;
        }
        self.canonical_budget = budget;
        let normalized = self.materialize_polynomial(polynomial);
        if normalized != id {
            self.domain_obligations.push(DomainObligation::Defined(id));
            match self.nodes[id.0 as usize] {
                ExpressionNode::Divide { denominator, .. } => self
                    .domain_obligations
                    .push(DomainObligation::NonZero(denominator)),
                ExpressionNode::Power { base, exponent } => {
                    if let ExpressionNode::Rational(value) = self.nodes[exponent.0 as usize] {
                        let exponent = &self.rationals[value.0 as usize];
                        if exponent.is_zero() || exponent.is_negative() {
                            self.domain_obligations
                                .push(DomainObligation::NonZero(base));
                        }
                    }
                }
                ExpressionNode::Rational(_)
                | ExpressionNode::Exact(_)
                | ExpressionNode::Constant(_)
                | ExpressionNode::Add(_)
                | ExpressionNode::Multiply(_)
                | ExpressionNode::LogBase { .. }
                | ExpressionNode::Function { .. }
                | ExpressionNode::BinaryFunction { .. } => {}
            }
        }
        normalized
    }

    fn polynomial_from_expression(
        &self,
        id: ExprId,
        budget: &mut CanonicalBudget,
    ) -> Result<BuilderPolynomial, ComputationLimitKind> {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => Ok(BuilderPolynomial {
                terms: vec![BuilderMonomial {
                    coefficient: self.rationals[value.0 as usize].clone(),
                    factors: Vec::new(),
                }],
            }),
            ExpressionNode::Add(values) => {
                let mut polynomial = BuilderPolynomial { terms: Vec::new() };
                for child in &self.lists[values.0 as usize] {
                    let child = self.polynomial_from_expression(*child, budget)?;
                    polynomial = self.add_polynomials(polynomial, child, budget)?;
                }
                Ok(polynomial)
            }
            ExpressionNode::Multiply(values) => {
                let mut polynomial = BuilderPolynomial::one();
                for child in &self.lists[values.0 as usize] {
                    let child = self.polynomial_from_expression(*child, budget)?;
                    polynomial = self.multiply_polynomials(polynomial, child, budget)?;
                }
                Ok(polynomial)
            }
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                let numerator = self.polynomial_from_expression(numerator, budget)?;
                let denominator = self.polynomial_from_expression(denominator, budget)?;
                if denominator.terms.len() != 1 {
                    return Ok(self.atomic_polynomial(id));
                }
                let denominator = &denominator.terms[0];
                if denominator.coefficient.is_zero()
                    || denominator
                        .factors
                        .iter()
                        .any(|(base, _)| !self.is_proven_nonzero(*base))
                {
                    return Ok(self.atomic_polynomial(id));
                }
                let coefficient = Rational::one()
                    .divide(&denominator.coefficient)
                    .expect("a canonical monomial denominator was checked as non-zero");
                let factors = denominator
                    .factors
                    .iter()
                    .map(|(base, exponent)| {
                        exponent
                            .checked_neg()
                            .map(|exponent| (*base, exponent))
                            .ok_or(ComputationLimitKind::LogicalWorkUnits)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let reciprocal = BuilderPolynomial {
                    terms: vec![BuilderMonomial {
                        coefficient,
                        factors,
                    }],
                };
                self.multiply_polynomials(numerator, reciprocal, budget)
            }
            ExpressionNode::Power { base, exponent } => {
                let ExpressionNode::Rational(exponent) = self.nodes[exponent.0 as usize] else {
                    return Ok(self.atomic_polynomial(id));
                };
                let Some(exponent) = self.rationals[exponent.0 as usize].as_i64_if_integer() else {
                    return Ok(self.atomic_polynomial(id));
                };
                if exponent < 0 && !self.is_proven_nonzero(base) {
                    return Ok(self.atomic_polynomial(id));
                }
                let base = self.polynomial_from_expression(base, budget)?;
                if exponent < 0 && base.terms.len() != 1 {
                    return Ok(self.atomic_polynomial(id));
                }
                self.power_polynomial(base, exponent, budget)
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_)
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => Ok(self.atomic_polynomial(id)),
        }
    }

    fn atomic_polynomial(&self, id: ExprId) -> BuilderPolynomial {
        BuilderPolynomial {
            terms: vec![BuilderMonomial {
                coefficient: Rational::one(),
                factors: vec![(id, 1)],
            }],
        }
    }

    fn add_polynomials(
        &self,
        mut left: BuilderPolynomial,
        right: BuilderPolynomial,
        budget: &mut CanonicalBudget,
    ) -> Result<BuilderPolynomial, ComputationLimitKind> {
        let work = left.terms.len().saturating_add(right.terms.len());
        budget.reserve(work, work)?;
        left.terms.extend(right.terms);
        self.normalize_polynomial_terms(left, budget)
    }

    fn multiply_polynomials(
        &self,
        left: BuilderPolynomial,
        right: BuilderPolynomial,
        budget: &mut CanonicalBudget,
    ) -> Result<BuilderPolynomial, ComputationLimitKind> {
        let work = left.terms.len().saturating_mul(right.terms.len());
        if work > self.canonical_term_limit {
            return Err(ComputationLimitKind::LogicalWorkUnits);
        }
        budget.reserve(work, work)?;
        budget.reserve(
            0,
            polynomial_factor_product_work(&left, &right, |id| {
                self.structural_sizes[id.0 as usize]
            }),
        )?;
        let mut terms = Vec::with_capacity(work);
        for left in &left.terms {
            for right in &right.terms {
                terms.push(self.multiply_monomials(left, right)?);
            }
        }
        self.normalize_polynomial_terms(BuilderPolynomial { terms }, budget)
    }

    fn multiply_monomials(
        &self,
        left: &BuilderMonomial,
        right: &BuilderMonomial,
    ) -> Result<BuilderMonomial, ComputationLimitKind> {
        let mut merged = Vec::with_capacity(left.factors.len().saturating_add(right.factors.len()));
        let mut left_index = 0;
        let mut right_index = 0;
        while left_index < left.factors.len() && right_index < right.factors.len() {
            let left_factor = left.factors[left_index];
            let right_factor = right.factors[right_index];
            match self.compare_expressions(left_factor.0, right_factor.0) {
                Ordering::Less => {
                    merged.push(left_factor);
                    left_index += 1;
                }
                Ordering::Greater => {
                    merged.push(right_factor);
                    right_index += 1;
                }
                Ordering::Equal => {
                    let exponent = left_factor
                        .1
                        .checked_add(right_factor.1)
                        .ok_or(ComputationLimitKind::LogicalWorkUnits)?;
                    if exponent != 0 {
                        merged.push((left_factor.0, exponent));
                    } else if !self.is_proven_nonzero(left_factor.0) {
                        return Err(ComputationLimitKind::LogicalWorkUnits);
                    }
                    left_index += 1;
                    right_index += 1;
                }
            }
        }
        merged.extend_from_slice(&left.factors[left_index..]);
        merged.extend_from_slice(&right.factors[right_index..]);
        Ok(BuilderMonomial {
            coefficient: left.coefficient.multiply(&right.coefficient),
            factors: merged,
        })
    }

    fn power_polynomial(
        &self,
        base: BuilderPolynomial,
        exponent: i64,
        budget: &mut CanonicalBudget,
    ) -> Result<BuilderPolynomial, ComputationLimitKind> {
        if exponent == 0 {
            return Ok(BuilderPolynomial::one());
        }
        if exponent < 0 {
            let [term] = base.terms.as_slice() else {
                unreachable!("negative polynomial powers require a non-zero monomial base")
            };
            let magnitude = exponent
                .checked_abs()
                .ok_or(ComputationLimitKind::LogicalWorkUnits)?;
            let coefficient = term
                .coefficient
                .pow_i64(exponent)
                .map_err(|_| ComputationLimitKind::LogicalWorkUnits)?;
            let factors = term
                .factors
                .iter()
                .map(|(base, factor_exponent)| {
                    factor_exponent
                        .checked_mul(-magnitude)
                        .map(|exponent| (*base, exponent))
                        .ok_or(ComputationLimitKind::LogicalWorkUnits)
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(BuilderPolynomial {
                terms: vec![BuilderMonomial {
                    coefficient,
                    factors,
                }],
            });
        }

        let mut exponent =
            u64::try_from(exponent).map_err(|_| ComputationLimitKind::LogicalWorkUnits)?;
        let mut factor = base;
        let mut result = BuilderPolynomial::one();
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = self.multiply_polynomials(result, factor.clone(), budget)?;
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = self.multiply_polynomials(factor.clone(), factor, budget)?;
            }
        }
        Ok(result)
    }

    fn normalize_polynomial_terms(
        &self,
        mut polynomial: BuilderPolynomial,
        budget: &mut CanonicalBudget,
    ) -> Result<BuilderPolynomial, ComputationLimitKind> {
        budget.reserve(
            0,
            polynomial_term_merge_work(&polynomial, |id| self.structural_sizes[id.0 as usize]),
        )?;
        polynomial.terms.retain(|term| !term.coefficient.is_zero());
        debug_assert!(polynomial.terms.iter().all(|term| {
            term.factors.iter().all(|(_, exponent)| *exponent != 0)
                && term.factors.windows(2).all(|factors| {
                    self.compare_expressions(factors[0].0, factors[1].0) == Ordering::Less
                })
        }));
        polynomial
            .terms
            .sort_by(|left, right| self.compare_monomial_factors(&left.factors, &right.factors));
        let mut terms = Vec::<BuilderMonomial>::with_capacity(polynomial.terms.len());
        for term in polynomial.terms {
            if let Some(last) = terms.last_mut() {
                if self.compare_monomial_factors(&last.factors, &term.factors) == Ordering::Equal {
                    last.coefficient = last.coefficient.add(&term.coefficient);
                    continue;
                }
            }
            terms.push(term);
        }
        terms.retain(|term| !term.coefficient.is_zero());
        let (complement_rewrites, complement_work) = polynomial_trig_complement_plan(
            &terms,
            |id| self.structural_sizes[id.0 as usize],
            |id, exponent| trig_square_candidate_kind(&self.nodes[id.0 as usize], exponent),
            |left, right| self.trig_complement_monomials_match(left, right),
        );
        budget.reserve(complement_rewrites, complement_work)?;
        self.reduce_trig_square_complements(&mut terms);
        if terms.len() > self.canonical_term_limit {
            return Err(ComputationLimitKind::LogicalWorkUnits);
        }
        Ok(BuilderPolynomial { terms })
    }

    fn reduce_trig_square_complements(&self, terms: &mut Vec<BuilderMonomial>) {
        loop {
            let mut matched = None;
            'pairs: for left_index in 0..terms.len() {
                for right_index in left_index + 1..terms.len() {
                    for left_factor in 0..terms[left_index].factors.len() {
                        let Some((left_function, left_argument)) =
                            self.trig_square_factor(&terms[left_index], left_factor)
                        else {
                            continue;
                        };
                        for right_factor in 0..terms[right_index].factors.len() {
                            let Some((right_function, right_argument)) =
                                self.trig_square_factor(&terms[right_index], right_factor)
                            else {
                                continue;
                            };
                            if !trig_functions_are_complements(left_function, right_function)
                                || self.compare_expressions(left_argument, right_argument)
                                    != Ordering::Equal
                                || !self.monomial_factors_equal_without(
                                    &terms[left_index],
                                    left_factor,
                                    &terms[right_index],
                                    right_factor,
                                )
                            {
                                continue;
                            }
                            let Some(coefficient) = common_signed_coefficient(
                                &terms[left_index].coefficient,
                                &terms[right_index].coefficient,
                            ) else {
                                continue;
                            };
                            matched = Some((left_index, right_index, left_factor, coefficient));
                            break 'pairs;
                        }
                    }
                }
            }

            let Some((left_index, right_index, left_factor, coefficient)) = matched else {
                break;
            };
            let mut common_factors = terms[left_index].factors.clone();
            common_factors.remove(left_factor);
            terms[left_index].coefficient = terms[left_index].coefficient.subtract(&coefficient);
            terms[right_index].coefficient = terms[right_index].coefficient.subtract(&coefficient);
            terms.push(BuilderMonomial {
                coefficient,
                factors: common_factors,
            });
            terms.retain(|term| !term.coefficient.is_zero());
            terms.sort_by(|left, right| {
                self.compare_monomial_factors(&left.factors, &right.factors)
            });
            let mut merged = Vec::<BuilderMonomial>::with_capacity(terms.len());
            for term in terms.drain(..) {
                if let Some(last) = merged.last_mut() {
                    if self.compare_monomial_factors(&last.factors, &term.factors)
                        == Ordering::Equal
                    {
                        last.coefficient = last.coefficient.add(&term.coefficient);
                        continue;
                    }
                }
                merged.push(term);
            }
            merged.retain(|term| !term.coefficient.is_zero());
            *terms = merged;
        }
    }

    fn trig_square_factor(
        &self,
        term: &BuilderMonomial,
        factor_index: usize,
    ) -> Option<(Function, ExprId)> {
        let (base, exponent) = term.factors[factor_index];
        if exponent != 2 {
            return None;
        }
        let ExpressionNode::Function { function, argument } = self.nodes[base.0 as usize] else {
            return None;
        };
        matches!(function, Function::Sin | Function::Cos).then_some((function, argument))
    }

    fn trig_complement_monomials_match(
        &self,
        left: &BuilderMonomial,
        right: &BuilderMonomial,
    ) -> bool {
        for left_factor in 0..left.factors.len() {
            let Some((left_function, left_argument)) = self.trig_square_factor(left, left_factor)
            else {
                continue;
            };
            for right_factor in 0..right.factors.len() {
                let Some((right_function, right_argument)) =
                    self.trig_square_factor(right, right_factor)
                else {
                    continue;
                };
                if trig_functions_are_complements(left_function, right_function)
                    && self.compare_expressions(left_argument, right_argument) == Ordering::Equal
                    && self.monomial_factors_equal_without(left, left_factor, right, right_factor)
                    && common_signed_coefficient(&left.coefficient, &right.coefficient).is_some()
                {
                    return true;
                }
            }
        }
        false
    }

    fn monomial_factors_equal_without(
        &self,
        left: &BuilderMonomial,
        left_skip: usize,
        right: &BuilderMonomial,
        right_skip: usize,
    ) -> bool {
        if left.factors.len() != right.factors.len() {
            return false;
        }
        left.factors
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != left_skip)
            .zip(
                right
                    .factors
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != right_skip),
            )
            .all(|((_, left), (_, right))| {
                left.1 == right.1 && self.compare_expressions(left.0, right.0) == Ordering::Equal
            })
    }

    fn compare_monomial_factors(
        &self,
        left: &[(ExprId, i64)],
        right: &[(ExprId, i64)],
    ) -> Ordering {
        for ((left_base, left_exponent), (right_base, right_exponent)) in left.iter().zip(right) {
            let order = self.compare_expressions(*left_base, *right_base);
            if order != Ordering::Equal {
                return order;
            }
            let order = left_exponent.cmp(right_exponent);
            if order != Ordering::Equal {
                return order;
            }
        }
        left.len().cmp(&right.len())
    }

    fn materialize_polynomial(&mut self, polynomial: BuilderPolynomial) -> ExprId {
        let mut terms = Vec::with_capacity(polynomial.terms.len());
        for term in polynomial.terms {
            terms.push(self.materialize_monomial(term));
        }
        self.add_many_linear(terms, Rational::zero())
    }

    fn materialize_monomial(&mut self, monomial: BuilderMonomial) -> ExprId {
        let mut numerator = Vec::new();
        let mut denominator = Vec::new();
        let mut exponential_arguments = Vec::new();
        for (base, exponent) in monomial.factors {
            if let ExpressionNode::Function {
                function: Function::Exp,
                argument,
            } = self.nodes[base.0 as usize]
            {
                let argument = if exponent == 1 {
                    argument
                } else {
                    let exponent = self.push_rational(rational_integer(exponent));
                    self.multiply(exponent, argument)
                };
                exponential_arguments.push(argument);
                continue;
            }
            let (target, magnitude) = if exponent < 0 {
                (&mut denominator, exponent.unsigned_abs())
            } else {
                (&mut numerator, exponent as u64)
            };
            if magnitude == 0 {
                continue;
            }
            let factor = if magnitude == 1 {
                base
            } else {
                let exponent = self.push_rational(rational_integer(
                    i64::try_from(magnitude)
                        .expect("canonical factor exponent magnitude fits in i64"),
                ));
                self.power_raw(base, exponent)
            };
            target.push(factor);
        }
        if !exponential_arguments.is_empty() {
            let argument = self.add_many(exponential_arguments);
            numerator.push(self.exp(argument));
        }
        if monomial.coefficient != Rational::one() || numerator.is_empty() {
            numerator.insert(0, self.push_rational(monomial.coefficient));
        }
        let numerator = self.multiply_many_factors(numerator);
        if denominator.is_empty() {
            numerator
        } else {
            let denominator = self.multiply_many_factors(denominator);
            self.divide_raw(numerator, denominator)
        }
    }

    fn exp(&mut self, argument: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Function {
            function: Function::Exp,
            argument,
        })
    }

    fn log(&mut self, argument: ExprId) -> ExprId {
        self.push_node(ExpressionNode::Function {
            function: Function::Log,
            argument,
        })
    }

    fn sqrt(&mut self, argument: ExprId) -> ExprId {
        if let ExpressionNode::Function {
            function: Function::Exp,
            argument,
        } = self.nodes[argument.0 as usize]
        {
            let two = self.push_rational(rational_integer(2));
            let half_argument = self.divide(argument, two);
            return self.exp(half_argument);
        }
        self.push_node(ExpressionNode::Function {
            function: Function::Sqrt,
            argument,
        })
    }

    fn is_proven_defined(&self, id: ExprId) -> bool {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => true,
            ExpressionNode::Exact(_) => false,
            ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => self.lists
                [values.0 as usize]
                .iter()
                .all(|child| self.is_proven_defined(*child)),
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                self.is_proven_defined(numerator)
                    && self.is_proven_defined(denominator)
                    && self.is_proven_nonzero(denominator)
            }
            ExpressionNode::Power { base, exponent } => {
                self.is_proven_defined(base)
                    && self.is_proven_defined(exponent)
                    && self.power_domain_is_proven(base, exponent)
            }
            ExpressionNode::LogBase { argument, base } => {
                self.is_proven_positive(argument)
                    && self.is_proven_positive(base)
                    && self.is_proven_not_one(base)
            }
            ExpressionNode::Function { function, argument } => match function {
                Function::Exp | Function::Sin | Function::Cos | Function::Abs | Function::Floor => {
                    self.is_proven_defined(argument)
                }
                Function::Log | Function::Ln => self.is_proven_positive(argument),
                Function::Sqrt => self.is_proven_nonnegative(argument),
                Function::Sinh | Function::Cosh => self.is_proven_defined(argument),
                Function::Tan
                | Function::Asin
                | Function::Acos
                | Function::Atan
                | Function::Root
                | Function::Factorial
                | Function::Permutation
                | Function::Combination
                | Function::Modulo
                | Function::Gcd
                | Function::Lcm
                | Function::Tanh
                | Function::Asinh
                | Function::Acosh
                | Function::Atanh => false,
            },
            ExpressionNode::BinaryFunction { .. } => false,
        }
    }

    fn power_domain_is_proven(&self, base: ExprId, exponent: ExprId) -> bool {
        if self.is_proven_positive(base) {
            return true;
        }
        let ExpressionNode::Rational(value) = self.nodes[exponent.0 as usize] else {
            return false;
        };
        let exponent = &self.rationals[value.0 as usize];
        exponent.is_integer()
            && if exponent.is_zero() || exponent.is_negative() {
                self.is_proven_nonzero(base)
            } else {
                true
            }
    }

    fn is_proven_positive(&self, id: ExprId) -> bool {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => {
                let value = &self.rationals[value.0 as usize];
                !value.is_zero() && !value.is_negative()
            }
            ExpressionNode::Constant(Constant::Pi | Constant::Euler) => true,
            ExpressionNode::Function {
                function: Function::Exp,
                argument,
            } => self.is_proven_defined(argument),
            ExpressionNode::Multiply(values) => {
                let factors = &self.lists[values.0 as usize];
                factors
                    .iter()
                    .all(|factor| self.is_proven_positive(*factor))
            }
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => self.is_proven_positive(numerator) && self.is_proven_positive(denominator),
            ExpressionNode::Power { base, exponent } => {
                self.is_proven_positive(base) && self.is_proven_defined(exponent)
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => false,
        }
    }

    fn is_proven_nonnegative(&self, id: ExprId) -> bool {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => !self.rationals[value.0 as usize].is_negative(),
            ExpressionNode::Constant(Constant::Pi | Constant::Euler) => true,
            ExpressionNode::Function {
                function: Function::Exp | Function::Abs | Function::Sqrt,
                argument,
            } => self.is_proven_defined(argument),
            ExpressionNode::Power { exponent, .. } => {
                let ExpressionNode::Rational(value) = self.nodes[exponent.0 as usize] else {
                    return false;
                };
                self.is_proven_defined(id)
                    && self.rationals[value.0 as usize]
                        .as_i64_if_integer()
                        .is_some_and(|value| value >= 0 && value % 2 == 0)
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => false,
        }
    }

    fn is_proven_nonzero(&self, id: ExprId) -> bool {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => !self.rationals[value.0 as usize].is_zero(),
            ExpressionNode::Constant(Constant::Pi | Constant::Euler) => true,
            ExpressionNode::Function {
                function: Function::Exp,
                argument,
            } => self.is_proven_defined(argument),
            ExpressionNode::Multiply(values) => self.lists[values.0 as usize]
                .iter()
                .all(|factor| self.is_proven_nonzero(*factor)),
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => self.is_proven_nonzero(numerator) && self.is_proven_nonzero(denominator),
            ExpressionNode::Power { base, exponent } => {
                self.is_proven_nonzero(base)
                    && self.is_proven_defined(exponent)
                    && self.power_domain_is_proven(base, exponent)
            }
            ExpressionNode::Exact(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => false,
        }
    }

    fn is_proven_not_one(&self, id: ExprId) -> bool {
        match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(value) => self.rationals[value.0 as usize] != Rational::one(),
            ExpressionNode::Constant(Constant::Pi | Constant::Euler) => true,
            ExpressionNode::Function {
                function: Function::Exp,
                argument,
            } => self.is_proven_nonzero(argument),
            ExpressionNode::Exact(_)
            | ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. } => false,
        }
    }

    fn compare_expressions(&self, left: ExprId, right: ExprId) -> Ordering {
        if left == right {
            return Ordering::Equal;
        }
        let left_node = &self.nodes[left.0 as usize];
        let right_node = &self.nodes[right.0 as usize];
        let rank_order = expression_node_rank(left_node).cmp(&expression_node_rank(right_node));
        if rank_order != Ordering::Equal {
            return rank_order;
        }
        #[allow(clippy::wildcard_enum_match_arm)]
        match (left_node, right_node) {
            (ExpressionNode::Rational(left), ExpressionNode::Rational(right)) => {
                self.rationals[left.0 as usize].compare(&self.rationals[right.0 as usize])
            }
            (ExpressionNode::Constant(left), ExpressionNode::Constant(right)) => {
                constant_rank(*left).cmp(&constant_rank(*right))
            }
            (
                ExpressionNode::Power {
                    base: left_base,
                    exponent: left_exponent,
                },
                ExpressionNode::Power {
                    base: right_base,
                    exponent: right_exponent,
                },
            ) => self
                .compare_expressions(*left_base, *right_base)
                .then_with(|| self.compare_expressions(*left_exponent, *right_exponent)),
            (
                ExpressionNode::Function {
                    function: left_function,
                    argument: left_argument,
                },
                ExpressionNode::Function {
                    function: right_function,
                    argument: right_argument,
                },
            ) => function_rank(*left_function)
                .cmp(&function_rank(*right_function))
                .then_with(|| self.compare_expressions(*left_argument, *right_argument)),
            (ExpressionNode::Add(left), ExpressionNode::Add(right))
            | (ExpressionNode::Multiply(left), ExpressionNode::Multiply(right)) => {
                self.compare_expression_lists(*left, *right)
            }
            (
                ExpressionNode::Divide {
                    numerator: left_numerator,
                    denominator: left_denominator,
                },
                ExpressionNode::Divide {
                    numerator: right_numerator,
                    denominator: right_denominator,
                },
            ) => self
                .compare_expressions(*left_numerator, *right_numerator)
                .then_with(|| self.compare_expressions(*left_denominator, *right_denominator)),
            (
                ExpressionNode::LogBase {
                    argument: left_argument,
                    base: left_base,
                },
                ExpressionNode::LogBase {
                    argument: right_argument,
                    base: right_base,
                },
            ) => self
                .compare_expressions(*left_argument, *right_argument)
                .then_with(|| self.compare_expressions(*left_base, *right_base)),
            (
                ExpressionNode::BinaryFunction {
                    function: left_function,
                    left: left_argument,
                    right: left_base,
                },
                ExpressionNode::BinaryFunction {
                    function: right_function,
                    left: right_argument,
                    right: right_base,
                },
            ) => function_rank(*left_function)
                .cmp(&function_rank(*right_function))
                .then_with(|| self.compare_expressions(*left_argument, *right_argument))
                .then_with(|| self.compare_expressions(*left_base, *right_base)),
            (ExpressionNode::Exact(_), ExpressionNode::Exact(_)) => {
                unreachable!("source canonicalization cannot contain stored exact values")
            }
            (
                ExpressionNode::Rational(_)
                | ExpressionNode::Exact(_)
                | ExpressionNode::Constant(_)
                | ExpressionNode::Add(_)
                | ExpressionNode::Multiply(_)
                | ExpressionNode::Divide { .. }
                | ExpressionNode::Power { .. }
                | ExpressionNode::LogBase { .. }
                | ExpressionNode::Function { .. }
                | ExpressionNode::BinaryFunction { .. },
                ExpressionNode::Rational(_)
                | ExpressionNode::Exact(_)
                | ExpressionNode::Constant(_)
                | ExpressionNode::Add(_)
                | ExpressionNode::Multiply(_)
                | ExpressionNode::Divide { .. }
                | ExpressionNode::Power { .. }
                | ExpressionNode::LogBase { .. }
                | ExpressionNode::Function { .. }
                | ExpressionNode::BinaryFunction { .. },
            ) => unreachable!("different expression variants have distinct structural ranks"),
        }
    }

    fn compare_add_terms(&self, left: ExprId, right: ExprId) -> Ordering {
        let category = |id: ExprId| {
            match self.nodes[id.0 as usize] {
            ExpressionNode::Rational(_) => 1,
            ExpressionNode::Multiply(values)
                if self.lists[values.0 as usize].first().is_some_and(|factor| {
                    matches!(self.nodes[factor.0 as usize], ExpressionNode::Rational(value) if self.rationals[value.0 as usize].is_negative())
                }) =>
            {
                2
            }
                ExpressionNode::Exact(_)
                | ExpressionNode::Constant(_)
                | ExpressionNode::Add(_)
                | ExpressionNode::Multiply(_)
                | ExpressionNode::Divide { .. }
                | ExpressionNode::Power { .. }
                | ExpressionNode::LogBase { .. }
                | ExpressionNode::Function { .. }
                | ExpressionNode::BinaryFunction { .. } => 0,
            }
        };
        match category(left).cmp(&category(right)) {
            Ordering::Equal => self.compare_expressions(left, right),
            order @ (Ordering::Less | Ordering::Greater) => order,
        }
    }

    fn compare_expression_lists(&self, left: ExprListId, right: ExprListId) -> Ordering {
        let left = &self.lists[left.0 as usize];
        let right = &self.lists[right.0 as usize];
        for (left, right) in left.iter().zip(right) {
            let order = self.compare_expressions(*left, *right);
            if order != Ordering::Equal {
                return order;
            }
        }
        left.len().cmp(&right.len())
    }

    fn push_rational(&mut self, rational: Rational) -> ExprId {
        let (hash, value_work) = rational_hash_and_work(&rational);
        let candidate_count = self
            .rational_index
            .get(&hash)
            .map_or(0, |candidates| candidates.len());
        let lookup_allowed =
            self.reserve_intern_work(value_work.saturating_mul(candidate_count.saturating_add(1)));
        if lookup_allowed {
            if let Some(id) = self.rational_index.get(&hash).and_then(|candidates| {
                candidates
                    .iter()
                    .copied()
                    .find(|id| self.rationals[id.0 as usize] == rational)
            }) {
                return self.push_node(ExpressionNode::Rational(id));
            }
        }
        let id = RationalId(self.rationals.len() as u32);
        self.rationals.push(rational);
        self.rational_index.entry(hash).or_default().push(id);
        self.push_node(ExpressionNode::Rational(id))
    }

    fn push_list(&mut self, values: Vec<ExprId>) -> ExprListId {
        let hash = expression_list_hash(&values);
        let candidate_count = self
            .list_index
            .get(&hash)
            .map_or(0, |candidates| candidates.len());
        let lookup_work = values
            .len()
            .saturating_add(1)
            .saturating_mul(candidate_count.saturating_add(1));
        if self.reserve_intern_work(lookup_work) {
            if let Some(id) = self.list_index.get(&hash).and_then(|candidates| {
                candidates
                    .iter()
                    .copied()
                    .find(|id| self.lists[id.0 as usize] == values)
            }) {
                return id;
            }
        }
        let id = ExprListId(self.lists.len() as u32);
        self.lists.push(values);
        self.list_index.entry(hash).or_default().push(id);
        id
    }

    fn push_node(&mut self, node: ExpressionNode) -> ExprId {
        let hash = expression_node_hash(&node);
        let candidate_count = self
            .node_index
            .get(&hash)
            .map_or(0, |candidates| candidates.len());
        if self.reserve_intern_work(candidate_count.saturating_add(1)) {
            if let Some(id) = self.node_index.get(&hash).and_then(|candidates| {
                candidates
                    .iter()
                    .copied()
                    .find(|id| self.nodes[id.0 as usize] == node)
            }) {
                return id;
            }
        }
        let id = ExprId(self.nodes.len() as u32);
        let structural_size = expression_node_structural_size(
            &node,
            &self.structural_sizes,
            &self.lists,
            &self.rationals,
        );
        self.nodes.push(node);
        self.structural_sizes.push(structural_size);
        self.node_index.entry(hash).or_default().push(id);
        id
    }

    fn reserve_intern_work(&mut self, work: usize) -> bool {
        if self.canonical_limit_reached.is_some() {
            return false;
        }
        match self.canonical_budget.reserve(1, work) {
            Ok(()) => true,
            Err(kind) => {
                self.canonical_limit_reached.get_or_insert(kind);
                false
            }
        }
    }
}

fn hash_word(hash: u64, word: u64) -> u64 {
    hash.wrapping_mul(1_099_511_628_211).wrapping_add(word)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(14_695_981_039_346_656_037, |hash, byte| {
        hash_word(hash, u64::from(*byte))
    })
}

fn rational_hash_and_work(value: &Rational) -> (u64, usize) {
    let numerator = value.numerator.inner.to_signed_bytes_le();
    let denominator = value.denominator.inner.inner.to_signed_bytes_le();
    (
        hash_word(hash_bytes(&numerator), hash_bytes(&denominator)),
        numerator.len().saturating_add(denominator.len()),
    )
}

fn expression_list_hash(values: &[ExprId]) -> u64 {
    values.iter().fold(0x4c49_5354, |hash, value| {
        hash_word(hash, u64::from(value.0))
    })
}

fn expression_node_hash(node: &ExpressionNode) -> u64 {
    let pair = |tag, left: ExprId, right: ExprId| {
        hash_word(hash_word(tag, u64::from(left.0)), u64::from(right.0))
    };
    match node {
        ExpressionNode::Rational(value) => hash_word(0, u64::from(value.0)),
        ExpressionNode::Exact(value) => hash_word(1, u64::from(value.0)),
        ExpressionNode::Constant(value) => hash_word(2, u64::from(constant_rank(*value))),
        ExpressionNode::Add(values) => hash_word(3, u64::from(values.0)),
        ExpressionNode::Multiply(values) => hash_word(4, u64::from(values.0)),
        ExpressionNode::Divide {
            numerator,
            denominator,
        } => pair(5, *numerator, *denominator),
        ExpressionNode::Power { base, exponent } => pair(6, *base, *exponent),
        ExpressionNode::LogBase { argument, base } => pair(7, *argument, *base),
        ExpressionNode::Function { function, argument } => hash_word(
            hash_word(8, u64::from(function_rank(*function))),
            u64::from(argument.0),
        ),
        ExpressionNode::BinaryFunction {
            function,
            left,
            right,
        } => pair(
            hash_word(9, u64::from(function_rank(*function))),
            *left,
            *right,
        ),
    }
}

fn constant_rank(constant: Constant) -> u8 {
    match constant {
        Constant::Pi => 0,
        Constant::Euler => 1,
    }
}

fn function_rank(function: Function) -> u8 {
    match function {
        Function::Sin => 0,
        Function::Cos => 1,
        Function::Tan => 2,
        Function::Asin => 3,
        Function::Acos => 4,
        Function::Atan => 5,
        Function::Sqrt => 6,
        Function::Root => 7,
        Function::Exp => 8,
        Function::Log => 9,
        Function::Ln => 10,
        Function::Abs => 11,
        Function::Floor => 12,
        Function::Factorial => 13,
        Function::Permutation => 14,
        Function::Combination => 15,
        Function::Modulo => 16,
        Function::Gcd => 17,
        Function::Lcm => 18,
        Function::Sinh => 19,
        Function::Cosh => 20,
        Function::Tanh => 21,
        Function::Asinh => 22,
        Function::Acosh => 23,
        Function::Atanh => 24,
    }
}

fn additive_numeric_literal(expression: &SourceExpr, negative: bool) -> Option<(&str, bool)> {
    match expression {
        SourceExpr::Number { literal, .. } => Some((literal, negative)),
        SourceExpr::Unary {
            op: UnaryOperator::Plus,
            expr,
            ..
        } => additive_numeric_literal(expr, negative),
        SourceExpr::Unary {
            op: UnaryOperator::Negate,
            expr,
            ..
        } => additive_numeric_literal(expr, !negative),
        SourceExpr::Constant { .. }
        | SourceExpr::Binary { .. }
        | SourceExpr::Percent { .. }
        | SourceExpr::Function { .. } => None,
    }
}

fn rational_add_work(left: &Rational, right: &Rational) -> usize {
    let left_numerator = integer_structural_size(&left.numerator);
    let right_numerator = integer_structural_size(&right.numerator);
    if left.is_integer() && right.is_integer() {
        return left_numerator.max(right_numerator);
    }
    let left_denominator = integer_structural_size(&left.denominator.inner);
    let right_denominator = integer_structural_size(&right.denominator.inner);
    if right.is_integer() {
        let product_size = right_numerator.saturating_add(left_denominator);
        return right_numerator
            .saturating_mul(left_denominator)
            .saturating_add(left_numerator.max(product_size));
    }
    if left.is_integer() {
        let product_size = left_numerator.saturating_add(right_denominator);
        return left_numerator
            .saturating_mul(right_denominator)
            .saturating_add(right_numerator.max(product_size));
    }
    let numerator_size = left_numerator
        .saturating_add(right_denominator)
        .max(right_numerator.saturating_add(left_denominator));
    let denominator_size = left_denominator.saturating_add(right_denominator);
    let cross_products = left_numerator
        .saturating_mul(right_denominator)
        .saturating_add(right_numerator.saturating_mul(left_denominator))
        .saturating_add(left_denominator.saturating_mul(right_denominator));
    let numerator_addition = numerator_size;
    let normalization = numerator_size
        .saturating_mul(denominator_size)
        .saturating_mul(3);
    cross_products
        .saturating_add(numerator_addition)
        .saturating_add(normalization)
}

fn rational_multiply_work(left: &Rational, right: &Rational) -> usize {
    let left_numerator = integer_structural_size(&left.numerator);
    let right_numerator = integer_structural_size(&right.numerator);
    if left.is_zero() || right.is_zero() {
        return 1;
    }
    if left.is_integer() && left.numerator.inner.is_one() {
        return rational_structural_size(right);
    }
    if right.is_integer() && right.numerator.inner.is_one() {
        return rational_structural_size(left);
    }
    if left.is_integer() && right.is_integer() {
        return left_numerator.saturating_mul(right_numerator);
    }
    let left_denominator = integer_structural_size(&left.denominator.inner);
    let right_denominator = integer_structural_size(&right.denominator.inner);
    let numerator_size = left_numerator.saturating_add(right_numerator);
    let denominator_size = left_denominator.saturating_add(right_denominator);
    let products = left_numerator
        .saturating_mul(right_numerator)
        .saturating_add(left_denominator.saturating_mul(right_denominator));
    let normalization = numerator_size
        .saturating_mul(denominator_size)
        .saturating_mul(3);
    products.saturating_add(normalization)
}

fn decimal_literal_error_to_evaluation_error(error: DecimalLiteralError) -> EvaluationError {
    match error {
        DecimalLiteralError::InvalidExponent | DecimalLiteralError::ExponentTooLarge => {
            EvaluationError::InputLimit(InputLimitError {
                kind: InputLimitErrorKind::IntegerTooLarge,
            })
        }
        DecimalLiteralError::Empty | DecimalLiteralError::InvalidDigit => {
            EvaluationError::InternalInvariant(InternalInvariantError {
                code: InternalInvariantCode::InvalidParsedNumberLiteral,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;
    use crate::{syntax::parse_source, types::ParseSettings};

    fn lower(source: &str) -> ExactExpressionDag {
        let parsed = parse_source(source, &ParseSettings::default()).expect(source);
        lower_source_expression(
            &parsed,
            SemanticSettings::default(),
            &ResourceLimits::default(),
        )
        .expect(source)
    }

    #[test]
    fn decimal_addition_lowers_to_rational_addition() {
        let dag = lower("0.1 + 0.2");
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Rational(_)));
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "3/10");
    }

    #[test]
    fn additive_numeric_literals_materialize_only_the_folded_constant() {
        let dag = lower("1.25 - (2 - 0.75) + -0.5 + +1");
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "1/2");
        assert_eq!(dag.nodes.len(), 1);
        assert_eq!(dag.rationals.len(), 1);
        assert!(dag.domain_obligations.is_empty());

        for (source, expected) in [
            ("1-(2-(3-4))", "-2"),
            ("1 + -2 + 3", "2"),
            ("1 + +2 - 3", "0"),
            ("1 - 1 + 0", "0"),
        ] {
            let dag = lower(source);
            assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), expected);
            assert_eq!(dag.nodes.len(), 1);
            assert_eq!(dag.rationals.len(), 1);
        }
    }

    #[test]
    fn additive_literal_fold_keeps_nonliteral_terms_in_the_dag() {
        let dag = lower("1 + sin(1) - 2 + 3");
        assert!(dag.nodes.iter().any(|node| matches!(
            node,
            ExpressionNode::Function {
                function: Function::Sin,
                ..
            }
        )));
        assert!(dag.rationals.iter().any(|value| value.to_string() == "2"));
    }

    #[test]
    fn additive_literal_fold_does_not_resume_after_work_limit() {
        let parsed =
            parse_source("1 + 18446744073709551616 + 0", &ParseSettings::default()).unwrap();
        let dag = lower_source_expression(
            &parsed,
            SemanticSettings::default(),
            &ResourceLimits {
                max_logical_work_units: 2,
                ..ResourceLimits::default()
            },
        )
        .unwrap();
        assert_eq!(
            dag.normalization.limit_reached,
            Some(ComputationLimitKind::LogicalWorkUnits)
        );
        assert!(dag.rationals.iter().any(Rational::is_zero));
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Add(_)));
    }

    #[test]
    fn additive_literal_fallback_is_not_selected_for_an_unrelated_rewrite_limit() {
        let parsed = parse_source("1 + sin(1)^2 + cos(1)^2", &ParseSettings::default()).unwrap();
        let mut builder = DagBuilder {
            semantics: SemanticSettings::default(),
            canonical_budget: CanonicalBudget {
                rewrite_steps_remaining: 0,
                logical_work_remaining: ResourceLimits::default().max_logical_work_units,
            },
            canonical_term_limit: usize::try_from(ResourceLimits::default().max_expression_nodes)
                .unwrap(),
            ..DagBuilder::default()
        };

        let root = builder.lower(&parsed).unwrap();

        assert_eq!(
            builder.canonical_limit_reached,
            Some(ComputationLimitKind::RewriteSteps)
        );
        assert!(!builder.additive_literal_fold_limit_reached);
        assert!(matches!(
            builder.nodes[root.0 as usize],
            ExpressionNode::Add(_)
        ));
    }

    #[test]
    fn additive_literal_mixed_work_covers_product_and_sum_in_both_orders() {
        let integer = Rational::from_decimal_literal(&"9".repeat(2_048)).unwrap();
        let fraction = Rational::from_decimal_literal("0.5").unwrap();
        let integer_size = integer_structural_size(&integer.numerator);
        let forward = rational_add_work(&integer, &fraction);
        let reverse = rational_add_work(&fraction, &integer);
        assert_eq!(forward, reverse);
        assert!(forward >= integer_size.saturating_mul(2));
    }

    #[test]
    fn multiplicative_numeric_literals_materialize_only_the_folded_coefficient() {
        let dag = lower("2 * -3 * +0.5 * 4");
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "-12");
        assert_eq!(dag.nodes.len(), 1);
        assert_eq!(dag.rationals.len(), 1);

        for (source, expected) in [
            ("2*(3*4)", "24"),
            ("-2 * -3 * 4", "24"),
            ("2 * 0 * 999", "0"),
            ("0.25 * 8", "2"),
        ] {
            let dag = lower(source);
            assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), expected);
            assert_eq!(dag.nodes.len(), 1);
            assert_eq!(dag.rationals.len(), 1);
        }
    }

    #[test]
    fn multiplicative_literal_fold_retains_nonliteral_factors() {
        let dag = lower("2 * sin(1) * 3");
        assert!(dag.nodes.iter().any(|node| matches!(
            node,
            ExpressionNode::Function {
                function: Function::Sin,
                ..
            }
        )));
        assert!(dag.rationals.iter().any(|value| value.to_string() == "6"));
    }

    #[test]
    fn multiplicative_literal_fold_does_not_resume_after_work_limit() {
        let parsed =
            parse_source("2 * 18446744073709551616 * 1", &ParseSettings::default()).unwrap();
        let dag = lower_source_expression(
            &parsed,
            SemanticSettings::default(),
            &ResourceLimits {
                max_logical_work_units: 2,
                ..ResourceLimits::default()
            },
        )
        .unwrap();
        assert_eq!(
            dag.normalization.limit_reached,
            Some(ComputationLimitKind::LogicalWorkUnits)
        );
        assert!(dag.rationals.iter().any(|value| *value == Rational::one()));
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Multiply(_)));
    }

    #[test]
    fn multiplicative_literal_fallback_is_selected_after_a_prior_rewrite_limit() {
        let parsed = parse_source("sin(1)^2 * 2", &ParseSettings::default()).unwrap();
        let mut builder = DagBuilder {
            semantics: SemanticSettings::default(),
            canonical_budget: CanonicalBudget {
                rewrite_steps_remaining: 0,
                logical_work_remaining: ResourceLimits::default().max_logical_work_units,
            },
            canonical_term_limit: usize::try_from(ResourceLimits::default().max_expression_nodes)
                .unwrap(),
            ..DagBuilder::default()
        };

        let root = builder.lower(&parsed).unwrap();

        assert_eq!(
            builder.canonical_limit_reached,
            Some(ComputationLimitKind::RewriteSteps)
        );
        assert!(builder.multiplicative_literal_fold_limit_reached);
        assert!(matches!(
            builder.nodes[root.0 as usize],
            ExpressionNode::Multiply(_)
        ));
    }

    #[test]
    fn multiplicative_lowering_preserves_division_and_percent_boundaries() {
        for (source, expected) in [("2*3/4", "3/2"), ("2*(3/4)", "3/2"), ("2*50%*3", "3")] {
            let dag = lower(source);
            assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), expected);
        }
    }

    #[test]
    fn integer_structural_size_matches_signed_byte_limb_boundaries() {
        for shift in [63_usize, 64, 127, 128] {
            let power = BigInt::one() << shift;
            for value in [
                -power.clone() - 1_u8,
                -power.clone(),
                -power.clone() + 1_u8,
                power.clone() - 1_u8,
                power.clone(),
                power.clone() + 1_u8,
            ] {
                let expected = value
                    .to_signed_bytes_le()
                    .len()
                    .saturating_add(core::mem::size_of::<u64>() - 1)
                    .saturating_div(core::mem::size_of::<u64>())
                    .max(1);
                assert_eq!(
                    integer_structural_size(&Integer::from_bigint(value)),
                    expected
                );
            }
        }
    }

    #[test]
    fn canonical_lowering_preserves_children_before_parents() {
        let dag = lower("sin((2^(1/3)+2^(1/3))/2)");
        for (index, node) in dag.nodes.iter().enumerate() {
            let parent = u32::try_from(index).unwrap();
            let children = match node {
                ExpressionNode::Rational(_)
                | ExpressionNode::Exact(_)
                | ExpressionNode::Constant(_) => Vec::new(),
                ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => {
                    dag.list(*values).to_vec()
                }
                ExpressionNode::Divide {
                    numerator,
                    denominator,
                } => vec![*numerator, *denominator],
                ExpressionNode::Power { base, exponent } => vec![*base, *exponent],
                ExpressionNode::LogBase { argument, base } => vec![*argument, *base],
                ExpressionNode::Function { argument, .. } => vec![*argument],
                ExpressionNode::BinaryFunction { left, right, .. } => vec![*left, *right],
            };
            assert!(
                children.iter().all(|child| child.0 < parent),
                "node {index} references a non-child-first id: {node:?}"
            );
        }
    }

    #[test]
    fn canonical_lowering_retains_domain_sensitive_zero_powers() {
        let dag = lower("0^0");
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Power { .. }));
    }

    #[test]
    fn canonical_lowering_records_eliminated_domain_obligations() {
        let dag = lower("(exp(1)*sin(1))/exp(1)");
        assert!(dag.domain_obligations.iter().any(|obligation| {
            matches!(
                obligation,
                DomainObligation::Defined(id)
                    if matches!(dag.node(*id), ExpressionNode::Divide { .. })
            )
        }));
        assert!(dag.domain_obligations.iter().any(|obligation| {
            matches!(
                obligation,
                DomainObligation::NonZero(id)
                    if matches!(
                        dag.node(*id),
                        ExpressionNode::Function {
                            function: Function::Exp,
                            ..
                        }
                    )
            )
        }));
    }

    #[test]
    fn normalization_enforces_nonzero_domain_obligations() {
        let mut dag = lower("(exp(1)*sin(1))/exp(1)");
        let unproven = dag
            .nodes
            .iter()
            .enumerate()
            .find_map(|(index, node)| {
                matches!(
                    node,
                    ExpressionNode::Function {
                        function: Function::Sin,
                        ..
                    }
                )
                .then_some(ExprId(index as u32))
            })
            .expect("source contains a sine factor");
        let obligation = dag
            .domain_obligations
            .iter_mut()
            .find(|obligation| matches!(obligation, DomainObligation::NonZero(_)))
            .expect("quotient cancellation records a non-zero obligation");
        *obligation = DomainObligation::NonZero(unproven);

        let error = normalize_exact_subexpressions(dag, &ResourceLimits::default())
            .expect_err("an unproved non-zero obligation must stop normalization");
        assert_eq!(
            error,
            EvaluationError::InternalInvariant(InternalInvariantError {
                code: InternalInvariantCode::UnprovenDomainObligation,
            })
        );
    }

    #[test]
    fn polynomial_work_estimate_covers_pairwise_factor_comparisons() {
        let wide_monomial = BuilderMonomial {
            coefficient: Rational::one(),
            factors: (0..32).map(|factor| (ExprId(factor), 1)).collect(),
        };
        let wide = BuilderPolynomial {
            terms: vec![wide_monomial.clone()],
        };
        assert!(polynomial_factor_product_work(&BuilderPolynomial::one(), &wide, |_| 1) >= 33);

        let equal_wide_terms = BuilderPolynomial {
            terms: vec![wide_monomial.clone(), wide_monomial],
        };
        assert!(polynomial_term_merge_work(&equal_wide_terms, |_| 1) >= 64);

        let polynomial = BuilderPolynomial {
            terms: (0..32)
                .map(|term| BuilderMonomial {
                    coefficient: Rational::one(),
                    factors: (0..8)
                        .map(|factor| (ExprId(term * 8 + factor), 1))
                        .collect(),
                })
                .collect(),
        };
        let pairwise_factor_comparisons = 32usize * 31 / 2 * 9;
        assert!(polynomial_term_merge_work(&polynomial, |_| 1) >= pairwise_factor_comparisons);

        let shallow = lower("sin(1)");
        let deep = lower("exp(exp(exp(exp(sin(1)))))");
        let repeated_atom = |id| BuilderPolynomial {
            terms: vec![
                BuilderMonomial {
                    coefficient: Rational::one(),
                    factors: vec![(id, 1)],
                },
                BuilderMonomial {
                    coefficient: rational_integer(-1),
                    factors: vec![(id, 1)],
                },
            ],
        };
        let shallow_work = polynomial_term_merge_work(&repeated_atom(shallow.root()), |id| {
            shallow.structural_sizes[id.0 as usize]
        });
        let deep_work = polynomial_term_merge_work(&repeated_atom(deep.root()), |id| {
            deep.structural_sizes[id.0 as usize]
        });
        assert!(deep_work > shallow_work);

        let small_integer = lower("1");
        let large_integer = lower("340282366920938463463374607431768211455");
        assert!(
            large_integer.structural_sizes[large_integer.root().0 as usize]
                > small_integer.structural_sizes[small_integer.root().0 as usize]
        );
    }

    #[test]
    fn trig_complement_plan_reserves_rewrites_only_for_compatible_pairs() {
        for (source, expected_rewrites) in [("sin(1)^2+cos(2)^2", 0), ("sin(1)^2-cos(1)^2", 0)] {
            let dag = lower(source);
            let mut budget = ReductionBudget::new(&ResourceLimits::default(), 0, 0, None);
            let polynomial =
                exact_polynomial_from_expression(&dag, dag.root(), &mut budget, usize::MAX)
                    .unwrap()
                    .unwrap();
            let (rewrites, _) = polynomial_trig_complement_plan(
                &polynomial.terms,
                |id| dag.structural_sizes[id.0 as usize],
                |id, exponent| trig_square_candidate_kind(dag.semantic_node(id), exponent),
                |left, right| trig_complement_monomials_match(&dag, left, right),
            );
            assert_eq!(rewrites, expected_rewrites, "{source}");
        }
    }

    #[test]
    fn percent_lowers_to_division_by_one_hundred() {
        let dag = lower("50%");
        let ExpressionNode::Rational(value) = dag.node(dag.root()) else {
            panic!("expected a canonical rational percent");
        };
        assert_eq!(dag.rational(*value).to_string(), "1/2");
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "1/2");
    }

    #[test]
    fn subtraction_lowers_to_addition_with_negated_rhs() {
        let dag = lower("7 - 2");
        assert!(matches!(dag.node(dag.root()), ExpressionNode::Rational(_)));
        assert_eq!(evaluate_rational_dag(&dag).unwrap().to_string(), "5");
    }

    #[test]
    fn normalization_materializes_nested_real_algebraic_values() {
        let dag = lower("sin((2^(1/3)+2^(1/3))/2)");
        let source_node_count = dag.nodes.len();
        let (dag, normalization) =
            normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert!(dag.nodes.len() >= source_node_count);
        assert_eq!(dag.structural_sizes.len(), dag.nodes.len());
        for (index, node) in dag.nodes.iter().enumerate().skip(source_node_count) {
            assert!(expression_node_children(&dag, node)
                .iter()
                .all(|child| child.0 < index as u32));
        }
        let ExpressionNode::Exact(root) = dag.node(dag.root()) else {
            panic!("expected memoized symbolic root");
        };
        assert!(matches!(dag.exact_value(*root), ExactReduction::Symbolic));
        let ExpressionNode::Function { argument, .. } = dag.exact_presentation(*root) else {
            panic!("expected outer function presentation");
        };
        let ExpressionNode::Exact(value) = dag.node(*argument) else {
            panic!("expected the nested algebraic value to be materialized");
        };
        let ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(value)) =
            dag.exact_value(*value)
        else {
            panic!("expected a stored real algebraic reduction");
        };
        assert_eq!(
            value.minimal_polynomial,
            PrimitivePolynomial::new(vec![
                Integer::from(-2),
                Integer::zero(),
                Integer::zero(),
                Integer::one(),
            ])
            .unwrap()
        );
        assert!(normalization.used_algebraic_reduction());

        let expected = dag.clone();
        let (renormalized, repeated_metadata) =
            normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert_eq!(renormalized, expected);
        assert_eq!(repeated_metadata, normalization);
    }

    #[test]
    fn normalization_idempotence_preserves_rational_and_cyclotomic_provenance() {
        let dag = lower("exp(sin(pi/6)+sin(pi/5))");
        let (dag, metadata) =
            normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert!(metadata.used_special_angle());
        assert!(metadata.used_cyclotomic_reduction());

        let expected = dag.clone();
        let (renormalized, repeated_metadata) =
            normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert_eq!(renormalized, expected);
        assert_eq!(repeated_metadata, metadata);
    }

    #[test]
    fn post_exact_canonical_presentation_removes_reduced_factors() {
        let dag = lower("sin(pi/2)*exp(1)+cos(1)");
        let source_node_count = dag.nodes.len();
        let (dag, _) = normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert!(
            !expression_contains_function(&dag, dag.root(), Function::Sin),
            "normalized root still contains the reduced sine factor: {dag:#?}"
        );
        for (index, node) in dag.nodes.iter().enumerate().skip(source_node_count) {
            let children = expression_node_children(&dag, node);
            assert!(
                children.iter().all(|child| child.0 < index as u32),
                "presentation extension node {index} is not children-before-parent: {node:?}"
            );
        }
        let expected = dag.clone();
        let (renormalized, _) =
            normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        assert_eq!(renormalized, expected);
    }

    #[test]
    fn post_exact_canonical_presentation_absorbs_algebraic_coefficients() {
        let dag = lower("2*sin(pi/6)*2^(1/3)");
        let (dag, _) = normalize_exact_subexpressions(dag, &ResourceLimits::default()).unwrap();
        let ExpressionNode::Exact(root) = dag.node(dag.root()) else {
            panic!("expected a stored exact algebraic root: {dag:#?}");
        };
        assert!(
            matches!(
                dag.exact_value(*root),
                ExactReduction::RealAlgebraic(RealAlgebraicEvaluation::Algebraic(_))
            ),
            "expected real algebraic reduction: {dag:#?}"
        );
        assert!(
            matches!(dag.exact_presentation(*root), ExpressionNode::Exact(_)),
            "canonical algebraic presentation should alias the coefficient-free factor: {dag:#?}"
        );
    }

    fn expression_node_children(dag: &ExactExpressionDag, node: &ExpressionNode) -> Vec<ExprId> {
        match node {
            ExpressionNode::Rational(_)
            | ExpressionNode::Exact(_)
            | ExpressionNode::Constant(_) => Vec::new(),
            ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => {
                dag.list(*values).to_vec()
            }
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => vec![*numerator, *denominator],
            ExpressionNode::Power { base, exponent } => vec![*base, *exponent],
            ExpressionNode::LogBase { argument, base } => vec![*argument, *base],
            ExpressionNode::Function { argument, .. } => vec![*argument],
            ExpressionNode::BinaryFunction { left, right, .. } => vec![*left, *right],
        }
    }

    fn expression_contains_function(
        dag: &ExactExpressionDag,
        id: ExprId,
        expected: Function,
    ) -> bool {
        match dag.semantic_node(id) {
            ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => false,
            ExpressionNode::Exact(value) => {
                expression_node_contains_function(dag, dag.exact_presentation(*value), expected)
            }
            node @ (ExpressionNode::Add(_)
            | ExpressionNode::Multiply(_)
            | ExpressionNode::Divide { .. }
            | ExpressionNode::Power { .. }
            | ExpressionNode::LogBase { .. }
            | ExpressionNode::Function { .. }
            | ExpressionNode::BinaryFunction { .. }) => {
                expression_node_contains_function(dag, node, expected)
            }
        }
    }

    fn expression_node_contains_function(
        dag: &ExactExpressionDag,
        node: &ExpressionNode,
        expected: Function,
    ) -> bool {
        match node {
            ExpressionNode::Rational(_) | ExpressionNode::Constant(_) => false,
            ExpressionNode::Exact(value) => {
                expression_node_contains_function(dag, dag.exact_presentation(*value), expected)
            }
            ExpressionNode::Add(values) | ExpressionNode::Multiply(values) => dag
                .list(*values)
                .iter()
                .any(|child| expression_contains_function(dag, *child, expected)),
            ExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                expression_contains_function(dag, *numerator, expected)
                    || expression_contains_function(dag, *denominator, expected)
            }
            ExpressionNode::Power { base, exponent } => {
                expression_contains_function(dag, *base, expected)
                    || expression_contains_function(dag, *exponent, expected)
            }
            ExpressionNode::LogBase { argument, base } => {
                expression_contains_function(dag, *argument, expected)
                    || expression_contains_function(dag, *base, expected)
            }
            ExpressionNode::Function { function, argument } => {
                *function == expected || expression_contains_function(dag, *argument, expected)
            }
            ExpressionNode::BinaryFunction { left, right, .. } => {
                expression_contains_function(dag, *left, expected)
                    || expression_contains_function(dag, *right, expected)
            }
        }
    }

    #[test]
    fn rational_algebraic_power_reserves_integer_power_before_root() {
        let limits = ResourceLimits {
            max_logical_work_units: 500_000,
            ..ResourceLimits::default()
        };
        let dag = lower("(2^(1/3))^(7/5)");
        let (dag, _) = normalize_exact_subexpressions(dag, &limits).unwrap();
        let root_only_work = u64::from(limits.max_factorization_work)
            + u64::from(limits.max_root_isolation_steps)
            + 5;
        let positive_work = reserved_algebraic_work(&dag, dag.root(), &limits);
        assert!(
            positive_work
                > root_only_work + u64::from(limits.max_root_isolation_steps).saturating_mul(2)
        );

        let dag = lower("(2^(1/3))^(-7/5)");
        let (dag, _) = normalize_exact_subexpressions(dag, &limits).unwrap();
        assert!(reserved_algebraic_work(&dag, dag.root(), &limits) > positive_work);
    }

    #[test]
    fn rational_over_real_algebraic_is_recognized_as_real_algebraic() {
        let dag = lower("1/2^(1/3)");
        let ExpressionNode::Divide {
            numerator,
            denominator,
        } = dag.node(dag.root())
        else {
            panic!("expected divide");
        };
        let denominator =
            evaluate_real_algebraic_node(&dag, *denominator, &ResourceLimits::default())
                .unwrap()
                .expect("denominator should be algebraic")
                .into_algebraic()
                .expect("denominator should remain irrational algebraic");
        assert!(evaluate_node(&dag, *numerator).is_ok());
        assert!(
            reciprocal_real_algebraic(denominator, &ResourceLimits::default())
                .unwrap()
                .is_some()
        );
        assert!(
            evaluate_real_algebraic_dag(&dag, &ResourceLimits::default())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn real_algebraic_over_real_algebraic_is_recognized_as_real_algebraic() {
        let dag = lower("2^(1/3)/4^(1/3)");
        let quotient = evaluate_real_algebraic_dag(&dag, &ResourceLimits::default())
            .unwrap()
            .expect("algebraic quotient should be recognized")
            .into_algebraic()
            .expect("quotient should remain irrational algebraic");

        assert_eq!(
            quotient.minimal_polynomial(),
            &PrimitivePolynomial::new(vec![
                Integer::from(-1),
                Integer::zero(),
                Integer::zero(),
                Integer::from(2),
            ])
            .expect("minimal polynomial normalizes")
        );
    }

    #[test]
    fn interval_add_fold_matches_identity_seeded_bounds() {
        let dag = lower("sin(1)+ln(2)-sqrt(2)");
        let ExpressionNode::Add(list_id) = dag.node(dag.root()) else {
            panic!("expected n-ary addition");
        };

        for precision_bits in [32, 128, 512] {
            let mut expected = interval::from_rational(&Rational::zero(), precision_bits);
            for child in dag.list(*list_id) {
                expected = interval::add(
                    &expected,
                    &evaluate_interval_node(&dag, *child, precision_bits).unwrap(),
                )
                .unwrap();
            }
            assert_eq!(
                evaluate_interval_node(&dag, dag.root(), precision_bits).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn interval_multiply_fold_matches_identity_seeded_bounds() {
        let dag = lower("sin(1)*ln(2)*-sqrt(2)");
        let ExpressionNode::Multiply(list_id) = dag.node(dag.root()) else {
            panic!("expected n-ary multiplication");
        };

        for precision_bits in [32, 128, 512] {
            let mut expected = interval::from_rational(&Rational::one(), precision_bits);
            for child in dag.list(*list_id) {
                expected = interval::multiply(
                    &expected,
                    &evaluate_interval_node(&dag, *child, precision_bits).unwrap(),
                )
                .unwrap();
            }
            assert_eq!(
                evaluate_interval_node(&dag, dag.root(), precision_bits).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn interval_folds_preserve_defensive_empty_and_singleton_semantics() {
        let mut dag = lower("sin(1)");
        let child = dag.root();
        let singleton = dag.push_list(vec![child]);
        let empty = dag.push_list(Vec::new());

        assert_eq!(
            evaluate_interval_expression_node(&dag, &ExpressionNode::Add(empty), None, 128,)
                .unwrap(),
            interval::from_rational(&Rational::zero(), 128)
        );
        assert_eq!(
            evaluate_interval_expression_node(&dag, &ExpressionNode::Multiply(empty), None, 128,)
                .unwrap(),
            interval::from_rational(&Rational::one(), 128)
        );
        let expected = evaluate_interval_node(&dag, child, 128).unwrap();
        assert_eq!(
            evaluate_interval_expression_node(&dag, &ExpressionNode::Add(singleton), None, 128,)
                .unwrap(),
            expected
        );
        assert_eq!(
            evaluate_interval_expression_node(
                &dag,
                &ExpressionNode::Multiply(singleton),
                None,
                128,
            )
            .unwrap(),
            expected
        );

        let zero_rational = RationalId(dag.rationals.len() as u32);
        dag.rationals.push(Rational::zero());
        let zero = ExprId(dag.nodes.len() as u32);
        dag.nodes.push(ExpressionNode::Rational(zero_rational));
        let zero_last = dag.push_list(vec![child, zero]);
        assert_eq!(
            evaluate_interval_expression_node(&dag, &ExpressionNode::Add(zero_last), None, 128,)
                .unwrap(),
            expected
        );
        assert_eq!(
            evaluate_interval_expression_node(
                &dag,
                &ExpressionNode::Multiply(zero_last),
                None,
                128,
            )
            .unwrap(),
            interval::from_rational(&Rational::zero(), 128)
        );
    }
}
