#![forbid(unsafe_code)]

mod answer;
mod contract;
mod editor;
mod effort;
mod error;
mod exact;
mod generator;
mod grade;
mod identity;
mod mathlive_input;
mod model;
mod normalize;
mod registry;
mod rng;
mod themes;

pub use answer::{AnswerNode, AnswerRepresentation};
pub use contract::{web_contract, WebContract, WebLayoutContract, WebThemeContract};
pub use editor::apply_editor_action;
pub use effort::{
    arithmetic_expression_graph, big_num_operations, calculate_effort, calculate_graph_effort,
    default_effort, integer_addition_graph, integer_division_graph, integer_multiplication_graph,
    integer_subtraction_graph, linear_equation_graph, one_digit_addition_graph,
    one_digit_subtraction_graph, quadratic_factoring_graph, quadratic_formula_graph,
    quadratic_square_graph, signed_addition_graph, signed_subtraction_graph,
    simultaneous_equation_graph, two_digit_addition_graph, EffortResult, EffortWeights, Operation,
    OperationKind, OperationVector, OperationWeights, SolutionGraph, SolutionStep, WeightError,
    WeightMultipliers, WeightProfile, OPERATION_KIND_COUNT,
};
pub use error::{EditorError, GenerationError};
pub use generator::{
    generate_identity_with_clock, generate_problem, generate_problem_request, generate_worksheet,
    generate_worksheet_request, generate_worksheet_request_with_clock, regenerate_problem_set,
    registered_generator, ArithmeticThemeGenerator, GenerationConfig, LinearEquationGenerator,
    MonotonicClock, OneDigitAdditionGenerator, ProblemGenerator, StepClock, SystemClock,
    DEFAULT_MAX_ATTEMPTS, DEFAULT_TIMEOUT,
};
pub use grade::{grade_answer, grade_answer_with_schema};
pub use identity::{
    validate_seed, Difficulty, IdentityError, ProblemSetIdentity, DEFAULT_DIFFICULTY,
    MAX_DIFFICULTY, MAX_SEED_LENGTH, MIN_DIFFICULTY,
};
pub use mathlive_input::parse_mathlive_answer;
pub use model::{
    AnswerInputInterface, AnswerSchema, ArithmeticExpression, ArithmeticOperator, EditorAction,
    EditorState, EditorStructure, GenerateProblemRequest, GenerateWorksheetRequest, GradeResult,
    GradeStatus, GradeWarning, LayoutMetadata, Problem, ProblemPrompt, RationalCoefficient,
    Worksheet, CURRICULUM_PATH, CURRICULUM_PATH_LINEAR_EQUATION_1,
    CURRICULUM_PATH_LINEAR_EQUATION_2, DEFAULT_COLUMNS, DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS,
    GENERATOR_REVISION_LINEAR_EQUATION_1, GENERATOR_REVISION_LINEAR_EQUATION_2,
    GENERATOR_REVISION_ONE_DIGIT_ADDITION, LINEAR_EQUATION_COLUMNS, LINEAR_EQUATION_PROBLEM_COUNT,
    LINEAR_EQUATION_ROWS, MAX_ANSWER, MAX_ANSWER_AST_SIZE, MAX_OPERAND, MIN_ANSWER, MIN_OPERAND,
    SCHEMA_VERSION, SKILL_ID, SKILL_ID_LINEAR_EQUATION_1, SKILL_ID_LINEAR_EQUATION_2,
    THEME_ID_LINEAR_EQUATION_1, THEME_ID_LINEAR_EQUATION_2, THEME_ID_ONE_DIGIT_ADDITION,
};
pub use normalize::normalize_answer;
pub use registry::{
    active_registration, registration, resolved_weights, ThemeRegistration, GENERATOR_REGISTRY,
    LINEAR_EQUATION_1_REGISTRATION, LINEAR_EQUATION_2_REGISTRATION,
    ONE_DIGIT_ADDITION_REGISTRATION,
};
pub use rng::{seed_to_u64, DeterministicRng};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use proptest::prelude::*;

    use super::*;

    fn structured_interface() -> AnswerInputInterface {
        AnswerInputInterface::StructuredMath {
            allowed_structures: vec![
                EditorStructure::Fraction,
                EditorStructure::MixedFraction,
                EditorStructure::Decimal,
                EditorStructure::Root,
                EditorStructure::Negative,
                EditorStructure::PlusMinus,
                EditorStructure::Tuple,
            ],
        }
    }

    fn apply_editor_action(
        state: &EditorState,
        action: &EditorAction,
    ) -> Result<EditorState, EditorError> {
        super::apply_editor_action(state, action, &structured_interface())
    }

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
        assert_eq!(
            first.problem_set_id,
            format!("4-1-{}-Ab3Z-2", GENERATOR_REVISION_ONE_DIGIT_ADDITION)
        );
        assert_eq!(
            regenerate_problem_set(&first.problem_set_id).unwrap(),
            first
        );
        assert_eq!(first.layout.problem_count, 20);
    }

    #[test]
    fn legacy_schema_requests_and_ids_fail_closed() {
        let request = GenerateWorksheetRequest {
            schema_version: 2,
            seed: "Ab3Z".to_owned(),
            ..GenerateWorksheetRequest::default()
        };
        assert_eq!(
            generate_worksheet_request(&request).unwrap_err(),
            GenerationError::UnsupportedSchemaVersion {
                received: 2,
                expected: SCHEMA_VERSION,
            }
        );
        assert_eq!(
            "2-1-2-Ab3Z-3".parse::<ProblemSetIdentity>().unwrap_err(),
            IdentityError::UnsupportedSchemaVersion {
                received: 2,
                expected: SCHEMA_VERSION,
            }
        );
        let legacy_v3 = GenerateWorksheetRequest {
            schema_version: 3,
            seed: "Ab3Z".to_owned(),
            ..GenerateWorksheetRequest::default()
        };
        assert_eq!(
            generate_worksheet_request(&legacy_v3).unwrap_err(),
            GenerationError::UnsupportedSchemaVersion {
                received: 3,
                expected: SCHEMA_VERSION,
            }
        );
        assert_eq!(
            "3-1-3-Ab3Z-3".parse::<ProblemSetIdentity>().unwrap_err(),
            IdentityError::UnsupportedSchemaVersion {
                received: 3,
                expected: SCHEMA_VERSION,
            }
        );
        assert!(serde_json::from_str::<GenerateWorksheetRequest>(
            r#"{"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#
        )
        .is_err());
        assert!(serde_json::from_str::<EditorState>(
            r#"{"answer":{"type":"empty"},"cursor":0,"committed":false}"#
        )
        .is_err());
        assert!(serde_json::from_str::<EditorAction>(r#"{"type":"insert","digit":1}"#).is_err());
    }

    #[test]
    fn generated_addition_uses_restricted_simple_numeric_interface() {
        let problem = generate_problem("Ab3Z").unwrap();
        assert_eq!(
            problem.input_interface,
            AnswerInputInterface::SimpleNumeric {
                allow_decimal: false,
                allow_negative: false,
            }
        );
        assert!(!problem
            .input_interface
            .allows_structure(EditorStructure::Decimal));
        assert!(!problem
            .input_interface
            .allows_structure(EditorStructure::Negative));
    }

    #[test]
    fn nan_error_is_exact_raw_text_and_never_grades_correct() {
        let raw = AnswerNode::NanError("3.1.4.5".to_owned());
        assert_eq!(
            serde_json::to_value(&raw).unwrap(),
            serde_json::json!({"type": "nan_error", "value": "3.1.4.5"})
        );
        assert_eq!(normalize_answer(&raw), raw);
        assert!(raw.is_within_size_limit());
        assert!(AnswerNode::NanError("123456789012345678".to_owned()).is_within_size_limit());
        assert!(!AnswerNode::NanError("1234567890123456789".to_owned()).is_within_size_limit());
        for (expected, actual) in [
            (AnswerNode::Integer(4), raw.clone()),
            (raw.clone(), AnswerNode::Integer(4)),
            (raw.clone(), raw.clone()),
        ] {
            let result = grade_answer(&expected, &actual);
            assert_eq!(result.status, GradeStatus::Incorrect);
            assert!(!result.is_correct);
            assert!(result.warnings.is_empty());
        }
    }

    #[test]
    fn nested_nan_errors_never_compare_equal_or_emit_warnings() {
        let expected = AnswerNode::Tuple(vec![AnswerNode::NanError("1e+".to_owned())]);
        let actual = expected.clone();
        let result = grade_answer(&expected, &actual);
        assert_eq!(result.status, GradeStatus::Incorrect);
        assert!(!result.is_correct);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn nan_error_is_an_editable_slot_at_cursor_boundaries() {
        let interface = AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        };
        let state = EditorState {
            answer: AnswerNode::NanError("1e+".to_owned()),
            cursor: 3,
            active_path: Vec::new(),
            committed: false,
        };
        let moved_left = super::apply_editor_action(&state, &EditorAction::MoveLeft, &interface)
            .expect("left movement should remain in the raw-text slot");
        assert_eq!(moved_left.cursor, 2);
        let moved_right =
            super::apply_editor_action(&moved_left, &EditorAction::MoveRight, &interface)
                .expect("right movement should return to the raw-text slot boundary");
        assert_eq!(moved_right.cursor, 3);
        assert_eq!(
            super::apply_editor_action(&moved_right, &EditorAction::MoveRight, &interface).unwrap(),
            moved_right
        );
    }

    #[test]
    fn nan_error_editing_uses_character_offsets_for_raw_text() {
        let interface = AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        };
        let state = EditorState {
            answer: AnswerNode::NanError("3😀.1".to_owned()),
            cursor: 2,
            active_path: Vec::new(),
            committed: false,
        };
        let inserted =
            super::apply_editor_action(&state, &EditorAction::InsertDigit { digit: 9 }, &interface)
                .unwrap();
        assert_eq!(inserted.answer, AnswerNode::NanError("3😀9.1".to_owned()));
        assert_eq!(inserted.cursor, 3);

        let recovered =
            super::apply_editor_action(&inserted, &EditorAction::Backspace, &interface).unwrap();
        assert_eq!(recovered, state);
    }

    #[test]
    fn malformed_numeric_drafts_are_recoverable_nan_nodes() {
        let interface = AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        };
        let state = EditorState {
            answer: AnswerNode::NanError("3.1.4.5".to_owned()),
            cursor: 3,
            active_path: Vec::new(),
            committed: false,
        };
        let inserted =
            super::apply_editor_action(&state, &EditorAction::InsertDigit { digit: 9 }, &interface)
                .unwrap();
        assert_eq!(inserted.answer, AnswerNode::NanError("3.19.4.5".to_owned()));

        let mut recovered = state;
        for _ in 0..3 {
            recovered =
                super::apply_editor_action(&recovered, &EditorAction::Delete, &interface).unwrap();
        }
        // A malformed raw spelling that would become a decimal remains a
        // nan_error because this interface does not allow decimal nodes.
        assert_eq!(recovered.answer, AnswerNode::NanError("3.15".to_owned()));
        assert_eq!(recovered.cursor, 3);
        assert_eq!(
            super::apply_editor_action(&recovered, &EditorAction::Clear, &interface).unwrap(),
            EditorState::empty()
        );
    }

    #[test]
    fn editor_rejects_invalid_active_positions_and_select_targets() {
        let interface = AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        };
        let invalid_path = EditorState {
            answer: AnswerNode::Integer(12),
            cursor: 0,
            active_path: vec![4],
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(&invalid_path, &EditorAction::Commit, &interface)
                .unwrap_err(),
            EditorError::InvalidPath
        );

        let invalid_cursor = EditorState {
            answer: AnswerNode::Integer(12),
            cursor: 3,
            active_path: Vec::new(),
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(&invalid_cursor, &EditorAction::MoveLeft, &interface)
                .unwrap_err(),
            EditorError::InvalidPath
        );

        let unicode = EditorState {
            answer: AnswerNode::NanError("😀1".to_owned()),
            cursor: 2,
            active_path: Vec::new(),
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(
                &EditorState {
                    cursor: 3,
                    ..unicode.clone()
                },
                &EditorAction::MoveLeft,
                &interface,
            )
            .unwrap_err(),
            EditorError::InvalidPath
        );
        let unicode_inserted = super::apply_editor_action(
            &unicode,
            &EditorAction::InsertDigit { digit: 9 },
            &interface,
        )
        .unwrap();
        assert_eq!(
            unicode_inserted.answer,
            AnswerNode::NanError("😀19".to_owned())
        );
        assert_eq!(unicode_inserted.cursor, 3);

        let fraction = EditorState {
            answer: AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(1)),
                denominator: Box::new(AnswerNode::Integer(2)),
            },
            cursor: 1,
            active_path: vec![0],
            committed: false,
        };
        let structured = structured_interface();
        assert_eq!(
            super::apply_editor_action(
                &fraction,
                &EditorAction::SelectSlot {
                    path: vec![9],
                    cursor: 0,
                },
                &structured,
            )
            .unwrap_err(),
            EditorError::InvalidPath
        );
        assert_eq!(
            super::apply_editor_action(
                &fraction,
                &EditorAction::SelectSlot {
                    path: vec![1],
                    cursor: 2,
                },
                &structured,
            )
            .unwrap_err(),
            EditorError::InvalidPath
        );
    }

    #[test]
    fn editor_enforces_existing_and_candidate_input_capabilities() {
        let simple = AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        };
        for answer in [
            AnswerNode::exact_decimal(12, 1),
            AnswerNode::Negative(Box::new(AnswerNode::Integer(1))),
            AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(1)),
                denominator: Box::new(AnswerNode::Integer(2)),
            },
        ] {
            let state = EditorState {
                answer,
                cursor: 0,
                active_path: Vec::new(),
                committed: false,
            };
            assert_eq!(
                super::apply_editor_action(&state, &EditorAction::Commit, &simple).unwrap_err(),
                EditorError::InputInterfaceViolation
            );
        }

        let signed_integer = EditorState {
            answer: AnswerNode::Integer(-1),
            cursor: 0,
            active_path: Vec::new(),
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(&signed_integer, &EditorAction::Commit, &simple)
                .unwrap_err(),
            EditorError::InputInterfaceViolation
        );

        let signed_decimal = EditorState {
            answer: AnswerNode::ExactDecimal {
                coefficient: -12,
                scale: 1,
            },
            cursor: 0,
            active_path: Vec::new(),
            committed: false,
        };
        let decimal_only = AnswerInputInterface::SimpleNumeric {
            allow_decimal: true,
            allow_negative: false,
        };
        assert_eq!(
            super::apply_editor_action(&signed_decimal, &EditorAction::Commit, &decimal_only)
                .unwrap_err(),
            EditorError::InputInterfaceViolation
        );

        let fraction_only = AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Fraction],
        };
        let signed_nested = EditorState {
            answer: AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(-1)),
                denominator: Box::new(AnswerNode::Integer(2)),
            },
            cursor: 1,
            active_path: vec![1],
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(&signed_nested, &EditorAction::Commit, &fraction_only)
                .unwrap_err(),
            EditorError::InputInterfaceViolation
        );

        let tuple_state = EditorState {
            answer: AnswerNode::Tuple(vec![AnswerNode::Integer(1)]),
            cursor: 1,
            active_path: vec![0],
            committed: false,
        };
        assert_eq!(
            super::apply_editor_action(&tuple_state, &EditorAction::Commit, &fraction_only)
                .unwrap_err(),
            EditorError::InputInterfaceViolation
        );
        assert_eq!(
            super::apply_editor_action(
                &EditorState::empty(),
                &EditorAction::InsertStructure {
                    structure: EditorStructure::Tuple,
                },
                &fraction_only,
            )
            .unwrap_err(),
            EditorError::StructureNotAllowed {
                structure: EditorStructure::Tuple
            }
        );

        let malformed_decimal = EditorState {
            answer: AnswerNode::NanError("1.2".to_owned()),
            cursor: 2,
            active_path: Vec::new(),
            committed: false,
        };
        let retained =
            super::apply_editor_action(&malformed_decimal, &EditorAction::Delete, &simple).unwrap();
        assert_eq!(retained.answer, AnswerNode::NanError("1.".to_owned()));
        let recovered =
            super::apply_editor_action(&retained, &EditorAction::Backspace, &simple).unwrap();
        assert_eq!(recovered.answer, AnswerNode::Integer(1));

        let fraction_only = AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Fraction],
        };
        let structured_nan = EditorState {
            answer: AnswerNode::NanError("1.2".to_owned()),
            cursor: 2,
            active_path: Vec::new(),
            committed: false,
        };
        let retained_structured =
            super::apply_editor_action(&structured_nan, &EditorAction::Delete, &fraction_only)
                .unwrap();
        assert_eq!(
            retained_structured.answer,
            AnswerNode::NanError("1.".to_owned())
        );

        let digits_only = EditorState {
            answer: AnswerNode::NanError("12x3".to_owned()),
            cursor: 2,
            active_path: Vec::new(),
            committed: false,
        };
        let digits_recovered =
            super::apply_editor_action(&digits_only, &EditorAction::Delete, &simple).unwrap();
        assert_eq!(digits_recovered.answer, AnswerNode::Integer(123));
        assert_eq!(digits_recovered.cursor, 2);
    }

    proptest! {
        #[test]
        fn bounded_two_dot_drafts_always_remain_typed_nan_errors(
            left in proptest::collection::vec(0_u8..=9, 0..=3),
            middle in proptest::collection::vec(0_u8..=9, 0..=3),
            right in proptest::collection::vec(0_u8..=9, 0..=3),
        ) {
            let raw = format!(
                "{}.{}.{}",
                left.iter().map(u8::to_string).collect::<String>(),
                middle.iter().map(u8::to_string).collect::<String>(),
                right.iter().map(u8::to_string).collect::<String>(),
            );
            let state = EditorState {
                cursor: raw.len(),
                answer: AnswerNode::NanError(raw.clone()),
                active_path: Vec::new(),
                committed: false,
            };
            let interface = AnswerInputInterface::SimpleNumeric {
                allow_decimal: false,
                allow_negative: false,
            };
            let next = super::apply_editor_action(
                &state,
                &EditorAction::InsertDigit { digit: 1 },
                &interface,
            ).unwrap();
            prop_assert!(matches!(next.answer, AnswerNode::NanError(_)));
            prop_assert!(next.answer.is_within_size_limit());
        }
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
            vec![
                GradeWarning::RedundantDecimal,
                GradeWarning::IntegerFormRequired,
            ]
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
            vec![
                GradeWarning::RedundantNegative,
                GradeWarning::IntegerFormRequired,
            ]
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
    fn grading_handles_redundant_signs_solution_sets_and_exact_square_roots() {
        let two = AnswerNode::Integer(2);
        let plus_minus_two = AnswerNode::PlusMinus(Box::new(two.clone()));

        let double_negative =
            AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(two.clone()))));
        let result = grade_answer(&two, &double_negative);
        assert!(result.is_correct);
        assert!(result.warnings.contains(&GradeWarning::RedundantNegative));

        let double_plus_minus =
            AnswerNode::PlusMinus(Box::new(AnswerNode::PlusMinus(Box::new(two.clone()))));
        let result = grade_answer(&plus_minus_two, &double_plus_minus);
        assert!(result.is_correct);
        assert_eq!(result.warnings, vec![GradeWarning::RedundantPlusMinus]);

        let explicit_symmetric_roots =
            AnswerNode::Tuple(vec![AnswerNode::Integer(2), AnswerNode::Integer(-2)]);
        assert!(grade_answer(&plus_minus_two, &explicit_symmetric_roots).is_correct);
        assert!(grade_answer(&explicit_symmetric_roots, &plus_minus_two).is_correct);

        let explicit_offset_roots =
            AnswerNode::Tuple(vec![AnswerNode::Integer(-2), AnswerNode::Integer(6)]);
        let offset_plus_minus = AnswerNode::Binary {
            operator: crate::answer::AnswerBinaryOperator::Add,
            left: Box::new(AnswerNode::Integer(2)),
            right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(4)))),
        };
        let result = grade_answer(&explicit_offset_roots, &offset_plus_minus);
        assert!(result.is_correct);
        assert_eq!(result.warnings, vec![GradeWarning::SolutionListRequired]);

        let duplicate_solution = AnswerNode::Tuple(vec![two.clone(), two.clone()]);
        let result = grade_answer(&two, &duplicate_solution);
        assert!(!result.is_correct);
        assert_eq!(result.warnings, vec![GradeWarning::DuplicateSolution]);

        let sqrt_16 = AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(16)),
            index: None,
        };
        let result = grade_answer(&AnswerNode::Integer(4), &sqrt_16);
        assert!(result.is_correct);
        assert!(result.warnings.contains(&GradeWarning::IntegerFormRequired));
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
        assert_eq!(generated.get(OperationKind::Increment), 0.0);
        assert_eq!(generated.get(OperationKind::Identity), 1.0);
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
    fn linear_equation_effort_follows_transpose_then_divide_model() {
        let q = |value: i64| RationalCoefficient::new(value, 1).unwrap();
        let answer = AnswerNode::Integer(2);
        let graph = linear_equation_graph(q(3), q(3), q(1), q(7), &answer);
        let vector = graph.operation_vector();
        assert_eq!(vector.get(OperationKind::OverheadLinear), 1.0);
        assert_eq!(vector.get(OperationKind::Transposition), 2.0);
        assert_eq!(vector.get(OperationKind::BaseMinus), 2.0);
        assert_eq!(vector.get(OperationKind::BaseDivide), 1.0);
        assert_eq!(vector.get(OperationKind::BigNum), 2_f64.log10());
    }

    #[test]
    fn linear_equation_effort_compares_different_answers_globally() {
        let q = |value: i64| RationalCoefficient::new(value, 1).unwrap();
        let one = AnswerNode::Integer(1);
        let thirteen_twelfths = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(13)),
            denominator: Box::new(AnswerNode::Integer(12)),
        };
        let easy = calculate_graph_effort(
            &linear_equation_graph(q(12), q(0), q(0), q(12), &one),
            &OperationWeights::default(),
        );
        let hard = calculate_graph_effort(
            &linear_equation_graph(q(12), q(0), q(0), q(13), &thirteen_twelfths),
            &OperationWeights::default(),
        );
        assert!(
            hard.value > easy.value,
            "easy={}, hard={}",
            easy.value,
            hard.value
        );
        assert_eq!(easy.operation_vector.get(OperationKind::OverheadGcd), 0.0);
        // 13/12 is already irreducible, but the standard reduction procedure
        // still performs the GCD search to certify that fact.
        assert_eq!(hard.operation_vector.get(OperationKind::OverheadGcd), 1.0);
        assert!(
            hard.operation_vector.get(OperationKind::BigNum)
                > easy.operation_vector.get(OperationKind::BigNum)
        );
    }

    #[test]
    fn rational_answer_schema_enforces_requested_fraction_representation() {
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(3)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let schema = AnswerSchema::Rational {
            max_abs_numerator: 30,
            max_denominator: 30,
            require_reduced_fraction_form: true,
        };

        let unreduced = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(6)),
            denominator: Box::new(AnswerNode::Integer(4)),
        };
        let unreduced_result = grade_answer_with_schema(&expected, &unreduced, Some(&schema));
        assert!(unreduced_result.is_correct);
        assert_eq!(unreduced_result.status, GradeStatus::Correct);
        assert!(unreduced_result
            .warnings
            .contains(&GradeWarning::FractionNotReduced));

        let mixed = AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(1)),
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let mixed_result = grade_answer_with_schema(&expected, &mixed, Some(&schema));
        assert!(mixed_result.is_correct);
        assert!(mixed_result
            .warnings
            .contains(&GradeWarning::FractionFormRequired));

        let decimal = AnswerNode::ExactDecimal {
            coefficient: 15,
            scale: 1,
        };
        let decimal_result = grade_answer_with_schema(&expected, &decimal, Some(&schema));
        assert!(decimal_result.is_correct);
        assert!(decimal_result
            .warnings
            .contains(&GradeWarning::FractionFormRequired));
    }

    #[test]
    fn equivalent_noncanonical_forms_return_visible_warnings() {
        let integer_schema = AnswerSchema::Integer { min: -15, max: 15 };
        let integer_expected = AnswerNode::Integer(-1);
        let fraction_one = AnswerNode::Negative(Box::new(AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(1)),
        }));
        let fraction_one_result =
            grade_answer_with_schema(&integer_expected, &fraction_one, Some(&integer_schema));
        assert!(fraction_one_result.is_correct);
        assert!(fraction_one_result
            .warnings
            .contains(&GradeWarning::IntegerFormRequired));

        let zero_root = AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(0)),
            index: None,
        };
        let zero_root_result =
            grade_answer_with_schema(&AnswerNode::Integer(0), &zero_root, Some(&integer_schema));
        assert!(zero_root_result.is_correct);
        assert!(zero_root_result
            .warnings
            .contains(&GradeWarning::IntegerFormRequired));

        let fraction_schema = AnswerSchema::Rational {
            max_abs_numerator: 20,
            max_denominator: 12,
            require_reduced_fraction_form: true,
        };
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(11)),
            denominator: Box::new(AnswerNode::Integer(6)),
        };
        let nested = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(11)),
                denominator: Box::new(AnswerNode::Integer(3)),
            }),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let nested_result = grade_answer_with_schema(&expected, &nested, Some(&fraction_schema));
        assert!(nested_result.is_correct);
        assert!(nested_result
            .warnings
            .contains(&GradeWarning::FractionFormRequired));
    }

    #[test]
    fn linear_equation_themes_generate_registered_bounded_solutions() {
        for &(theme_id, expected_integer) in &[
            (THEME_ID_LINEAR_EQUATION_1, true),
            (THEME_ID_LINEAR_EQUATION_2, false),
        ] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: theme_id,
                seed: "LinEqA7".to_owned(),
                difficulty: Difficulty::try_from(3).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(20_000),
            })
            .unwrap();
            assert_eq!(worksheet.layout.problem_count, 16);
            assert_eq!(worksheet.layout.columns, 2);
            assert_eq!(worksheet.layout.rows, 8);
            assert_eq!(worksheet.problems.len(), 16);
            assert_eq!(
                worksheet.identity.generator_revision,
                active_registration(theme_id).unwrap().generator_revision
            );

            for problem in &worksheet.problems {
                let ProblemPrompt::LinearEquation { a, b, c, d, .. } = problem.prompt else {
                    panic!("linear theme returned a non-linear prompt");
                };
                assert!(!a.is_zero(), "every admitted shape must contain ax");
                if !c.is_zero() {
                    assert_ne!(a, c, "two-sided x terms must have a unique solution");
                }
                match (c.is_zero(), d.is_zero()) {
                    (true, true) => {
                        // ax + b = 0; b may itself be zero.
                    }
                    (true, false) => {
                        // ax + b = d
                    }
                    (false, true) => {
                        // ax + b = cx, but the degenerate ax = cx form is banned.
                        assert!(!b.is_zero(), "ax = cx must be rejected");
                    }
                    (false, false) => {
                        // ax + b = cx + d
                    }
                }
                for coefficient in [a, b, c, d] {
                    if theme_id == THEME_ID_LINEAR_EQUATION_1 {
                        assert_eq!(coefficient.denominator, 1);
                        assert!(coefficient.numerator.abs() <= 15);
                    } else if coefficient.denominator == 1 {
                        assert!(coefficient.numerator.abs() <= 15);
                    } else {
                        assert!(
                            coefficient.numerator.unsigned_abs() + coefficient.denominator as u64
                                <= 10
                        );
                    }
                }
                match &problem.input_interface {
                    AnswerInputInterface::StructuredMath { allowed_structures } => {
                        assert_eq!(allowed_structures.len(), 7);
                    }
                    _ => panic!("linear equations must expose the rich keyboard"),
                }
                let normalized = normalize_answer(&problem.canonical_answer);
                match normalized {
                    AnswerNode::Integer(value) => {
                        assert!(value.abs() <= 15);
                    }
                    AnswerNode::Fraction {
                        numerator,
                        denominator,
                    } if !expected_integer => {
                        let numerator = numerator.as_integer().unwrap();
                        let denominator = denominator.as_integer().unwrap();
                        if denominator == 2 {
                            assert!(numerator.unsigned_abs() <= 20);
                        } else {
                            assert!((3..=12).contains(&denominator));
                            assert!(numerator.unsigned_abs() <= 15);
                        }
                    }
                    _ => panic!("theme returned a solution outside its answer domain"),
                }
                if expected_integer {
                    assert!(matches!(problem.canonical_answer, AnswerNode::Integer(_)));
                }
            }
        }
    }

    #[test]
    fn linear_equation_difficulty_compares_the_full_candidate_pool() {
        let means = (MIN_DIFFICULTY..=3)
            .map(|level| {
                ["EqA", "EqB", "EqC", "EqD", "EqE", "EqF", "EqG", "EqH"]
                    .iter()
                    .map(|seed| {
                        generate_worksheet_request(&GenerateWorksheetRequest {
                            schema_version: SCHEMA_VERSION,
                            numeric_theme_id: THEME_ID_LINEAR_EQUATION_2,
                            seed: (*seed).to_owned(),
                            difficulty: Difficulty::try_from(level).unwrap(),
                            timeout_ms: Some(1_000),
                            max_attempts: Some(50_000),
                        })
                        .unwrap()
                        .problems
                        .iter()
                        .map(|problem| problem.effort)
                        .sum::<f64>()
                            / 16.0
                    })
                    .sum::<f64>()
                    / 8.0
            })
            .collect::<Vec<_>>();
        assert!(means.windows(2).all(|pair| pair[0] < pair[1]), "{means:?}");
    }

    #[test]
    fn rational_linear_equations_frequently_require_final_reduction() {
        fn gcd(mut left: u64, mut right: u64) -> u64 {
            while right != 0 {
                let remainder = left % right;
                left = right;
                right = remainder;
            }
            left
        }

        fn requires_reduction(problem: &Problem) -> bool {
            let ProblemPrompt::LinearEquation { a, b, c, d, .. } = problem.prompt else {
                return false;
            };
            let Some(a_total) = a.subtract(c) else {
                return false;
            };
            let Some(b_total) = d.subtract(b) else {
                return false;
            };
            let Some(raw_numerator) = b_total.numerator.checked_mul(a_total.denominator) else {
                return false;
            };
            let Some(raw_denominator) = b_total.denominator.checked_mul(a_total.numerator) else {
                return false;
            };
            raw_denominator != 0
                && gcd(raw_numerator.unsigned_abs(), raw_denominator.unsigned_abs()) > 1
        }

        let mut total = 0_usize;
        let mut reducible = 0_usize;
        for seed in ["RedA", "RedB", "RedC", "RedD", "RedE", "RedF"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_LINEAR_EQUATION_2,
                seed: seed.to_owned(),
                difficulty: Difficulty::try_from(3).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            for problem in &worksheet.problems {
                total += 1;
                reducible += usize::from(requires_reduction(problem));
            }
        }
        assert!(
            reducible * 4 >= total * 3,
            "expected at least 75% to require reduction; got {reducible}/{total}"
        );
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
            0.5, 2.0, 2.0, 2.0, 4.0, 3.0, 2.0, 5.0, 6.0, 3.0, 1.0, 1.0,
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
        let means: Vec<f64> = (MIN_DIFFICULTY..=3)
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

    #[test]
    fn random_difficulty_has_no_easy_or_hard_rank_bias() {
        let mean = |level: u8| {
            (1..=128)
                .map(|seed| {
                    let request = GenerateWorksheetRequest {
                        seed: format!("R{}", seed.to_string().replace('0', "A")),
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
                / 128.0
        };
        let easy = mean(1);
        let random = mean(4);
        let hard = mean(3);
        assert!(
            easy < random && random < hard,
            "easy={easy}, random={random}, hard={hard}"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(8))]

        #[test]
        fn difficulty_order_statistic_bias_is_monotonic_property(salt in 1_u8..=9) {
            let means: Vec<f64> = (MIN_DIFFICULTY..=3)
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

    #[test]
    fn oversized_composites_are_rejected_before_non_mutating_editor_actions() {
        let oversized = AnswerNode::Tuple(
            (0..=MAX_ANSWER_AST_SIZE)
                .map(|_| AnswerNode::Integer(1))
                .collect(),
        );
        assert!(!oversized.is_within_size_limit());
        let state = EditorState {
            answer: oversized,
            cursor: 0,
            active_path: vec![0],
            committed: false,
        };
        let expected_error = EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        };

        assert_eq!(
            apply_editor_action(&state, &EditorAction::Commit).unwrap_err(),
            expected_error
        );
        assert_eq!(
            apply_editor_action(
                &state,
                &EditorAction::SelectSlot {
                    path: vec![0],
                    cursor: 0,
                },
            )
            .unwrap_err(),
            expected_error
        );
        assert_eq!(
            apply_editor_action(&state, &EditorAction::Clear).unwrap(),
            EditorState::empty()
        );
    }

    #[test]
    fn empty_children_use_structural_budget_without_changing_display_size() {
        let within_limit = AnswerNode::Tuple(
            (0..MAX_ANSWER_AST_SIZE - 1)
                .map(|_| AnswerNode::Empty)
                .collect(),
        );
        assert_eq!(within_limit.size(), 1);
        assert!(within_limit.is_within_size_limit());

        let oversized = AnswerNode::Tuple(
            (0..MAX_ANSWER_AST_SIZE)
                .map(|_| AnswerNode::Empty)
                .collect(),
        );
        assert_eq!(AnswerNode::Empty.size(), 0);
        assert_eq!(oversized.size(), 1);
        assert!(!oversized.is_within_size_limit());

        let state = EditorState {
            answer: oversized,
            cursor: 0,
            active_path: vec![0],
            committed: false,
        };
        let expected_error = EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        };
        assert_eq!(
            apply_editor_action(&state, &EditorAction::Commit).unwrap_err(),
            expected_error
        );
        assert_eq!(
            apply_editor_action(
                &state,
                &EditorAction::SelectSlot {
                    path: vec![0],
                    cursor: 0,
                },
            )
            .unwrap_err(),
            expected_error
        );
        assert_eq!(
            apply_editor_action(&state, &EditorAction::Clear).unwrap(),
            EditorState::empty()
        );
    }

    #[test]
    fn partially_filled_structures_stay_within_the_combined_input_limit() {
        let answers = [
            AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Empty),
                denominator: Box::new(AnswerNode::Integer(2)),
            },
            AnswerNode::MixedFraction {
                whole: Box::new(AnswerNode::Integer(1)),
                numerator: Box::new(AnswerNode::Empty),
                denominator: Box::new(AnswerNode::Empty),
            },
            AnswerNode::Root {
                radicand: Box::new(AnswerNode::Empty),
                index: Some(Box::new(AnswerNode::Empty)),
            },
        ];

        for answer in answers {
            assert!(answer.size() <= MAX_ANSWER_AST_SIZE);
            assert!(answer.is_within_size_limit());
        }
    }

    #[test]
    fn size_validation_and_extreme_decimal_normalization_are_bounded() {
        let extreme = AnswerNode::exact_decimal(0, u32::MAX);
        assert!(!extreme.is_within_size_limit());
        assert_eq!(normalize_answer(&extreme), AnswerNode::Integer(0));
    }

    #[test]
    fn backspace_on_empty_structured_slot_removes_the_shallowest_ast_node() {
        let apply = |state: &EditorState, action: EditorAction| {
            apply_editor_action(state, &action).unwrap()
        };

        let fraction = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::Fraction,
            },
        );
        assert_eq!(fraction.active_path, vec![0]);
        assert_eq!(
            apply(&fraction, EditorAction::Backspace),
            EditorState::empty()
        );

        let negative = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::Negative,
            },
        );
        let nested = apply(
            &negative,
            EditorAction::InsertStructure {
                structure: EditorStructure::Fraction,
            },
        );
        assert_eq!(nested.active_path, vec![0, 0]);
        assert_eq!(
            apply(&nested, EditorAction::Backspace),
            EditorState::empty()
        );
    }

    #[test]
    fn structured_editor_builds_exact_decimal_fraction_mixed_root_and_tuple_nodes() {
        let apply = |state: &EditorState, action: EditorAction| {
            apply_editor_action(state, &action).unwrap()
        };

        let mut fraction = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::Fraction,
            },
        );
        assert_eq!(fraction.active_path, vec![0]);
        fraction = apply(&fraction, EditorAction::InsertDigit { digit: 1 });
        fraction = apply(&fraction, EditorAction::MoveRight);
        assert_eq!(fraction.active_path, vec![1]);
        fraction = apply(&fraction, EditorAction::InsertDigit { digit: 2 });
        assert_eq!(
            fraction.answer,
            AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(1)),
                denominator: Box::new(AnswerNode::Integer(2)),
            }
        );

        let mut decimal = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::Decimal,
            },
        );
        decimal = apply(&decimal, EditorAction::InsertDigit { digit: 5 });
        assert_eq!(decimal.answer, AnswerNode::exact_decimal(5, 1));
        assert_eq!(decimal.cursor, 3);

        let decimal_from_front = apply(
            &EditorState {
                answer: AnswerNode::Integer(12),
                cursor: 0,
                active_path: Vec::new(),
                committed: false,
            },
            EditorAction::InsertStructure {
                structure: EditorStructure::Decimal,
            },
        );
        assert_eq!(decimal_from_front.answer, AnswerNode::exact_decimal(12, 2));
        assert_eq!(decimal_from_front.cursor, 2);

        let mut mixed = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::MixedFraction,
            },
        );
        mixed = apply(&mixed, EditorAction::InsertDigit { digit: 1 });
        mixed = apply(&mixed, EditorAction::MoveRight);
        mixed = apply(&mixed, EditorAction::InsertDigit { digit: 1 });
        mixed = apply(&mixed, EditorAction::MoveRight);
        mixed = apply(&mixed, EditorAction::InsertDigit { digit: 2 });
        assert_eq!(
            mixed.answer,
            AnswerNode::MixedFraction {
                whole: Box::new(AnswerNode::Integer(1)),
                numerator: Box::new(AnswerNode::Integer(1)),
                denominator: Box::new(AnswerNode::Integer(2)),
            }
        );

        let rooted = apply(
            &fraction,
            EditorAction::SelectSlot {
                path: vec![0],
                cursor: 1,
            },
        );
        let rooted = apply(
            &rooted,
            EditorAction::InsertStructure {
                structure: EditorStructure::Root,
            },
        );
        assert!(matches!(
            rooted.answer,
            AnswerNode::Fraction { numerator, .. }
                if matches!(numerator.as_ref(), AnswerNode::Root { .. })
        ));

        let mut tuple = apply(
            &decimal,
            EditorAction::InsertStructure {
                structure: EditorStructure::Tuple,
            },
        );
        tuple = apply(&tuple, EditorAction::InsertDigit { digit: 5 });
        assert_eq!(
            tuple.answer,
            AnswerNode::Tuple(vec![
                AnswerNode::exact_decimal(5, 1),
                AnswerNode::Integer(5)
            ])
        );
        assert_eq!(
            normalize_answer(&tuple.answer),
            AnswerNode::Tuple(vec![
                AnswerNode::Fraction {
                    numerator: Box::new(AnswerNode::Integer(1)),
                    denominator: Box::new(AnswerNode::Integer(2)),
                },
                AnswerNode::Integer(5),
            ])
        );

        let negative = apply(
            &EditorState::empty(),
            EditorAction::InsertStructure {
                structure: EditorStructure::Negative,
            },
        );
        let plus_minus = apply(
            &negative,
            EditorAction::InsertStructure {
                structure: EditorStructure::PlusMinus,
            },
        );
        assert!(matches!(
            plus_minus.answer,
            AnswerNode::Negative(value) if matches!(value.as_ref(), AnswerNode::PlusMinus(_))
        ));

        let cleared_variable = apply(
            &EditorState {
                answer: AnswerNode::Variable("x".to_owned()),
                cursor: 10,
                active_path: vec![9],
                committed: true,
            },
            EditorAction::Clear,
        );
        assert_eq!(cleared_variable, EditorState::empty());
    }

    #[test]
    fn structured_editor_rejects_templates_beyond_the_ast_size_limit() {
        let mut state = EditorState::empty();
        for _ in 0..MAX_ANSWER_AST_SIZE {
            state = apply_editor_action(&state, &EditorAction::InsertDigit { digit: 1 }).unwrap();
        }
        let before = state.clone();
        let error = apply_editor_action(
            &state,
            &EditorAction::InsertStructure {
                structure: EditorStructure::Fraction,
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            EditorError::AnswerSizeLimit {
                max_size: MAX_ANSWER_AST_SIZE
            }
        );
        assert_eq!(state, before);
    }

    #[test]
    fn structured_editor_rejects_over_limit_decimal_before_formatting_it() {
        let state = EditorState {
            answer: AnswerNode::exact_decimal(0, u32::MAX),
            cursor: 0,
            active_path: Vec::new(),
            committed: false,
        };
        let error =
            apply_editor_action(&state, &EditorAction::InsertDigit { digit: 1 }).unwrap_err();
        assert_eq!(
            error,
            EditorError::AnswerSizeLimit {
                max_size: MAX_ANSWER_AST_SIZE
            }
        );
    }
}
