use serde::{Deserialize, Deserializer, Serialize};

use crate::answer::AnswerNode;
use crate::model::{ArithmeticExpression, ArithmeticOperator, Problem, RationalCoefficient};

pub const OPERATION_KIND_COUNT: usize = 27;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// Dense fixed-size vector. Parameterized primitives store the quantity that
/// their base weight multiplies: Count stores n, TimeTen stores n+5,
/// Distribution stores n, and BigNum stores exact-magnitude log10 cost.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationVector {
    values: [f64; OPERATION_KIND_COUNT],
}

impl<'de> Deserialize<'de> for OperationVector {
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
                "operation vector values must be finite and nonnegative",
            ))
        }
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
        magnitude: u64,
    },
    Round,
    TimeTen {
        exponent: u32,
    },
    OverheadPf,
    OverheadGcd,
    OverheadGcdDivisible,
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
            // If one GCD operand divides the other, the GCD is recognized as
            // the smaller operand without a full divisor search. Keep the same
            // tunable weight dimension but charge only one quarter of it.
            Self::OverheadGcdDivisible => (OperationKind::OverheadGcd, 0.25),
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
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SolutionStep {
    pub id: u32,
    pub operation: Operation,
    pub depends_on: Vec<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
        operations.push(Operation::BaseDivide);
        if coefficient.numerator < 0 || constant.numerator < 0 {
            operations.push(Operation::OverheadNegative);
        }
        let solved = constant
            .divide(coefficient)
            .expect("nonzero linear coefficient");
        if solved.denominator > 1 {
            if let Some(operation) = rational_division_reduction_operation(constant, coefficient) {
                operations.push(operation);
            }
        }
    }
    operations.extend(big_num_operations(answer));

    let mut steps = Vec::with_capacity(operations.len());
    for (index, operation) in operations.into_iter().enumerate() {
        let id = index as u32;
        let depends_on = if id == 0 { vec![] } else { vec![id - 1] };
        steps.push(step(id, operation, depends_on));
    }
    SolutionGraph { steps }
}

fn rational_subtraction_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    result: RationalCoefficient,
) -> Vec<Operation> {
    let mut operations = Vec::new();
    if left.denominator != right.denominator {
        operations.push(Operation::OverheadLcm);
        operations.push(Operation::BaseTimes);
        operations.push(Operation::BaseTimes);
    }
    operations.push(Operation::BaseMinus);
    if left.numerator < 0 || right.numerator < 0 {
        operations.push(Operation::OverheadNegative);
    }
    if result.denominator > 1 {
        if let Some(operation) = rational_subtraction_reduction_operation(left, right) {
            operations.push(operation);
        }
    }
    operations
}

fn reduction_gcd_operation(left: i64, right: i64) -> Option<Operation> {
    let left = left.unsigned_abs();
    let right = right.unsigned_abs();
    if left == 0 || right == 0 || gcd_u64(left, right) <= 1 {
        return None;
    }
    if left.is_multiple_of(right) || right.is_multiple_of(left) {
        Some(Operation::OverheadGcdDivisible)
    } else {
        Some(Operation::OverheadGcd)
    }
}

fn rational_division_reduction_operation(
    dividend: RationalCoefficient,
    divisor: RationalCoefficient,
) -> Option<Operation> {
    let reduced = dividend.divide(divisor)?;
    if reduced.denominator == 1 {
        return None;
    }
    let raw_numerator = dividend.numerator.checked_mul(divisor.denominator)?;
    let raw_denominator = dividend.denominator.checked_mul(divisor.numerator)?;
    if raw_denominator == 0 {
        return None;
    }
    reduction_gcd_operation(raw_numerator, raw_denominator)
}

