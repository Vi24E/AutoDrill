/// Explicit theme exception: multiplication-table difficulty is ranked by
/// the logarithm of the answer, not by the reusable arithmetic primitive model.
pub(crate) fn effort(answer: u8) -> f64 {
    if answer == 0 {
        0.0
    } else {
        f64::from(answer).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_worksheet_request;
    use crate::identity::DEFAULT_DIFFICULTY;
    use crate::model::GenerateWorksheetRequest;
    use crate::schema::SCHEMA_VERSION;
    use crate::themes::basic_arithmetic::THEME_ID_MULTIPLICATION_TABLE;

    #[test]
    fn multiplication_table_keeps_its_theme_specific_effort_model() {
        assert_eq!(effort(56), 56_f64.log10());
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: THEME_ID_MULTIPLICATION_TABLE,
            seed: "Kuku56".to_owned(),
            difficulty: DEFAULT_DIFFICULTY,
            timeout_ms: None,
            max_attempts: None,
        })
        .unwrap();
        assert!(worksheet.problems().iter().all(|problem| {
            problem.theme_specific_effort() == Some(problem.effort())
                && problem.operation_plan().is_none()
                && problem.operation_vector() == crate::effort::OperationVector::zero()
        }));
    }
}
