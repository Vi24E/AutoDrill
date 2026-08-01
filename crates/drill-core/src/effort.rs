use serde::{Deserialize, Deserializer, Serialize};

use crate::answer::AnswerNode;
use crate::model::Problem;

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
