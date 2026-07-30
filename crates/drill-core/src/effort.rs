use crate::model::{EffortResult, EffortWeights, OperationCounts, Problem};

pub fn calculate_effort(problem: &Problem, weights: &EffortWeights) -> EffortResult {
    let counts = problem.operation_counts.clone();
    let value = counts
        .additions
        .saturating_mul(weights.addition)
        .saturating_add(counts.carries.saturating_mul(weights.carry));
    EffortResult {
        value,
        operation_counts: counts,
    }
}

pub fn default_effort(problem: &Problem) -> EffortResult {
    calculate_effort(problem, &EffortWeights::default())
}

pub fn operation_counts_for(left: u8, right: u8) -> OperationCounts {
    OperationCounts::one_digit_addition(left + right >= 10)
}
