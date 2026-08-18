use crate::effort::{Operation, OperationPlan};

/// Explicit theme exception: a learner who has not mastered inverse table
/// lookup searches the 9x9 table with three probes even for exact quotients.
pub(crate) fn operation_plan(dividend: u8) -> OperationPlan {
    OperationPlan::new(vec![
        Operation::BaseTimes,
        Operation::BaseTimes,
        Operation::BaseTimes,
        Operation::BigNum {
            magnitude: u64::from(dividend),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::OperationKind;

    #[test]
    fn division_table_plan_uses_three_inverse_table_probes() {
        let vector = operation_plan(56).operation_vector();
        assert_eq!(vector.get(OperationKind::BaseTimes), 3.0);
        assert_eq!(vector.get(OperationKind::BaseDivide), 0.0);
        assert_eq!(vector.get(OperationKind::BigNum), 56_f64.log10());
    }
}
