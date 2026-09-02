use crate::answer::AnswerNode;
use crate::exact::{exact_square_root_u128, gcd_u64, square_free_sqrt_decomposition};
use crate::model::{
    ArithmeticExpression, ArithmeticOperator, LinearEquationSurface, LinearExpression,
    LinearScalar, QuadraticEquationSurface, QuadraticExpression, QuadraticSolveMethod,
    RationalCoefficient, SimultaneousSolveMethod,
};
pub const OPERATION_KIND_COUNT: usize = 29;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum OperationKind {
    Identity,
    Count,
    Increment,
    Decrement,
    BasePlus,
    BaseMinus,
    BaseTimes,
    BaseDivide,
    BigNum,
    TimeTen,
    OverheadPf,
    OverheadGcd,
    OverheadLcm,
    OverheadNegative,
    OverheadCarryPlus,
    OverheadCarryMinus,
    OverheadCarryMult,
    Transposition,
    OverheadLinear,
    OverheadEqSystem,
    OverheadFactorPerfectSquare,
    OverheadFactorDifferenceOfSquares,
    OverheadFactorGeneral,
    OverheadQuadratic,
    BaseRoot,
    Compare,
    Reciprocal,
    BaseFractionCancel,
    FractionSelfDivision,
}

impl OperationKind {
    #[cfg(any(test, feature = "qa-diagnostics"))]
    pub const ALL: [Self; OPERATION_KIND_COUNT] = [
        Self::Identity,
        Self::Count,
        Self::Increment,
        Self::Decrement,
        Self::BasePlus,
        Self::BaseMinus,
        Self::BaseTimes,
        Self::BaseDivide,
        Self::BigNum,
        Self::TimeTen,
        Self::OverheadPf,
        Self::OverheadGcd,
        Self::OverheadLcm,
        Self::OverheadNegative,
        Self::OverheadCarryPlus,
        Self::OverheadCarryMinus,
        Self::OverheadCarryMult,
        Self::Transposition,
        Self::OverheadLinear,
        Self::OverheadEqSystem,
        Self::OverheadFactorPerfectSquare,
        Self::OverheadFactorDifferenceOfSquares,
        Self::OverheadFactorGeneral,
        Self::OverheadQuadratic,
        Self::BaseRoot,
        Self::Compare,
        Self::Reciprocal,
        Self::BaseFractionCancel,
        Self::FractionSelfDivision,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    #[cfg(feature = "qa-diagnostics")]
    const fn qa_name(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Count => "count",
            Self::Increment => "increment",
            Self::Decrement => "decrement",
            Self::BasePlus => "base_plus",
            Self::BaseMinus => "base_minus",
            Self::BaseTimes => "base_times",
            Self::BaseDivide => "base_divide",
            Self::BigNum => "big_num",
            Self::TimeTen => "time_ten",
            Self::OverheadPf => "overhead_pf",
            Self::OverheadGcd => "overhead_gcd",
            Self::OverheadLcm => "overhead_lcm",
            Self::OverheadNegative => "overhead_negative",
            Self::OverheadCarryPlus => "overhead_carry_plus",
            Self::OverheadCarryMinus => "overhead_carry_minus",
            Self::OverheadCarryMult => "overhead_carry_mult",
            Self::Transposition => "transposition",
            Self::OverheadLinear => "overhead_linear",
            Self::OverheadEqSystem => "overhead_eq_system",
            Self::OverheadFactorPerfectSquare => "overhead_factor_perfect_square",
            Self::OverheadFactorDifferenceOfSquares => "overhead_factor_difference_of_squares",
            Self::OverheadFactorGeneral => "overhead_factor_general",
            Self::OverheadQuadratic => "overhead_quadratic",
            Self::BaseRoot => "base_root",
            Self::Compare => "compare",
            Self::Reciprocal => "reciprocal",
            Self::BaseFractionCancel => "base_fraction_cancel",
            Self::FractionSelfDivision => "fraction_self_division",
        }
    }
}

#[cfg(feature = "qa-diagnostics")]
/// QA-only names for the current internal operation-vector basis.
/// This is not part of the production Worksheet wire contract.
pub const QA_OPERATION_VECTOR_BASIS: [&str; OPERATION_KIND_COUNT] = {
    let mut names = [""; OPERATION_KIND_COUNT];
    let mut index = 0;
    while index < OPERATION_KIND_COUNT {
        names[index] = OperationKind::ALL[index].qa_name();
        index += 1;
    }
    names
};

/// Dense operation-count vector for the current internal effort basis.
///
/// This is generator-internal diagnostic state, not a Web/WASM wire contract.
#[derive(Clone, Debug, PartialEq)]
pub struct OperationVector {
    values: [f64; OPERATION_KIND_COUNT],
}

impl Default for OperationVector {
    fn default() -> Self {
        Self::zero()
    }
}

impl OperationVector {
    pub const fn zero() -> Self {
        Self {
            values: [0.0; OPERATION_KIND_COUNT],
        }
    }

    #[cfg(test)]
    pub fn get(&self, kind: OperationKind) -> f64 {
        self.values[kind.index()]
    }

    #[cfg(test)]
    pub fn as_array(&self) -> &[f64; OPERATION_KIND_COUNT] {
        &self.values
    }

    #[cfg(feature = "qa-diagnostics")]
    pub(crate) const fn qa_values(&self) -> [f64; OPERATION_KIND_COUNT] {
        self.values
    }

    fn add(&mut self, kind: OperationKind, amount: f64) {
        debug_assert!(amount.is_finite() && amount >= 0.0);
        self.values[kind.index()] += amount;
    }

