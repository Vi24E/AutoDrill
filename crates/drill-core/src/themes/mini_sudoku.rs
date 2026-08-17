use std::sync::OnceLock;

use crate::answer::AnswerNode;
use crate::effort::{OperationWeights, OperationVector, SolutionGraph};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::input_interface;
use crate::model::{AnswerSchema, Problem, ProblemPrompt};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, ThemeAnswerContract as AnswerContract,
    ThemeAnswerSchemaKind as Schema, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt, ThemeRegistration,
    ThemeRegistrationSpec, ThemeTag, PUZZLE_4_LAYOUT,
};

pub const THEME_ID_MINI_SUDOKU: u32 = 38;
pub const GENERATOR_REVISION_MINI_SUDOKU: u32 = 1;
pub const SKILL_ID_MINI_SUDOKU: &str = "bonus.logic.mini_sudoku";
pub const CURRICULUM_PATH_MINI_SUDOKU: [&str; 3] = ["root", "おまけ", "すうじはひとりぼっち"];
const TAGS: &[ThemeTag] = &[ThemeTag::Bonus];
const SIDE: usize = 4;
const CELL_COUNT: usize = SIDE * SIDE;
const MIN_BLANKS: usize = 5;
const MAX_BLANKS: usize = 10;
const UNIQUE_PUZZLE_ATTEMPTS_PER_BLANK_COUNT: usize = 64;

pub const MINI_SUDOKU_REGISTRATION: ThemeRegistration = ThemeRegistration::new(ThemeRegistrationSpec {
    numeric_theme_id: THEME_ID_MINI_SUDOKU,
    generator_revision: GENERATOR_REVISION_MINI_SUDOKU,
    skill_id: SKILL_ID_MINI_SUDOKU,
    curriculum_path: &CURRICULUM_PATH_MINI_SUDOKU,
    grade: None,
    tags: TAGS,
    safety: Safety::NonNegativeOnly,
    presentation: Presentation::WORKSHEET_GRID,
    dedup: Dedup::PreserveOperandOrder,
    answer_contract: AnswerContract {
        prompt_kind: Prompt::MiniSudoku,
        answer_schema_kind: Schema::OrderedTuple,
        input_profile: Input::DigitGrid { min_digit: 1, max_digit: 4, cell_count: 16 },
    },
    layout: PUZZLE_4_LAYOUT,
});

pub struct MiniSudokuGenerator;
pub static GENERATOR: MiniSudokuGenerator = MiniSudokuGenerator;

impl ProblemGenerator for MiniSudokuGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        &MINI_SUDOKU_REGISTRATION
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        _weights: &OperationWeights,
    ) -> Option<Problem> {
        // Draw the bootstrap blank count exactly once. Uniqueness retries keep
        // that count fixed so rejection probability cannot bias the requested
        // uniform [5, 10] blank-count source population.
        let blank_count = draw_blank_count(rng);
        let solutions = solved_boards();
        let mut selected = None;
        for _ in 0..UNIQUE_PUZZLE_ATTEMPTS_PER_BLANK_COUNT {
            let solved = solutions[rng.next_bounded(solutions.len() as u64) as usize];
            let mut order = std::array::from_fn::<usize, CELL_COUNT, _>(|index| index);
            for end in (1..CELL_COUNT).rev() {
                let swap = rng.next_bounded((end + 1) as u64) as usize;
                order.swap(end, swap);
            }
            let mut puzzle = solved;
            for &index in &order[..blank_count] {
                puzzle[index] = 0;
            }
            if count_solutions(&puzzle, 2) == 1 {
                selected = Some((solved, puzzle));
                break;
            }
        }
        let (solved, puzzle) = selected?;

        let trivial = trivial_blank_count(&puzzle);
        let nontrivial = blank_count - trivial;
        let theme_specific_effort = nontrivial as f64 + 0.3 * trivial as f64;
        let solution_graph = SolutionGraph::default();

        Some(Problem {
            schema_version: SCHEMA_VERSION,
            id: ordinal,
            numeric_theme_id: THEME_ID_MINI_SUDOKU,
            prompt: ProblemPrompt::MiniSudoku {
                givens: puzzle.iter().map(|&value| (value != 0).then_some(value)).collect(),
            },
            input_interface: input_interface(Input::DigitGrid { min_digit: 1, max_digit: 4, cell_count: 16 }),
            answer_schema: AnswerSchema::OrderedTuple { length: CELL_COUNT as u8 },
            canonical_answer: AnswerNode::Tuple(
                solved.iter().map(|&value| AnswerNode::Integer(i64::from(value))).collect(),
            ),
            worked_solution: None,
            solution_graph,
            operation_vector: OperationVector::zero(),
            theme_specific_effort: Some(theme_specific_effort),
            effort: theme_specific_effort,
        })
    }

    fn deduplicate_bootstrap_pool(&self) -> bool {
        true
    }
}

