use crate::answer::AnswerNode;
use crate::error::EditorError;
use crate::model::{
    AnswerInputInterface, EditorAction, EditorState, EditorStructure, MAX_ANSWER_AST_SIZE,
};

/// Apply one typed editor action to an immutable state snapshot. Composite
/// answers keep an AST path to the active numeric slot and a cursor within the
/// slot's exact decimal display text.
pub fn apply_editor_action(
    state: &EditorState,
    action: &EditorAction,
    input_interface: &AnswerInputInterface,
) -> Result<EditorState, EditorError> {
    if matches!(action, EditorAction::Clear) {
        return Ok(EditorState::empty());
    }

    // Validate before cloning or inspecting an active slot. Clear remains
    // available even when recovery is needed for an over-limit state.
    ensure_size(&state.answer)?;
    ensure_capability(&state.answer, input_interface)?;
    validate_position(&state.answer, &state.active_path, state.cursor)?;

    if let EditorAction::SelectSlot { path, cursor } = action {
        let text = editable_text_at(&state.answer, path)?;
        validate_cursor(*cursor, &text)?;
        let mut next = state.clone();
        next.active_path.clone_from(path);
        next.cursor = *cursor;
        return Ok(next);
    }

    let mut next = state.clone();

    match action {
        EditorAction::InsertDigit { digit } => insert_digit(&mut next, *digit, input_interface)?,
        EditorAction::Backspace => backspace(&mut next, input_interface)?,
        EditorAction::Delete => edit_active_text(
            &mut next,
            |text, cursor| {
                if cursor >= text.chars().count() {
                    return (text, cursor);
                }
                let mut candidate = text;
                remove_char_at(&mut candidate, cursor);
                (candidate, cursor)
            },
            input_interface,
        )?,
        EditorAction::MoveLeft => move_cursor(&mut next, false)?,
        EditorAction::MoveRight => move_cursor(&mut next, true)?,
        EditorAction::InsertStructure { structure } => {
            if !input_interface.allows_structure(*structure) {
                return Err(EditorError::StructureNotAllowed {
                    structure: *structure,
                });
            }
            insert_structure(&mut next, *structure, input_interface)?;
        }
        EditorAction::SelectSlot { .. } | EditorAction::Clear => unreachable!(),
        EditorAction::Commit => next.committed = true,
    }

    ensure_size(&next.answer)?;
    ensure_capability(&next.answer, input_interface)?;
    validate_position(&next.answer, &next.active_path, next.cursor)?;
    Ok(next)
}

fn backspace(
    state: &mut EditorState,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    let current = node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?;
    let text = draft_text_for(current)?;

    if text.is_empty() && state.cursor == 0 && !state.active_path.is_empty() {
        // An empty slot has no character to erase. Remove the shallowest
        // structural node containing that slot instead. Because active_path is
        // non-empty, the root answer is necessarily that shallowest ancestor.
        state.answer = AnswerNode::Empty;
        state.active_path.clear();
        state.cursor = 0;
        state.committed = false;
        return Ok(());
    }

    edit_active_text(
        state,
        |text, cursor| {
            if cursor == 0 {
                return (text, cursor);
            }
            let mut candidate = text;
            remove_char_at(&mut candidate, cursor - 1);
            (candidate, cursor - 1)
        },
        input_interface,
    )
}

fn insert_digit(
    state: &mut EditorState,
    digit: u8,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    if digit > 9 {
        return Err(EditorError::InvalidDigit);
    }
    edit_active_text(
        state,
        |mut text, cursor| {
            insert_char_at(&mut text, cursor, char::from(b'0' + digit));
            (text, cursor + 1)
        },
        input_interface,
    )
}