    #[cfg(test)]
    pub fn is_nonnegative_finite(&self) -> bool {
        self.values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationWeights {
    values: [f64; OPERATION_KIND_COUNT],
}

impl Default for OperationWeights {
    fn default() -> Self {
        let mut values = [1.0; OPERATION_KIND_COUNT];
        values[OperationKind::Count.index()] = 0.2;
        values[OperationKind::BasePlus.index()] = 3.0;
        values[OperationKind::BaseMinus.index()] = 3.1;
        values[OperationKind::BaseTimes.index()] = 3.5;
        values[OperationKind::BaseDivide.index()] = 4.0;
        values[OperationKind::TimeTen.index()] = 1.0;
        values[OperationKind::OverheadPf.index()] = 2.0;
        values[OperationKind::OverheadGcd.index()] = 4.0;
        values[OperationKind::OverheadLcm.index()] = 4.0;
        values[OperationKind::OverheadNegative.index()] = 1.5;
        values[OperationKind::OverheadCarryPlus.index()] = 0.5;
        values[OperationKind::OverheadCarryMinus.index()] = 0.5;
        values[OperationKind::OverheadCarryMult.index()] = 0.5;
        values[OperationKind::Transposition.index()] = 2.0;
        values[OperationKind::OverheadLinear.index()] = 2.0;
        values[OperationKind::OverheadEqSystem.index()] = 4.0;
        values[OperationKind::OverheadFactorPerfectSquare.index()] = 3.0;
        values[OperationKind::OverheadFactorDifferenceOfSquares.index()] = 2.0;
        values[OperationKind::OverheadFactorGeneral.index()] = 5.0;
        values[OperationKind::OverheadQuadratic.index()] = 6.0;
        values[OperationKind::BaseRoot.index()] = 3.0;
        values[OperationKind::Compare.index()] = 1.0;
        values[OperationKind::Reciprocal.index()] = 1.0;
        values[OperationKind::BaseFractionCancel.index()] = 1.0;
        Self { values }
    }
}

impl OperationWeights {
    #[cfg(test)]
    pub fn get(&self, kind: OperationKind) -> f64 {
        self.values[kind.index()]
    }

    #[cfg(test)]
    pub fn override_weight(&mut self, kind: OperationKind, weight: f64) -> Result<(), WeightError> {
        validate_nonnegative_finite(weight)?;
        self.values[kind.index()] = weight;
        Ok(())
    }

    pub fn weighted_sum(&self, vector: &OperationVector) -> f64 {
        self.values
            .iter()
            .zip(vector.values.iter())
            .map(|(weight, count)| weight * count)
            .sum()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Identity,
    Count { amount: u32 },
    Increment,
    Decrement,
    BasePlus,
    BaseMinus,
    BaseTimes,
    BaseDivide,
    BigNum { magnitude: u64 },
    TimeTen { exponent: u32 },
    OverheadPf,
    OverheadGcd,
    OverheadLcm,
    OverheadNegative,
    OverheadCarryPlus,
    OverheadCarryMinus,
    OverheadCarryMult,
    Transposition,
    OverheadLinear,
    OverheadEqSystem,
    OverheadFactorPerfectSquare,
    OverheadFactorDifferenceOfSquares,
    OverheadFactorGeneral,
    OverheadQuadratic,
    BaseRoot,
    Compare,
    Reciprocal,
    BaseFractionCancel,
    FractionSelfDivision,
}

impl Operation {
    fn vector_contribution(&self) -> (OperationKind, f64) {
        match self {
            Self::Identity => (OperationKind::Identity, 1.0),
            Self::Count { amount } => (OperationKind::Count, f64::from(*amount)),
            Self::Increment => (OperationKind::Increment, 1.0),
            Self::Decrement => (OperationKind::Decrement, 1.0),
            Self::BasePlus => (OperationKind::BasePlus, 1.0),
            Self::BaseMinus => (OperationKind::BaseMinus, 1.0),
            Self::BaseTimes => (OperationKind::BaseTimes, 1.0),
            Self::BaseDivide => (OperationKind::BaseDivide, 1.0),
            Self::BigNum { magnitude } => (OperationKind::BigNum, exact_log10_cost(*magnitude)),
            Self::TimeTen { .. } => (OperationKind::TimeTen, 1.0),
            Self::OverheadPf => (OperationKind::OverheadPf, 1.0),
            Self::OverheadGcd => (OperationKind::OverheadGcd, 1.0),
            Self::OverheadLcm => (OperationKind::OverheadLcm, 1.0),
            Self::OverheadNegative => (OperationKind::OverheadNegative, 1.0),
            Self::OverheadCarryPlus => (OperationKind::OverheadCarryPlus, 1.0),
            Self::OverheadCarryMinus => (OperationKind::OverheadCarryMinus, 1.0),
            Self::OverheadCarryMult => (OperationKind::OverheadCarryMult, 1.0),
            Self::Transposition => (OperationKind::Transposition, 1.0),
            Self::OverheadLinear => (OperationKind::OverheadLinear, 1.0),
            Self::OverheadEqSystem => (OperationKind::OverheadEqSystem, 1.0),
            Self::OverheadFactorPerfectSquare => (OperationKind::OverheadFactorPerfectSquare, 1.0),
            Self::OverheadFactorDifferenceOfSquares => {
                (OperationKind::OverheadFactorDifferenceOfSquares, 1.0)
            }
            Self::OverheadFactorGeneral => (OperationKind::OverheadFactorGeneral, 1.0),
            Self::OverheadQuadratic => (OperationKind::OverheadQuadratic, 1.0),
            Self::BaseRoot => (OperationKind::BaseRoot, 1.0),
            Self::Compare => (OperationKind::Compare, 1.0),
            Self::Reciprocal => (OperationKind::Reciprocal, 1.0),
            Self::BaseFractionCancel => (OperationKind::BaseFractionCancel, 1.0),
            Self::FractionSelfDivision => (OperationKind::FractionSelfDivision, 1.0),
        }
    }
}

/// Ordered primitive operations for one standard human solution path.
///
/// AutoDrill currently consumes only the primitive multiset/order for effort
/// accounting; prerequisite DAG edges had no product consumer and are therefore
/// deliberately not represented in the core model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationPlan {
    operations: Vec<Operation>,
}

impl OperationPlan {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self { operations }
    }

    pub fn from_operations(operations: impl IntoIterator<Item = Operation>) -> Self {
        Self::new(operations.into_iter().collect())
    }

    #[cfg(test)]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    pub fn operation_vector(&self) -> OperationVector {
        let mut vector = OperationVector::zero();
        for operation in &self.operations {
            let (kind, contribution) = operation.vector_contribution();
            vector.add(kind, contribution);
            if let Operation::TimeTen { exponent } = operation {
                vector.add(OperationKind::Count, f64::from(*exponent));
            }
        }
        vector
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffortModel {
    Operations(OperationPlan),
    ThemeSpecific(ThemeSpecificEffort),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeSpecificEffort(f64);

impl ThemeSpecificEffort {
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value >= 0.0).then_some(Self(value))
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl EffortModel {
    pub fn operations(plan: OperationPlan) -> Self {
        Self::Operations(plan)
    }

    pub fn theme_specific(value: f64) -> Option<Self> {
        ThemeSpecificEffort::new(value).map(Self::ThemeSpecific)
    }

    pub fn value(&self) -> f64 {
        match self {
            Self::Operations(plan) => {
                OperationWeights::default().weighted_sum(&plan.operation_vector())
            }
            Self::ThemeSpecific(effort) => effort.value(),
        }
    }

    #[cfg(test)]
    pub fn operation_vector(&self) -> OperationVector {
        match self {
            Self::Operations(plan) => plan.operation_vector(),
            Self::ThemeSpecific(_) => OperationVector::zero(),
        }
    }

    #[cfg(feature = "qa-diagnostics")]
    pub(crate) fn qa_operation_vector(&self) -> Option<[f64; OPERATION_KIND_COUNT]> {
        match self {
            Self::Operations(plan) => Some(plan.operation_vector().qa_values()),
            Self::ThemeSpecific(_) => None,
        }
    }

    #[cfg(feature = "qa-diagnostics")]
    pub(crate) const fn qa_model_kind(&self) -> &'static str {
        match self {
            Self::Operations(_) => "operations",
            Self::ThemeSpecific(_) => "theme_specific",
        }
    }

    #[cfg(test)]
    pub fn operation_plan(&self) -> Option<&OperationPlan> {
        match self {
            Self::Operations(plan) => Some(plan),
            Self::ThemeSpecific(_) => None,
        }
    }

    #[cfg(test)]
    pub fn theme_specific_value(&self) -> Option<f64> {
        match self {
            Self::Operations(_) => None,
            Self::ThemeSpecific(effort) => Some(effort.value()),
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffortResult {
    pub value: f64,
    pub operation_vector: OperationVector,
}

#[cfg(test)]
pub(crate) fn calculate_plan_effort(
    plan: &OperationPlan,
    weights: &OperationWeights,
) -> EffortResult {
    let operation_vector = plan.operation_vector();
    EffortResult {
        value: weights.weighted_sum(&operation_vector),
        operation_vector,
    }
}

pub(crate) fn big_num_operations(answer: &AnswerNode) -> Vec<Operation> {
    let mut magnitudes = Vec::new();
    answer.exact_integer_magnitudes(&mut magnitudes);
    magnitudes
        .into_iter()
        .map(|magnitude| Operation::BigNum { magnitude })
        .collect()
}

/// Standard effort model for `ax + b = cx + d`: transpose to `Ax = B`,
/// compute `A = a - c` and `B = d - b`, then divide `B / A`. The operation plan
/// records only the operations prescribed by curriculum.md; exact rational
/// values remain outside Float.
pub(crate) fn linear_equation_plan(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    d: RationalCoefficient,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let coefficient = a.subtract(c)?;
    if coefficient.is_zero() {
        return None;
    }
    let constant = d.subtract(b)?;

    let mut operations = vec![Operation::OverheadLinear];
    if !c.is_zero() {
        operations.push(Operation::Transposition);
        operations.extend(rational_subtraction_operations(a, c, coefficient)?);
    }
    if !b.is_zero() {
        operations.push(Operation::Transposition);
        operations.extend(rational_subtraction_operations(d, b, constant)?);
    }

    if coefficient.numerator() == coefficient.denominator() {
        operations.push(Operation::Identity);
    } else {
        operations.extend(coefficient_division_operations(constant, coefficient)?);
    }
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn linear_scalar_rational(value: LinearScalar) -> Option<RationalCoefficient> {
    match value {
        LinearScalar::Integer { value } => RationalCoefficient::new(value, 1),
        LinearScalar::Fraction { value } => Some(value),
        LinearScalar::ExactDecimal { coefficient, scale } => {
            RationalCoefficient::new(coefficient, 10_i64.checked_pow(scale)?)
        }
    }
}

fn linear_expression_expansion_operations(expression: &LinearExpression) -> Option<Vec<Operation>> {
    match expression {
        LinearExpression::Variable { .. } | LinearExpression::Constant { .. } => Some(Vec::new()),
        LinearExpression::Add { left, right } | LinearExpression::Subtract { left, right } => {
            let mut operations = linear_expression_expansion_operations(left)?;
            operations.extend(linear_expression_expansion_operations(right)?);
            Some(operations)
        }
        LinearExpression::Scale { factor, expression } => {
            let mut operations = linear_expression_expansion_operations(expression)?;
            let factor = linear_scalar_rational(*factor)?;
            let (x_coefficient, y_coefficient, constant) =
                crate::semantics::normalize_linear_expression(expression)?;
            for coefficient in [x_coefficient, y_coefficient, constant] {
                if !coefficient.is_zero() {
                    operations.extend(rational_multiplication_operations(factor, coefficient)?);
                }
            }
            Some(operations)
        }
    }
}

pub(crate) fn linear_expression_equation_plan(
    left: &LinearExpression,
    right: &LinearExpression,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let (a, left_y, b) = crate::semantics::normalize_linear_expression(left)?;
    let (c, right_y, d) = crate::semantics::normalize_linear_expression(right)?;
    if !left_y.is_zero() || !right_y.is_zero() {
        return None;
    }
    let mut operations = linear_expression_expansion_operations(left)?;
    operations.extend(linear_expression_expansion_operations(right)?);
    operations.extend(linear_equation_plan(a, b, c, d, answer)?.operations);
    Some(operation_plan(operations))
}

fn scaled_equation_operations(
    equation: (i64, i64, i64),
    multiplier: u64,
) -> Option<((i64, i64, i64), Vec<Operation>)> {
    if multiplier == 1 {
        return Some((equation, Vec::new()));
    }
    let multiplier_i64 = i64::try_from(multiplier).ok()?;
    let scaled = (
        equation.0.checked_mul(multiplier_i64)?,
        equation.1.checked_mul(multiplier_i64)?,
        equation.2.checked_mul(multiplier_i64)?,
    );
    let mut operations = Vec::new();
    operations.extend(signed_multiplication_operations(equation.0, multiplier_i64));
    operations.extend(signed_multiplication_operations(equation.1, multiplier_i64));
    operations.extend(signed_multiplication_operations(equation.2, multiplier_i64));
    Some((scaled, operations))
}

fn combine_equation_terms(left: i64, right: i64, subtract: bool) -> Option<(i64, Vec<Operation>)> {
    if subtract {
        Some((
            left.checked_sub(right)?,
            signed_subtraction_operations(left, right),
        ))
    } else {
        Some((
            left.checked_add(right)?,
            signed_addition_operations(left, right),
        ))
    }
}

fn substitution_operations(
    unknown_coefficient: i64,
    known_coefficient: i64,
    constant: i64,
    known_value: i64,
) -> Option<Vec<Operation>> {
    if unknown_coefficient == 0 {
        return None;
    }
    let product = known_coefficient.checked_mul(known_value)?;
    let rhs = constant.checked_sub(product)?;
    let mut operations = multiply_or_identity_operations(known_coefficient, known_value);
    operations.extend(signed_subtraction_operations(constant, product));
    operations.extend(divide_or_identity_operations(rhs, unknown_coefficient));
    Some(operations)
}

#[allow(clippy::too_many_arguments)]
fn simultaneous_elimination_strategy_operations(
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    e: i64,
    f: i64,
    eliminate_x: bool,
    x: i64,
    y: i64,
    weights: &OperationWeights,
) -> Option<Vec<Operation>> {
    let (first_eliminate, second_eliminate) = if eliminate_x { (a, d) } else { (b, e) };
    if first_eliminate == 0 || second_eliminate == 0 {
        return None;
    }
    let target = lcm_u64(
        first_eliminate.unsigned_abs(),
        second_eliminate.unsigned_abs(),
    )?;
    let first_multiplier = target / first_eliminate.unsigned_abs();
    let second_multiplier = target / second_eliminate.unsigned_abs();

    let mut operations = vec![Operation::OverheadEqSystem];
    if first_eliminate.unsigned_abs() != second_eliminate.unsigned_abs() {
        operations.extend(lcm_search_operations(
            first_eliminate.unsigned_abs(),
            second_eliminate.unsigned_abs(),
        )?);
    }
    let (first_scaled, first_scale_ops) = scaled_equation_operations((a, b, c), first_multiplier)?;
    let (second_scaled, second_scale_ops) =
        scaled_equation_operations((d, e, f), second_multiplier)?;
    operations.extend(first_scale_ops);
    operations.extend(second_scale_ops);

    let first_scaled_eliminate = if eliminate_x {
        first_scaled.0
    } else {
        first_scaled.1
    };
    let second_scaled_eliminate = if eliminate_x {
        second_scaled.0
    } else {
        second_scaled.1
    };
    let subtract_rows = first_scaled_eliminate.signum() == second_scaled_eliminate.signum();

    let (_, eliminate_ops) = combine_equation_terms(
        first_scaled_eliminate,
        second_scaled_eliminate,
        subtract_rows,
    )?;
    operations.extend(eliminate_ops);
    let first_remaining = if eliminate_x {
        first_scaled.1
    } else {
        first_scaled.0
    };
    let second_remaining = if eliminate_x {
        second_scaled.1
    } else {
        second_scaled.0
    };
    let (remaining_coefficient, remaining_ops) =
        combine_equation_terms(first_remaining, second_remaining, subtract_rows)?;
    operations.extend(remaining_ops);
    let (remaining_constant, constant_ops) =
        combine_equation_terms(first_scaled.2, second_scaled.2, subtract_rows)?;
    operations.extend(constant_ops);
    if remaining_coefficient == 0 {
        return None;
    }
    operations.extend(divide_or_identity_operations(
        remaining_constant,
        remaining_coefficient,
    ));

    let (substitution_first, substitution_second) = if eliminate_x {
        (
            substitution_operations(a, b, c, y)?,
            substitution_operations(d, e, f, y)?,
        )
    } else {
        (
            substitution_operations(b, a, c, x)?,
            substitution_operations(e, d, f, x)?,
        )
    };
    let first_effort =
        weights.weighted_sum(&operation_plan(substitution_first.clone()).operation_vector());
    let second_effort =
        weights.weighted_sum(&operation_plan(substitution_second.clone()).operation_vector());
    if first_effort <= second_effort {
        operations.extend(substitution_first);
    } else {
        operations.extend(substitution_second);
    }
    Some(operations)
}

type IntegerLinearEquation = (i64, i64, i64);

fn clear_linear_equation_denominators(
    equation: &LinearEquationSurface,
) -> Option<(IntegerLinearEquation, Vec<Operation>)> {
    let (x, y, rhs) = crate::semantics::normalize_linear_equation(equation)?;
    let xy_lcm = lcm_u64(x.denominator() as u64, y.denominator() as u64)?;
    let common_denominator = lcm_u64(xy_lcm, rhs.denominator() as u64)?;
    let common_denominator_i64 = i64::try_from(common_denominator).ok()?;
    let mut operations = Vec::new();
    if x.denominator() != y.denominator() {
        operations.extend(lcm_search_operations(
            x.denominator() as u64,
            y.denominator() as u64,
        )?);
    }
    if xy_lcm != rhs.denominator() as u64 {
        operations.extend(lcm_search_operations(xy_lcm, rhs.denominator() as u64)?);
    }

    let scale = |value: RationalCoefficient, operations: &mut Vec<Operation>| -> Option<i64> {
        if common_denominator > 1 && !value.is_zero() {
            operations.extend(rational_multiplication_operations(
                RationalCoefficient::new(common_denominator_i64, 1)?,
                value,
            )?);
        }
        value
            .numerator()
            .checked_mul(common_denominator_i64.checked_div(value.denominator())?)
    };
    Some((
        (
            scale(x, &mut operations)?,
            scale(y, &mut operations)?,
            scale(rhs, &mut operations)?,
        ),
        operations,
    ))
}

fn simultaneous_surface_operations(
    equations: &[LinearEquationSurface; 2],
) -> Option<([IntegerLinearEquation; 2], Vec<Operation>)> {
    let mut operations = Vec::new();
    let mut normalized = [(0_i64, 0_i64, 0_i64); 2];
    for (index, equation) in equations.iter().enumerate() {
        operations.extend(linear_expression_expansion_operations(&equation.left)?);
        operations.extend(linear_expression_expansion_operations(&equation.right)?);
        let (integer_equation, denominator_operations) =
            clear_linear_equation_denominators(equation)?;
        normalized[index] = integer_equation;
        operations.extend(denominator_operations);
    }
    Some((normalized, operations))
}

fn answer_ordered_integer_pair(answer: &AnswerNode) -> Option<(i64, i64)> {
    let AnswerNode::Tuple(values) = answer else {
        return None;
    };
    let [AnswerNode::Integer(x), AnswerNode::Integer(y)] = values.as_slice() else {
        return None;
    };
    Some((*x, *y))
}

fn substitution_candidate_operations(
    isolated: (i64, i64, i64),
    other: (i64, i64, i64),
    isolate_x: bool,
    x: i64,
    y: i64,
) -> Option<Vec<Operation>> {
    let (isolated_coefficient, other_isolated_coefficient, isolated_rhs) = if isolate_x {
        (isolated.0, isolated.1, isolated.2)
    } else {
        (isolated.1, isolated.0, isolated.2)
    };
    if isolated_coefficient == 0
        || other_isolated_coefficient % isolated_coefficient != 0
        || isolated_rhs % isolated_coefficient != 0
    {
        return None;
    }
    let slope = other_isolated_coefficient
        .checked_neg()?
        .checked_div(isolated_coefficient)?;
    let intercept = isolated_rhs.checked_div(isolated_coefficient)?;
    let (other_substituted_coefficient, other_remaining_coefficient, other_rhs) = if isolate_x {
        (other.0, other.1, other.2)
    } else {
        (other.1, other.0, other.2)
    };

    let product_coefficient = other_substituted_coefficient.checked_mul(slope)?;
    let combined_coefficient = product_coefficient.checked_add(other_remaining_coefficient)?;
    if combined_coefficient == 0 {
        return None;
    }
    let product_constant = other_substituted_coefficient.checked_mul(intercept)?;
    let combined_rhs = other_rhs.checked_sub(product_constant)?;
    let remaining_value = if isolate_x { y } else { x };
    let isolated_value = if isolate_x { x } else { y };
    if combined_coefficient.checked_mul(remaining_value)? != combined_rhs
        || slope.checked_mul(remaining_value)?.checked_add(intercept)? != isolated_value
    {
        return None;
    }

    let mut operations = vec![Operation::OverheadEqSystem];
    if isolated_coefficient.unsigned_abs() != 1 {
        for coefficient in [
            isolated_coefficient,
            other_isolated_coefficient,
            isolated_rhs,
        ] {
            if coefficient != 0 {
                operations.extend(divide_or_identity_operations(
                    coefficient,
                    isolated_coefficient,
                ));
            }
        }
    }
    operations.extend(multiply_or_identity_operations(
        other_substituted_coefficient,
        slope,
    ));
    operations.extend(multiply_or_identity_operations(
        other_substituted_coefficient,
        intercept,
    ));
    operations.extend(signed_addition_operations(
        product_coefficient,
        other_remaining_coefficient,
    ));
    operations.extend(signed_subtraction_operations(other_rhs, product_constant));
    operations.extend(divide_or_identity_operations(
        combined_rhs,
        combined_coefficient,
    ));
    operations.extend(multiply_or_identity_operations(slope, remaining_value));
    operations.extend(signed_addition_operations(
        slope.checked_mul(remaining_value)?,
        intercept,
    ));
    Some(operations)
}

fn simultaneous_substitution_strategy_operations(
    equations: [(i64, i64, i64); 2],
    x: i64,
    y: i64,
    weights: &OperationWeights,
) -> Option<Vec<Operation>> {
    let mut candidates = Vec::new();
    for (isolated_index, other_index) in [(0_usize, 1_usize), (1_usize, 0_usize)] {
        for isolate_x in [true, false] {
            if let Some(operations) = substitution_candidate_operations(
                equations[isolated_index],
                equations[other_index],
                isolate_x,
                x,
                y,
            ) {
                let effort =
                    weights.weighted_sum(&operation_plan(operations.clone()).operation_vector());
                candidates.push((effort, operations));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, operations)| operations)
}

pub(crate) fn simultaneous_equation_plan(
    equations: &[LinearEquationSurface; 2],
    solve_method: SimultaneousSolveMethod,
    answer: &AnswerNode,
    weights: &OperationWeights,
) -> Option<OperationPlan> {
    let ((x, y), (normalized, mut operations)) = (
        answer_ordered_integer_pair(answer)?,
        simultaneous_surface_operations(equations)?,
    );
    let (a, b, c) = normalized[0];
    let (d, e, f) = normalized[1];
    let determinant = a.checked_mul(e)?.checked_sub(b.checked_mul(d)?)?;
    if determinant == 0
        || a.checked_mul(x)?.checked_add(b.checked_mul(y)?)? != c
        || d.checked_mul(x)?.checked_add(e.checked_mul(y)?)? != f
    {
        return None;
    }

    let strategy_operations = match solve_method {
        SimultaneousSolveMethod::Elimination => {
            let x_strategy = simultaneous_elimination_strategy_operations(
                a, b, c, d, e, f, true, x, y, weights,
            )?;
            let y_strategy = simultaneous_elimination_strategy_operations(
                a, b, c, d, e, f, false, x, y, weights,
            )?;
            let x_effort =
                weights.weighted_sum(&operation_plan(x_strategy.clone()).operation_vector());
            let y_effort =
                weights.weighted_sum(&operation_plan(y_strategy.clone()).operation_vector());
            if x_effort <= y_effort {
                x_strategy
            } else {
                y_strategy
            }
        }
        SimultaneousSolveMethod::Substitution => {
            simultaneous_substitution_strategy_operations(normalized, x, y, weights)?
        }
    };
    operations.extend(strategy_operations);
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn fraction_reduction_operations(
    raw_numerator: i64,
    raw_denominator: i64,
) -> Option<Vec<Operation>> {
    if raw_numerator == 0 || raw_denominator == 0 || raw_denominator.unsigned_abs() == 1 {
        return Some(Vec::new());
    }
    // Curriculum model: reduction is attempted for every non-unit-numerator
    // fraction. The GCD search therefore remains part of the operation plan even when
    // the eventual GCD is 1.
    if raw_numerator.unsigned_abs() == 1 {
        return Some(Vec::new());
    }
    let divisor = gcd_u64(raw_numerator.unsigned_abs(), raw_denominator.unsigned_abs());
    let mut operations =
        gcd_search_operations(raw_numerator.unsigned_abs(), raw_denominator.unsigned_abs());
    if divisor > 1 {
        operations.extend(signed_division_operations(
            raw_numerator,
            i64::try_from(divisor).ok()?,
        ));
        operations.extend(unsigned_division_operations(
            raw_denominator.unsigned_abs(),
            divisor,
        ));
    }
    Some(operations)
}

fn rational_addition_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    _result: RationalCoefficient,
) -> Option<Vec<Operation>> {
    rational_add_subtract_operations(left, right, false)
}

fn rational_subtraction_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    _result: RationalCoefficient,
) -> Option<Vec<Operation>> {
    rational_add_subtract_operations(left, right, true)
}

fn rational_add_subtract_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    subtract: bool,
) -> Option<Vec<Operation>> {
    let mut operations = Vec::new();
    let common_denominator = if left.denominator() == right.denominator() {
        left.denominator()
    } else {
        operations.extend(lcm_search_operations(
            left.denominator() as u64,
            right.denominator() as u64,
        )?);
        i64::try_from(lcm_u64(
            left.denominator() as u64,
            right.denominator() as u64,
        )?)
        .ok()?
    };
    let left_scale = common_denominator.checked_div(left.denominator())?;
    let right_scale = common_denominator.checked_div(right.denominator())?;
    let left_scaled = left.numerator().checked_mul(left_scale)?;
    let right_scaled = right.numerator().checked_mul(right_scale)?;

    if left.denominator() != right.denominator() {
        operations.extend(multiply_or_identity_operations(
            left.numerator(),
            left_scale,
        ));
        operations.extend(multiply_or_identity_operations(
            right.numerator(),
            right_scale,
        ));
    }

    let raw_numerator = if subtract {
        operations.extend(signed_subtraction_operations(left_scaled, right_scaled));
        left_scaled.checked_sub(right_scaled)?
    } else {
        operations.extend(signed_addition_operations(left_scaled, right_scaled));
        left_scaled.checked_add(right_scaled)?
    };
    operations.extend(fraction_reduction_operations(
        raw_numerator,
        common_denominator,
    )?);
    Some(operations)
}

fn fraction_integer_cancellation_operations(
    fraction: RationalCoefficient,
    integer: RationalCoefficient,
) -> Option<Vec<Operation>> {
    if fraction.denominator() == 1 || integer.denominator() != 1 {
        return None;
    }
    if integer.numerator().unsigned_abs() != fraction.denominator() as u64 {
        return None;
    }
    let mut operations = vec![Operation::BaseFractionCancel];
    if fraction.numerator() < 0 || integer.numerator() < 0 {
        operations.push(Operation::OverheadNegative);
    }
    Some(operations)
}

fn rational_multiplication_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Option<Vec<Operation>> {
    if let Some(operations) = fraction_integer_cancellation_operations(left, right)
        .or_else(|| fraction_integer_cancellation_operations(right, left))
    {
        return Some(operations);
    }

    let raw_numerator = left.numerator().checked_mul(right.numerator())?;
    let raw_denominator = left.denominator().checked_mul(right.denominator())?;
    let mut operations = multiply_or_identity_operations(left.numerator(), right.numerator());
    operations.extend(multiply_or_identity_operations(
        left.denominator(),
        right.denominator(),
    ));
    operations.extend(fraction_reduction_operations(
        raw_numerator,
        raw_denominator,
    )?);
    Some(operations)
}

fn rational_division_operations(
    dividend: RationalCoefficient,
    divisor: RationalCoefficient,
) -> Option<Vec<Operation>> {
    if divisor.is_zero() {
        return None;
    }
    let sign = divisor.numerator().signum();
    let reciprocal = RationalCoefficient::new(
        divisor.denominator().checked_mul(sign)?,
        i64::try_from(divisor.numerator().unsigned_abs()).ok()?,
    )?;
    let mut operations = vec![Operation::Reciprocal];
    operations.extend(rational_multiplication_operations(dividend, reciprocal)?);
    Some(operations)
}

fn lcm_u64(left: u64, right: u64) -> Option<u64> {
    if left == 0 || right == 0 {
        return Some(0);
    }
    left.checked_div(gcd_u64(left, right))?.checked_mul(right)
}

fn decimal_digits(mut value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let mut digits = Vec::new();
    while value > 0 {
        digits.push((value % 10) as u8);
        value /= 10;
    }
    digits
}

fn unsigned_addition_operations(left: u64, right: u64) -> Vec<Operation> {
    let left_digits = decimal_digits(left);
    let right_digits = decimal_digits(right);
    let width = left_digits.len().max(right_digits.len());
    let mut carry = 0_u8;
    let mut operations = Vec::with_capacity(width * 3 + 1);

    for index in 0..width {
        let left_digit = left_digits.get(index).copied().unwrap_or(0);
        let right_digit = right_digits.get(index).copied().unwrap_or(0);
        let incoming_carry = carry;

        let (base_sum, has_real_addition) = match (left_digit, right_digit) {
            (0, 0) => (0, false),
            (0, value) | (value, 0) => (value, false),
            (left_digit, right_digit) => {
                operations.push(Operation::BasePlus);
                (left_digit + right_digit, true)
            }
        };

        let sum = if incoming_carry == 1 {
            operations.push(Operation::Increment);
            base_sum + 1
        } else {
            if !has_real_addition && base_sum != 0 {
                operations.push(Operation::Identity);
            }
            base_sum
        };
        carry = u8::from(sum >= 10);
        if carry == 1 {
            operations.push(Operation::OverheadCarryPlus);
        }
    }

    if carry == 1 {
        // The final carried 1 has no addend in the next column. Writing it is
        // a transfer, not another addition.
        operations.push(Operation::Identity);
    }
    operations
}

fn base_minus_lookup(left: u64, right: u64) -> bool {
    left >= right && left <= 18 && right <= 9 && left - right <= 9
}

fn unsigned_subtraction_operations(left: u64, right: u64) -> Vec<Operation> {
    debug_assert!(left >= right);
    if right == 0 {
        return vec![Operation::Identity];
    }
    if base_minus_lookup(left, right) {
        return vec![Operation::BaseMinus];
    }

    let left_digits = decimal_digits(left);
    let right_digits = decimal_digits(right);
    let mut borrow = 0_i16;
    let mut operations = Vec::with_capacity(left_digits.len() * 3);
    for (index, left_digit) in left_digits.iter().copied().enumerate() {
        let left_digit = i16::from(left_digit);
        let right_digit = i16::from(right_digits.get(index).copied().unwrap_or(0));
        let adjusted = left_digit - borrow;
        let (column_value, next_borrow) = if adjusted < right_digit {
            operations.push(Operation::Decrement);
            operations.push(Operation::OverheadCarryMinus);
            (adjusted + 10, 1)
        } else {
            (adjusted, 0)
        };

        if right_digit == 0 {
            operations.push(Operation::Identity);
        } else {
            debug_assert!((0..=18).contains(&column_value));
            debug_assert!((0..=9).contains(&(column_value - right_digit)));
            operations.push(Operation::BaseMinus);
        }
        borrow = next_borrow;
    }
    debug_assert_eq!(borrow, 0);
    operations
}

fn signed_addition_operations(left: i64, right: i64) -> Vec<Operation> {
    let left_abs = left.unsigned_abs();
    let right_abs = right.unsigned_abs();
    let mut operations = if (left < 0) == (right < 0) {
        unsigned_addition_operations(left_abs, right_abs)
    } else {
        let mut ops = vec![Operation::Compare];
        match left_abs.cmp(&right_abs) {
            std::cmp::Ordering::Greater => {
                ops.extend(unsigned_subtraction_operations(left_abs, right_abs));
            }
            std::cmp::Ordering::Less => {
                ops.extend(unsigned_subtraction_operations(right_abs, left_abs));
            }
            std::cmp::Ordering::Equal => ops.push(Operation::Identity),
        }
        ops
    };
    // The sole negative-operation exception is the structural a + (-b) form
    // with positive a. Every other addition containing a negative operand pays
    // the sign-handling overhead once for this operation.
    if (left < 0 || right < 0) && !(left > 0 && right < 0) {
        operations.push(Operation::OverheadNegative);
    }
    operations
}

fn signed_subtraction_operations(left: i64, right: i64) -> Vec<Operation> {
    let left_abs = left.unsigned_abs();
    let right_abs = right.unsigned_abs();
    let mut operations = match (left < 0, right < 0) {
        (false, false) => {
            let mut ops = vec![Operation::Compare];
            match left_abs.cmp(&right_abs) {
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => {
                    ops.extend(unsigned_subtraction_operations(left_abs, right_abs));
                }
                std::cmp::Ordering::Less => {
                    ops.extend(unsigned_subtraction_operations(right_abs, left_abs));
                }
            }
            ops
        }
        (false, true) => unsigned_addition_operations(left_abs, right_abs),
        (true, false) => unsigned_addition_operations(left_abs, right_abs),
        (true, true) => {
            let mut ops = vec![Operation::Compare];
            match right_abs.cmp(&left_abs) {
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => {
                    ops.extend(unsigned_subtraction_operations(right_abs, left_abs));
                }
                std::cmp::Ordering::Less => {
                    ops.extend(unsigned_subtraction_operations(left_abs, right_abs));
                }
            }
            ops
        }
    };
    if left < 0 || right < 0 {
        operations.push(Operation::OverheadNegative);
    }
    operations
}

fn multiplication_carry_addition_operations(product: u16, carry: u16) -> Vec<Operation> {
    if carry == 0 {
        return Vec::new();
    }
    if product == 0 {
        return vec![Operation::Identity];
    }
    if carry == 1 {
        return vec![Operation::Increment];
    }

    let ones = product % 10;
    if product < 10 {
        return if ones == 0 {
            vec![Operation::Identity]
        } else {
            vec![Operation::BasePlus]
        };
    }

    if ones == 0 {
        return vec![Operation::Identity];
    }
    let mut operations = vec![Operation::BasePlus];
    if ones + carry >= 10 {
        // The ones-column addition raises the already-existing multiplication
        // carry by one. That arithmetic is an Increment, not another BasePlus.
        operations.push(Operation::Increment);
    }
    operations
}

fn unsigned_multiplication_operations(left: u64, right: u64) -> Vec<Operation> {
    let left_digits = decimal_digits(left);
    let right_digits = decimal_digits(right);
    let mut operations = Vec::new();
    let mut partials = Vec::new();

    for (row, right_digit) in right_digits.iter().copied().enumerate() {
        if right_digit == 0 {
            continue;
        }
        let mut carry = 0_u16;
        for (index, left_digit) in left_digits.iter().copied().enumerate() {
            let product = if left_digit == 0 {
                0_u16
            } else {
                operations.push(Operation::BaseTimes);
                u16::from(left_digit) * u16::from(right_digit)
            };

            operations.extend(multiplication_carry_addition_operations(product, carry));
            let total = product + carry;

            let outgoing_carry = total / 10;
            if outgoing_carry > 0 {
                operations.push(Operation::OverheadCarryMult);
                if index + 1 == left_digits.len() {
                    // No multiplication result remains to receive the final
                    // carry, so the leading carried digits are just written.
                    operations.push(Operation::Identity);
                }
            }
            carry = outgoing_carry;
        }
        let row_value = left
            .checked_mul(u64::from(right_digit))
            .and_then(|value| value.checked_mul(10_u64.checked_pow(row as u32)?))
            .expect("bounded integer multiplication model");
        if row_value != 0 {
            partials.push(row_value);
        }
    }

    if partials.len() > 1 {
        let mut accumulated = partials[0];
        for partial in partials.into_iter().skip(1) {
            operations.extend(unsigned_addition_operations(accumulated, partial));
            accumulated = accumulated
                .checked_add(partial)
                .expect("bounded multiplication partial sum");
        }
    }
    operations
}

fn signed_multiplication_operations(left: i64, right: i64) -> Vec<Operation> {
    let mut operations =
        unsigned_multiplication_operations(left.unsigned_abs(), right.unsigned_abs());
    if left < 0 || right < 0 {
        operations.push(Operation::OverheadNegative);
    }
    operations
}

fn base_divide_lookup(dividend: u64, divisor: u64) -> bool {
    divisor > 0 && divisor <= 9 && dividend.is_multiple_of(divisor) && dividend / divisor <= 9
}

fn remainder_quotient_search_operations() -> Vec<Operation> {
    std::iter::repeat_n(Operation::BaseTimes, 3).collect()
}

fn unsigned_division_operations(dividend: u64, divisor: u64) -> Vec<Operation> {
    debug_assert!(divisor > 0);
    if dividend == 0 {
        return vec![Operation::Identity];
    }
    if base_divide_lookup(dividend, divisor) {
        return vec![Operation::BaseDivide];
    }

    let mut digits = decimal_digits(dividend);
    digits.reverse();
    let mut operations = Vec::new();
    let mut partial = 0_u64;
    let mut quotient_started = false;

    for (index, digit) in digits.into_iter().enumerate() {
        if index > 0 {
            operations.push(Operation::Identity); // bring down the next digit
        }
        partial = partial * 10 + u64::from(digit);
        operations.push(Operation::Compare);
        if partial < divisor {
            if quotient_started {
                operations.push(Operation::Identity); // quotient digit 0
            }
            continue;
        }
        quotient_started = true;

        if base_divide_lookup(partial, divisor) {
            operations.push(Operation::BaseDivide);
            partial = 0;
            continue;
        }

        operations.extend(remainder_quotient_search_operations());
        let quotient_digit = (partial / divisor).min(9);
        let product = divisor
            .checked_mul(quotient_digit)
            .expect("bounded long-division product");
        if partial != product {
            operations.extend(unsigned_subtraction_operations(partial, product));
        }
        partial -= product;
    }
    operations
}

fn signed_division_operations(dividend: i64, divisor: i64) -> Vec<Operation> {
    debug_assert!(divisor != 0);
    let mut operations =
        unsigned_division_operations(dividend.unsigned_abs(), divisor.unsigned_abs());
    if dividend < 0 || divisor < 0 {
        operations.push(Operation::OverheadNegative);
    }
    operations
}

fn divide_or_identity_operations(dividend: i64, divisor: i64) -> Vec<Operation> {
    debug_assert!(divisor != 0);
    if dividend == 0 || divisor.unsigned_abs() == 1 {
        let mut operations = vec![Operation::Identity];
        if dividend < 0 || divisor < 0 {
            operations.push(Operation::OverheadNegative);
        }
        operations
    } else {
        signed_division_operations(dividend, divisor)
    }
}

fn coefficient_division_operations(
    dividend: RationalCoefficient,
    divisor: RationalCoefficient,
) -> Option<Vec<Operation>> {
    if dividend.is_integer() && divisor.is_integer() {
        let left = dividend.numerator();
        let right = divisor.numerator();
        if right != 0 && left % right == 0 {
            return Some(divide_or_identity_operations(left, right));
        }
    }
    rational_division_operations(dividend, divisor)
}

fn multiply_or_identity_operations(left: i64, right: i64) -> Vec<Operation> {
    if left.unsigned_abs() == 1 || right.unsigned_abs() == 1 {
        let mut operations = vec![Operation::Identity];
        if left < 0 || right < 0 {
            operations.push(Operation::OverheadNegative);
        }
        operations
    } else {
        signed_multiplication_operations(left, right)
    }
}

fn multiplication_table_factorization(value: u64) -> Option<Vec<u64>> {
    if value < 2 {
        return Some(Vec::new());
    }
    if value <= 9 {
        let mut factors = Vec::new();
        let mut remaining = value;
        for prime in [2_u64, 3, 5, 7] {
            while remaining.is_multiple_of(prime) {
                factors.push(prime);
                remaining /= prime;
            }
        }
        if remaining == 1 {
            return Some(factors);
        }
        return None;
    }
    for left in 2_u64..=9 {
        if !value.is_multiple_of(left) {
            continue;
        }
        let right = value / left;
        if right > 9 {
            continue;
        }
        let mut factors = multiplication_table_factorization(left)?;
        factors.extend(multiplication_table_factorization(right)?);
        factors.sort_unstable();
        return Some(factors);
    }
    None
}

fn prime_factorization_model(value: u64) -> (Vec<u64>, Vec<Operation>) {
    let mut operations = vec![Operation::OverheadPf];
    if value <= 1 {
        return (Vec::new(), operations);
    }
    if let Some(factors) = multiplication_table_factorization(value) {
        return (factors, operations);
    }

    let mut remaining = value;
    let mut factors = Vec::new();
    let mut candidate = 2_u64;
    while candidate.saturating_mul(candidate) <= remaining {
        if !is_prime(candidate) {
            candidate += 1;
            continue;
        }
        operations.push(Operation::Count { amount: 1 });
        loop {
            if let Some(mut table_factors) = multiplication_table_factorization(remaining) {
                factors.append(&mut table_factors);
                remaining = 1;
                break;
            }
            operations.extend(unsigned_division_operations(remaining, candidate));
            if !remaining.is_multiple_of(candidate) {
                break;
            }
            factors.push(candidate);
            remaining /= candidate;
            if remaining == 1 {
                break;
            }
        }
        if remaining == 1 {
            break;
        }
        candidate += 1;
    }
    if remaining > 1 {
        factors.push(remaining);
    }
    factors.sort_unstable();
    (factors, operations)
}

fn gcd_search_operations(left: u64, right: u64) -> Vec<Operation> {
    debug_assert!(left > 0 && right > 0);
    let mut operations = vec![Operation::OverheadGcd];
    if left == 1 || right == 1 {
        operations.push(Operation::Identity);
        return operations;
    }

    let (left_factors, left_operations) = prime_factorization_model(left);
    let (right_factors, right_operations) = prime_factorization_model(right);
    operations.extend(left_operations);
    operations.extend(right_operations);

    let mut common = Vec::new();
    let (mut left_index, mut right_index) = (0_usize, 0_usize);
    while left_index < left_factors.len() && right_index < right_factors.len() {
        operations.push(Operation::Compare);
        match left_factors[left_index].cmp(&right_factors[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                common.push(left_factors[left_index]);
                operations.push(Operation::Identity);
                left_index += 1;
                right_index += 1;
            }
        }
    }

    if common.len() > 1 {
        let mut product = common[0];
        for factor in common.into_iter().skip(1) {
            let next_product = product
                .checked_mul(factor)
                .expect("common prime-factor product cannot exceed the original GCD operands");
            operations.extend(unsigned_multiplication_operations(product, factor));
            product = next_product;
        }
    }
    operations
}

fn lcm_search_operations(left: u64, right: u64) -> Option<Vec<Operation>> {
    if left == 0 || right == 0 {
        return None;
    }
    let target = lcm_u64(left, right)?;
    let mut operations = vec![Operation::OverheadLcm];
    let mut left_index = 1_u64;
    let mut right_index = 1_u64;
    let mut left_multiple = left;
    let mut right_multiple = right;
    operations.push(Operation::Count { amount: 2 });
    operations.push(Operation::Identity);
    operations.push(Operation::Identity);

    loop {
        operations.push(Operation::Compare);
        if left_multiple == right_multiple {
            debug_assert_eq!(left_multiple, target);
            break;
        }
        if left_multiple < right_multiple {
            left_index = left_index.checked_add(1)?;
            left_multiple = left.checked_mul(left_index)?;
            operations.push(Operation::Count { amount: 1 });
            operations.extend(multiply_or_identity_operations(
                i64::try_from(left).ok()?,
                i64::try_from(left_index).ok()?,
            ));
        } else {
            right_index = right_index.checked_add(1)?;
            right_multiple = right.checked_mul(right_index)?;
            operations.push(Operation::Count { amount: 1 });
            operations.extend(multiply_or_identity_operations(
                i64::try_from(right).ok()?,
                i64::try_from(right_index).ok()?,
            ));
        }
    }
    Some(operations)
}

fn is_prime(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    let mut divisor = 2_u64;
    while divisor * divisor <= value {
        if value.is_multiple_of(divisor) {
            return false;
        }
        divisor += 1;
    }
    true
}

fn square_factor_search_operations(radicand: u64) -> Vec<Operation> {
    let mut operations = vec![Operation::OverheadFactorPerfectSquare];
    let mut remaining = radicand;
    let mut outside_factors = Vec::new();
    let mut prime = 2_u64;
    while prime.saturating_mul(prime) <= radicand {
        if !is_prime(prime) {
            prime += 1;
            continue;
        }
        let square = prime * prime;
        operations.push(Operation::Count { amount: 1 });
        loop {
            if remaining < square {
                break;
            }
            operations.extend(unsigned_division_operations(remaining, square));
            if !remaining.is_multiple_of(square) {
                break;
            }
            operations.push(Operation::Identity);
            outside_factors.push(prime);
            remaining /= square;
        }
        prime += 1;
    }

    if outside_factors.len() > 1 {
        let mut outside = outside_factors[0];
        for factor in outside_factors.into_iter().skip(1) {
            operations.extend(unsigned_multiplication_operations(outside, factor));
            outside = outside
                .checked_mul(factor)
                .expect("bounded square-root outside factor");
        }
    }
    operations
}

fn square_root_operations(radicand: u64) -> Vec<Operation> {
    if let Some(root) = exact_square_root_u128(u128::from(radicand)) {
        if root <= 9 {
            return vec![Operation::BaseRoot];
        }
    }
    square_factor_search_operations(radicand)
}

pub(crate) fn one_digit_subtraction_plan(left: u8, right: u8) -> Option<OperationPlan> {
    if left > 18 || right > 9 || left < right {
        return None;
    }
    let answer = u64::from(left - right);
    let mut operations = unsigned_subtraction_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    Some(operation_plan(operations))
}

pub(crate) fn two_digit_addition_plan(left: u8, right: u8) -> Option<OperationPlan> {
    if !(10..=99).contains(&left) || !(10..=99).contains(&right) {
        return None;
    }
    let answer = u64::from(left) + u64::from(right);
    let mut operations = unsigned_addition_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    Some(operation_plan(operations))
}

fn clear_quadratic_denominators(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
) -> Option<(i64, i64, i64, Vec<Operation>)> {
    let mut operations = Vec::new();
    let ab_lcm = lcm_u64(a.denominator() as u64, b.denominator() as u64)?;
    if a.denominator() != b.denominator() {
        operations.extend(lcm_search_operations(
            a.denominator() as u64,
            b.denominator() as u64,
        )?);
    }
    let common = lcm_u64(ab_lcm, c.denominator() as u64)?;
    if ab_lcm != c.denominator() as u64 {
        operations.extend(lcm_search_operations(ab_lcm, c.denominator() as u64)?);
    }
    let common_i64 = i64::try_from(common).ok()?;
    let scale =
        |coefficient: RationalCoefficient| common_i64.checked_div(coefficient.denominator());
    let a_scale = scale(a)?;
    let b_scale = scale(b)?;
    let c_scale = scale(c)?;
    let scaled_a = a.numerator().checked_mul(a_scale)?;
    let scaled_b = b.numerator().checked_mul(b_scale)?;
    let scaled_c = c.numerator().checked_mul(c_scale)?;
    if common > 1 {
        operations.extend(multiply_or_identity_operations(a.numerator(), a_scale));
        operations.extend(multiply_or_identity_operations(b.numerator(), b_scale));
        operations.extend(multiply_or_identity_operations(c.numerator(), c_scale));
    }
    Some((scaled_a, scaled_b, scaled_c, operations))
}

fn quadratic_expression_has_visible_square(expression: &QuadraticExpression) -> bool {
    match expression {
        QuadraticExpression::Square { .. } => true,
        QuadraticExpression::Linear { .. } => false,
        QuadraticExpression::Add { left, right }
        | QuadraticExpression::Subtract { left, right } => {
            quadratic_expression_has_visible_square(left)
                || quadratic_expression_has_visible_square(right)
        }
        QuadraticExpression::Scale { expression, .. } => {
            quadratic_expression_has_visible_square(expression)
        }
    }
}

fn quadratic_square_root_plan(
    equation: &QuadraticEquationSurface,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    if !quadratic_expression_has_visible_square(&equation.left)
        && !quadratic_expression_has_visible_square(&equation.right)
    {
        return None;
    }
    let (a, b, c) = crate::semantics::normalize_quadratic_equation(equation)?;
    if a.is_zero() {
        return None;
    }
    let mut operations = vec![Operation::OverheadQuadratic];
    let (_, _, _, clearing_operations) = clear_quadratic_denominators(a, b, c)?;
    operations.extend(clearing_operations);

    let two = RationalCoefficient::new(2, 1)?;
    let shift = b.divide(a.multiply(two)?)?;
    let square_constant = c.subtract(a.multiply(shift.multiply(shift)?)?)?;
    let rhs = RationalCoefficient::new(
        square_constant.numerator().checked_neg()?,
        square_constant.denominator(),
    )?;
    if !square_constant.is_zero() {
        operations.push(Operation::Transposition);
    }
    let square_value = rhs.divide(a)?;
    if a.numerator() != a.denominator() {
        operations.extend(coefficient_division_operations(rhs, a)?);
    } else {
        operations.push(Operation::Identity);
    }
    if square_value.denominator() == 1 && square_value.numerator() >= 0 {
        operations.extend(square_root_operations(square_value.numerator() as u64));
    } else {
        operations.push(Operation::BaseRoot);
    }
    if !shift.is_zero() {
        operations.push(Operation::Transposition);
    }
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn quadratic_is_perfect_square(b: i64, c: i64) -> bool {
    if c <= 0 || b % 2 != 0 {
        return false;
    }
    let half = b / 2;
    half.checked_mul(half) == Some(c)
}

pub(crate) fn quadratic_factoring_plan(
    b: i64,
    c: i64,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let mut operations = vec![Operation::OverheadQuadratic];
    if b == 0 && c < 0 && exact_square_root_u128(u128::from(c.unsigned_abs())).is_some() {
        operations.push(Operation::OverheadFactorDifferenceOfSquares);
        operations.extend(square_root_operations(c.unsigned_abs()));
        operations.extend([Operation::Transposition, Operation::Transposition]);
        operations.extend(big_num_operations(answer));
        return Some(operation_plan(operations));
    }
    if quadratic_is_perfect_square(b, c) {
        operations.push(Operation::OverheadFactorPerfectSquare);
        operations.extend(square_root_operations(c as u64));
        operations.push(Operation::Transposition);
        operations.extend(big_num_operations(answer));
        return Some(operation_plan(operations));
    }

    operations.push(Operation::OverheadFactorGeneral);
    if c == 0 {
        operations.push(Operation::Identity);
        operations.push(Operation::Transposition);
        operations.extend(big_num_operations(answer));
        return Some(operation_plan(operations));
    }

    let magnitude = c.unsigned_abs();
    let (prime_factors, factor_operations) = prime_factorization_model(magnitude);
    operations.extend(factor_operations);

    let mut divisors = vec![1_u64];
    for factor in prime_factors {
        let existing = divisors.clone();
        for divisor in existing {
            divisors.push(divisor.checked_mul(factor)?);
        }
        divisors.sort_unstable();
        divisors.dedup();
    }
    operations.push(Operation::Count {
        amount: u32::try_from(divisors.len()).ok()?,
    });
    let mut found = false;
    'factor_search: for divisor in divisors {
        let other = magnitude / divisor;
        if divisor > other || divisor.checked_mul(other) != Some(magnitude) {
            continue;
        }
        let divisor = i64::try_from(divisor).ok()?;
        let other = i64::try_from(other).ok()?;
        let signed_pairs = if c > 0 {
            [
                (divisor, other),
                (divisor.checked_neg()?, other.checked_neg()?),
            ]
        } else {
            [
                (divisor, other.checked_neg()?),
                (divisor.checked_neg()?, other),
            ]
        };
        for (left, right) in signed_pairs {
            operations.extend(signed_addition_operations(left, right));
            operations.push(Operation::Compare);
            if left.checked_add(right) == Some(b) {
                found = true;
                break 'factor_search;
            }
        }
    }
    if !found {
        return None;
    }
    operations.extend([Operation::Transposition, Operation::Transposition]);
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn quadratic_formula_coefficients_plan(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let mut operations = vec![Operation::OverheadQuadratic];
    let (a_int, b_int, c_int, clearing_operations) = clear_quadratic_denominators(a, b, c)?;
    if a_int == 0 {
        return None;
    }
    operations.extend(clearing_operations);

    let b_squared = b_int.checked_mul(b_int)?;
    operations.extend(multiply_or_identity_operations(b_int, b_int));
    let ac = a_int.checked_mul(c_int)?;
    operations.extend(multiply_or_identity_operations(a_int, c_int));
    let four_ac = 4_i64.checked_mul(ac)?;
    operations.extend(multiply_or_identity_operations(4, ac));
    let discriminant = b_squared.checked_sub(four_ac)?;
    if discriminant <= 0 {
        return None;
    }
    operations.extend(signed_subtraction_operations(b_squared, four_ac));
    operations.extend(square_root_operations(discriminant as u64));

    let two_a = 2_i64.checked_mul(a_int)?;
    operations.extend(multiply_or_identity_operations(2, a_int));
    operations.push(Operation::Reciprocal);

    let (sqrt_coefficient, _) = square_free_sqrt_decomposition(discriminant as u64)?;
    let sqrt_coefficient = i64::try_from(sqrt_coefficient).ok()?;
    let constant = b_int.checked_neg()?;
    let first_gcd = if constant.unsigned_abs() == 1 || sqrt_coefficient.unsigned_abs() == 1 {
        1
    } else if constant == 0 {
        sqrt_coefficient.unsigned_abs()
    } else {
        operations.extend(gcd_search_operations(
            constant.unsigned_abs(),
            sqrt_coefficient.unsigned_abs(),
        ));
        gcd_u64(constant.unsigned_abs(), sqrt_coefficient.unsigned_abs())
    };
    let common = if first_gcd <= 1 {
        1
    } else if two_a.unsigned_abs() == 1 {
        first_gcd
    } else {
        operations.extend(gcd_search_operations(first_gcd, two_a.unsigned_abs()));
        gcd_u64(first_gcd, two_a.unsigned_abs())
    };
    if common > 1 {
        let common_i64 = i64::try_from(common).ok()?;
        if constant != 0 {
            operations.extend(divide_or_identity_operations(constant, common_i64));
        }
        operations.extend(divide_or_identity_operations(sqrt_coefficient, common_i64));
        operations.extend(divide_or_identity_operations(two_a, common_i64));
    }

    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn quadratic_factoring_equation_plan(
    equation: &QuadraticEquationSurface,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let (a, b, c) = crate::semantics::normalize_quadratic_equation(equation)?;
    let (mut a_int, mut b_int, mut c_int, mut operations) = clear_quadratic_denominators(a, b, c)?;
    if a_int == 0 {
        return None;
    }
    if a_int != 1 {
        if b_int % a_int != 0 || c_int % a_int != 0 {
            return None;
        }
        if a_int != 0 {
            operations.extend(divide_or_identity_operations(a_int, a_int));
            if b_int != 0 {
                operations.extend(divide_or_identity_operations(b_int, a_int));
            }
            if c_int != 0 {
                operations.extend(divide_or_identity_operations(c_int, a_int));
            }
        }
        b_int /= a_int;
        c_int /= a_int;
        a_int = 1;
    }
    if a_int != 1 {
        return None;
    }
    operations.extend(quadratic_factoring_plan(b_int, c_int, answer)?.operations);
    Some(operation_plan(operations))
}

pub(crate) fn quadratic_equation_plan(
    equation: &QuadraticEquationSurface,
    solve_method: QuadraticSolveMethod,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    match solve_method {
        QuadraticSolveMethod::SquareRoot => quadratic_square_root_plan(equation, answer),
        QuadraticSolveMethod::Factoring => quadratic_factoring_equation_plan(equation, answer),
        QuadraticSolveMethod::Formula => {
            let (a, b, c) = crate::semantics::normalize_quadratic_equation(equation)?;
            quadratic_formula_coefficients_plan(a, b, c, answer)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DecimalEffortValue {
    coefficient: i64,
    scale: u32,
}

fn normalize_decimal_effort(mut value: DecimalEffortValue) -> DecimalEffortValue {
    while value.scale > 0 && value.coefficient % 10 == 0 {
        value.coefficient /= 10;
        value.scale -= 1;
    }
    value
}

fn checked_pow10_i64(exponent: u32) -> Option<i64> {
    10_i64.checked_pow(exponent)
}

fn decimal_effort_to_rational(value: DecimalEffortValue) -> Option<RationalCoefficient> {
    RationalCoefficient::new(value.coefficient, checked_pow10_i64(value.scale)?)
}

fn rational_to_decimal_effort(value: RationalCoefficient) -> Option<DecimalEffortValue> {
    let mut denominator = value.denominator() as u64;
    let mut twos = 0_u32;
    let mut fives = 0_u32;
    while denominator.is_multiple_of(2) {
        denominator /= 2;
        twos += 1;
    }
    while denominator.is_multiple_of(5) {
        denominator /= 5;
        fives += 1;
    }
    if denominator != 1 {
        return None;
    }
    let scale = twos.max(fives);
    let mut coefficient = value.numerator();
    coefficient = coefficient.checked_mul(2_i64.checked_pow(scale - twos)?)?;
    coefficient = coefficient.checked_mul(5_i64.checked_pow(scale - fives)?)?;
    Some(normalize_decimal_effort(DecimalEffortValue {
        coefficient,
        scale,
    }))
}

fn contains_exact_decimal(expression: &ArithmeticExpression) -> bool {
    match expression {
        ArithmeticExpression::ExactDecimal { .. } => true,
        ArithmeticExpression::Binary { left, right, .. } => {
            contains_exact_decimal(left) || contains_exact_decimal(right)
        }
        ArithmeticExpression::Integer { .. } | ArithmeticExpression::Rational { .. } => false,
    }
}

fn decimal_expression_operations(
    expression: &ArithmeticExpression,
) -> Option<(DecimalEffortValue, Vec<Operation>)> {
    match expression {
        ArithmeticExpression::Integer { value } => Some((
            DecimalEffortValue {
                coefficient: *value,
                scale: 0,
            },
            Vec::new(),
        )),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => Some((
            DecimalEffortValue {
                coefficient: *coefficient,
                scale: *scale,
            },
            Vec::new(),
        )),
        ArithmeticExpression::Rational { .. } => None,
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let (left_value, mut operations) = decimal_expression_operations(left)?;
            let (right_value, right_operations) = decimal_expression_operations(right)?;
            operations.extend(right_operations);
            match operator {
                ArithmeticOperator::Add | ArithmeticOperator::Subtract => {
                    let common_scale = left_value.scale.max(right_value.scale);
                    let left_shift = common_scale - left_value.scale;
                    let right_shift = common_scale - right_value.scale;
                    if left_shift + right_shift > 0 {
                        operations.push(Operation::Count {
                            amount: left_shift + right_shift,
                        });
                    }
                    let left_aligned = left_value
                        .coefficient
                        .checked_mul(checked_pow10_i64(left_shift)?)?;
                    let right_aligned = right_value
                        .coefficient
                        .checked_mul(checked_pow10_i64(right_shift)?)?;
                    let coefficient = if *operator == ArithmeticOperator::Add {
                        operations.extend(signed_addition_operations(left_aligned, right_aligned));
                        left_aligned.checked_add(right_aligned)?
                    } else {
                        operations
                            .extend(signed_subtraction_operations(left_aligned, right_aligned));
                        left_aligned.checked_sub(right_aligned)?
                    };
                    Some((
                        normalize_decimal_effort(DecimalEffortValue {
                            coefficient,
                            scale: common_scale,
                        }),
                        operations,
                    ))
                }
                ArithmeticOperator::Multiply => {
                    let coefficient = left_value
                        .coefficient
                        .checked_mul(right_value.coefficient)?;
                    operations.extend(signed_multiplication_operations(
                        left_value.coefficient,
                        right_value.coefficient,
                    ));
                    let scale = left_value.scale.checked_add(right_value.scale)?;
                    if scale > 0 {
                        operations.push(Operation::TimeTen { exponent: scale });
                    }
                    Some((
                        normalize_decimal_effort(DecimalEffortValue { coefficient, scale }),
                        operations,
                    ))
                }
                ArithmeticOperator::Divide => {
                    if right_value.coefficient == 0 {
                        return None;
                    }
                    let quotient = decimal_effort_to_rational(left_value)?
                        .divide(decimal_effort_to_rational(right_value)?)?;
                    let quotient_decimal = rational_to_decimal_effort(quotient)?;
                    if right_value.scale > 0 {
                        // Move the divisor's decimal point to make it an integer,
                        // then move the dividend by exactly the same number of places.
                        operations.push(Operation::TimeTen {
                            exponent: right_value.scale,
                        });
                        operations.push(Operation::TimeTen {
                            exponent: right_value.scale,
                        });
                    }
                    let exponent = i64::from(right_value.scale) + i64::from(quotient_decimal.scale)
                        - i64::from(left_value.scale);
                    let (scaled_dividend, scaled_divisor) = if exponent >= 0 {
                        (
                            left_value
                                .coefficient
                                .checked_mul(checked_pow10_i64(u32::try_from(exponent).ok()?)?)?,
                            right_value.coefficient,
                        )
                    } else {
                        (
                            left_value.coefficient,
                            right_value
                                .coefficient
                                .checked_mul(checked_pow10_i64(u32::try_from(-exponent).ok()?)?)?,
                        )
                    };
                    operations.extend(signed_division_operations(scaled_dividend, scaled_divisor));
                    if quotient_decimal.scale > 0 {
                        operations.push(Operation::TimeTen {
                            exponent: quotient_decimal.scale,
                        });
                    }
                    Some((quotient_decimal, operations))
                }
            }
        }
    }
}

pub(crate) fn arithmetic_expression_plan(
    expression: &ArithmeticExpression,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    let mut operations = if contains_exact_decimal(expression) {
        decimal_expression_operations(expression)?.1
    } else {
        arithmetic_expression_operations(expression)?.1
    };
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn arithmetic_expression_operations(
    expression: &ArithmeticExpression,
) -> Option<(RationalCoefficient, Vec<Operation>)> {
    match expression {
        ArithmeticExpression::Integer { value } => {
            Some((RationalCoefficient::new(*value, 1)?, Vec::new()))
        }
        ArithmeticExpression::Rational { value } => Some((*value, Vec::new())),
        ArithmeticExpression::ExactDecimal { .. } => None,
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let (left_value, mut operations) = arithmetic_expression_operations(left)?;
            let (right_value, right_operations) = arithmetic_expression_operations(right)?;
            operations.extend(right_operations);
            let (result, mut operator_operations) = match operator {
                ArithmeticOperator::Add => {
                    let result = left_value.checked_add(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_addition_operations(left_value.numerator(), right_value.numerator())
                    } else {
                        rational_addition_operations(left_value, right_value, result)?
                    };
                    (result, ops)
                }
                ArithmeticOperator::Subtract => {
                    let result = left_value.subtract(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_subtraction_operations(
                            left_value.numerator(),
                            right_value.numerator(),
                        )
                    } else {
                        rational_subtraction_operations(left_value, right_value, result)?
                    };
                    (result, ops)
                }
                ArithmeticOperator::Multiply => {
                    let result = left_value.multiply(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_multiplication_operations(
                            left_value.numerator(),
                            right_value.numerator(),
                        )
                    } else {
                        rational_multiplication_operations(left_value, right_value)?
                    };
                    (result, ops)
                }
                ArithmeticOperator::Divide => {
                    let result = left_value.divide(right_value)?;
                    let ops = if left_value.is_integer()
                        && right_value.is_integer()
                        && right_value.numerator() != 0
                        && left_value.numerator() % right_value.numerator() == 0
                    {
                        divide_or_identity_operations(
                            left_value.numerator(),
                            right_value.numerator(),
                        )
                    } else {
                        rational_division_operations(left_value, right_value)?
                    };
                    (result, ops)
                }
            };
            operations.append(&mut operator_operations);
            Some((result, operations))
        }
    }
}

fn operation_plan(operations: Vec<Operation>) -> OperationPlan {
    OperationPlan::from_operations(operations)
}

pub(crate) fn one_digit_addition_plan(left: u8, right: u8) -> Option<OperationPlan> {
    if left > 9 || right > 9 {
        return None;
    }
    let answer = u64::from(left) + u64::from(right);
    let mut operations = unsigned_addition_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    Some(operation_plan(operations))
}

/// Signed integer addition after the curriculum's negative rewrite rules.
#[cfg(test)]
pub(crate) fn signed_addition_plan(left: i64, right: i64) -> OperationPlan {
    operation_plan(signed_addition_operations(left, right))
}

/// Signed integer subtraction after the curriculum's negative rewrite rules.
#[cfg(test)]
pub(crate) fn signed_subtraction_plan(left: i64, right: i64) -> OperationPlan {
    operation_plan(signed_subtraction_operations(left, right))
}

#[cfg(test)]
fn integer_addition_plan(left: i64, right: i64) -> OperationPlan {
    operation_plan(signed_addition_operations(left, right))
}

#[cfg(test)]
fn integer_multiplication_plan(left: i64, right: i64) -> OperationPlan {
    operation_plan(signed_multiplication_operations(left, right))
}

/// Long division with a quotient/remainder final answer. The arithmetic work is
/// exactly the shared integer-division model; the tuple contributes only the
/// normal answer read/write cost.
pub(crate) fn integer_division_with_remainder_plan(
    dividend: i64,
    divisor: i64,
    answer: &AnswerNode,
) -> Option<OperationPlan> {
    if divisor == 0 {
        return None;
    }
    let mut operations = divide_or_identity_operations(dividend, divisor);
    operations.extend(big_num_operations(answer));
    Some(operation_plan(operations))
}

fn exact_log10_cost(magnitude: u64) -> f64 {
    if magnitude == 0 {
        0.0
    } else {
        (magnitude as f64).log10()
    }
}

#[cfg(test)]
fn validate_nonnegative_finite(value: f64) -> Result<(), WeightError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(WeightError)
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("operation weight or multiplier must be finite and nonnegative")]
pub(crate) struct WeightError;

#[cfg(test)]
mod effort_model_tests {
    use super::*;
    use crate::answer::AnswerNode;

    fn rational(numerator: i64, denominator: i64) -> RationalCoefficient {
        RationalCoefficient::new(numerator, denominator).unwrap()
    }

    fn assert_only(vector: &OperationVector, expected: &[(OperationKind, f64)]) {
        for kind in OperationKind::ALL {
            let expected_value = expected
                .iter()
                .find_map(|(expected_kind, value)| (*expected_kind == kind).then_some(*value))
                .unwrap_or(0.0);
            assert_eq!(
                vector.get(kind),
                expected_value,
                "unexpected count for {kind:?}"
            );
        }
    }

    #[test]
    fn operation_vector_dimension_matches_current_internal_basis() {
        let current = OperationVector::zero();
        assert_eq!(current.as_array().len(), OPERATION_KIND_COUNT);

        let plan = OperationPlan::new(vec![Operation::FractionSelfDivision]);
        let vector = calculate_plan_effort(&plan, &OperationWeights::default()).operation_vector;
        assert_eq!(vector.get(OperationKind::FractionSelfDivision), 1.0);
        assert_eq!(vector.as_array().len(), OPERATION_KIND_COUNT);
    }

    #[test]
    fn lookup_table_is_bidirectional_for_subtraction_and_division() {
        assert_only(
            &operation_plan(unsigned_subtraction_operations(13, 5)).operation_vector(),
            &[(OperationKind::BaseMinus, 1.0)],
        );
        assert_only(
            &operation_plan(unsigned_division_operations(56, 7)).operation_vector(),
            &[(OperationKind::BaseDivide, 1.0)],
        );
        assert_only(
            &operation_plan(unsigned_division_operations(72, 8)).operation_vector(),
            &[(OperationKind::BaseDivide, 1.0)],
        );
    }

    #[test]
    fn remainder_quotient_search_costs_three_table_probes() {
        assert_only(
            &operation_plan(unsigned_division_operations(7, 3)).operation_vector(),
            &[
                (OperationKind::BaseTimes, 3.0),
                (OperationKind::BaseMinus, 1.0),
                (OperationKind::Compare, 1.0),
            ],
        );
    }

    #[test]
    fn addition_reuses_lookup_and_only_increments_existing_next_column() {
        assert_only(
            &integer_addition_plan(23, 48).operation_vector(),
            &[
                (OperationKind::BasePlus, 2.0),
                (OperationKind::Increment, 1.0),
                (OperationKind::OverheadCarryPlus, 1.0),
            ],
        );
        assert_only(
            &integer_addition_plan(97, 86).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BasePlus, 2.0),
                (OperationKind::Increment, 1.0),
                (OperationKind::OverheadCarryPlus, 2.0),
            ],
        );
        assert_only(
            &integer_addition_plan(23, 4).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BasePlus, 1.0),
            ],
        );
        assert_only(
            &integer_addition_plan(99, 0).operation_vector(),
            &[(OperationKind::Identity, 2.0)],
        );
    }

    #[test]
    fn borrow_and_multiplication_carry_do_not_create_phantom_additions() {
        assert_only(
            &operation_plan(unsigned_subtraction_operations(42, 17)).operation_vector(),
            &[
                (OperationKind::BaseMinus, 2.0),
                (OperationKind::Decrement, 1.0),
                (OperationKind::OverheadCarryMinus, 1.0),
            ],
        );
        assert_only(
            &integer_multiplication_plan(7, 8).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BaseTimes, 1.0),
                (OperationKind::OverheadCarryMult, 1.0),
            ],
        );
    }

    #[test]
    fn fraction_cancellation_uses_its_structural_primitive() {
        let fraction_cancel =
            operation_plan(vec![Operation::BaseFractionCancel]).operation_vector();
        assert_only(
            &fraction_cancel,
            &[(OperationKind::BaseFractionCancel, 1.0)],
        );

        let expression = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Multiply,
            left: Box::new(ArithmeticExpression::Rational {
                value: rational(5, 7),
            }),
            right: Box::new(ArithmeticExpression::Integer { value: 7 }),
        };
        let answer = AnswerNode::Integer(5);
        let vector = arithmetic_expression_plan(&expression, &answer)
            .unwrap()
            .operation_vector();
        assert_eq!(vector.get(OperationKind::BaseFractionCancel), 1.0);
    }

    #[test]
    fn gcd_uses_prime_factorization_not_full_divisor_enumeration() {
        let vector = operation_plan(gcd_search_operations(6, 12)).operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadGcd), 1.0);
        assert_eq!(vector.get(OperationKind::OverheadPf), 2.0);
        assert!(vector.get(OperationKind::BaseTimes) < 10.0);
        assert!(vector.get(OperationKind::Compare) <= 3.0);
    }

    #[test]
    fn factorization_on_multiplication_table_needs_only_pf_overhead() {
        let vector = operation_plan(prime_factorization_model(72).1).operation_vector();
        assert_only(&vector, &[(OperationKind::OverheadPf, 1.0)]);

        let vector = operation_plan(prime_factorization_model(77).1).operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadPf), 1.0);
        assert!(
            vector.get(OperationKind::BaseTimes) > 0.0
                || vector.get(OperationKind::BaseDivide) > 0.0
        );
    }

    #[test]
    fn square_root_simplification_tests_prime_squares() {
        assert_only(
            &operation_plan(square_root_operations(49)).operation_vector(),
            &[(OperationKind::BaseRoot, 1.0)],
        );
        let eight = operation_plan(square_root_operations(8)).operation_vector();
        assert_eq!(eight.get(OperationKind::OverheadFactorPerfectSquare), 1.0);
        assert_eq!(eight.get(OperationKind::Count), 1.0); // test 2^2
        let forty_five = operation_plan(square_root_operations(45)).operation_vector();
        assert_eq!(
            forty_five.get(OperationKind::OverheadFactorPerfectSquare),
            1.0
        );
        assert!(forty_five.get(OperationKind::Count) >= 2.0); // 2^2, 3^2, ...
        let seventy_two = operation_plan(square_root_operations(72)).operation_vector();
        assert!(seventy_two.get(OperationKind::BaseTimes) >= 1.0); // outside 2 * 3
    }

    #[test]
    fn fractions_reuse_shared_gcd_and_cancellation_models() {
        let expression = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Multiply,
            left: Box::new(ArithmeticExpression::Rational {
                value: rational(2, 3),
            }),
            right: Box::new(ArithmeticExpression::Rational {
                value: rational(3, 4),
            }),
        };
        let answer = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let vector = arithmetic_expression_plan(&expression, &answer)
            .unwrap()
            .operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadGcd), 1.0);
        assert_eq!(vector.get(OperationKind::OverheadPf), 2.0);
        assert!(vector.get(OperationKind::BaseTimes) < 10.0);
    }

    #[test]
    fn exact_decimals_reuse_shared_integer_column_arithmetic() {
        let expression = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Add,
            left: Box::new(ArithmeticExpression::ExactDecimal {
                coefficient: 125,
                scale: 2,
            }),
            right: Box::new(ArithmeticExpression::ExactDecimal {
                coefficient: 7,
                scale: 1,
            }),
        };
        let answer = AnswerNode::ExactDecimal {
            coefficient: 195,
            scale: 2,
        };
        let vector = arithmetic_expression_plan(&expression, &answer)
            .unwrap()
            .operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadLcm), 0.0);
        assert_eq!(vector.get(OperationKind::Count), 1.0);
        assert_eq!(vector.get(OperationKind::BasePlus), 1.0);
    }

    #[test]
    fn simultaneous_solve_method_selects_a_method_specific_operation_plan() {
        use crate::model::LinearVariable;

        let variable = |variable| LinearExpression::Variable { variable };
        let constant = |value| LinearExpression::Constant {
            value: LinearScalar::Integer { value },
        };
        let scale = |value, expression| LinearExpression::Scale {
            factor: LinearScalar::Integer { value },
            expression: Box::new(expression),
        };
        let first = LinearEquationSurface {
            left: variable(LinearVariable::X),
            right: LinearExpression::Add {
                left: Box::new(scale(2, variable(LinearVariable::Y))),
                right: Box::new(constant(1)),
            },
        };
        let second = LinearEquationSurface {
            left: LinearExpression::Add {
                left: Box::new(scale(3, variable(LinearVariable::X))),
                right: Box::new(scale(4, variable(LinearVariable::Y))),
            },
            right: constant(13),
        };
        let equations = [first, second];
        let answer = AnswerNode::Tuple(vec![AnswerNode::Integer(3), AnswerNode::Integer(1)]);
        let weights = OperationWeights::default();

        let elimination = simultaneous_equation_plan(
            &equations,
            SimultaneousSolveMethod::Elimination,
            &answer,
            &weights,
        )
        .expect("system supports elimination");
        let substitution = simultaneous_equation_plan(
            &equations,
            SimultaneousSolveMethod::Substitution,
            &answer,
            &weights,
        )
        .expect("isolated first equation supports substitution");

        assert_ne!(elimination.operations, substitution.operations);
        assert_eq!(
            elimination
                .operation_vector()
                .get(OperationKind::OverheadEqSystem),
            1.0
        );
        assert_eq!(
            substitution
                .operation_vector()
                .get(OperationKind::OverheadEqSystem),
            1.0
        );
    }

    #[test]
    fn quadratic_factoring_uses_pf_then_unique_factor_pairs() {
        let answer_20 = AnswerNode::Tuple(vec![AnswerNode::Integer(4), AnswerNode::Integer(5)]);
        let answer_21 = AnswerNode::Tuple(vec![AnswerNode::Integer(3), AnswerNode::Integer(7)]);
        let weights = OperationWeights::default();
        let twenty = calculate_plan_effort(
            &quadratic_factoring_plan(-9, 20, &answer_20).expect("factorable quadratic"),
            &weights,
        );
        let twenty_one = calculate_plan_effort(
            &quadratic_factoring_plan(-10, 21, &answer_21).expect("factorable quadratic"),
            &weights,
        );
        assert_eq!(twenty.operation_vector.get(OperationKind::OverheadPf), 1.0);
        assert_eq!(
            twenty_one.operation_vector.get(OperationKind::OverheadPf),
            1.0
        );
        assert_ne!(
            *twenty.operation_vector.as_array(),
            *twenty_one.operation_vector.as_array()
        );
        assert!(twenty.value.is_finite() && twenty_one.value.is_finite());
    }

    #[test]
    fn unit_prime_factorization_is_trivial_and_safe() {
        let (factors, operations) = prime_factorization_model(1);
        assert!(factors.is_empty());
        assert_only(
            &operation_plan(operations).operation_vector(),
            &[(OperationKind::OverheadPf, 1.0)],
        );
    }

    #[test]
    fn quadratic_special_forms_do_not_run_general_factor_search() {
        let difference = quadratic_factoring_plan(
            0,
            -81,
            &AnswerNode::Tuple(vec![AnswerNode::Integer(-9), AnswerNode::Integer(9)]),
        )
        .expect("difference of squares")
        .operation_vector();
        assert_eq!(
            difference.get(OperationKind::OverheadFactorDifferenceOfSquares),
            1.0
        );
        assert_eq!(difference.get(OperationKind::OverheadFactorGeneral), 0.0);
        assert_eq!(difference.get(OperationKind::BaseRoot), 1.0);
    }

    #[test]
    fn extreme_rational_division_fails_without_panicking() {
        let extreme = RationalCoefficient::new(i64::MIN, 3).expect("valid extreme rational");
        let expression = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Divide,
            left: Box::new(ArithmeticExpression::Rational { value: extreme }),
            right: Box::new(ArithmeticExpression::Rational { value: extreme }),
        };
        assert_eq!(
            arithmetic_expression_plan(&expression, &AnswerNode::Integer(1)),
            None
        );
    }

    #[test]
    fn time_ten_keeps_fixed_and_per_digit_costs_in_separate_vector_coordinates() {
        let plan = OperationPlan::from_operations([Operation::TimeTen { exponent: 3 }]);
        let vector = plan.operation_vector();
        assert_eq!(vector.get(OperationKind::TimeTen), 1.0);
        assert_eq!(vector.get(OperationKind::Count), 3.0);

        let mut weights = OperationWeights::default();
        assert!((calculate_plan_effort(&plan, &weights).value - 1.6).abs() < 1e-12);
        weights
            .override_weight(OperationKind::TimeTen, 2.0)
            .unwrap();
        assert!((calculate_plan_effort(&plan, &weights).value - 2.6).abs() < 1e-12);
    }

    #[test]
    fn all_operation_weights_remain_finite_nonnegative() {
        let weights = OperationWeights::default();
        for kind in OperationKind::ALL {
            assert!(weights.get(kind).is_finite());
            assert!(weights.get(kind) >= 0.0);
        }
    }
}
