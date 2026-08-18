use crate::answer::AnswerNode;
use crate::effort::{EffortModel, OperationWeights};
use crate::error::GenerationError;
use crate::generator::{
    BootstrapDedup, GeneratorEntry, ProblemGenerator, RandomCandidateSource, SamplingStrategy,
};
use crate::model::{
    AnswerSchema, LiarCount, LiarStatement, PeopleCount, PersonIndex, Problem, ProblemPrompt,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, ThemeAnswerContract as AnswerContract,
    ThemePresentationPolicy as Presentation, ThemeRegistration, ThemeRegistrationSpec, ThemeTag,
    LIAR_6_LAYOUT,
};

pub const THEME_ID_LIAR_PUZZLE: u32 = 20;
pub const GENERATOR_REVISION_LIAR_PUZZLE: u32 = 4;
pub const SKILL_ID_LIAR_PUZZLE: &str = "bonus.logic.liar_puzzle";
pub const CURRICULUM_PATH_LIAR_PUZZLE: [&str; 3] = ["root", "おまけ", "うそつきだれだ"];

pub const LIAR_PUZZLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LIAR_PUZZLE),
        generator_revision: crate::theme::GeneratorRevision::new(GENERATOR_REVISION_LIAR_PUZZLE),
        skill_id: SKILL_ID_LIAR_PUZZLE,
        curriculum_path: &CURRICULUM_PATH_LIAR_PUZZLE,
        grade: None,
        tags: &[ThemeTag::Bonus],
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::LiarPuzzle,
        layout: LIAR_6_LAYOUT,
    });

#[derive(Debug)]
pub(crate) struct Generator;

pub(crate) static GENERATOR: Generator = Generator;

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        &LIAR_PUZZLE_REGISTRATION
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        Ok(SamplingStrategy::random(
            self,
            BootstrapDedup::AllowDuplicates,
        ))
    }
}

impl RandomCandidateSource for Generator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        draw_problem(rng, ordinal, weights)
    }
}

pub(crate) fn statement_effort(statement: &LiarStatement, people_count: u8) -> u32 {
    match statement {
        LiarStatement::SaysLiar { .. } | LiarStatement::SaysNotLiar { .. } => 1,
        LiarStatement::ExactlyOneLiar { .. }
        | LiarStatement::BothLiar { .. }
        | LiarStatement::BothNotLiar { .. } => 2,
        LiarStatement::ExactLiarCount { .. } => u32::from(people_count),
    }
}

#[cfg(test)]
pub(crate) fn statement_truth(statement: &LiarStatement, mask: u32) -> bool {
    statement.is_true_for_mask(mask)
}

pub(crate) fn solutions(people_count: PeopleCount, statements: &[LiarStatement]) -> Vec<u32> {
    crate::semantics::liar_solutions(people_count, statements)
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
    _weights: &OperationWeights,
) -> Result<Option<Problem>, GenerationError> {
    let people_count = 3 + rng.next_bounded(2) as u8;
    let mut statements = Vec::with_capacity(usize::from(people_count));
    for speaker in 1..=people_count {
        let statement = match rng.next_bounded(6) {
            0 => LiarStatement::SaysLiar {
                person: PersonIndex::new(draw_other_person(rng, people_count, speaker)).ok_or(
                    GenerationError::InvalidGeneratedProblem {
                        reason: "liar generator produced an out-of-range person index",
                    },
                )?,
            },
            1 => LiarStatement::SaysNotLiar {
                person: PersonIndex::new(draw_other_person(rng, people_count, speaker)).ok_or(
                    GenerationError::InvalidGeneratedProblem {
                        reason: "liar generator produced an out-of-range person index",
                    },
                )?,
            },
            2 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::ExactlyOneLiar {
                    first: PersonIndex::new(first).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                    second: PersonIndex::new(second).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                }
            }
            3 => LiarStatement::ExactLiarCount {
                count: LiarCount::new(1 + rng.next_bounded(u64::from(people_count - 1)) as u8)
                    .ok_or(GenerationError::InvalidGeneratedProblem {
                        reason: "liar generator produced an out-of-range liar count",
                    })?,
            },
            4 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothLiar {
                    first: PersonIndex::new(first).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                    second: PersonIndex::new(second).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                }
            }
            _ => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothNotLiar {
                    first: PersonIndex::new(first).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                    second: PersonIndex::new(second).ok_or(
                        GenerationError::InvalidGeneratedProblem {
                            reason: "liar generator produced an out-of-range person index",
                        },
                    )?,
                }
            }
        };
        statements.push(statement);
    }

    let typed_people_count =
        PeopleCount::new(people_count).ok_or(GenerationError::InvalidGeneratedProblem {
            reason: "liar generator produced an unsupported people count",
        })?;
    let solutions = solutions(typed_people_count, &statements);
    if solutions.len() != 1 {
        return Ok(None);
    }
    let solution = solutions[0];
    let liar_count = solution.count_ones();
    if liar_count == 0 || liar_count == u32::from(people_count) {
        return Ok(None);
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
    let effort_model = EffortModel::theme_specific(f64::from(formula_length)).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "liar generator produced a non-finite effort",
        },
    )?;
    Problem::generated(
        &LIAR_PUZZLE_REGISTRATION,
        id,
        ProblemPrompt::LiarPuzzle {
            people_count: typed_people_count,
            statements,
        },
        AnswerSchema::Algebraic,
        canonical_answer,
        effort_model,
    )
    .map(Some)
    .map_err(GenerationError::from)
}

