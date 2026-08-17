use crate::answer::AnswerNode;
use crate::effort::{
    calculate_graph_effort, Operation, OperationWeights, SolutionGraph, SolutionStep,
};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::model::{
    AnswerInputInterface, AnswerSchema, EditorStructure, LiarStatement, Problem, ProblemPrompt,
};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, ThemeAnswerContract as AnswerContract,
    ThemeAnswerSchemaKind as Schema, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt, ThemeRegistration,
    ThemeRegistrationSpec, ThemeTag, LIAR_6_LAYOUT,
};

pub const THEME_ID_LIAR_PUZZLE: u32 = 20;
pub const GENERATOR_REVISION_LIAR_PUZZLE: u32 = 4;
pub const SKILL_ID_LIAR_PUZZLE: &str = "bonus.logic.liar_puzzle";
pub const CURRICULUM_PATH_LIAR_PUZZLE: [&str; 3] = ["root", "おまけ", "うそつきだれだ"];

pub const LIAR_PUZZLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_LIAR_PUZZLE,
        generator_revision: GENERATOR_REVISION_LIAR_PUZZLE,
        skill_id: SKILL_ID_LIAR_PUZZLE,
        curriculum_path: &CURRICULUM_PATH_LIAR_PUZZLE,
        grade: None,
        tags: &[ThemeTag::Bonus],
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::LiarPuzzle,
            answer_schema_kind: Schema::Algebraic,
            input_profile: Input::TupleOnly,
        },
        layout: LIAR_6_LAYOUT,
    });

#[derive(Debug)]
pub(crate) struct Generator;

pub(crate) static GENERATOR: Generator = Generator;

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        &LIAR_PUZZLE_REGISTRATION
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        draw_problem(rng, ordinal, weights)
    }
}

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

pub(crate) fn statement_truth(statement: &LiarStatement, mask: u32) -> bool {
    let is_liar = |person: u8| ((mask >> u32::from(person - 1)) & 1) == 1;
    match *statement {
        LiarStatement::SaysLiar { person } => is_liar(person),
        LiarStatement::SaysNotLiar { person } => !is_liar(person),
        LiarStatement::ExactlyOneLiar { first, second } => is_liar(first) ^ is_liar(second),
        LiarStatement::ExactLiarCount { count } => mask.count_ones() == u32::from(count),
        LiarStatement::BothLiar { first, second } => is_liar(first) && is_liar(second),
        LiarStatement::BothNotLiar { first, second } => !is_liar(first) && !is_liar(second),
        LiarStatement::Implication {
            antecedent_person,
            antecedent_is_liar,
            consequent_person,
            consequent_is_liar,
        } => {
            let antecedent = is_liar(antecedent_person) == antecedent_is_liar;
            let consequent = is_liar(consequent_person) == consequent_is_liar;
            !antecedent || consequent
        }
    }
}

pub(crate) fn solutions(people_count: u8, statements: &[LiarStatement]) -> Vec<u32> {
    let mut solutions = Vec::new();
    for mask in 0_u32..(1_u32 << people_count) {
        let valid = statements.iter().enumerate().all(|(speaker, statement)| {
            let speaker_is_liar = ((mask >> speaker) & 1) == 1;
            statement_truth(statement, mask) == !speaker_is_liar
        });
        if valid {
            solutions.push(mask);
        }
    }
    solutions
}

fn draw_other_person(rng: &mut DeterministicRng, people_count: u8, speaker: u8) -> u8 {
    let offset = 1 + rng.next_bounded(u64::from(people_count - 1)) as u8;
    ((speaker - 1 + offset) % people_count) + 1
}

fn draw_two_other_people(rng: &mut DeterministicRng, people_count: u8, speaker: u8) -> (u8, u8) {
    let first = draw_other_person(rng, people_count, speaker);
    let mut second = draw_other_person(rng, people_count, speaker);
    while second == first {
        second = draw_other_person(rng, people_count, speaker);
    }
    if first < second {
        (first, second)
    } else {
        (second, first)
    }
}

fn draw_problem(
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    let people_count = 3 + rng.next_bounded(2) as u8;
    let mut statements = Vec::with_capacity(usize::from(people_count));
    for speaker in 1..=people_count {
        let statement = match rng.next_bounded(6) {
            0 => LiarStatement::SaysLiar {
                person: draw_other_person(rng, people_count, speaker),
            },
            1 => LiarStatement::SaysNotLiar {
                person: draw_other_person(rng, people_count, speaker),
            },
            2 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::ExactlyOneLiar { first, second }
            }
            3 => LiarStatement::ExactLiarCount {
                count: 1 + rng.next_bounded(u64::from(people_count - 1)) as u8,
            },
            4 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothLiar { first, second }
            }
            _ => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothNotLiar { first, second }
            }
        };
        statements.push(statement);
    }

    let solutions = solutions(people_count, &statements);
    if solutions.len() != 1 {
        return None;
    }
    let solution = solutions[0];
    let liar_count = solution.count_ones();
    if liar_count == 0 || liar_count == u32::from(people_count) {
        return None;
    }
    let liars = (1..=people_count)
        .filter(|person| ((solution >> u32::from(*person - 1)) & 1) == 1)
        .map(|person| AnswerNode::Integer(i64::from(person)))
        .collect::<Vec<_>>();
    let canonical_answer = AnswerNode::Tuple(liars);

    let formula_length = statements
        .iter()
        .map(|statement| statement_effort(statement, people_count))
        .sum::<u32>();
    let solution_graph = solution_graph(&statements, people_count);
    let effort = calculate_graph_effort(&solution_graph, weights);
    debug_assert_eq!(effort.value, f64::from(formula_length));

    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: THEME_ID_LIAR_PUZZLE,
        prompt: ProblemPrompt::LiarPuzzle {
            people_count,
            statements,
        },
        input_interface: AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Tuple],
        },
        answer_schema: AnswerSchema::Algebraic,
        canonical_answer,
        worked_solution: None,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

/// Current generators owned by this theme.
pub(crate) static GENERATORS: [GeneratorEntry; 1] = [GeneratorEntry::current(&GENERATOR)];
