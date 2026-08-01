#![forbid(unsafe_code)]

mod answer;
mod editor;
mod effort;
mod error;
mod exact;
mod generator;
mod grade;
mod identity;
mod model;
mod normalize;
mod registry;
mod rng;

pub use answer::{AnswerNode, AnswerRepresentation};
pub use editor::apply_editor_action;
pub use effort::{
    big_num_operations, calculate_effort, calculate_graph_effort, default_effort,
    one_digit_addition_graph, signed_addition_graph, signed_subtraction_graph, EffortResult,
    EffortWeights, Operation, OperationKind, OperationVector, OperationWeights, SolutionGraph,
    SolutionStep, WeightError, WeightMultipliers, WeightProfile, OPERATION_KIND_COUNT,
};
pub use error::{EditorError, GenerationError};
pub use generator::{
    generate_identity_with_clock, generate_problem, generate_problem_request, generate_worksheet,
    generate_worksheet_request, generate_worksheet_request_with_clock, regenerate_problem_set,
    registered_generator, GenerationConfig, MonotonicClock, OneDigitAdditionGenerator,
    ProblemGenerator, StepClock, SystemClock, DEFAULT_MAX_ATTEMPTS, DEFAULT_TIMEOUT,
};
pub use grade::grade_answer;
pub use identity::{
    validate_seed, Difficulty, IdentityError, ProblemSetIdentity, DEFAULT_DIFFICULTY,
    MAX_DIFFICULTY, MAX_SEED_LENGTH, MIN_DIFFICULTY,
};
pub use model::{
    AnswerSchema, EditorAction, EditorState, GenerateProblemRequest, GenerateWorksheetRequest,
    GradeResult, GradeStatus, GradeWarning, LayoutMetadata, Problem, ProblemPrompt, Worksheet,
    CURRICULUM_PATH, DEFAULT_COLUMNS, DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS,
    GENERATOR_REVISION_ONE_DIGIT_ADDITION, MAX_ANSWER, MAX_ANSWER_AST_SIZE, MAX_OPERAND,
    MIN_ANSWER, MIN_OPERAND, SCHEMA_VERSION, SKILL_ID, THEME_ID_ONE_DIGIT_ADDITION,
};
pub use normalize::normalize_answer;
pub use registry::{
    active_registration, registration, resolved_weights, ThemeRegistration, GENERATOR_REGISTRY,
    ONE_DIGIT_ADDITION_REGISTRATION,
};
pub use rng::{seed_to_u64, DeterministicRng};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use proptest::prelude::*;

    use super::*;

    fn valid_seed_strategy() -> impl Strategy<Value = String> {
        const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        proptest::collection::vec(0_usize..ALPHABET.len(), 1..=MAX_SEED_LENGTH).prop_map(
            |indices| {
                indices
                    .into_iter()
                    .map(|index| char::from(ALPHABET[index]))
                    .collect()
            },
        )
    }

    proptest! {
        #[test]
        fn request_seed_and_problem_set_id_round_trip(
            seed in valid_seed_strategy(),
            difficulty in MIN_DIFFICULTY..=MAX_DIFFICULTY,
        ) {
            let difficulty = Difficulty::try_from(difficulty).unwrap();
            let request = GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
                seed: seed.clone(),
                difficulty,
                timeout_ms: Some(1_000),
                max_attempts: Some(DEFAULT_MAX_ATTEMPTS),
            };
            let encoded = serde_json::to_string(&request).unwrap();
            let decoded: GenerateWorksheetRequest = serde_json::from_str(&encoded).unwrap();
            prop_assert_eq!(decoded, request);

            let identity = ProblemSetIdentity::new(
                THEME_ID_ONE_DIGIT_ADDITION,
                GENERATOR_REVISION_ONE_DIGIT_ADDITION,
                seed,
                difficulty,
            ).unwrap();
            let id = identity.to_string();
            prop_assert_eq!(id.parse::<ProblemSetIdentity>().unwrap(), identity);
        }

        #[test]
        fn generated_operands_answers_and_final_expressions_are_valid(seed in valid_seed_strategy()) {
            let worksheet = generate_worksheet(&seed).unwrap();
            let mut unique = HashSet::new();
            prop_assert_eq!(worksheet.problems.len(), DEFAULT_PROBLEM_COUNT);
            for problem in worksheet.problems {
                let (left, right) = problem.ordered_pair();
                prop_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&left));
                prop_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&right));
                prop_assert!((MIN_ANSWER..=MAX_ANSWER).contains(&problem.answer()));
                prop_assert_eq!(problem.answer(), left + right);
                prop_assert!(unique.insert((left, right)));
            }
        }

        #[test]
        fn answer_ast_serde_size_and_exact_decimal_are_stable(
            coefficient in any::<i32>(),
            scale in 0_u32..=8,
        ) {
            let node = AnswerNode::ExactDecimal {
                coefficient: i64::from(coefficient),
                scale,
            };
            let encoded = serde_json::to_string(&node).unwrap();
            let decoded: AnswerNode = serde_json::from_str(&encoded).unwrap();
            prop_assert_eq!(decoded, node.clone());
            prop_assert!(node.size() >= 1);
            let operations = big_num_operations(&node);
            prop_assert_eq!(operations, vec![Operation::BigNum {
                magnitude: i64::from(coefficient).unsigned_abs()
            }]);
        }

        #[test]
        fn operation_vectors_are_dense_nonnegative_and_overrides_recompute(
            multiplier in 0.0_f64..10.0,
        ) {
            let graph = one_digit_addition_graph(8, 9);
            let vector = graph.operation_vector();
            prop_assert_eq!(vector.as_array().len(), OPERATION_KIND_COUNT);
            prop_assert!(vector.is_nonnegative_finite());
            let base = OperationWeights::default();
            let baseline = base.weighted_sum(&vector);
            let mut changed = base.clone();
            changed.override_weight(OperationKind::BasePlus, multiplier).unwrap();
            let recomputed = changed.weighted_sum(&vector);
            prop_assert!((recomputed - baseline - (multiplier - 3.0)).abs() < 1e-9);

            let mut invalid = serde_json::to_value(&vector).unwrap();
            invalid["values"][OperationKind::BasePlus as usize] = serde_json::json!(-1.0);
            prop_assert!(serde_json::from_value::<OperationVector>(invalid).is_err());
        }

        #[test]
        fn graph_dependency_fanout_does_not_duplicate_operation_nodes(fanout in 1_usize..64) {
            let mut steps = vec![SolutionStep {
                id: 0,
                operation: Operation::BasePlus,
                depends_on: vec![],
            }];
            for id in 1..=fanout {
                steps.push(SolutionStep {
                    id: id as u32,
                    operation: Operation::Identity,
                    depends_on: vec![0],
                });
            }
            let vector = SolutionGraph { steps }.operation_vector();
            prop_assert_eq!(vector.get(OperationKind::BasePlus), 1.0);
            prop_assert_eq!(vector.get(OperationKind::Identity), fanout as f64);
        }
    }

    #[test]
    fn worksheet_is_reproducible_and_id_regenerates_it() {
        let first = generate_worksheet("Ab3Z").unwrap();
        let second = generate_worksheet("Ab3Z").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.problem_set_id, "2-1-2-Ab3Z-3");
        assert_eq!(
            regenerate_problem_set(&first.problem_set_id).unwrap(),
            first
        );
        assert_eq!(first.layout.problem_count, 20);
    }

    #[test]
    fn exact_wire_integers_are_canonical_decimal_strings() {
        let integer = AnswerNode::Integer(999_999_999_999_999_999);
        assert_eq!(
            serde_json::to_value(&integer).unwrap(),
            serde_json::json!({"type":"integer","value":"999999999999999999"})
        );
        assert!(serde_json::from_str::<AnswerNode>(
            r#"{"type":"integer","value":999999999999999999}"#
        )
        .is_err());
        assert!(serde_json::from_str::<AnswerNode>(
            r#"{"type":"integer","value":"0999999999999999999"}"#
        )
        .is_err());

        let schema = AnswerSchema::Integer {
            min: i64::MIN,
            max: i64::MAX,
        };
        let schema_json = serde_json::to_value(schema).unwrap();
        assert_eq!(schema_json["min"], i64::MIN.to_string());
        assert_eq!(schema_json["max"], i64::MAX.to_string());

        let magnitude = serde_json::to_value(Operation::BigNum {
            magnitude: i64::MIN.unsigned_abs(),
        })
        .unwrap();
        assert_eq!(magnitude["magnitude"], "9223372036854775808");
    }

    #[test]
    fn grading_compares_exact_numeric_ast_values_not_display_shapes() {
        let half = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let two_fourths = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(2)),
            denominator: Box::new(AnswerNode::Integer(4)),
        };
        let decimal_half = AnswerNode::ExactDecimal {
            coefficient: 5,
            scale: 1,
        };
        let reduced = grade_answer(&half, &two_fourths);
        assert!(reduced.is_correct);
        assert_eq!(reduced.warnings, vec![GradeWarning::FractionNotReduced]);

        let alternate_exact_form = grade_answer(&half, &decimal_half);
        assert!(alternate_exact_form.is_correct);
        assert!(alternate_exact_form.warnings.is_empty());

        let decimal_integer = grade_answer(
            &AnswerNode::Integer(4),
            &AnswerNode::ExactDecimal {
                coefficient: 40,
                scale: 1,
            },
        );
        assert!(decimal_integer.is_correct);
        assert_eq!(
            decimal_integer.warnings,
            vec![GradeWarning::RedundantDecimal]
        );

        let mixed = AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(1)),
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let three_halves = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(3)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        assert!(grade_answer(&mixed, &three_halves).is_correct);

        let double_negative = AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(
            AnswerNode::Integer(2),
        ))));
        let negative_warning = grade_answer(&AnswerNode::Integer(2), &double_negative);
        assert!(negative_warning.is_correct);
        assert_eq!(
            negative_warning.warnings,
            vec![GradeWarning::RedundantNegative]
        );

        let negative_fraction = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(-1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let wrapped_negative_fraction = AnswerNode::Negative(Box::new(negative_fraction));
        let signed_fraction_warning = grade_answer(&half, &wrapped_negative_fraction);
        assert!(signed_fraction_warning.is_correct);
        assert_eq!(
            signed_fraction_warning.warnings,
            vec![GradeWarning::RedundantNegative]
        );

        let multiple = AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(two_fourths))));
        let multiple_warnings = grade_answer(&half, &multiple);
        assert!(multiple_warnings.is_correct);
        assert_eq!(
            multiple_warnings.warnings,
            vec![
                GradeWarning::FractionNotReduced,
                GradeWarning::RedundantNegative,
            ]
        );

        let same_notation = grade_answer(&half, &half);
        assert!(same_notation.is_correct);
        assert!(same_notation.warnings.is_empty());

        let incorrect = grade_answer(&AnswerNode::Integer(3), &double_negative);
        assert!(!incorrect.is_correct);
        assert!(incorrect.warnings.is_empty());
    }

    #[test]
    fn normalization_never_saturates_i64_min_negation() {
        let value = AnswerNode::Negative(Box::new(AnswerNode::Integer(i64::MIN)));
        assert_eq!(normalize_answer(&value), value);
    }

    #[test]
    fn ordered_pairs_are_directional() {
        let observed = (1..=128).any(|seed| {
            let sheet = generate_worksheet(&seed.to_string()).unwrap();
            let set: HashSet<_> = sheet.problems.iter().map(Problem::ordered_pair).collect();
            set.iter()
                .any(|(left, right)| left != right && set.contains(&(*right, *left)))
        });
        assert!(observed);
    }

    #[test]
    fn timeout_and_attempt_limit_are_distinct() {
        let request = GenerateWorksheetRequest {
            seed: "Ab3Z".to_owned(),
            timeout_ms: Some(5),
            ..GenerateWorksheetRequest::default()
        };
        let timeout_clock = StepClock::new(Duration::ZERO, Duration::from_millis(10));
        let timeout = generate_worksheet_request_with_clock(&request, &timeout_clock).unwrap_err();
        assert!(matches!(timeout, GenerationError::Timeout { .. }));
        assert_eq!(timeout.code(), "generation_timeout");

        let request = GenerateWorksheetRequest {
            seed: "Ab3Z".to_owned(),
            timeout_ms: Some(1_000),
            max_attempts: Some(1),
            ..GenerateWorksheetRequest::default()
        };
        let clock = StepClock::new(Duration::ZERO, Duration::ZERO);
        let limit = generate_worksheet_request_with_clock(&request, &clock).unwrap_err();
        assert!(matches!(limit, GenerationError::AttemptLimit { .. }));
        assert_eq!(limit.code(), "generation_attempt_limit");
    }

    #[test]
    fn composite_size_and_exact_big_num_sources_match_contract() {
        let fraction = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(12)),
            denominator: Box::new(AnswerNode::Integer(42)),
        };
        assert_eq!(fraction.size(), 5);
        assert_eq!(
            big_num_operations(&fraction),
            vec![
                Operation::BigNum { magnitude: 12 },
                Operation::BigNum { magnitude: 42 }
            ]
        );
        assert_eq!(
            big_num_operations(&AnswerNode::exact_decimal(57, 2)),
            vec![Operation::BigNum { magnitude: 57 }]
        );
        assert_eq!(
            big_num_operations(&AnswerNode::Negative(Box::new(AnswerNode::exact_decimal(
                57, 2
            )))),
            vec![Operation::BigNum { magnitude: 57 }]
        );
    }

    #[test]
    fn graph_counts_shared_nodes_once_and_carry_has_both_primitives() {
        let graph = SolutionGraph {
            steps: vec![
                SolutionStep {
                    id: 0,
                    operation: Operation::BasePlus,
                    depends_on: vec![],
                },
                SolutionStep {
                    id: 1,
                    operation: Operation::Increment,
                    depends_on: vec![0],
                },
                SolutionStep {
                    id: 2,
                    operation: Operation::OverheadCarryPlus,
                    depends_on: vec![0],
                },
            ],
        };
        let vector = graph.operation_vector();
        assert_eq!(vector.get(OperationKind::BasePlus), 1.0);
        assert_eq!(vector.get(OperationKind::Increment), 1.0);
        assert_eq!(vector.get(OperationKind::OverheadCarryPlus), 1.0);
        let generated = one_digit_addition_graph(8, 9).operation_vector();
        assert_eq!(generated.get(OperationKind::BasePlus), 1.0);
        assert_eq!(generated.get(OperationKind::Increment), 1.0);
        assert_eq!(generated.get(OperationKind::OverheadCarryPlus), 1.0);
        assert_eq!(generated.get(OperationKind::BigNum), 17f64.log10());
        let magnitudes: Vec<_> = one_digit_addition_graph(8, 9)
            .steps
            .iter()
            .filter_map(|step| match step.operation {
                Operation::BigNum { magnitude } => Some(magnitude),
                _ => None,
            })
            .collect();
        assert_eq!(magnitudes, vec![17]);
    }

    #[test]
    fn negative_operand_overhead_follows_structural_rewrite_rule() {
        assert_eq!(
            signed_addition_graph(2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_graph(5, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_graph(3, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_graph(-3, 2)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_addition_graph(-2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_addition_graph(0, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_graph(2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_graph(-2, 3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_graph(5, 3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            big_num_operations(&AnswerNode::Negative(Box::new(AnswerNode::exact_decimal(
                57, 2
            ))))
            .len(),
            1
        );
    }

    #[test]
    fn default_weights_and_identity_layers_match_the_curriculum_contract() {
        let weights = OperationWeights::default();
        let expected = [
            1.0, 0.2, 1.0, 1.0, 3.0, 3.1, 3.5, 4.0, 1.0, 1.0, 0.2, 2.0, 4.0, 4.0, 1.5, 0.5, 0.5,
            0.5, 2.0, 2.0, 2.0, 4.0, 3.0, 2.0, 5.0, 6.0, 3.0,
        ];
        for (kind, expected) in OperationKind::ALL.into_iter().zip(expected) {
            assert_eq!(weights.get(kind), expected, "{kind:?}");
        }
        assert_eq!(WeightProfile::default().resolve(&weights), weights);

        let graph = one_digit_addition_graph(8, 9);
        let vector_before = graph.operation_vector();
        let mut profile = WeightProfile::default();
        profile
            .theme
            .override_multiplier(OperationKind::OverheadCarryPlus, 2.0)
            .unwrap();
        let themed = profile.resolve(&weights);
        assert_eq!(graph.operation_vector(), vector_before);
        assert_eq!(
            themed.weighted_sum(&vector_before) - weights.weighted_sum(&vector_before),
            0.5
        );
        assert!(ONE_DIGIT_ADDITION_REGISTRATION
            .operation_weight_overrides
            .is_empty());
    }

    #[test]
    fn difficulty_bias_is_monotonic_over_a_robust_seed_sample() {
        let means: Vec<f64> = (MIN_DIFFICULTY..=MAX_DIFFICULTY)
            .map(|level| {
                let total: f64 = (1..=128)
                    .map(|seed| {
                        let request = GenerateWorksheetRequest {
                            seed: format!("S{}", seed.to_string().replace('0', "A")),
                            difficulty: Difficulty::try_from(level).unwrap(),
                            ..GenerateWorksheetRequest::default()
                        };
                        generate_worksheet_request(&request)
                            .unwrap()
                            .problems
                            .iter()
                            .map(|problem| problem.effort)
                            .sum::<f64>()
                            / DEFAULT_PROBLEM_COUNT as f64
                    })
                    .sum();
                total / 128.0
            })
            .collect();
        assert!(means.windows(2).all(|pair| pair[0] < pair[1]), "{means:?}");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(8))]

        #[test]
        fn difficulty_order_statistic_bias_is_monotonic_property(salt in 1_u8..=9) {
            let means: Vec<f64> = (MIN_DIFFICULTY..=MAX_DIFFICULTY)
                .map(|level| {
                    (1..=64)
                        .map(|index| {
                            let suffix = index.to_string().replace('0', "A");
                            let request = GenerateWorksheetRequest {
                                seed: format!("P{salt}{suffix}"),
                                difficulty: Difficulty::try_from(level).unwrap(),
                                ..GenerateWorksheetRequest::default()
                            };
                            generate_worksheet_request(&request)
                                .unwrap()
                                .problems
                                .iter()
                                .map(|problem| problem.effort)
                                .sum::<f64>()
                                / DEFAULT_PROBLEM_COUNT as f64
                        })
                        .sum::<f64>()
                        / 64.0
                })
                .collect();
            prop_assert!(means.windows(2).all(|pair| pair[0] < pair[1]), "{means:?}");
        }
    }

    #[test]
    fn editor_still_enforces_integer_ast_size() {
        let mut state = EditorState::empty();
        for _ in 0..MAX_ANSWER_AST_SIZE {
            state = apply_editor_action(&state, &EditorAction::InsertDigit { digit: 1 }).unwrap();
        }
        let error =
            apply_editor_action(&state, &EditorAction::InsertDigit { digit: 2 }).unwrap_err();
        assert_eq!(
            error,
            EditorError::AnswerSizeLimit {
                max_size: MAX_ANSWER_AST_SIZE
            }
        );
    }
}