fn edit_active_text<F>(
    state: &mut EditorState,
    edit: F,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError>
where
    F: FnOnce(String, usize) -> (String, usize),
{
    let current = node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?;
    let text = draft_text_for(current)?;
    let (candidate_text, candidate_cursor) = edit(text.clone(), state.cursor);
    let (mut candidate_node, mut normalized_cursor) =
        answer_from_draft(&candidate_text, candidate_cursor)?;
    if ensure_capability(&candidate_node, input_interface).is_err()
        && matches!(current, AnswerNode::NanError(_))
    {
        // Keep malformed raw text editable when its current spelling would
        // otherwise parse to a structure outside the interface capability.
        candidate_node = AnswerNode::NanError(candidate_text);
        normalized_cursor = candidate_cursor;
    }
    let mut candidate_answer = state.answer.clone();
    *node_at_mut(&mut candidate_answer, &state.active_path).ok_or(EditorError::InvalidPath)? =
        candidate_node;
    ensure_size(&candidate_answer)?;
    ensure_capability(&candidate_answer, input_interface)?;
    state.answer = candidate_answer;
    state.cursor = normalized_cursor;
    state.committed = false;
    Ok(())
}

fn insert_structure(
    state: &mut EditorState,
    structure: EditorStructure,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    if structure == EditorStructure::Tuple {
        return insert_tuple_item(state, input_interface);
    }
    if structure == EditorStructure::Decimal {
        return insert_decimal_point(state, input_interface);
    }

    let current = node_at(&state.answer, &state.active_path)
        .ok_or(EditorError::InvalidPath)?
        .clone();
    let is_empty = current.is_empty();
    let (replacement, child_index) = match structure {
        EditorStructure::Fraction => (
            AnswerNode::Fraction {
                numerator: Box::new(if is_empty { AnswerNode::Empty } else { current }),
                denominator: Box::new(AnswerNode::Empty),
            },
            usize::from(!is_empty),
        ),
        EditorStructure::MixedFraction => (
            AnswerNode::MixedFraction {
                whole: Box::new(if is_empty { AnswerNode::Empty } else { current }),
                numerator: Box::new(AnswerNode::Empty),
                denominator: Box::new(AnswerNode::Empty),
            },
            usize::from(!is_empty),
        ),
        EditorStructure::Root => (
            AnswerNode::Root {
                radicand: Box::new(current),
                index: None,
            },
            0,
        ),
        EditorStructure::Negative => (AnswerNode::Negative(Box::new(current)), 0),
        EditorStructure::PlusMinus => (AnswerNode::PlusMinus(Box::new(current)), 0),
        EditorStructure::Decimal | EditorStructure::Tuple => unreachable!(),
    };

    let mut candidate_answer = state.answer.clone();
    *node_at_mut(&mut candidate_answer, &state.active_path).ok_or(EditorError::InvalidPath)? =
        replacement;
    ensure_size(&candidate_answer)?;
    ensure_capability(&candidate_answer, input_interface)?;
    state.answer = candidate_answer;
    state.active_path.push(child_index);
    state.cursor = draft_text_for(
        node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?,
    )?
    .chars()
    .count();
    state.committed = false;
    Ok(())
}

fn insert_decimal_point(
    state: &mut EditorState,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    let current = node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?;
    let mut text = draft_text_for(current)?;
    if let Some(decimal_at) = text.chars().position(|character| character == '.') {
        state.cursor = decimal_at + 1;
        return Ok(());
    }
    let cursor = state.cursor;
    if text.is_empty() {
        text.push_str("0.");
        return replace_active_draft(state, &text, 2, input_interface);
    }
    insert_char_at(&mut text, cursor, '.');
    replace_active_draft(state, &text, cursor + 1, input_interface)
}

fn replace_active_draft(
    state: &mut EditorState,
    text: &str,
    cursor: usize,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    let (node, normalized_cursor) = answer_from_draft(text, cursor)?;
    let mut candidate_answer = state.answer.clone();
    *node_at_mut(&mut candidate_answer, &state.active_path).ok_or(EditorError::InvalidPath)? = node;
    ensure_size(&candidate_answer)?;
    ensure_capability(&candidate_answer, input_interface)?;
    state.answer = candidate_answer;
    state.cursor = normalized_cursor;
    state.committed = false;
    Ok(())
}

fn insert_tuple_item(
    state: &mut EditorState,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    let mut candidate = state.answer.clone();
    let next_index = match &mut candidate {
        AnswerNode::Tuple(values) => {
            values.push(AnswerNode::Empty);
            values.len() - 1
        }
        AnswerNode::Empty => {
            candidate = AnswerNode::Tuple(vec![AnswerNode::Empty, AnswerNode::Empty]);
            1
        }
        _ => {
            candidate = AnswerNode::Tuple(vec![candidate, AnswerNode::Empty]);
            1
        }
    };
    ensure_size(&candidate)?;
    ensure_capability(&candidate, input_interface)?;
    state.answer = candidate;
    state.active_path = vec![next_index];
    state.cursor = 0;
    state.committed = false;
    Ok(())
}

fn move_cursor(state: &mut EditorState, right: bool) -> Result<(), EditorError> {
    let text = draft_text_for(
        node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?,
    )?;
    if right && state.cursor < text.chars().count() {
        state.cursor += 1;
        return Ok(());
    }
    if !right && state.cursor > 0 {
        state.cursor -= 1;
        return Ok(());
    }

    let paths = editable_paths(&state.answer);
    let Some(current_index) = paths.iter().position(|path| path == &state.active_path) else {
        return Err(EditorError::InvalidPath);
    };
    let target_index = if right {
        current_index
            .checked_add(1)
            .filter(|index| *index < paths.len())
    } else {
        current_index.checked_sub(1)
    };
    if let Some(target_index) = target_index {
        state.active_path.clone_from(&paths[target_index]);
        let target_text = draft_text_for(
            node_at(&state.answer, &state.active_path).ok_or(EditorError::InvalidPath)?,
        )?;
        state.cursor = if right {
            0
        } else {
            target_text.chars().count()
        };
    }
    Ok(())
}

fn draft_text_for(answer: &AnswerNode) -> Result<String, EditorError> {
    ensure_size(answer)?;
    match answer {
        AnswerNode::Empty => Ok(String::new()),
        AnswerNode::Integer(value) if *value >= 0 => Ok(value.to_string()),
        AnswerNode::ExactDecimal { coefficient, scale } if *coefficient >= 0 => {
            Ok(decimal_display(*coefficient as u64, *scale))
        }
        AnswerNode::Integer(_) | AnswerNode::ExactDecimal { .. } => Err(EditorError::NegativeDraft),
        AnswerNode::NanError(raw) => Ok(raw.clone()),
        _ => Err(EditorError::UnsupportedDraftNode),
    }
}

fn editable_text_at(answer: &AnswerNode, path: &[usize]) -> Result<String, EditorError> {
    let node = node_at(answer, path).ok_or(EditorError::InvalidPath)?;
    draft_text_for(node).map_err(|_| EditorError::InvalidPath)
}

fn validate_position(
    answer: &AnswerNode,
    path: &[usize],
    cursor: usize,
) -> Result<(), EditorError> {
    let text = editable_text_at(answer, path)?;
    validate_cursor(cursor, &text)
}

fn validate_cursor(cursor: usize, text: &str) -> Result<(), EditorError> {
    if cursor <= text.chars().count() {
        Ok(())
    } else {
        Err(EditorError::InvalidPath)
    }
}

pub(crate) fn ensure_capability(
    answer: &AnswerNode,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    match answer {
        AnswerNode::Empty | AnswerNode::NanError(_) => Ok(()),
        AnswerNode::Integer(value) => {
            if *value < 0 {
                ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            }
            Ok(())
        }
        AnswerNode::ExactDecimal { coefficient, .. } => {
            ensure_structure_allowed(input_interface, EditorStructure::Decimal)?;
            if *coefficient < 0 {
                ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            }
            Ok(())
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => {
            ensure_structure_allowed(input_interface, EditorStructure::Fraction)?;
            ensure_capability(numerator, input_interface)?;
            ensure_capability(denominator, input_interface)
        }
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            ensure_structure_allowed(input_interface, EditorStructure::MixedFraction)?;
            ensure_capability(whole, input_interface)?;
            ensure_capability(numerator, input_interface)?;
            ensure_capability(denominator, input_interface)
        }
        AnswerNode::Root { radicand, index } => {
            ensure_structure_allowed(input_interface, EditorStructure::Root)?;
            ensure_capability(radicand, input_interface)?;
            if let Some(index) = index {
                ensure_capability(index, input_interface)?;
            }
            Ok(())
        }
        AnswerNode::Negative(value) => {
            ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            ensure_capability(value, input_interface)
        }
        AnswerNode::PlusMinus(value) => {
            ensure_structure_allowed(input_interface, EditorStructure::PlusMinus)?;
            ensure_capability(value, input_interface)
        }
        AnswerNode::Tuple(values) => {
            ensure_structure_allowed(input_interface, EditorStructure::Tuple)?;
            for value in values {
                ensure_capability(value, input_interface)?;
            }
            Ok(())
        }
        AnswerNode::Variable(_) => Err(EditorError::InputInterfaceViolation),
    }
}

fn ensure_structure_allowed(
    input_interface: &AnswerInputInterface,
    structure: EditorStructure,
) -> Result<(), EditorError> {
    if input_interface.allows_structure(structure) {
        Ok(())
    } else {
        Err(EditorError::InputInterfaceViolation)
    }
}

fn answer_from_draft(candidate: &str, cursor: usize) -> Result<(AnswerNode, usize), EditorError> {
    if candidate.is_empty() {
        return Ok((AnswerNode::Empty, 0));
    }
    if candidate.bytes().filter(|byte| *byte == b'.').count() > 1
        || !candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Ok((AnswerNode::NanError(candidate.to_owned()), cursor));
    }

    let answer = if let Some(decimal_at) = candidate.find('.') {
        let integer = &candidate[..decimal_at];
        let fractional = &candidate[decimal_at + 1..];
        let coefficient_text = format!(
            "{}{}",
            if integer.is_empty() { "0" } else { integer },
            fractional
        );
        let normalized = coefficient_text.trim_start_matches('0');
        let normalized = if normalized.is_empty() {
            "0"
        } else {
            normalized
        };
        let coefficient = normalized
            .parse::<i64>()
            .map_err(|_| EditorError::IntegerOverflow)?;
        AnswerNode::ExactDecimal {
            coefficient,
            scale: fractional
                .len()
                .try_into()
                .map_err(|_| EditorError::IntegerOverflow)?,
        }
    } else {
        let normalized = candidate.trim_start_matches('0');
        let normalized = if normalized.is_empty() {
            "0"
        } else {
            normalized
        };
        AnswerNode::Integer(
            normalized
                .parse::<i64>()
                .map_err(|_| EditorError::IntegerOverflow)?,
        )
    };
    let display = draft_text_for(&answer)?;
    let added_leading_zero = usize::from(candidate.starts_with('.'));
    let effective_candidate_len = candidate.chars().count() + added_leading_zero;
    let removed = effective_candidate_len.saturating_sub(display.chars().count());
    Ok((
        answer,
        (cursor + added_leading_zero)
            .saturating_sub(removed)
            .min(display.chars().count()),
    ))
}

