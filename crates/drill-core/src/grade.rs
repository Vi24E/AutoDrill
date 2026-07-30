use crate::model::{AnswerNode, GradeResult, GradeStatus};
use crate::normalize::normalize_answer;

pub fn grade_answer(expected: &AnswerNode, actual: &AnswerNode) -> GradeResult {
    let expected = normalize_answer(expected);
    let actual = normalize_answer(actual);
    let status = match actual {
        AnswerNode::Empty => GradeStatus::Unanswered,
        _ if expected == actual => GradeStatus::Correct,
        _ => GradeStatus::Incorrect,
    };
    let is_correct = matches!(status, GradeStatus::Correct);
    GradeResult {
        status,
        is_correct,
        expected,
        actual,
    }
}
