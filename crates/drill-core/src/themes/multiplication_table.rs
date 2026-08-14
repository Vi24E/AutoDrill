use crate::effort::{Operation, SolutionGraph, SolutionStep};

/// Explicit theme exception: multiplication-table difficulty is ranked by
/// answer-size rather than the reusable primitive arithmetic model.
pub(crate) fn solution_graph(answer: u8) -> SolutionGraph {
    SolutionGraph {
        steps: vec![SolutionStep {
            id: 0,
            operation: Operation::BigNum {
                magnitude: u64::from(answer),
            },
            depends_on: vec![],
        }],
    }
}