fn decimal_display(coefficient: u64, scale: u32) -> String {
    let digits = coefficient.to_string();
    if scale == 0 {
        return format!("{digits}.");
    }
    let scale = scale as usize;
    if digits.len() <= scale {
        format!("0.{}{digits}", "0".repeat(scale - digits.len()))
    } else {
        let split = digits.len() - scale;
        format!("{}.{}", &digits[..split], &digits[split..])
    }
}

fn ensure_size(answer: &AnswerNode) -> Result<(), EditorError> {
    if !answer.is_within_size_limit() {
        Err(EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        })
    } else {
        Ok(())
    }
}

fn insert_char_at(text: &mut String, index: usize, character: char) {
    let byte_index = text
        .char_indices()
        .nth(index)
        .map_or(text.len(), |(byte_index, _)| byte_index);
    text.insert(byte_index, character);
}

fn remove_char_at(text: &mut String, index: usize) {
    if let Some((byte_index, character)) = text.char_indices().nth(index) {
        text.replace_range(byte_index..byte_index + character.len_utf8(), "");
    }
}

fn editable_paths(answer: &AnswerNode) -> Vec<Vec<usize>> {
    let mut output = Vec::new();
    collect_editable_paths(answer, &mut Vec::new(), &mut output);
    output
}

fn collect_editable_paths(
    answer: &AnswerNode,
    path: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    match answer {
        AnswerNode::Empty | AnswerNode::Integer(_) | AnswerNode::ExactDecimal { .. } => {
            output.push(path.clone());
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => {
            collect_child(numerator, 0, path, output);
            collect_child(denominator, 1, path, output);
        }
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            collect_child(whole, 0, path, output);
            collect_child(numerator, 1, path, output);
            collect_child(denominator, 2, path, output);
        }
        AnswerNode::Root { radicand, index } => {
            collect_child(radicand, 0, path, output);
            if let Some(index) = index {
                collect_child(index, 1, path, output);
            }
        }
        AnswerNode::Negative(value) | AnswerNode::PlusMinus(value) => {
            collect_child(value, 0, path, output);
        }
        AnswerNode::Tuple(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_child(value, index, path, output);
            }
        }
        AnswerNode::NanError(_) => output.push(path.clone()),
        AnswerNode::Variable(_) => {}
    }
}

