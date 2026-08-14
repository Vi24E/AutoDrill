use crate::effort::{Operation, SolutionGraph, SolutionStep};

/// Explicit theme exception: a learner who has not mastered inverse table
/// lookup searches the 9x9 table with three probes even for exact quotients.
pub(crate) fn solution_graph(dividend: u8) -> SolutionGraph {
    let operations = [
        Operation::BaseTimes,
        Operation::BaseTimes,
        Operation::BaseTimes,
        Operation::BigNum {
            magnitude: u64::from(dividend),
        },
    ];
    SolutionGraph {
        steps: operations
            .into_iter()
            .enumerate()
            .map(|(index, operation)| SolutionStep {
                id: index as u32,
                operation,
                depends_on: vec![],
            })
            .collect(),
    }
}
