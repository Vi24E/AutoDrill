#![forbid(unsafe_code)]

mod answer;
mod contract;
mod effort;
mod error;
mod exact;
mod exact_value;
mod generator;
mod generator_support;
mod grade;
mod identity;
mod input;
mod mathlive_input;
mod model;
mod normalize;
mod registry;
mod rng;
mod schema;
mod semantics;
mod theme;
mod themes;
mod wire;

pub use answer::{AnswerBinaryOperator, AnswerNode};
pub use contract::web_contract;
#[cfg(feature = "qa-diagnostics")]
#[doc(hidden)]
pub use effort::QA_OPERATION_VECTOR_BASIS;
pub use error::{EditorError, GenerationError};
pub use generator::{
    generate_identity_with_clock, generate_problem_set_from_id,
    generate_problem_set_from_id_with_clock, generate_worksheet_request,
    generate_worksheet_request_with_clock, GenerationConfig, MonotonicClock, DEFAULT_MAX_ATTEMPTS,
    DEFAULT_TIMEOUT,
};
pub use grade::{grade_answer, grade_answer_with_schema, GradeError};
pub use identity::{
    validate_seed, Difficulty, IdentityError, ProblemSetIdentity, DEFAULT_DIFFICULTY,
    MAX_DIFFICULTY, MAX_SEED_LENGTH, MIN_DIFFICULTY,
};
pub use mathlive_input::parse_mathlive_answer;
pub use model::{
    AnswerInputInterface, AnswerSchema, EditorStructure, GenerateWorksheetRequest, GradeResult,
    GradeStatus, GradeWarning, LayoutMetadata, Problem, Worksheet, MAX_ANSWER_AST_SIZE,
};
pub use schema::SCHEMA_VERSION;
#[doc(hidden)]
pub use wire::GradeResultWire;
#[cfg(feature = "wire-types")]
pub use wire::WorksheetWire;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use proptest::prelude::*;

    use super::*;
    use crate::effort::{
        big_num_operations, calculate_plan_effort, linear_equation_plan, one_digit_addition_plan,
        signed_addition_plan, signed_subtraction_plan, Operation, OperationKind, OperationPlan,
        OperationWeights, OPERATION_KIND_COUNT,
    };
    use crate::generator::StepClock;
    use crate::model::{ProblemPrompt, RationalCoefficient};
    use crate::normalize::normalize_answer;
    use crate::registry::active_registration;
    use crate::themes::basic_arithmetic::{
        GENERATOR_REVISION_ONE_DIGIT_ADDITION, MAX_ANSWER, MAX_OPERAND, MIN_ANSWER, MIN_OPERAND,
        ONE_DIGIT_ADDITION_REGISTRATION, THEME_ID_ONE_DIGIT_ADDITION,
    };
    use crate::themes::equations::{
        THEME_ID_LINEAR_EQUATION_1, THEME_ID_LINEAR_EQUATION_2, THEME_ID_LINEAR_EQUATION_3,
        THEME_ID_LINEAR_EQUATION_SIMPLE,
    };

    fn grade_answer_with_schema(
        expected: &AnswerNode,
        actual: &AnswerNode,
        answer_schema: Option<&AnswerSchema>,
    ) -> GradeResult {
        crate::grade::grade_answer_with_schema(expected, actual, answer_schema)
            .expect("test answer schema must be valid")
    }

    fn one_digit_pair(problem: &Problem) -> (u8, u8) {
        match problem.prompt() {
            ProblemPrompt::Addition { left, right } => (*left, *right),
            other => panic!("expected one-digit addition prompt, got {other:?}"),
        }
    }

    fn one_digit_answer(problem: &Problem) -> u8 {
        match problem.canonical_answer() {
            AnswerNode::Integer(value) => u8::try_from(*value).expect("one-digit answer"),
            other => panic!("expected integer answer, got {other:?}"),
        }
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

    fn one_digit_worksheet_request(
        seed: impl Into<String>,
        difficulty: Difficulty,
    ) -> GenerateWorksheetRequest {
        GenerateWorksheetRequest::new(THEME_ID_ONE_DIGIT_ADDITION, seed, difficulty)
    }

    fn one_digit_worksheet(seed: impl Into<String>) -> Worksheet {
        generate_worksheet_request(&one_digit_worksheet_request(seed, DEFAULT_DIFFICULTY)).unwrap()
    }

    fn one_digit_problem(seed: impl Into<String>) -> Problem {
        one_digit_worksheet(seed).problems()[0].clone()
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
            prop_assert_eq!(&decoded, &request);

            let identity = ProblemSetIdentity::new(
                THEME_ID_ONE_DIGIT_ADDITION,
                GENERATOR_REVISION_ONE_DIGIT_ADDITION,
                seed,
                difficulty,
            ).unwrap();
            let id = identity.to_string();
            prop_assert_eq!(id.parse::<ProblemSetIdentity>().unwrap(), identity);
            let generated = generate_worksheet_request(&request).unwrap();
            let replayed = generate_problem_set_from_id(&id).unwrap();
            prop_assert_eq!(replayed, generated);
        }

        #[test]
        fn generated_operands_answers_and_final_expressions_are_valid(seed in valid_seed_strategy()) {
            let worksheet = one_digit_worksheet(seed.clone());
            let mut unique = HashSet::new();
            prop_assert_eq!(worksheet.problems().len(), ONE_DIGIT_ADDITION_REGISTRATION.layout().problem_count());
            for problem in worksheet.into_problems() {
                let (left, right) = one_digit_pair(&problem);
                prop_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&left));
                prop_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&right));
                prop_assert!((MIN_ANSWER..=MAX_ANSWER).contains(&one_digit_answer(&problem)));
                prop_assert_eq!(one_digit_answer(&problem), left + right);
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
            let plan = one_digit_addition_plan(8, 9).expect("valid one-digit addition");
            let vector = plan.operation_vector();
            prop_assert_eq!(vector.as_array().len(), OPERATION_KIND_COUNT);
            prop_assert!(vector.is_nonnegative_finite());
            let base = OperationWeights::default();
            let baseline = base.weighted_sum(&vector);
            let mut changed = base.clone();
            changed.override_weight(OperationKind::BasePlus, multiplier).unwrap();
            let recomputed = changed.weighted_sum(&vector);
            prop_assert!((recomputed - baseline - (multiplier - 3.0)).abs() < 1e-9);
        }

    }

    #[test]
    fn problem_set_id_and_layout_match_current_identity_contract() {
        let first = one_digit_worksheet("Ab3Z");
        assert_eq!(
            first.problem_set_id(),
            format!(
                "{}-1-{}-Ab3Z-2",
                SCHEMA_VERSION, GENERATOR_REVISION_ONE_DIGIT_ADDITION
            )
        );
        assert_eq!(first.layout().problem_count, 20);
    }

    #[test]
    fn unsupported_schema_requests_and_ids_fail_closed() {
        let mut request = one_digit_worksheet_request("Ab3Z", DEFAULT_DIFFICULTY);
        request.schema_version = 2;
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
        let mut unsupported_v3 = one_digit_worksheet_request("Ab3Z", DEFAULT_DIFFICULTY);
        unsupported_v3.schema_version = 3;
        assert_eq!(
            generate_worksheet_request(&unsupported_v3).unwrap_err(),
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
    }

    #[test]
    fn generated_addition_uses_restricted_simple_numeric_interface() {
        let problem = one_digit_problem("Ab3Z");
        assert_eq!(
            problem.input_interface(),
            &AnswerInputInterface::SimpleNumeric {
                allow_decimal: false,
                allow_negative: false,
            }
        );
        assert!(!problem
            .input_interface()
            .allows_structure(EditorStructure::Decimal));
        assert!(!problem
            .input_interface()
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
            assert_eq!(result.status(), GradeStatus::Incorrect);
            assert!(!result.is_correct());
            assert!(result.warnings().is_empty());
        }
    }

    #[test]
    fn nested_nan_errors_never_compare_equal_or_emit_warnings() {
        let expected = AnswerNode::Tuple(vec![AnswerNode::NanError("1e+".to_owned())]);
        let actual = expected.clone();
        let result = grade_answer(&expected, &actual);
        assert_eq!(result.status(), GradeStatus::Incorrect);
        assert!(!result.is_correct());
        assert!(result.warnings().is_empty());
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
        assert!(reduced.is_correct());
        assert_eq!(reduced.warnings(), vec![GradeWarning::FractionNotReduced]);

        let alternate_exact_form = grade_answer(&half, &decimal_half);
        assert!(alternate_exact_form.is_correct());
        assert!(alternate_exact_form.warnings().is_empty());

        let decimal_integer = grade_answer(
            &AnswerNode::Integer(4),
            &AnswerNode::ExactDecimal {
                coefficient: 40,
                scale: 1,
            },
        );
        assert!(decimal_integer.is_correct());
        assert_eq!(
            decimal_integer.warnings(),
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
        assert!(grade_answer(&mixed, &three_halves).is_correct());

        let double_negative = AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(
            AnswerNode::Integer(2),
        ))));
        let negative_warning = grade_answer(&AnswerNode::Integer(2), &double_negative);
        assert!(negative_warning.is_correct());
        assert_eq!(
            negative_warning.warnings(),
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
        assert!(signed_fraction_warning.is_correct());
        assert_eq!(
            signed_fraction_warning.warnings(),
            vec![GradeWarning::RedundantNegative]
        );

        let multiple = AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(two_fourths))));
        let multiple_warnings = grade_answer(&half, &multiple);
        assert!(multiple_warnings.is_correct());
        assert_eq!(
            multiple_warnings.warnings(),
            vec![
                GradeWarning::FractionNotReduced,
                GradeWarning::RedundantNegative,
            ]
        );

        let same_notation = grade_answer(&half, &half);
        assert!(same_notation.is_correct());
        assert!(same_notation.warnings().is_empty());

        let incorrect = grade_answer(&AnswerNode::Integer(3), &double_negative);
        assert!(!incorrect.is_correct());
        assert!(incorrect.warnings().is_empty());
    }

    #[test]
    fn grading_handles_redundant_signs_solution_sets_and_exact_square_roots() {
        let two = AnswerNode::Integer(2);
        let plus_minus_two = AnswerNode::PlusMinus(Box::new(two.clone()));

        let double_negative =
            AnswerNode::Negative(Box::new(AnswerNode::Negative(Box::new(two.clone()))));
        let result = grade_answer(&two, &double_negative);
        assert!(result.is_correct());
        assert!(result.warnings().contains(&GradeWarning::RedundantNegative));

        let double_plus_minus =
            AnswerNode::PlusMinus(Box::new(AnswerNode::PlusMinus(Box::new(two.clone()))));
        let result = grade_answer(&plus_minus_two, &double_plus_minus);
        assert!(result.is_correct());
        assert_eq!(result.warnings(), vec![GradeWarning::RedundantPlusMinus]);

        let explicit_symmetric_roots =
            AnswerNode::Tuple(vec![AnswerNode::Integer(2), AnswerNode::Integer(-2)]);
        assert!(grade_answer(&plus_minus_two, &explicit_symmetric_roots).is_correct());
        assert!(grade_answer(&explicit_symmetric_roots, &plus_minus_two).is_correct());

        let explicit_offset_roots =
            AnswerNode::Tuple(vec![AnswerNode::Integer(-2), AnswerNode::Integer(6)]);
        let offset_plus_minus = AnswerNode::Binary {
            operator: crate::answer::AnswerBinaryOperator::Add,
            left: Box::new(AnswerNode::Integer(2)),
            right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(4)))),
        };
        let result = grade_answer(&explicit_offset_roots, &offset_plus_minus);
        assert!(result.is_correct());
        assert_eq!(result.warnings(), vec![GradeWarning::SolutionListRequired]);

        let duplicate_solution = AnswerNode::Tuple(vec![two.clone(), two.clone()]);
        let result = grade_answer(&two, &duplicate_solution);
        assert!(!result.is_correct());
        assert_eq!(result.warnings(), vec![GradeWarning::DuplicateSolution]);

        let sqrt_16 = AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(16)),
            index: None,
        };
        let result = grade_answer(&AnswerNode::Integer(4), &sqrt_16);
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::IntegerFormRequired));
    }

    #[test]
    fn normalization_never_saturates_i64_min_negation() {
        let value = AnswerNode::Negative(Box::new(AnswerNode::Integer(i64::MIN)));
        assert_eq!(normalize_answer(&value), value);
    }

    #[test]
    fn ordered_pairs_are_directional() {
        let observed = (1..=128).any(|seed| {
            let sheet = one_digit_worksheet(seed.to_string());
            let set: HashSet<_> = sheet.problems().iter().map(one_digit_pair).collect();
            set.iter()
                .any(|(left, right)| left != right && set.contains(&(*right, *left)))
        });
        assert!(observed);
    }

    #[test]
    fn timeout_and_attempt_limit_are_distinct() {
        let mut request = one_digit_worksheet_request("Ab3Z", DEFAULT_DIFFICULTY);
        request.timeout_ms = Some(5);
        let timeout_clock = StepClock::new(Duration::ZERO, Duration::from_millis(10));
        let timeout = generate_worksheet_request_with_clock(&request, &timeout_clock).unwrap_err();
        assert!(matches!(timeout, GenerationError::Timeout { .. }));
        assert_eq!(timeout.code(), "generation_timeout");

        let mut request = one_digit_worksheet_request("Ab3Z", DEFAULT_DIFFICULTY);
        request.timeout_ms = Some(1_000);
        request.max_attempts = Some(1);
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
    fn operation_plan_counts_each_primitive_once() {
        let plan = OperationPlan::new(vec![
            Operation::BasePlus,
            Operation::Increment,
            Operation::OverheadCarryPlus,
        ]);
        let vector = plan.operation_vector();
        assert_eq!(vector.get(OperationKind::BasePlus), 1.0);
        assert_eq!(vector.get(OperationKind::Increment), 1.0);
        assert_eq!(vector.get(OperationKind::OverheadCarryPlus), 1.0);
        let generated = one_digit_addition_plan(8, 9)
            .expect("valid one-digit addition")
            .operation_vector();
        assert_eq!(generated.get(OperationKind::BasePlus), 1.0);
        assert_eq!(generated.get(OperationKind::Increment), 0.0);
        assert_eq!(generated.get(OperationKind::Identity), 1.0);
        assert_eq!(generated.get(OperationKind::OverheadCarryPlus), 1.0);
        assert_eq!(generated.get(OperationKind::BigNum), 17f64.log10());
        let magnitudes: Vec<_> = one_digit_addition_plan(8, 9)
            .expect("valid one-digit addition")
            .operations()
            .iter()
            .filter_map(|operation| match operation {
                Operation::BigNum { magnitude } => Some(*magnitude),
                _ => None,
            })
            .collect();
        assert_eq!(magnitudes, vec![17]);
    }

    #[test]
    fn linear_equation_effort_follows_transpose_then_divide_model() {
        let q = |value: i64| RationalCoefficient::new(value, 1).unwrap();
        let answer = AnswerNode::Integer(2);
        let plan =
            linear_equation_plan(q(3), q(3), q(1), q(7), &answer).expect("valid linear equation");
        let vector = plan.operation_vector();
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
        let easy = calculate_plan_effort(
            &linear_equation_plan(q(12), q(0), q(0), q(12), &one).expect("valid linear equation"),
            &OperationWeights::default(),
        );
        let hard = calculate_plan_effort(
            &linear_equation_plan(q(12), q(0), q(0), q(13), &thirteen_twelfths)
                .expect("valid linear equation"),
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
        assert!(unreduced_result.is_correct());
        assert_eq!(unreduced_result.status(), GradeStatus::Correct);
        assert!(unreduced_result
            .warnings()
            .contains(&GradeWarning::FractionNotReduced));

        let mixed = AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(1)),
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let mixed_result = grade_answer_with_schema(&expected, &mixed, Some(&schema));
        assert!(mixed_result.is_correct());
        assert!(mixed_result
            .warnings()
            .contains(&GradeWarning::FractionFormRequired));

        let decimal = AnswerNode::ExactDecimal {
            coefficient: 15,
            scale: 1,
        };
        let decimal_result = grade_answer_with_schema(&expected, &decimal, Some(&schema));
        assert!(decimal_result.is_correct());
        assert!(decimal_result
            .warnings()
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
        assert!(fraction_one_result.is_correct());
        assert!(fraction_one_result
            .warnings()
            .contains(&GradeWarning::IntegerFormRequired));

        let zero_root = AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(0)),
            index: None,
        };
        let zero_root_result =
            grade_answer_with_schema(&AnswerNode::Integer(0), &zero_root, Some(&integer_schema));
        assert!(zero_root_result.is_correct());
        assert!(zero_root_result
            .warnings()
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
        assert!(nested_result.is_correct());
        assert!(nested_result
            .warnings()
            .contains(&GradeWarning::FractionFormRequired));
    }

    #[test]
    fn linear_equation_themes_generate_registered_bounded_solutions() {
        for &(theme_id, expected_integer) in &[
            (THEME_ID_LINEAR_EQUATION_SIMPLE, true),
            (THEME_ID_LINEAR_EQUATION_1, true),
            (THEME_ID_LINEAR_EQUATION_2, false),
            (THEME_ID_LINEAR_EQUATION_3, false),
        ] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: theme_id,
                seed: "LinEqA7".to_owned(),
                difficulty: Difficulty::try_from(3).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            assert_eq!(worksheet.layout().problem_count, 16);
            assert_eq!(worksheet.layout().columns, 2);
            assert_eq!(worksheet.layout().rows, 8);
            assert_eq!(worksheet.problems().len(), 16);
            assert_eq!(
                worksheet.identity().generator_revision(),
                active_registration(theme_id)
                    .unwrap()
                    .unwrap()
                    .generator_revision()
            );

            for problem in worksheet.problems() {
                let ProblemPrompt::LinearEquation { left, right } = problem.prompt() else {
                    panic!("linear theme returned a non-linear prompt");
                };
                let (a, left_y, b) = crate::semantics::normalize_linear_expression(left).unwrap();
                assert!(left_y.is_zero());
                let (c, right_y, d) = crate::semantics::normalize_linear_expression(right).unwrap();
                assert!(right_y.is_zero());
                assert_ne!(a, c, "linear equation must have a unique solution");
                if theme_id != THEME_ID_LINEAR_EQUATION_3 {
                    for coefficient in [a, b, c, d] {
                        assert_eq!(coefficient.denominator(), 1);
                    }
                }
                match problem.input_interface() {
                    AnswerInputInterface::StructuredMath { allowed_structures } => {
                        assert_eq!(allowed_structures.len(), 7);
                    }
                    _ => panic!("linear equations must expose the rich keyboard"),
                }
                let normalized = normalize_answer(problem.canonical_answer());
                match &normalized {
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
                    assert!(matches!(problem.canonical_answer(), AnswerNode::Integer(_)));
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
                        .problems()
                        .iter()
                        .map(|problem| problem.effort())
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
    fn negative_operand_overhead_follows_structural_rewrite_rule() {
        assert_eq!(
            signed_addition_plan(2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_plan(5, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_plan(3, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            0.0
        );
        assert_eq!(
            signed_addition_plan(-3, 2)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_addition_plan(-2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_addition_plan(0, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_plan(2, -3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_plan(-2, 3)
                .operation_vector()
                .get(OperationKind::OverheadNegative),
            1.0
        );
        assert_eq!(
            signed_subtraction_plan(5, 3)
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
    fn random_difficulty_has_no_easy_or_hard_rank_bias() {
        let mean = |level: u8| {
            (1..=128)
                .map(|seed| {
                    let request = one_digit_worksheet_request(
                        format!("R{}", seed.to_string().replace('0', "A")),
                        Difficulty::try_from(level).unwrap(),
                    );
                    generate_worksheet_request(&request)
                        .unwrap()
                        .problems()
                        .iter()
                        .map(|problem| problem.effort())
                        .sum::<f64>()
                        / ONE_DIGIT_ADDITION_REGISTRATION.layout().problem_count() as f64
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
                            let request = one_digit_worksheet_request(
                                format!("P{salt}{suffix}"),
                                Difficulty::try_from(level).unwrap(),
                            );
                            generate_worksheet_request(&request)
                                .unwrap()
                                .problems()
                                .iter()
                                .map(|problem| problem.effort())
                                .sum::<f64>()
                                / ONE_DIGIT_ADDITION_REGISTRATION.layout().problem_count() as f64
                        })
                        .sum::<f64>()
                        / 64.0
                })
                .collect();
            prop_assert!(means.windows(2).all(|pair| pair[0] < pair[1]), "{means:?}");
        }
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
    fn raw_public_answer_entrypoints_bound_deep_external_trees_before_semantic_recursion() {
        let mut deep = AnswerNode::Integer(1);
        for _ in 0..100_000 {
            deep = AnswerNode::Negative(Box::new(deep));
        }

        assert_eq!(deep.size(), 100_001);
        assert!(!deep.is_within_size_limit());
        assert!(matches!(
            normalize_answer(&deep),
            AnswerNode::NanError(ref code) if code == "answer_ast_size_limit"
        ));
        assert!(!grade_answer(&AnswerNode::Integer(1), &deep).is_correct());
        assert_eq!(
            crate::grade::grade_answer_with_schema(&AnswerNode::Integer(1), &deep, None),
            Err(GradeError::AnswerAstSizeLimit)
        );
        let input_interface = AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Negative],
        };
        assert_eq!(
            input_interface.validate_answer(&deep),
            Err(EditorError::AnswerSizeLimit {
                max_size: MAX_ANSWER_AST_SIZE,
            })
        );

        let cloned = deep.clone();
        assert_eq!(deep, cloned);
        assert_eq!(deep.cmp(&cloned), std::cmp::Ordering::Equal);
        assert_eq!(
            format!("{deep:?}"),
            "AnswerNode(<structural-limit-exceeded>)"
        );
        assert!(serde_json::to_string(&deep).is_err());

        let nested_json = format!(
            "{}{{\"type\":\"integer\",\"value\":\"1\"}}{}",
            "{\"type\":\"negative\",\"value\":".repeat(MAX_ANSWER_AST_SIZE + 1),
            "}".repeat(MAX_ANSWER_AST_SIZE + 1),
        );
        assert!(serde_json::from_str::<AnswerNode>(&nested_json).is_err());

        // Both adversarial trees are dropped normally here. Drop itself is part
        // of the public safe-Rust contract and must remain constant-stack.
        drop(cloned);
        drop(deep);
    }

    #[test]
    fn size_validation_and_extreme_decimal_normalization_are_bounded() {
        let extreme = AnswerNode::exact_decimal(0, u32::MAX);
        assert!(!extreme.is_within_size_limit());
        assert_eq!(normalize_answer(&extreme), AnswerNode::Integer(0));
    }
}