fn collect_child(
    answer: &AnswerNode,
    index: usize,
    path: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    path.push(index);
    collect_editable_paths(answer, path, output);
    path.pop();
}

fn node_at<'a>(answer: &'a AnswerNode, path: &[usize]) -> Option<&'a AnswerNode> {
    let mut node = answer;
    for &index in path {
        node = child_at(node, index)?;
    }
    Some(node)
}

fn node_at_mut<'a>(answer: &'a mut AnswerNode, path: &[usize]) -> Option<&'a mut AnswerNode> {
    let Some((&first, rest)) = path.split_first() else {
        return Some(answer);
    };
    node_at_mut(child_at_mut(answer, first)?, rest)
}

fn child_at(answer: &AnswerNode, index: usize) -> Option<&AnswerNode> {
    match (answer, index) {
        (AnswerNode::Fraction { numerator, .. }, 0) => Some(numerator),
        (AnswerNode::Fraction { denominator, .. }, 1) => Some(denominator),
        (AnswerNode::MixedFraction { whole, .. }, 0) => Some(whole),
        (AnswerNode::MixedFraction { numerator, .. }, 1) => Some(numerator),
        (AnswerNode::MixedFraction { denominator, .. }, 2) => Some(denominator),
        (AnswerNode::Root { radicand, .. }, 0) => Some(radicand),
        (
            AnswerNode::Root {
                index: Some(index), ..
            },
            1,
        ) => Some(index),
        (AnswerNode::Negative(value) | AnswerNode::PlusMinus(value), 0) => Some(value),
        (AnswerNode::Tuple(values), index) => values.get(index),
        _ => None,
    }
}

fn child_at_mut(answer: &mut AnswerNode, index: usize) -> Option<&mut AnswerNode> {
    match (answer, index) {
        (AnswerNode::Fraction { numerator, .. }, 0) => Some(numerator),
        (AnswerNode::Fraction { denominator, .. }, 1) => Some(denominator),
        (AnswerNode::MixedFraction { whole, .. }, 0) => Some(whole),
        (AnswerNode::MixedFraction { numerator, .. }, 1) => Some(numerator),
        (AnswerNode::MixedFraction { denominator, .. }, 2) => Some(denominator),
        (AnswerNode::Root { radicand, .. }, 0) => Some(radicand),
        (
            AnswerNode::Root {
                index: Some(index), ..
            },
            1,
        ) => Some(index),
        (AnswerNode::Negative(value) | AnswerNode::PlusMinus(value), 0) => Some(value),
        (AnswerNode::Tuple(values), index) => values.get_mut(index),
        _ => None,
    }
}
