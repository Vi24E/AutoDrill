#![forbid(unsafe_code)]

mod editor;
mod effort;
mod error;
mod generator;
mod grade;
mod model;
mod normalize;
mod rng;

pub use editor::apply_editor_action;
pub use effort::{calculate_effort, default_effort, operation_counts_for};
pub use error::{EditorError, GenerationError};
pub use generator::{
    generate_problem, generate_worksheet, generate_worksheet_with_clock,
    generate_worksheet_with_config, GenerationConfig, MonotonicClock, StepClock, SystemClock,
    DEFAULT_MAX_ATTEMPTS, DEFAULT_TIMEOUT,
};
pub use grade::grade_answer;
pub use model::{
    AnswerNode, EditorAction, EditorState, EffortResult, EffortWeights, GenerateProblemRequest,
    GenerateWorksheetRequest, GradeResult, GradeStatus, LayoutMetadata, OperationCounts, Problem,
    Worksheet, CURRICULUM_PATH, DEFAULT_COLUMNS, DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS,
    GENERATOR_VERSION, MAX_ANSWER, MAX_OPERAND, MIN_ANSWER, MIN_OPERAND, SCHEMA_VERSION, SKILL_ID,
};
pub use normalize::normalize_answer;
pub use rng::{seed_to_u64, DeterministicRng};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn worksheet_is_reproducible_and_has_fixed_metadata() {
        let first = generate_worksheet("18446744073709551615").expect("generation succeeds");
        let second = generate_worksheet("18446744073709551615").expect("generation succeeds");
        assert_eq!(first, second);
        assert_eq!(first.problems.len(), 20);
        assert_eq!(first.layout.problem_count, 20);
        assert_eq!(first.layout.columns, 2);
        assert_eq!(first.layout.rows, 10);
        assert_eq!(first.skill_id, SKILL_ID);
        assert_eq!(
            first.curriculum_path,
            ["root", "小学1年生", "1けたのたしざん(1)"]
        );
        assert_eq!(first.generator_version, GENERATOR_VERSION);
    }

    #[test]
    fn worksheet_has_unique_ordered_pairs_and_can_contain_swapped_pairs() {
        let worksheet = generate_worksheet("swapped-pairs").expect("generation succeeds");
        let mut pairs = std::collections::HashSet::new();
        for problem in &worksheet.problems {
            assert!((MIN_OPERAND..=MAX_OPERAND).contains(&problem.left));
            assert!((MIN_OPERAND..=MAX_OPERAND).contains(&problem.right));
            assert_eq!(problem.answer, problem.left + problem.right);
            assert!((MIN_ANSWER..=MAX_ANSWER).contains(&problem.answer));
            assert!(pairs.insert(problem.ordered_pair()));
        }

        // The generator draws from ordered pairs, not unordered combinations.
        // Across deterministic seeds we should observe at least one swapped pair
        // in a compact sample without imposing a particular worksheet sequence.
        let contains_swapped = (0..128).any(|seed| {
            let sheet = generate_worksheet(&seed.to_string()).expect("generation succeeds");
            let set: std::collections::HashSet<_> =
                sheet.problems.iter().map(Problem::ordered_pair).collect();
            set.iter()
                .any(|(left, right)| set.contains(&(*right, *left)) && left != right)
        });
        assert!(contains_swapped);
    }

    #[test]
    fn ordered_pair_draw_is_in_range_and_reproducible() {
        let mut first = DeterministicRng::from_seed("draws");
        let mut second = DeterministicRng::from_seed("draws");
        for _ in 0..1_000 {
            let first_pair = first.next_ordered_pair();
            let second_pair = second.next_ordered_pair();
            assert_eq!(first_pair, second_pair);
            let (left, right) = first_pair;
            assert!((1..=9).contains(&left));
            assert!((1..=9).contains(&right));
        }
    }

    #[test]
    fn normalization_is_idempotent_and_grade_uses_canonical_nodes() {
        let answer = AnswerNode::Integer(7);
        assert_eq!(normalize_answer(&normalize_answer(&answer)), answer);
        let correct = grade_answer(&AnswerNode::Integer(7), &AnswerNode::Integer(7));
        assert_eq!(correct.status, GradeStatus::Correct);
        assert!(correct.is_correct);
        let unanswered = grade_answer(&AnswerNode::Integer(7), &AnswerNode::Empty);
        assert_eq!(unanswered.status, GradeStatus::Unanswered);
        assert!(!unanswered.is_correct);
    }

    #[test]
    fn operation_counts_and_effort_reflect_carry() {
        let problem = generate_problem("0").expect("generation succeeds");
        let expected = operation_counts_for(problem.left, problem.right);
        assert_eq!(problem.operation_counts, expected);
        let weights = EffortWeights {
            addition: 3,
            carry: 5,
        };
        let effort = calculate_effort(&problem, &weights);
        assert_eq!(effort.value, 3 + 5 * u32::from(problem.answer >= 10));
    }

    #[test]
    fn editor_supports_empty_draft_digits_navigation_delete_clear_and_commit() {
        let state = EditorState::empty();
        let state = apply_editor_action(&state, &EditorAction::InsertDigit { digit: 1 }).unwrap();
        let state = apply_editor_action(&state, &EditorAction::InsertDigit { digit: 2 }).unwrap();
        assert_eq!(state.answer, AnswerNode::Integer(12));
        assert_eq!(state.cursor, 2);
        let state = apply_editor_action(&state, &EditorAction::MoveLeft).unwrap();
        let state = apply_editor_action(&state, &EditorAction::Delete).unwrap();
        assert_eq!(state.answer, AnswerNode::Integer(1));
        let state = apply_editor_action(&state, &EditorAction::Commit).unwrap();
        assert!(state.committed);
        let state = apply_editor_action(&state, &EditorAction::Clear).unwrap();
        assert_eq!(state, EditorState::empty());
    }

    #[test]
    fn timeout_and_attempt_limit_are_distinct_and_deterministic() {
        let timeout_config = GenerationConfig::default()
            .with_problem_count(20)
            .with_timeout(Duration::from_millis(5));
        let timeout_clock = StepClock::new(Duration::ZERO, Duration::from_millis(10));
        let timeout = generate_worksheet_with_clock("timeout", &timeout_config, &timeout_clock)
            .expect_err("clock should force timeout");
        assert_eq!(timeout.code(), "generation_timeout");
        assert!(matches!(timeout, GenerationError::Timeout { .. }));

        let attempt_config = GenerationConfig::default()
            .with_problem_count(2)
            .with_timeout(Duration::from_secs(1))
            .with_max_attempts(1);
        let attempt_clock = StepClock::new(Duration::ZERO, Duration::ZERO);
        let attempt = generate_worksheet_with_clock("attempt", &attempt_config, &attempt_clock)
            .expect_err("one draw cannot produce two unique problems");
        assert_eq!(attempt.code(), "generation_attempt_limit");
        assert!(matches!(attempt, GenerationError::AttemptLimit { .. }));
    }
}
