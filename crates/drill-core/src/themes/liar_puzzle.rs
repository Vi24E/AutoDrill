use crate::effort::{Operation, SolutionGraph, SolutionStep};
use crate::model::LiarStatement;

pub(crate) fn statement_effort(statement: &LiarStatement, people_count: u8) -> u32 {
    match statement {
        LiarStatement::SaysLiar { .. } | LiarStatement::SaysNotLiar { .. } => 1,
        LiarStatement::ExactlyOneLiar { .. }
        | LiarStatement::BothLiar { .. }
        | LiarStatement::BothNotLiar { .. }
        | LiarStatement::Implication { .. } => 2,
        LiarStatement::ExactLiarCount { .. } => u32::from(people_count),
    }
}

pub(crate) fn solution_graph(statements: &[LiarStatement], people_count: u8) -> SolutionGraph {
    let formula_length = statements
        .iter()
        .map(|statement| statement_effort(statement, people_count))
        .sum::<u32>();
    SolutionGraph {
        steps: (1..=formula_length)
            .map(|step_id| SolutionStep {
                id: step_id,
                operation: Operation::Identity,
                depends_on: vec![],
            })
            .collect(),
    }
}
