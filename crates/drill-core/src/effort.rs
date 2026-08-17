use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::answer::AnswerNode;
use crate::exact::gcd_u64;
use crate::model::{
    ArithmeticExpression, ArithmeticOperator, Problem, QuadraticEquationForm, RationalCoefficient,
};
#[cfg(feature = "wire-types")]
use ts_rs::TS;

pub const OPERATION_KIND_COUNT: usize = crate::schema::OPERATION_KIND_COUNT;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
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
    Round,
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
    OverheadDistribution,
    OverheadEqSystem,
    OverheadFactorPerfectSquare,
    OverheadFactorDifferenceOfSquares,
    OverheadFactorGeneral,
    OverheadQuadratic,
    BaseRoot,
    Compare,
    Reciprocal,
    BaseFractionCancel,
    BaseRootSquareCancel,
    FractionSelfDivision,
}

impl OperationKind {
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
        Self::Round,
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
        Self::OverheadDistribution,
        Self::OverheadEqSystem,
        Self::OverheadFactorPerfectSquare,
        Self::OverheadFactorDifferenceOfSquares,
        Self::OverheadFactorGeneral,
        Self::OverheadQuadratic,
        Self::BaseRoot,
        Self::Compare,
        Self::Reciprocal,
        Self::BaseFractionCancel,
        Self::BaseRootSquareCancel,
        Self::FractionSelfDivision,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// Dense operation-count vector for the current schema.
///
/// AutoDrill is pre-release and does not retain historic wire dimensions. A
/// schema change replaces the previous shape; all serialized vectors therefore
/// have exactly `OPERATION_KIND_COUNT` coordinates.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct OperationVector {
    #[cfg_attr(feature = "wire-types", ts(type = "number[]"))]
    values: [f64; OPERATION_KIND_COUNT],
}

impl Serialize for OperationVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OperationVector", 1)?;
        state.serialize_field("values", &self.values)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for OperationVector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            values: Vec<f64>,
        }

        let values = Repr::deserialize(deserializer)?.values;
        if values.len() != OPERATION_KIND_COUNT {
            return Err(serde::de::Error::custom(
                "operation vector has an unsupported dimension",
            ));
        }
        if !values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(serde::de::Error::custom(
                "operation vector values must be finite and nonnegative",
            ));
        }
        let mut dense = [0.0; OPERATION_KIND_COUNT];
        dense.copy_from_slice(&values);
        Ok(Self { values: dense })
    }
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

    pub fn get(&self, kind: OperationKind) -> f64 {
        self.values[kind.index()]
    }

    pub fn as_array(&self) -> &[f64; OPERATION_KIND_COUNT] {
        &self.values
    }

    fn add(&mut self, kind: OperationKind, amount: f64) {
        debug_assert!(amount.is_finite() && amount >= 0.0);
        self.values[kind.index()] += amount;
    }

    pub fn is_nonnegative_finite(&self) -> bool {
        self.values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationWeights {
    values: [f64; OPERATION_KIND_COUNT],
}

impl<'de> Deserialize<'de> for OperationWeights {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            values: [f64; OPERATION_KIND_COUNT],
        }

        let values = Repr::deserialize(deserializer)?.values;
        if values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            Ok(Self { values })
        } else {
            Err(serde::de::Error::custom(
                "operation weights must be finite and nonnegative",
            ))
        }
    }
}

impl Default for OperationWeights {
    fn default() -> Self {
        let mut values = [1.0; OPERATION_KIND_COUNT];
        values[OperationKind::Count.index()] = 0.2;
        values[OperationKind::BasePlus.index()] = 3.0;
        values[OperationKind::BaseMinus.index()] = 3.1;
        values[OperationKind::BaseTimes.index()] = 3.5;
        values[OperationKind::BaseDivide.index()] = 4.0;
        values[OperationKind::TimeTen.index()] = 0.2;
        values[OperationKind::OverheadPf.index()] = 2.0;
        values[OperationKind::OverheadGcd.index()] = 4.0;
        values[OperationKind::OverheadLcm.index()] = 4.0;
        values[OperationKind::OverheadNegative.index()] = 1.5;
        values[OperationKind::OverheadCarryPlus.index()] = 0.5;
        values[OperationKind::OverheadCarryMinus.index()] = 0.5;
        values[OperationKind::OverheadCarryMult.index()] = 0.5;
        values[OperationKind::Transposition.index()] = 2.0;
        values[OperationKind::OverheadLinear.index()] = 2.0;
        values[OperationKind::OverheadDistribution.index()] = 2.0;
        values[OperationKind::OverheadEqSystem.index()] = 4.0;
        values[OperationKind::OverheadFactorPerfectSquare.index()] = 3.0;
        values[OperationKind::OverheadFactorDifferenceOfSquares.index()] = 2.0;
        values[OperationKind::OverheadFactorGeneral.index()] = 5.0;
        values[OperationKind::OverheadQuadratic.index()] = 6.0;
        values[OperationKind::BaseRoot.index()] = 3.0;
        values[OperationKind::Compare.index()] = 1.0;
        values[OperationKind::Reciprocal.index()] = 1.0;
        values[OperationKind::BaseFractionCancel.index()] = 1.0;
        values[OperationKind::BaseRootSquareCancel.index()] = 1.0;
        Self { values }
    }
}

impl OperationWeights {
    pub fn get(&self, kind: OperationKind) -> f64 {
        self.values[kind.index()]
    }

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

pub type EffortWeights = OperationWeights;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WeightMultipliers {
    values: [f64; OPERATION_KIND_COUNT],
}

impl<'de> Deserialize<'de> for WeightMultipliers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            values: [f64; OPERATION_KIND_COUNT],
        }

        let values = Repr::deserialize(deserializer)?.values;
        if values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            Ok(Self { values })
        } else {
            Err(serde::de::Error::custom(
                "operation multipliers must be finite and nonnegative",
            ))
        }
    }
}