fn rational_subtraction_reduction_operation(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Option<Operation> {
    let denominator_gcd = gcd_u64(left.denominator as u64, right.denominator as u64);
    let left_scale = right.denominator / denominator_gcd as i64;
    let right_scale = left.denominator / denominator_gcd as i64;
    let left_numerator = left.numerator.checked_mul(left_scale)?;
    let right_numerator = right.numerator.checked_mul(right_scale)?;
    let raw_numerator = left_numerator.checked_sub(right_numerator)?;
    let common_denominator = left.denominator.checked_mul(left_scale)?;
    let reduced = left.subtract(right)?;
    (reduced.denominator > 1)
        .then(|| reduction_gcd_operation(raw_numerator, common_denominator))
        .flatten()
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

pub fn one_digit_subtraction_graph(left: u8, right: u8) -> SolutionGraph {
    let answer = u64::from(left - right);
    let mut operations = vec![Operation::BaseMinus];
    if left >= 10 {
        operations.push(Operation::Decrement);
        operations.push(Operation::OverheadCarryMinus);
    }
    operations.push(Operation::BigNum { magnitude: answer });
    operations_graph(operations)
}

pub fn two_digit_addition_graph(left: u8, right: u8) -> SolutionGraph {
    let answer = u64::from(left) + u64::from(right);
    let mut operations = vec![Operation::BasePlus, Operation::BasePlus];
    let ones_carry = u8::from(left % 10 + right % 10 >= 10);
    if ones_carry == 1 {
        operations.push(Operation::Increment);
        operations.push(Operation::OverheadCarryPlus);
    }
    if left / 10 + right / 10 + ones_carry >= 10 {
        operations.push(Operation::Increment);
        operations.push(Operation::OverheadCarryPlus);
    }
    operations.push(Operation::BigNum { magnitude: answer });
    operations_graph(operations)
}

/// The multiplication-table theme intentionally ranks questions only by the
/// answer-size cost requested by the product specification: log10(c).
pub fn multiplication_table_graph(answer: u8) -> SolutionGraph {
    operations_graph(vec![Operation::BigNum {
        magnitude: u64::from(answer),
    }])
}

pub fn arithmetic_expression_graph(
    expression: &ArithmeticExpression,
    answer: &AnswerNode,
) -> Option<SolutionGraph> {
    let (_, mut operations) = arithmetic_expression_operations(expression)?;
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
                        integer_signed_addition_operations(
                            left_value.numerator,
                            right_value.numerator,
                        )
                    } else {
                        rational_addition_operations(left_value, right_value, result)
                    };
                    (result, ops)
                }
                ArithmeticOperator::Subtract => {
                    let result = left_value.subtract(right_value)?;
                    let ops = if left_value.is_integer() && right_value.is_integer() {
                        integer_signed_subtraction_operations(
                            left_value.numerator,
                            right_value.numerator,
                        )
                    } else {
                        rational_subtraction_operations(left_value, right_value, result)
                    };
                    (result, ops)
                }
                ArithmeticOperator::Multiply => {
                    let result = left_value.multiply(right_value)?;
                    let mut ops = if left_value.is_integer() && right_value.is_integer() {
                        vec![Operation::BaseTimes]
                    } else {
                        // Fraction multiplication has two elementary products: numerator
                        // and denominator. Multiplication by ±1 is a recognition/copy
                        // operation rather than a full one-digit multiplication. Keeping
                        // those costs distinct gives difficulty selection information that
                        // was previously flattened by unconditional BaseTimes × 2.
                        vec![
                            rational_component_multiply_operation(
                                left_value.numerator,
                                right_value.numerator,
                            ),
                            rational_component_multiply_operation(
                                left_value.denominator,
                                right_value.denominator,
                            ),
                        ]
                    };
                    if left_value.numerator < 0 || right_value.numerator < 0 {
                        ops.push(Operation::OverheadNegative);
                    }
                    if result.denominator > 1 {
                        if let Some(operation) =
                            rational_multiplication_reduction_operation(left_value, right_value)
                        {
                            ops.push(operation);
                        }
                    }
                    (result, ops)
                }
                ArithmeticOperator::Divide => {
                    let result = left_value.divide(right_value)?;
                    let mut ops = vec![Operation::BaseDivide];
                    if left_value.numerator < 0 || right_value.numerator < 0 {
                        ops.push(Operation::OverheadNegative);
                    }
                    if result.denominator > 1 {
                        if let Some(operation) =
                            rational_division_reduction_operation(left_value, right_value)
                        {
                            ops.push(operation);
                        }
                    }
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

fn rational_addition_operations(
    left: RationalCoefficient,
    right: RationalCoefficient,
    result: RationalCoefficient,
) -> Vec<Operation> {
    let mut operations = Vec::new();
    if left.denominator != right.denominator {
        operations.push(Operation::OverheadLcm);
        operations.push(Operation::BaseTimes);
        operations.push(Operation::BaseTimes);
    }
    operations.push(Operation::BasePlus);
    if left.numerator < 0 || right.numerator < 0 {
        operations.push(Operation::OverheadNegative);
    }
    if result.denominator > 1 {
        if let Some(operation) = rational_addition_reduction_operation(left, right) {
            operations.push(operation);
        }
    }
    operations
}

fn rational_addition_reduction_operation(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Option<Operation> {
    let denominator_gcd = gcd_u64(left.denominator as u64, right.denominator as u64);
    let left_scale = right.denominator / denominator_gcd as i64;
    let right_scale = left.denominator / denominator_gcd as i64;
    let raw_numerator = left
        .numerator
        .checked_mul(left_scale)?
        .checked_add(right.numerator.checked_mul(right_scale)?)?;
    let common_denominator = left.denominator.checked_mul(left_scale)?;
    reduction_gcd_operation(raw_numerator, common_denominator)
}

fn rational_component_multiply_operation(left: i64, right: i64) -> Operation {
    if left.unsigned_abs() == 1 || right.unsigned_abs() == 1 {
        Operation::Identity
    } else {
        Operation::BaseTimes
    }
}

fn rational_multiplication_reduction_operation(
    left: RationalCoefficient,
    right: RationalCoefficient,
) -> Option<Operation> {
    let raw_numerator = left.numerator.checked_mul(right.numerator)?;
    let raw_denominator = left.denominator.checked_mul(right.denominator)?;
    reduction_gcd_operation(raw_numerator, raw_denominator)
}

fn integer_signed_addition_operations(left: i64, right: i64) -> Vec<Operation> {
    let graph = signed_addition_graph(left, right);
    graph.steps.into_iter().map(|step| step.operation).collect()
}

fn integer_signed_subtraction_operations(left: i64, right: i64) -> Vec<Operation> {
    let graph = signed_subtraction_graph(left, right);
    graph.steps.into_iter().map(|step| step.operation).collect()
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
    let mut steps = vec![
        step(0, Operation::BigNum { magnitude: answer }, vec![]),
        step(1, Operation::BasePlus, vec![0]),
    ];
    if left + right >= 10 {
        steps.push(step(2, Operation::Increment, vec![1]));
        steps.push(step(3, Operation::OverheadCarryPlus, vec![1]));
    }
    SolutionGraph { steps }
}

/// Signed addition after the curriculum's negative rewrite rules.
pub fn signed_addition_graph(left: i64, right: i64) -> SolutionGraph {
    let operation = if left >= 0 && right < 0 {
        if u64::try_from(left).ok() == Some(right.unsigned_abs()) {
            Operation::Identity
        } else {
            Operation::BaseMinus
        }
    } else if left < 0 && right >= 0 {
        if u64::try_from(right).ok() == Some(left.unsigned_abs()) {
            Operation::Identity
        } else {
            Operation::BaseMinus
        }
    } else {
        Operation::BasePlus
    };
    let mut steps = vec![step(0, operation, vec![])];
    // Every operation containing a negative operand incurs one overhead,
    // except the structural `a + (-b)` rewrite for positive a and b,
    // regardless of whether a is greater than, equal to, or less than b.
    let is_exception = left > 0 && right < 0;
    if (left < 0 || right < 0) && !is_exception {
        steps.push(step(1, Operation::OverheadNegative, vec![0]));
    }
    SolutionGraph { steps }
}

/// Subtraction normalizes `a - (-b)` to addition, but that form remains a
/// negative-operand operation and therefore still incurs the overhead.
pub fn signed_subtraction_graph(left: i64, right: i64) -> SolutionGraph {
    let (operation, has_negative_operand) = if right < 0 {
        (Operation::BasePlus, true)
    } else {
        (Operation::BaseMinus, left < 0)
    };
    let mut steps = vec![step(0, operation, vec![])];
    if has_negative_operand {
        steps.push(step(1, Operation::OverheadNegative, vec![0]));
    }
    SolutionGraph { steps }
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
mod fraction_multiplication_tests {
    use super::*;

    fn rational(numerator: i64, denominator: i64) -> RationalCoefficient {
        RationalCoefficient::new(numerator, denominator).unwrap()
    }

    fn multiply(left: RationalCoefficient, right: RationalCoefficient) -> ArithmeticExpression {
        ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Multiply,
            left: Box::new(ArithmeticExpression::Rational { value: left }),
            right: Box::new(ArithmeticExpression::Rational { value: right }),
        }
    }

    #[test]
    fn fraction_multiplication_treats_one_times_n_as_identity_cost() {
        let (_, easy) =
            arithmetic_expression_operations(&multiply(rational(1, 2), rational(1, 2))).unwrap();
        assert_eq!(easy, vec![Operation::Identity, Operation::BaseTimes]);

        let (_, harder) =
            arithmetic_expression_operations(&multiply(rational(2, 3), rational(3, 4))).unwrap();
        assert_eq!(
            harder,
            vec![
                Operation::BaseTimes,
                Operation::BaseTimes,
                Operation::OverheadGcdDivisible
            ]
        );
    }

    #[test]
    fn divisible_gcd_reduction_is_cheaper_than_general_gcd_reduction() {
        let easy = reduction_gcd_operation(6, 12).unwrap();
        let general = reduction_gcd_operation(12, 90).unwrap();
        assert_eq!(easy, Operation::OverheadGcdDivisible);
        assert_eq!(general, Operation::OverheadGcd);

        let weights = OperationWeights::default();
        let easy_effort = calculate_graph_effort(&operations_graph(vec![easy]), &weights).value;
        let general_effort =
            calculate_graph_effort(&operations_graph(vec![general]), &weights).value;
        assert_eq!(easy_effort, weights.get(OperationKind::OverheadGcd) * 0.25);
        assert_eq!(general_effort, weights.get(OperationKind::OverheadGcd));
        assert!(easy_effort < general_effort);
    }
}
