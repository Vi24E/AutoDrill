use crate::model::AnswerNode;

/// Return the canonical form used by grading.  Integer nodes are already
/// canonical; keeping this as a function establishes the extension point for
/// fractions, units, and other answer kinds in later curricula.
pub fn normalize_answer(answer: &AnswerNode) -> AnswerNode {
    match answer {
        AnswerNode::Empty => AnswerNode::Empty,
        AnswerNode::Integer(value) => AnswerNode::Integer(*value),
    }
}