fn draw_blank_count(rng: &mut DeterministicRng) -> usize {
    MIN_BLANKS + rng.next_bounded((MAX_BLANKS - MIN_BLANKS + 1) as u64) as usize
}

fn solved_boards() -> &'static [[u8; CELL_COUNT]] {
    static BOARDS: OnceLock<Vec<[u8; CELL_COUNT]>> = OnceLock::new();
    BOARDS.get_or_init(|| {
        let mut boards = Vec::new();
        enumerate_solutions([0; CELL_COUNT], 0, &mut boards, usize::MAX);
        boards
    })
}

fn count_solutions(board: &[u8; CELL_COUNT], limit: usize) -> usize {
    let mut solutions = Vec::new();
    enumerate_solutions(*board, 0, &mut solutions, limit);
    solutions.len()
}

fn enumerate_solutions(
    mut board: [u8; CELL_COUNT],
    start: usize,
    solutions: &mut Vec<[u8; CELL_COUNT]>,
    limit: usize,
) {
    if solutions.len() >= limit {
        return;
    }
    let Some(index) = (start..CELL_COUNT).find(|&index| board[index] == 0) else {
        solutions.push(board);
        return;
    };
    for digit in 1..=4 {
        if can_place(&board, index, digit) {
            board[index] = digit;
            enumerate_solutions(board, index + 1, solutions, limit);
            board[index] = 0;
            if solutions.len() >= limit {
                return;
            }
        }
    }
}

fn can_place(board: &[u8; CELL_COUNT], index: usize, digit: u8) -> bool {
    let row = index / SIDE;
    let column = index % SIDE;
    if (0..SIDE).any(|offset| board[row * SIDE + offset] == digit) {
        return false;
    }
    if (0..SIDE).any(|offset| board[offset * SIDE + column] == digit) {
        return false;
    }
    let block_row = (row / 2) * 2;
    let block_column = (column / 2) * 2;
    !(0..2).any(|dr| (0..2).any(|dc| board[(block_row + dr) * SIDE + block_column + dc] == digit))
}

fn trivial_blank_count(board: &[u8; CELL_COUNT]) -> usize {
    (0..CELL_COUNT)
        .filter(|&index| {
            if board[index] != 0 {
                return false;
            }
            let row = index / SIDE;
            let column = index % SIDE;
            let row_filled = (0..SIDE).filter(|&offset| board[row * SIDE + offset] != 0).count();
            let column_filled = (0..SIDE).filter(|&offset| board[offset * SIDE + column] != 0).count();
            let block_row = (row / 2) * 2;
            let block_column = (column / 2) * 2;
            let block_filled = (0..2)
                .flat_map(|dr| (0..2).map(move |dc| (dr, dc)))
                .filter(|&(dr, dc)| board[(block_row + dr) * SIDE + block_column + dc] != 0)
                .count();
            row_filled == 3 || column_filled == 3 || block_filled == 3
        })
        .count()
}

pub(crate) static GENERATORS: [GeneratorEntry; 1] = [GeneratorEntry::current(&GENERATOR)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::resolved_weights;

    #[test]
    fn four_by_four_sudoku_has_288_completed_boards() {
        assert_eq!(solved_boards().len(), 288);
        for board in solved_boards() {
            assert_eq!(count_solutions(board, 2), 1);
        }
    }

    #[test]
    fn generated_candidates_are_unique_and_follow_the_requested_effort() {
        let weights = resolved_weights(&MINI_SUDOKU_REGISTRATION);
        let mut rng = DeterministicRng::from_seed("Lonely42");
        for ordinal in 1..=200 {
            let mut expected_rng = rng.clone();
            let expected_blank_count = draw_blank_count(&mut expected_rng);
            let problem = GENERATOR
                .draw_candidate(&mut rng, ordinal, &weights)
                .expect("every uniformly drawn blank count must find a unique puzzle within the local retry budget");
            let ProblemPrompt::MiniSudoku { givens } = &problem.prompt else {
                panic!("expected mini sudoku prompt");
            };
            let board: [u8; CELL_COUNT] = std::array::from_fn(|index| givens[index].unwrap_or(0));
            let blanks = board.iter().filter(|&&value| value == 0).count();
            assert!((MIN_BLANKS..=MAX_BLANKS).contains(&blanks));
            assert_eq!(blanks, expected_blank_count, "uniqueness retries must not redraw the uniformly sampled blank count");
            assert_eq!(count_solutions(&board, 2), 1);
            let trivial = trivial_blank_count(&board);
            let expected = (blanks - trivial) as f64 + 0.3 * trivial as f64;
            assert!((problem.effort - expected).abs() < 1e-12);
            assert_eq!(problem.canonical_answer.size(), 17);
        }
    }

    #[test]
    fn trivial_blank_means_a_row_column_or_block_has_three_givens() {
        let board = [
            1, 2, 3, 0,
            3, 4, 1, 2,
            2, 1, 4, 3,
            4, 3, 2, 1,
        ];
        assert_eq!(trivial_blank_count(&board), 1);
    }
}