/// Current generators owned by this theme.
pub(crate) static GENERATORS: [GeneratorEntry; 1] = [GeneratorEntry::current(&GENERATOR)];

#[cfg(test)]
mod curriculum_tests {
    use super::*;
    use crate::generator::{generate_worksheet_request, registered_generator};
    use crate::model::GenerateWorksheetRequest;
    use crate::schema::SCHEMA_VERSION;

    #[test]
    fn liar_puzzle_is_not_a_layered_theme() {
        let generator = registered_generator(THEME_ID_LIAR_PUZZLE, GENERATOR_REVISION_LIAR_PUZZLE)
            .expect("registry must be valid")
            .expect("liar puzzle generator must be registered");
        assert!(!generator.sampling_strategy().unwrap().is_layered());
    }

    #[test]
    fn liar_puzzle_generates_all_six_statement_forms_with_three_or_four_people() {
        let mut seen = [false; 6];
        for seed in [
            "A1b2", "M7x9", "Q4r6", "Z8k3", "L1aR", "T2uV", "P3qX", "H4mN", "C5dK", "R6sW", "B7fJ",
            "G8vY",
        ] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_LIAR_PUZZLE,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(2).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            assert_eq!(
                worksheet.problems().len(),
                LIAR_PUZZLE_REGISTRATION.layout().problem_count()
            );
            for problem in worksheet.into_problems() {
                let ProblemPrompt::LiarPuzzle {
                    people_count,
                    statements,
                } = problem.prompt()
                else {
                    panic!("liar puzzle prompt");
                };
                assert!((3..=4).contains(&people_count.value()));
                assert_eq!(statements.len(), usize::from(*people_count));
                let expected_effort = statements
                    .iter()
                    .map(|statement| statement_effort(statement, people_count.value()))
                    .sum::<u32>();
                assert_eq!(problem.effort(), f64::from(expected_effort));
                assert!(problem.operation_plan().is_none());

                for (speaker_index, statement) in statements.iter().enumerate() {
                    let speaker = speaker_index as u8 + 1;
                    let assert_person = |person: crate::model::PersonIndex| {
                        assert!((1..=people_count.value()).contains(&person.value()));
                        assert_ne!(person.value(), speaker);
                    };
                    match *statement {
                        LiarStatement::SaysLiar { person } => {
                            seen[0] = true;
                            assert_person(person);
                        }
                        LiarStatement::SaysNotLiar { person } => {
                            seen[1] = true;
                            assert_person(person);
                        }
                        LiarStatement::ExactlyOneLiar { first, second } => {
                            seen[2] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                        LiarStatement::ExactLiarCount { count } => {
                            seen[3] = true;
                            assert!((1..people_count.value()).contains(&count.value()));
                        }
                        LiarStatement::BothLiar { first, second } => {
                            seen[4] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                        LiarStatement::BothNotLiar { first, second } => {
                            seen[5] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                    }
                }

                let solutions = solutions(*people_count, statements);
                assert_eq!(solutions.len(), 1);
                let solution = solutions[0];
                assert!((1..u32::from(*people_count)).contains(&solution.count_ones()));
                let expected_liars = (1..=people_count.value())
                    .filter(|person| ((solution >> u32::from(*person - 1)) & 1) == 1)
                    .map(|person| AnswerNode::Integer(i64::from(person)))
                    .collect::<Vec<_>>();
                assert_eq!(
                    problem.canonical_answer(),
                    &AnswerNode::Tuple(expected_liars)
                );
            }
        }
        assert!(
            seen.into_iter().all(|value| value),
            "not all liar statement forms were generated: {seen:?}"
        );
    }

    #[test]
    fn liar_statement_truth_and_effort_match_sat_semantics() {
        // Mask 0b0101 means people 1 and 3 are liars, people 2 and 4 are honest.
        let mask = 0b0101;
        assert!(statement_truth(
            &LiarStatement::SaysLiar {
                person: crate::model::PersonIndex::new(1).unwrap()
            },
            mask
        ));
        assert!(statement_truth(
            &LiarStatement::SaysNotLiar {
                person: crate::model::PersonIndex::new(2).unwrap()
            },
            mask
        ));
        assert!(statement_truth(
            &LiarStatement::ExactlyOneLiar {
                first: crate::model::PersonIndex::new(1).unwrap(),
                second: crate::model::PersonIndex::new(2).unwrap()
            },
            mask
        ));
        assert!(statement_truth(
            &LiarStatement::ExactLiarCount {
                count: crate::model::LiarCount::new(2).unwrap()
            },
            mask
        ));
        assert!(statement_truth(
            &LiarStatement::BothLiar {
                first: crate::model::PersonIndex::new(1).unwrap(),
                second: crate::model::PersonIndex::new(3).unwrap()
            },
            mask
        ));
        assert!(statement_truth(
            &LiarStatement::BothNotLiar {
                first: crate::model::PersonIndex::new(2).unwrap(),
                second: crate::model::PersonIndex::new(4).unwrap()
            },
            mask
        ));

        assert_eq!(
            statement_effort(
                &LiarStatement::SaysLiar {
                    person: crate::model::PersonIndex::new(1).unwrap()
                },
                5
            ),
            1
        );
        assert_eq!(
            statement_effort(
                &LiarStatement::ExactLiarCount {
                    count: crate::model::LiarCount::new(2).unwrap()
                },
                5
            ),
            5
        );
    }
}