impl Default for WeightMultipliers {
    fn default() -> Self {
        Self {
            values: [1.0; OPERATION_KIND_COUNT],
        }
    }
}

impl WeightMultipliers {
    pub fn override_multiplier(
        &mut self,
        kind: OperationKind,
        multiplier: f64,
    ) -> Result<(), WeightError> {
        validate_nonnegative_finite(multiplier)?;
        self.values[kind.index()] = multiplier;
        Ok(())
    }

    pub fn get(&self, kind: OperationKind) -> f64 {
        self.values[kind.index()]
    }
}

/// Grade/theme/mastery are independent composable layers. Alpha 1.1 uses
/// identity layers; registries may override only the theme multipliers.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct WeightProfile {
    pub grade: WeightMultipliers,
    pub theme: WeightMultipliers,
    pub mastery: WeightMultipliers,
}

impl WeightProfile {
    pub fn resolve(&self, base: &OperationWeights) -> OperationWeights {
        let mut values = [0.0; OPERATION_KIND_COUNT];
        for kind in OperationKind::ALL {
            values[kind.index()] = base.get(kind)
                * self.grade.get(kind)
                * self.theme.get(kind)
                * self.mastery.get(kind);
        }
        OperationWeights { values }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operation {
    Identity,
    Count {
        amount: u32,
    },
    Increment,
    Decrement,
    BasePlus,
    BaseMinus,
    BaseTimes,
    BaseDivide,
    BigNum {
        #[serde(with = "crate::exact::u64_decimal_string")]
        #[cfg_attr(feature = "wire-types", ts(type = "string"))]
        magnitude: u64,
    },
    Round,
    TimeTen {
        exponent: u32,
    },
    OverheadPf,
    OverheadGcd,
    OverheadLcm,
    OverheadNegative,
    OverheadCarryPlus,
    OverheadCarryMinus,
    OverheadCarryMult,
    Transposition,
    OverheadLinear,
    OverheadDistribution {
        terms: u32,
    },
    OverheadEqSystem,
    OverheadFactorPerfectSquare,
    OverheadFactorDifferenceOfSquares,
    OverheadFactorGeneral,
    OverheadQuadratic,
    BaseRoot,
    Compare,
    Reciprocal,
    BaseFractionCancel,
    BaseRootSquareCancel,
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
            Self::Round => (OperationKind::Round, 1.0),
            // 0.2 * (n + 5) == 1 + 0.2n.
            Self::TimeTen { exponent } => (OperationKind::TimeTen, f64::from(*exponent) + 5.0),
            Self::OverheadPf => (OperationKind::OverheadPf, 1.0),
            Self::OverheadGcd => (OperationKind::OverheadGcd, 1.0),
            Self::OverheadLcm => (OperationKind::OverheadLcm, 1.0),
            Self::OverheadNegative => (OperationKind::OverheadNegative, 1.0),
            Self::OverheadCarryPlus => (OperationKind::OverheadCarryPlus, 1.0),
            Self::OverheadCarryMinus => (OperationKind::OverheadCarryMinus, 1.0),
            Self::OverheadCarryMult => (OperationKind::OverheadCarryMult, 1.0),
            Self::Transposition => (OperationKind::Transposition, 1.0),
            Self::OverheadLinear => (OperationKind::OverheadLinear, 1.0),
            Self::OverheadDistribution { terms } => {
                (OperationKind::OverheadDistribution, f64::from(*terms))
            }
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
            Self::BaseRootSquareCancel => (OperationKind::BaseRootSquareCancel, 1.0),
            Self::FractionSelfDivision => (OperationKind::FractionSelfDivision, 1.0),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct SolutionStep {
    pub id: u32,
    pub operation: Operation,
    pub depends_on: Vec<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct SolutionGraph {
    pub steps: Vec<SolutionStep>,
}

impl SolutionGraph {
    pub fn operation_vector(&self) -> OperationVector {
        let mut vector = OperationVector::zero();
        // Every graph node counts exactly once; dependency fan-out never
        // recursively double-counts a shared prerequisite.
        for step in &self.steps {
            let (kind, contribution) = step.operation.vector_contribution();
            vector.add(kind, contribution);
        }
        vector
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EffortResult {
    pub value: f64,
    pub operation_vector: OperationVector,
}

pub fn calculate_effort(problem: &Problem, weights: &OperationWeights) -> EffortResult {
    EffortResult {
        value: weights.weighted_sum(&problem.operation_vector),
        operation_vector: problem.operation_vector.clone(),
    }
}

pub fn calculate_graph_effort(graph: &SolutionGraph, weights: &OperationWeights) -> EffortResult {
    let operation_vector = graph.operation_vector();
    EffortResult {
        value: weights.weighted_sum(&operation_vector),
        operation_vector,
    }
}

pub fn default_effort(problem: &Problem) -> EffortResult {
    calculate_effort(problem, &OperationWeights::default())
}

pub fn big_num_operations(answer: &AnswerNode) -> Vec<Operation> {
    let mut magnitudes = Vec::new();
    answer.exact_integer_magnitudes(&mut magnitudes);
    magnitudes
        .into_iter()
        .map(|magnitude| Operation::BigNum { magnitude })
        .collect()
}

/// Standard effort model for `ax + b = cx + d`: transpose to `Ax = B`,
/// compute `A = a - c` and `B = d - b`, then divide `B / A`.  The graph
/// records only the operations prescribed by curriculum.md; exact rational
/// values remain outside Float.
pub fn linear_equation_graph(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    d: RationalCoefficient,
    answer: &AnswerNode,
) -> SolutionGraph {
    let coefficient = a.subtract(c).expect("bounded rational subtraction");
    debug_assert!(!coefficient.is_zero());
    let constant = d.subtract(b).expect("bounded rational subtraction");

    let mut operations = vec![Operation::OverheadLinear];
    if !c.is_zero() {
        operations.push(Operation::Transposition);
        operations.extend(rational_subtraction_operations(a, c, coefficient));
    }
    if !b.is_zero() {
        operations.push(Operation::Transposition);
        operations.extend(rational_subtraction_operations(d, b, constant));
    }

    if coefficient.numerator == coefficient.denominator {
        operations.push(Operation::Identity);
    } else {
        operations.extend(coefficient_division_operations(constant, coefficient));
    }
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}

fn scaled_equation_operations(
    equation: (i64, i64, i64),
    multiplier: u64,
) -> ((i64, i64, i64), Vec<Operation>) {
    if multiplier == 1 {
        return (equation, Vec::new());
    }
    let multiplier_i64 = i64::try_from(multiplier).expect("bounded elimination multiplier");
    let mut operations = Vec::new();
    operations.extend(signed_multiplication_operations(equation.0, multiplier_i64));
    operations.extend(signed_multiplication_operations(equation.1, multiplier_i64));
    operations.extend(signed_multiplication_operations(equation.2, multiplier_i64));
    (
        (
            equation
                .0
                .checked_mul(multiplier_i64)
                .expect("bounded equation scale"),
            equation
                .1
                .checked_mul(multiplier_i64)
                .expect("bounded equation scale"),
            equation
                .2
                .checked_mul(multiplier_i64)
                .expect("bounded equation scale"),
        ),
        operations,
    )
}

fn combine_equation_terms(left: i64, right: i64, subtract: bool) -> (i64, Vec<Operation>) {
    if subtract {
        (
            left.checked_sub(right)
                .expect("bounded elimination subtraction"),
            signed_subtraction_operations(left, right),
        )
    } else {
        (
            left.checked_add(right)
                .expect("bounded elimination addition"),
            signed_addition_operations(left, right),
        )
    }
}

fn substitution_operations(
    unknown_coefficient: i64,
    known_coefficient: i64,
    constant: i64,
    known_value: i64,
) -> Vec<Operation> {
    let product = known_coefficient
        .checked_mul(known_value)
        .expect("bounded substitution product");
    let rhs = constant
        .checked_sub(product)
        .expect("bounded substitution subtraction");
    let mut operations = multiply_or_identity_operations(known_coefficient, known_value);
    operations.extend(signed_subtraction_operations(constant, product));
    operations.extend(divide_or_identity_operations(rhs, unknown_coefficient));
    operations
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
) -> Vec<Operation> {
    let (first_eliminate, second_eliminate) = if eliminate_x { (a, d) } else { (b, e) };
    let target = lcm_u64(
        first_eliminate.unsigned_abs(),
        second_eliminate.unsigned_abs(),
    )
    .expect("bounded elimination LCM");
    let first_multiplier = target / first_eliminate.unsigned_abs();
    let second_multiplier = target / second_eliminate.unsigned_abs();

    let mut operations = vec![Operation::OverheadEqSystem];
    if first_eliminate.unsigned_abs() != second_eliminate.unsigned_abs() {
        operations.extend(lcm_search_operations(
            first_eliminate.unsigned_abs(),
            second_eliminate.unsigned_abs(),
        ));
    }
    let (first_scaled, first_scale_ops) = scaled_equation_operations((a, b, c), first_multiplier);
    let (second_scaled, second_scale_ops) =
        scaled_equation_operations((d, e, f), second_multiplier);
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
    );
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
        combine_equation_terms(first_remaining, second_remaining, subtract_rows);
    operations.extend(remaining_ops);
    let (remaining_constant, constant_ops) =
        combine_equation_terms(first_scaled.2, second_scaled.2, subtract_rows);
    operations.extend(constant_ops);
    debug_assert_ne!(remaining_coefficient, 0);
    operations.extend(divide_or_identity_operations(
        remaining_constant,
        remaining_coefficient,
    ));

    let (substitution_first, substitution_second) = if eliminate_x {
        (
            substitution_operations(a, b, c, y),
            substitution_operations(d, e, f, y),
        )
    } else {
        (
            substitution_operations(b, a, c, x),
            substitution_operations(e, d, f, x),
        )
    };
    let first_effort =
        weights.weighted_sum(&operations_graph(substitution_first.clone()).operation_vector());
    let second_effort =
        weights.weighted_sum(&operations_graph(substitution_second.clone()).operation_vector());
    if first_effort <= second_effort {
        operations.extend(substitution_first);
    } else {
        operations.extend(substitution_second);
    }
    operations
}

#[allow(clippy::too_many_arguments)]
pub fn simultaneous_equation_graph(
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    e: i64,
    f: i64,
    answer: &AnswerNode,
    weights: &OperationWeights,
) -> SolutionGraph {
    debug_assert!(a != 0 && b != 0 && d != 0 && e != 0);
    let determinant = a
        .checked_mul(e)
        .and_then(|value| value.checked_sub(b.checked_mul(d)?))
        .expect("bounded system determinant");
    debug_assert_ne!(determinant, 0);
    let x_numerator = c
        .checked_mul(e)
        .and_then(|value| value.checked_sub(b.checked_mul(f)?))
        .expect("bounded system x numerator");
    let y_numerator = a
        .checked_mul(f)
        .and_then(|value| value.checked_sub(c.checked_mul(d)?))
        .expect("bounded system y numerator");
    debug_assert_eq!(x_numerator % determinant, 0);
    debug_assert_eq!(y_numerator % determinant, 0);
    let x = x_numerator / determinant;
    let y = y_numerator / determinant;

    let x_strategy =
        simultaneous_elimination_strategy_operations(a, b, c, d, e, f, true, x, y, weights);
    let y_strategy =
        simultaneous_elimination_strategy_operations(a, b, c, d, e, f, false, x, y, weights);
    let x_effort = weights.weighted_sum(&operations_graph(x_strategy.clone()).operation_vector());
    let y_effort = weights.weighted_sum(&operations_graph(y_strategy.clone()).operation_vector());
    let mut operations = if x_effort <= y_effort {
        x_strategy
    } else {
        y_strategy
    };
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}

fn fraction_reduction_operations(raw_numerator: i64, raw_denominator: i64) -> Vec<Operation> {
    if raw_numerator == 0 || raw_denominator == 0 || raw_denominator.unsigned_abs() == 1 {
        return Vec::new();
    }
    // Curriculum model: reduction is attempted for every non-unit-numerator
    // fraction. The GCD search therefore remains part of the graph even when
    // the eventual GCD is 1.
    if raw_numerator.unsigned_abs() == 1 {
        return Vec::new();
    }
    let divisor = gcd_u64(raw_numerator.unsigned_abs(), raw_denominator.unsigned_abs());
    let mut operations =
        gcd_search_operations(raw_numerator.unsigned_abs(), raw_denominator.unsigned_abs());
    if divisor > 1 {
        operations.extend(signed_division_operations(
            raw_numerator,
            i64::try_from(divisor).expect("bounded reduction divisor"),
        ));
        operations.extend(unsigned_division_operations(
            raw_denominator.unsigned_abs(),
            divisor,
        ));
    }
    operations
}

fn rational_addition_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    _result: RationalCoefficient,
) -> Vec<Operation> {
    rational_add_subtract_operations(left, right, false)
}

fn rational_subtraction_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    _result: RationalCoefficient,
) -> Vec<Operation> {
    rational_add_subtract_operations(left, right, true)
}

fn rational_add_subtract_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    subtract: bool,
) -> Vec<Operation> {
    let mut operations = Vec::new();
    let common_denominator = if left.denominator == right.denominator {
        left.denominator
    } else {
        operations.extend(lcm_search_operations(
            left.denominator as u64,
            right.denominator as u64,
        ));
        i64::try_from(
            lcm_u64(left.denominator as u64, right.denominator as u64)
                .expect("bounded rational LCM"),
        )
        .expect("bounded rational denominator")
    };
    let left_scale = common_denominator / left.denominator;
    let right_scale = common_denominator / right.denominator;
    let left_scaled = left
        .numerator
        .checked_mul(left_scale)
        .expect("bounded rational numerator scale");
    let right_scaled = right
        .numerator
        .checked_mul(right_scale)
        .expect("bounded rational numerator scale");

    if left.denominator != right.denominator {
        operations.extend(multiply_or_identity_operations(left.numerator, left_scale));
        operations.extend(multiply_or_identity_operations(
            right.numerator,
            right_scale,
        ));
    }

    let raw_numerator = if subtract {
        operations.extend(signed_subtraction_operations(left_scaled, right_scaled));
        left_scaled
            .checked_sub(right_scaled)
            .expect("bounded rational subtraction")
    } else {
        operations.extend(signed_addition_operations(left_scaled, right_scaled));
        left_scaled
            .checked_add(right_scaled)
            .expect("bounded rational addition")
    };
    operations.extend(fraction_reduction_operations(
        raw_numerator,
        common_denominator,
    ));
    operations
}

fn fraction_integer_cancellation_operations(
    fraction: RationalCoefficient,
    integer: RationalCoefficient,
) -> Option<Vec<Operation>> {
    if fraction.denominator == 1 || integer.denominator != 1 {
        return None;
    }
    if integer.numerator.unsigned_abs() != fraction.denominator as u64 {
        return None;
    }
    let mut operations = vec![Operation::BaseFractionCancel];
    if fraction.numerator < 0 || integer.numerator < 0 {
        operations.push(Operation::OverheadNegative);
    }
    Some(operations)
}

fn rational_multiplication_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Vec<Operation> {
    if let Some(operations) = fraction_integer_cancellation_operations(left, right)
        .or_else(|| fraction_integer_cancellation_operations(right, left))
    {
        return operations;
    }

    let raw_numerator = left
        .numerator
        .checked_mul(right.numerator)
        .expect("bounded rational product numerator");
    let raw_denominator = left
        .denominator
        .checked_mul(right.denominator)
        .expect("bounded rational product denominator");
    let mut operations = multiply_or_identity_operations(left.numerator, right.numerator);
    operations.extend(multiply_or_identity_operations(
        left.denominator,
        right.denominator,
    ));
    operations.extend(fraction_reduction_operations(
        raw_numerator,
        raw_denominator,
    ));
    operations
}

fn rational_division_operations(
    dividend: RationalCoefficient,
    divisor: RationalCoefficient,
) -> Vec<Operation> {
    debug_assert!(divisor.numerator != 0);
    let sign = divisor.numerator.signum();
    let reciprocal = RationalCoefficient::new(
        divisor
            .denominator
            .checked_mul(sign)
            .expect("bounded reciprocal numerator"),
        i64::try_from(divisor.numerator.unsigned_abs()).expect("bounded reciprocal denominator"),
    )
    .expect("nonzero divisor has a reciprocal");
    let mut operations = vec![Operation::Reciprocal];
    operations.extend(rational_multiplication_operations(dividend, reciprocal));
    operations
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
) -> Vec<Operation> {
    if dividend.is_integer() && divisor.is_integer() {
        let left = dividend.numerator;
        let right = divisor.numerator;
        if right != 0 && left % right == 0 {
            return divide_or_identity_operations(left, right);
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
            operations.extend(unsigned_multiplication_operations(product, factor));
            product = product
                .checked_mul(factor)
                .expect("bounded GCD factor product");
        }
    }
    operations
}

fn lcm_search_operations(left: u64, right: u64) -> Vec<Operation> {
    debug_assert!(left > 0 && right > 0);
    let target = lcm_u64(left, right).expect("bounded LCM");
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
            left_index += 1;
            operations.push(Operation::Count { amount: 1 });
            operations.extend(multiply_or_identity_operations(
                i64::try_from(left).expect("bounded LCM operand"),
                i64::try_from(left_index).expect("bounded LCM multiplier"),
            ));
            left_multiple = left * left_index;
        } else {
            right_index += 1;
            operations.push(Operation::Count { amount: 1 });
            operations.extend(multiply_or_identity_operations(
                i64::try_from(right).expect("bounded LCM operand"),
                i64::try_from(right_index).expect("bounded LCM multiplier"),
            ));
            right_multiple = right * right_index;
        }
    }
    operations
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

fn exact_square_root_u64(value: u64) -> Option<u64> {
    let root = (value as f64).sqrt() as u64;
    [root.saturating_sub(1), root, root.saturating_add(1)]
        .into_iter()
        .find(|candidate| candidate.checked_mul(*candidate) == Some(value))
}

fn square_root_decomposition(mut value: u64) -> (u64, u64) {
    let mut outside = 1_u64;
    let mut factor = 2_u64;
    while factor.saturating_mul(factor) <= value {
        let square = factor * factor;
        while value.is_multiple_of(square) {
            value /= square;
            outside = outside
                .checked_mul(factor)
                .expect("bounded square-root decomposition");
        }
        factor += 1;
    }
    (outside, value)
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
    if let Some(root) = exact_square_root_u64(radicand) {
        if root <= 9 {
            return vec![Operation::BaseRoot];
        }
    }
    square_factor_search_operations(radicand)
}

pub fn one_digit_subtraction_graph(left: u8, right: u8) -> SolutionGraph {
    let answer = u64::from(left - right);
    let mut operations = unsigned_subtraction_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    operations_graph(operations)
}

pub fn two_digit_addition_graph(left: u8, right: u8) -> SolutionGraph {
    let answer = u64::from(left) + u64::from(right);
    let mut operations = unsigned_addition_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    operations_graph(operations)
}

fn clear_quadratic_denominators(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
) -> (i64, i64, i64, Vec<Operation>) {
    let mut operations = Vec::new();
    let ab_lcm = lcm_u64(a.denominator as u64, b.denominator as u64)
        .expect("bounded quadratic denominator LCM");
    if a.denominator != b.denominator {
        operations.extend(lcm_search_operations(
            a.denominator as u64,
            b.denominator as u64,
        ));
    }
    let common = lcm_u64(ab_lcm, c.denominator as u64).expect("bounded quadratic denominator LCM");
    if ab_lcm != c.denominator as u64 {
        operations.extend(lcm_search_operations(ab_lcm, c.denominator as u64));
    }
    let common_i64 = i64::try_from(common).expect("bounded quadratic denominator");
    let scale = |coefficient: RationalCoefficient| {
        common_i64
            .checked_div(coefficient.denominator)
            .expect("quadratic denominator divides LCM")
    };
    let a_scale = scale(a);
    let b_scale = scale(b);
    let c_scale = scale(c);
    if common > 1 {
        operations.extend(multiply_or_identity_operations(a.numerator, a_scale));
        operations.extend(multiply_or_identity_operations(b.numerator, b_scale));
        operations.extend(multiply_or_identity_operations(c.numerator, c_scale));
    }
    (
        a.numerator
            .checked_mul(a_scale)
            .expect("bounded cleared quadratic coefficient"),
        b.numerator
            .checked_mul(b_scale)
            .expect("bounded cleared quadratic coefficient"),
        c.numerator
            .checked_mul(c_scale)
            .expect("bounded cleared quadratic coefficient"),
        operations,
    )
}

pub fn quadratic_square_graph(
    form: QuadraticEquationForm,
    a: RationalCoefficient,
    c: RationalCoefficient,
    answer: &AnswerNode,
) -> SolutionGraph {
    debug_assert!(matches!(
        form,
        QuadraticEquationForm::SquareEqualsConstant | QuadraticEquationForm::SquarePlusConstantZero
    ));
    debug_assert!(!a.is_zero());
    let mut operations = vec![Operation::OverheadQuadratic];
    let rhs = if form == QuadraticEquationForm::SquarePlusConstantZero {
        operations.push(Operation::Transposition);
        RationalCoefficient::new(
            c.numerator
                .checked_neg()
                .expect("bounded quadratic constant"),
            c.denominator,
        )
        .expect("valid transposed quadratic constant")
    } else {
        c
    };
    let square_value = rhs.divide(a).expect("nonzero quadratic coefficient");
    if a.numerator != a.denominator {
        operations.extend(coefficient_division_operations(rhs, a));
    } else {
        operations.push(Operation::Identity);
    }
    if square_value.denominator == 1 && square_value.numerator >= 0 {
        operations.extend(square_root_operations(square_value.numerator as u64));
    } else {
        operations.push(Operation::BaseRoot);
    }
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}

fn quadratic_is_perfect_square(b: i64, c: i64) -> bool {
    if c <= 0 || b % 2 != 0 {
        return false;
    }
    let half = b / 2;
    half.checked_mul(half) == Some(c)
}

pub fn quadratic_factoring_graph(b: i64, c: i64, answer: &AnswerNode) -> SolutionGraph {
    let mut operations = vec![Operation::OverheadQuadratic];
    if b == 0 && c < 0 && exact_square_root_u64(c.unsigned_abs()).is_some() {
        operations.push(Operation::OverheadFactorDifferenceOfSquares);
        operations.extend(square_root_operations(c.unsigned_abs()));
        operations.extend([Operation::Transposition, Operation::Transposition]);
        operations.extend(big_num_operations(answer));
        return operations_graph(operations);
    }
    if quadratic_is_perfect_square(b, c) {
        operations.push(Operation::OverheadFactorPerfectSquare);
        operations.extend(square_root_operations(c as u64));
        operations.push(Operation::Transposition);
        operations.extend(big_num_operations(answer));
        return operations_graph(operations);
    }

    operations.push(Operation::OverheadFactorGeneral);
    if c == 0 {
        operations.push(Operation::Identity);
        operations.push(Operation::Transposition);
        operations.extend(big_num_operations(answer));
        return operations_graph(operations);
    }

    let magnitude = c.unsigned_abs();
    let (prime_factors, factor_operations) = prime_factorization_model(magnitude);
    operations.extend(factor_operations);

    let mut divisors = vec![1_u64];
    for factor in prime_factors {
        let existing = divisors.clone();
        for divisor in existing {
            divisors.push(
                divisor
                    .checked_mul(factor)
                    .expect("bounded quadratic divisor"),
            );
        }
        divisors.sort_unstable();
        divisors.dedup();
    }
    operations.push(Operation::Count {
        amount: u32::try_from(divisors.len()).expect("bounded divisor count"),
    });
    let mut found = false;
    'factor_search: for divisor in divisors {
        let other = magnitude / divisor;
        if divisor > other || divisor * other != magnitude {
            continue;
        }
        let divisor = i64::try_from(divisor).expect("bounded factor divisor");
        let other = i64::try_from(other).expect("bounded factor partner");
        let signed_pairs = if c > 0 {
            [(divisor, other), (-divisor, -other)]
        } else {
            [(divisor, -other), (-divisor, other)]
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
    debug_assert!(found, "reverse-generated quadratic must have a factor pair");
    operations.extend([Operation::Transposition, Operation::Transposition]);
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}

pub fn quadratic_formula_graph(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    answer: &AnswerNode,
) -> SolutionGraph {
    let mut operations = vec![Operation::OverheadQuadratic];
    let (a_int, b_int, c_int, clearing_operations) = clear_quadratic_denominators(a, b, c);
    operations.extend(clearing_operations);

    let b_squared = b_int.checked_mul(b_int).expect("bounded b squared");
    operations.extend(multiply_or_identity_operations(b_int, b_int));
    let ac = a_int.checked_mul(c_int).expect("bounded ac");
    operations.extend(multiply_or_identity_operations(a_int, c_int));
    let four_ac = 4_i64.checked_mul(ac).expect("bounded 4ac");
    operations.extend(multiply_or_identity_operations(4, ac));
    let discriminant = b_squared
        .checked_sub(four_ac)
        .expect("bounded discriminant");
    operations.extend(signed_subtraction_operations(b_squared, four_ac));
    debug_assert!(discriminant > 0);
    operations.extend(square_root_operations(discriminant as u64));

    let two_a = 2_i64.checked_mul(a_int).expect("bounded 2a");
    operations.extend(multiply_or_identity_operations(2, a_int));
    // The final formula step is division by 2a. For an irrational numerator
    // this remains an algebraic quotient rather than a long integer division;
    // represent the standard rewrite as taking the reciprocal of the divisor.
    operations.push(Operation::Reciprocal);

    let (sqrt_coefficient, _) = square_root_decomposition(discriminant as u64);
    let sqrt_coefficient = i64::try_from(sqrt_coefficient).expect("bounded radical coefficient");
    let constant = b_int.checked_neg().expect("bounded negated b");
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
        let common_i64 = i64::try_from(common).expect("bounded formula common divisor");
        if constant != 0 {
            operations.extend(divide_or_identity_operations(constant, common_i64));
        }
        operations.extend(divide_or_identity_operations(sqrt_coefficient, common_i64));
        operations.extend(divide_or_identity_operations(two_a, common_i64));
    }

    operations.extend(big_num_operations(answer));
    operations_graph(operations)
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
    let mut denominator = value.denominator as u64;
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
    let mut coefficient = value.numerator;
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
                    operations.extend(signed_multiplication_operations(
                        left_value.coefficient,
                        right_value.coefficient,
                    ));
                    let scale = left_value.scale.checked_add(right_value.scale)?;
                    if scale > 0 {
                        operations.push(Operation::TimeTen { exponent: scale });
                    }
                    Some((
                        normalize_decimal_effort(DecimalEffortValue {
                            coefficient: left_value
                                .coefficient
                                .checked_mul(right_value.coefficient)?,
                            scale,
                        }),
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

pub fn arithmetic_expression_graph(
    expression: &ArithmeticExpression,
    answer: &AnswerNode,
) -> Option<SolutionGraph> {
    let mut operations = if contains_exact_decimal(expression) {
        decimal_expression_operations(expression)?.1
    } else {
        arithmetic_expression_operations(expression)?.1
    };
    operations.extend(big_num_operations(answer));
    Some(operations_graph(operations))
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
                    let result = rational_add(left_value, right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_addition_operations(left_value.numerator, right_value.numerator)
                    } else {
                        rational_addition_operations(left_value, right_value, result)
                    };
                    (result, ops)
                }
                ArithmeticOperator::Subtract => {
                    let result = left_value.subtract(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_subtraction_operations(left_value.numerator, right_value.numerator)
                    } else {
                        rational_subtraction_operations(left_value, right_value, result)
                    };
                    (result, ops)
                }
                ArithmeticOperator::Multiply => {
                    let result = left_value.multiply(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        signed_multiplication_operations(
                            left_value.numerator,
                            right_value.numerator,
                        )
                    } else {
                        rational_multiplication_operations(left_value, right_value)
                    };
                    (result, ops)
                }
                ArithmeticOperator::Divide => {
                    let result = left_value.divide(right_value)?;
                    let ops = if left_value.is_integer()
                        && right_value.is_integer()
                        && right_value.numerator != 0
                        && left_value.numerator % right_value.numerator == 0
                    {
                        divide_or_identity_operations(left_value.numerator, right_value.numerator)
                    } else {
                        rational_division_operations(left_value, right_value)
                    };
                    (result, ops)
                }
            };
            operations.append(&mut operator_operations);
            Some((result, operations))
        }
    }
}

