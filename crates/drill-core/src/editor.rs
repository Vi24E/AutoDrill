use crate::error::EditorError;
use crate::model::{AnswerNode, EditorAction, EditorState, MAX_ANSWER_AST_SIZE};

/// Apply one typed editor action to an immutable state snapshot.
pub fn apply_editor_action(
    state: &EditorState,
    action: &EditorAction,
) -> Result<EditorState, EditorError> {
    let mut next = state.clone();
    let digits = digits_for(&state.answer)?;
    next.cursor = next.cursor.min(digits.len());

    match action {
        EditorAction::InsertDigit { digit } => {
            if *digit > 9 {
                return Err(EditorError::InvalidDigit);
            }
            let mut candidate = digits.clone();
            candidate.insert(next.cursor, char::from(b'0' + *digit));
            if canonical_digit_count(&candidate) > MAX_ANSWER_AST_SIZE {
                // The immutable input state remains untouched while callers
                // receive a typed signal suitable for a size-limit notice.
                return Err(EditorError::AnswerSizeLimit {
                    max_size: MAX_ANSWER_AST_SIZE,
                });
            }
            let (answer, cursor) = answer_from_digits(&candidate, next.cursor + 1)?;
            next.answer = answer;
            next.cursor = cursor;
            next.committed = false;
        }
        EditorAction::Backspace => {
            if next.cursor > 0 {
                let remove_at = next.cursor - 1;
                let mut candidate = digits.clone();
                candidate.remove(remove_at);
                let (answer, cursor) = answer_from_digits(&candidate, remove_at)?;
                next.answer = answer;
                next.cursor = cursor;
                next.committed = false;
            }
        }
        EditorAction::Delete => {
            if next.cursor < digits.len() {
                let mut candidate = digits.clone();
                candidate.remove(next.cursor);
                let (answer, cursor) = answer_from_digits(&candidate, next.cursor)?;
                next.answer = answer;
                next.cursor = cursor;
                next.committed = false;
            }
        }
        EditorAction::MoveLeft => {
            next.cursor = next.cursor.saturating_sub(1);
        }
        EditorAction::MoveRight => {
            next.cursor = (next.cursor + 1).min(digits.len());
        }
        EditorAction::Clear => {
            next.answer = AnswerNode::Empty;
            next.cursor = 0;
            next.committed = false;
        }
        EditorAction::Commit => {
            next.committed = true;
        }
    }

    Ok(next)
}

fn digits_for(answer: &AnswerNode) -> Result<String, EditorError> {
    match answer {
        AnswerNode::Empty => Ok(String::new()),
        AnswerNode::Integer(value) if *value >= 0 => Ok(value.to_string()),
        AnswerNode::Integer(_) => Err(EditorError::NegativeDraft),
    }
}

fn answer_from_digits(candidate: &str, cursor: usize) -> Result<(AnswerNode, usize), EditorError> {
    if candidate.is_empty() {
        return Ok((AnswerNode::Empty, 0));
    }

    let normalized = candidate.trim_start_matches('0');
    let normalized = if normalized.is_empty() {
        "0"
    } else {
        normalized
    };
    let value = normalized
        .parse::<i64>()
        .map_err(|_| EditorError::IntegerOverflow)?;

    // Leading zeroes are not part of the canonical Integer AST.  Keep the
    // cursor at the closest valid position after canonicalization.
    let leading_zeroes = candidate.len() - normalized.len();
    let normalized_cursor = cursor.saturating_sub(leading_zeroes).min(normalized.len());
    Ok((AnswerNode::Integer(value), normalized_cursor))
}

fn canonical_digit_count(candidate: &str) -> usize {
    let digits = candidate.trim_start_matches('0');
    if digits.is_empty() {
        1
    } else {
        digits.len()
    }
}