fn rational_add(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Option<RationalCoefficient> {
    let left_scaled = left.numerator.checked_mul(right.denominator)?;
    let right_scaled = right.numerator.checked_mul(left.denominator)?;
    RationalCoefficient::new(
        left_scaled.checked_add(right_scaled)?,
        left.denominator.checked_mul(right.denominator)?,
    )
}

fn operations_graph(operations: Vec<Operation>) -> SolutionGraph {
    let mut steps = Vec::with_capacity(operations.len());
    for (index, operation) in operations.into_iter().enumerate() {
        let id = index as u32;
        let depends_on = if id == 0 { vec![] } else { vec![id - 1] };
        steps.push(step(id, operation, depends_on));
    }
    SolutionGraph { steps }
}

pub fn one_digit_addition_graph(left: u8, right: u8) -> SolutionGraph {
    let answer = u64::from(left + right);
    let mut operations = unsigned_addition_operations(u64::from(left), u64::from(right));
    operations.push(Operation::BigNum { magnitude: answer });
    operations_graph(operations)
}

/// Signed integer addition after the curriculum's negative rewrite rules.
pub fn signed_addition_graph(left: i64, right: i64) -> SolutionGraph {
    operations_graph(signed_addition_operations(left, right))
}

/// Signed integer subtraction after the curriculum's negative rewrite rules.
pub fn signed_subtraction_graph(left: i64, right: i64) -> SolutionGraph {
    operations_graph(signed_subtraction_operations(left, right))
}

pub fn integer_addition_graph(left: i64, right: i64) -> SolutionGraph {
    operations_graph(signed_addition_operations(left, right))
}

pub fn integer_subtraction_graph(left: i64, right: i64) -> SolutionGraph {
    operations_graph(signed_subtraction_operations(left, right))
}

pub fn integer_multiplication_graph(left: i64, right: i64) -> SolutionGraph {
    operations_graph(signed_multiplication_operations(left, right))
}

pub fn integer_division_graph(dividend: i64, divisor: i64) -> SolutionGraph {
    debug_assert_ne!(divisor, 0);
    operations_graph(divide_or_identity_operations(dividend, divisor))
}

/// Long division with a quotient/remainder final answer. The arithmetic work is
/// exactly the shared integer-division model; the tuple contributes only the
/// normal answer read/write cost.
pub fn integer_division_with_remainder_graph(
    dividend: i64,
    divisor: i64,
    answer: &AnswerNode,
) -> SolutionGraph {
    debug_assert_ne!(divisor, 0);
    let mut operations = divide_or_identity_operations(dividend, divisor);
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}

fn step(id: u32, operation: Operation, depends_on: Vec<u32>) -> SolutionStep {
    SolutionStep {
        id,
        operation,
        depends_on,
    }
}

fn exact_log10_cost(magnitude: u64) -> f64 {
    if magnitude == 0 {
        0.0
    } else {
        (magnitude as f64).log10()
    }
}

fn validate_nonnegative_finite(value: f64) -> Result<(), WeightError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(WeightError)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("operation weight or multiplier must be finite and nonnegative")]
pub struct WeightError;

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
    fn operation_vector_wire_dimension_matches_current_schema() {
        let current = OperationVector::zero();
        assert_eq!(
            serde_json::to_value(&current).unwrap()["values"]
                .as_array()
                .unwrap()
                .len(),
            OPERATION_KIND_COUNT
        );

        let graph = SolutionGraph {
            steps: vec![SolutionStep {
                id: 1,
                operation: Operation::FractionSelfDivision,
                depends_on: vec![],
            }],
        };
        let vector = calculate_graph_effort(&graph, &OperationWeights::default()).operation_vector;
        assert_eq!(vector.get(OperationKind::FractionSelfDivision), 1.0);
        assert_eq!(
            serde_json::to_value(&vector).unwrap()["values"]
                .as_array()
                .unwrap()
                .len(),
            OPERATION_KIND_COUNT
        );
    }

    #[test]
    fn lookup_table_is_bidirectional_for_subtraction_and_division() {
        assert_only(
            &operations_graph(unsigned_subtraction_operations(13, 5)).operation_vector(),
            &[(OperationKind::BaseMinus, 1.0)],
        );
        assert_only(
            &operations_graph(unsigned_division_operations(56, 7)).operation_vector(),
            &[(OperationKind::BaseDivide, 1.0)],
        );
        assert_only(
            &operations_graph(unsigned_division_operations(72, 8)).operation_vector(),
            &[(OperationKind::BaseDivide, 1.0)],
        );
    }

    #[test]
    fn remainder_quotient_search_costs_three_table_probes() {
        assert_only(
            &operations_graph(unsigned_division_operations(7, 3)).operation_vector(),
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
            &integer_addition_graph(23, 48).operation_vector(),
            &[
                (OperationKind::BasePlus, 2.0),
                (OperationKind::Increment, 1.0),
                (OperationKind::OverheadCarryPlus, 1.0),
            ],
        );
        assert_only(
            &integer_addition_graph(97, 86).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BasePlus, 2.0),
                (OperationKind::Increment, 1.0),
                (OperationKind::OverheadCarryPlus, 2.0),
            ],
        );
        assert_only(
            &integer_addition_graph(23, 4).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BasePlus, 1.0),
            ],
        );
        assert_only(
            &integer_addition_graph(99, 0).operation_vector(),
            &[(OperationKind::Identity, 2.0)],
        );
    }

    #[test]
    fn borrow_and_multiplication_carry_do_not_create_phantom_additions() {
        assert_only(
            &operations_graph(unsigned_subtraction_operations(42, 17)).operation_vector(),
            &[
                (OperationKind::BaseMinus, 2.0),
                (OperationKind::Decrement, 1.0),
                (OperationKind::OverheadCarryMinus, 1.0),
            ],
        );
        assert_only(
            &integer_multiplication_graph(7, 8).operation_vector(),
            &[
                (OperationKind::Identity, 1.0),
                (OperationKind::BaseTimes, 1.0),
                (OperationKind::OverheadCarryMult, 1.0),
            ],
        );
    }

    #[test]
    fn structural_cancellation_primitives_are_distinct_features() {
        let fraction_cancel =
            operations_graph(vec![Operation::BaseFractionCancel]).operation_vector();
        assert_only(
            &fraction_cancel,
            &[(OperationKind::BaseFractionCancel, 1.0)],
        );
        let root_square_cancel =
            operations_graph(vec![Operation::BaseRootSquareCancel]).operation_vector();
        assert_only(
            &root_square_cancel,
            &[(OperationKind::BaseRootSquareCancel, 1.0)],
        );

        let expression = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Multiply,
            left: Box::new(ArithmeticExpression::Rational {
                value: rational(5, 7),
            }),
            right: Box::new(ArithmeticExpression::Integer { value: 7 }),
        };
        let answer = AnswerNode::Integer(5);
        let vector = arithmetic_expression_graph(&expression, &answer)
            .unwrap()
            .operation_vector();
        assert_eq!(vector.get(OperationKind::BaseFractionCancel), 1.0);
    }

    #[test]
    fn gcd_uses_prime_factorization_not_full_divisor_enumeration() {
        let vector = operations_graph(gcd_search_operations(6, 12)).operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadGcd), 1.0);
        assert_eq!(vector.get(OperationKind::OverheadPf), 2.0);
        assert!(vector.get(OperationKind::BaseTimes) < 10.0);
        assert!(vector.get(OperationKind::Compare) <= 3.0);
    }

    #[test]
    fn factorization_on_multiplication_table_needs_only_pf_overhead() {
        let vector = operations_graph(prime_factorization_model(72).1).operation_vector();
        assert_only(&vector, &[(OperationKind::OverheadPf, 1.0)]);

        let vector = operations_graph(prime_factorization_model(77).1).operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadPf), 1.0);
        assert!(
            vector.get(OperationKind::BaseTimes) > 0.0
                || vector.get(OperationKind::BaseDivide) > 0.0
        );
    }

    #[test]
    fn square_root_simplification_tests_prime_squares() {
        assert_only(
            &operations_graph(square_root_operations(49)).operation_vector(),
            &[(OperationKind::BaseRoot, 1.0)],
        );
        let eight = operations_graph(square_root_operations(8)).operation_vector();
        assert_eq!(eight.get(OperationKind::OverheadFactorPerfectSquare), 1.0);
        assert_eq!(eight.get(OperationKind::Count), 1.0); // test 2^2
        let forty_five = operations_graph(square_root_operations(45)).operation_vector();
        assert_eq!(
            forty_five.get(OperationKind::OverheadFactorPerfectSquare),
            1.0
        );
        assert!(forty_five.get(OperationKind::Count) >= 2.0); // 2^2, 3^2, ...
        let seventy_two = operations_graph(square_root_operations(72)).operation_vector();
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
        let vector = arithmetic_expression_graph(&expression, &answer)
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
        let vector = arithmetic_expression_graph(&expression, &answer)
            .unwrap()
            .operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadLcm), 0.0);
        assert_eq!(vector.get(OperationKind::Count), 1.0);
        assert_eq!(vector.get(OperationKind::BasePlus), 1.0);
    }

    #[test]
    fn quadratic_factoring_uses_pf_then_unique_factor_pairs() {
        let answer_20 = AnswerNode::Tuple(vec![AnswerNode::Integer(4), AnswerNode::Integer(5)]);
        let answer_21 = AnswerNode::Tuple(vec![AnswerNode::Integer(3), AnswerNode::Integer(7)]);
        let weights = OperationWeights::default();
        let twenty =
            calculate_graph_effort(&quadratic_factoring_graph(-9, 20, &answer_20), &weights);
        let twenty_one =
            calculate_graph_effort(&quadratic_factoring_graph(-10, 21, &answer_21), &weights);
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
            &operations_graph(operations).operation_vector(),
            &[(OperationKind::OverheadPf, 1.0)],
        );
    }

    #[test]
    fn quadratic_special_forms_do_not_run_general_factor_search() {
        let difference = quadratic_factoring_graph(
            0,
            -81,
            &AnswerNode::Tuple(vec![AnswerNode::Integer(-9), AnswerNode::Integer(9)]),
        )
        .operation_vector();
        assert_eq!(
            difference.get(OperationKind::OverheadFactorDifferenceOfSquares),
            1.0
        );
        assert_eq!(difference.get(OperationKind::OverheadFactorGeneral), 0.0);
        assert_eq!(difference.get(OperationKind::BaseRoot), 1.0);
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
