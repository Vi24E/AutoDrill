use crate::effort::{arithmetic_expression_plan, EffortModel, OperationWeights};
use crate::error::GenerationError;
use crate::generator::{
    BootstrapDedup, GeneratorEntry, LayeredCandidateSource, ProblemGenerator,
    RandomCandidateSource, SamplingStrategy,
};
use crate::generator_support::{
    arithmetic_leaf_significant_digits, binary_expression, draw_decimal_operand,
    exact_decimal_expression, exact_decimal_rational, integer_expression, rational_less_than,
    rational_to_arithmetic_expression, rational_to_exact_decimal_answer,
};
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, Problem, ProblemPrompt,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SamplingLayerSpec, SchoolGrade,
    ThemeAnswerContract as AnswerContract, ThemePresentationPolicy as Presentation,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, STANDARD_20_LAYOUT,
};

pub const THEME_ID_DECIMAL_ADD_SUBTRACT: u32 = 17;
pub const THEME_ID_DECIMAL_MULTIPLY_DIVIDE: u32 = 18;
pub const THEME_ID_DECIMAL_DIVISION: u32 = 24;
pub const GENERATOR_REVISION_DECIMAL_ADD_SUBTRACT: u32 = 5;
pub const GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE: u32 = 6;
pub const GENERATOR_REVISION_DECIMAL_DIVISION: u32 = 1;
pub const SKILL_ID_DECIMAL_ADD_SUBTRACT: &str = "jp.grade4.decimal.add_subtract";
pub const SKILL_ID_DECIMAL_MULTIPLY_DIVIDE: &str = "jp.grade5.decimal.multiplication";
pub const SKILL_ID_DECIMAL_DIVISION: &str = "jp.grade5.decimal.division";
pub const CURRICULUM_PATH_DECIMAL_ADD_SUBTRACT: [&str; 3] =
    ["root", "小学4年生", "小数の足し算と引き算"];
pub const CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE: [&str; 3] =
    ["root", "小学5年生", "小数の掛け算"];
pub const CURRICULUM_PATH_DECIMAL_DIVISION: [&str; 3] = ["root", "小学5年生", "小数の割り算"];

const ADD_SUBTRACT: &[ThemeTag] = &[
    ThemeTag::Decimals,
    ThemeTag::Addition,
    ThemeTag::Subtraction,
];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Division];

pub const ADD_SUBTRACT_LAYERS: [SamplingLayerSpec; 2] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 0,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 0,
    },
];

pub const DECIMAL_ADD_SUBTRACT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DECIMAL_ADD_SUBTRACT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_DECIMAL_ADD_SUBTRACT,
        ),
        skill_id: SKILL_ID_DECIMAL_ADD_SUBTRACT,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_ADD_SUBTRACT,
        grade: Some(SchoolGrade::Elementary4),
        tags: ADD_SUBTRACT,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticDecimal,
        layout: STANDARD_20_LAYOUT,
    });
pub const DECIMAL_MULTIPLY_DIVIDE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DECIMAL_MULTIPLY_DIVIDE),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE,
        ),
        skill_id: SKILL_ID_DECIMAL_MULTIPLY_DIVIDE,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE,
        grade: Some(SchoolGrade::Elementary5),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticDecimal,
        layout: STANDARD_20_LAYOUT,
    });
pub const DECIMAL_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DECIMAL_DIVISION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_DECIMAL_DIVISION,
        ),
        skill_id: SKILL_ID_DECIMAL_DIVISION,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_DIVISION,
        grade: Some(SchoolGrade::Elementary5),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticDecimal,
        layout: STANDARD_20_LAYOUT,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    AddSubtract,
    Multiplication,
    Division,
}

#[derive(Debug)]
pub(crate) struct Generator {
    registration: &'static ThemeRegistration,
    mode: Mode,
}

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        if self.mode == Mode::AddSubtract {
            SamplingStrategy::layered(
                self,
                BootstrapDedup::AllowDuplicates,
                self.registration.layout().problem_count(),
            )
        } else {
            Ok(SamplingStrategy::random(
                self,
                BootstrapDedup::AllowDuplicates,
            ))
        }
    }
}

impl RandomCandidateSource for Generator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        draw_problem_v1(self.registration, self.mode, rng, ordinal, weights).transpose()
    }
}

impl LayeredCandidateSource for Generator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        &ADD_SUBTRACT_LAYERS
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        draw_problem_v1(self.registration, self.mode, rng, ordinal, weights).transpose()
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::Arithmetic {
            expression: ArithmeticExpression::Binary { operator, .. },
        } = problem.prompt()
        else {
            unreachable!("decimal add/sub generator always emits arithmetic binary prompts");
        };
        match operator {
            ArithmeticOperator::Add => 0,
            ArithmeticOperator::Subtract => 1,
            ArithmeticOperator::Multiply | ArithmeticOperator::Divide => {
                unreachable!("decimal add/sub generator emitted the wrong operator")
            }
        }
    }
}

pub(crate) static ADD_SUBTRACT_GENERATOR: Generator = Generator {
    registration: &DECIMAL_ADD_SUBTRACT_REGISTRATION,
    mode: Mode::AddSubtract,
};
pub(crate) static MULTIPLICATION_GENERATOR: Generator = Generator {
    registration: &DECIMAL_MULTIPLY_DIVIDE_REGISTRATION,
    mode: Mode::Multiplication,
};
pub(crate) static DIVISION_GENERATOR: Generator = Generator {
    registration: &DECIMAL_DIVISION_REGISTRATION,
    mode: Mode::Division,
};

// Current decimal generation rules.
fn draw_problem_v1(
    registration: &ThemeRegistration,
    mode: Mode,
    rng: &mut DeterministicRng,
    id: u32,
    _weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    let (expression, answer, operation_plan, max_scale) = match mode {
        Mode::AddSubtract => {
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3)?;
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3)?;
            let mut left_value = exact_decimal_rational(left_coefficient, left_scale)?;
            let mut right_value = exact_decimal_rational(right_coefficient, right_scale)?;
            let operator = if rng.next_bounded(2) == 0 {
                ArithmeticOperator::Add
            } else {
                ArithmeticOperator::Subtract
            };
            if operator == ArithmeticOperator::Subtract
                && rational_less_than(left_value, right_value)
            {
                std::mem::swap(&mut left_coefficient, &mut right_coefficient);
                std::mem::swap(&mut left_scale, &mut right_scale);
                std::mem::swap(&mut left_value, &mut right_value);
            }
            let result_value = match operator {
                ArithmeticOperator::Add => left_value.checked_add(right_value)?,
                ArithmeticOperator::Subtract => left_value.subtract(right_value)?,
                _ => unreachable!(),
            };
            let answer = rational_to_exact_decimal_answer(result_value, 3)?;
            let expression = binary_expression(
                operator,
                exact_decimal_expression(left_coefficient, left_scale),
                exact_decimal_expression(right_coefficient, right_scale),
            );
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (expression, answer, plan, 3)
        }
        Mode::Multiplication | Mode::Division => {
            let operator = match mode {
                Mode::Multiplication => ArithmeticOperator::Multiply,
                Mode::Division => ArithmeticOperator::Divide,
                Mode::AddSubtract => unreachable!(),
            };
            let use_integer_second_operand = rng.next_bounded(2) == 0;
            let (expression, answer) = match operator {
                ArithmeticOperator::Multiply => {
                    let (left_coefficient, left_scale) = draw_decimal_operand(rng, 2, 2)?;
                    let left_value = exact_decimal_rational(left_coefficient, left_scale)?;
                    let (right_expression, right_value) = if use_integer_second_operand {
                        let value = 1_i64 + rng.next_bounded(9) as i64;
                        (
                            integer_expression(value),
                            RationalCoefficient::new(value, 1)?,
                        )
                    } else {
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2)?;
                        (
                            exact_decimal_expression(coefficient, scale),
                            exact_decimal_rational(coefficient, scale)?,
                        )
                    };
                    let result_value = left_value.multiply(right_value)?;
                    let answer = rational_to_exact_decimal_answer(result_value, 4)?;
                    (
                        binary_expression(
                            ArithmeticOperator::Multiply,
                            exact_decimal_expression(left_coefficient, left_scale),
                            right_expression,
                        ),
                        answer,
                    )
                }
                ArithmeticOperator::Divide => {
                    let (quotient_coefficient, quotient_scale) = draw_decimal_operand(rng, 2, 2)?;
                    let quotient_value =
                        exact_decimal_rational(quotient_coefficient, quotient_scale)?;
                    let answer = rational_to_exact_decimal_answer(quotient_value, 2)?;
                    let (divisor_expression, divisor_value) = if use_integer_second_operand {
                        let value = 1_i64 + rng.next_bounded(9) as i64;
                        (
                            integer_expression(value),
                            RationalCoefficient::new(value, 1)?,
                        )
                    } else {
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2)?;
                        (
                            exact_decimal_expression(coefficient, scale),
                            exact_decimal_rational(coefficient, scale)?,
                        )
                    };
                    let dividend_value = quotient_value.multiply(divisor_value)?;
                    let dividend_expression = rational_to_arithmetic_expression(dividend_value, 4)?;
                    if arithmetic_leaf_significant_digits(&dividend_expression)? > 3 {
                        return None;
                    }
                    (
                        binary_expression(
                            ArithmeticOperator::Divide,
                            dividend_expression,
                            divisor_expression,
                        ),
                        answer,
                    )
                }
                _ => unreachable!(),
            };
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (expression, answer, plan, 4)
        }
    };
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::Arithmetic { expression },
            AnswerSchema::Decimal { max_scale },
            answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 3] = [
    GeneratorEntry::current(&ADD_SUBTRACT_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DIVISION_GENERATOR),
];

#[cfg(test)]
mod curriculum_tests {
    use super::*;
    use crate::answer::AnswerNode;
    use crate::generator::generate_worksheet_request;
    use crate::identity::Difficulty;
    use crate::model::GenerateWorksheetRequest;
    use crate::schema::SCHEMA_VERSION;
    use std::collections::HashSet;

    fn assert_decimal_operand(
        expression: &ArithmeticExpression,
        max_significant_digits: usize,
        max_scale: u32,
    ) -> (i64, u32) {
        let ArithmeticExpression::ExactDecimal { coefficient, scale } = expression else {
            panic!("expected exact decimal operand");
        };
        assert!(*coefficient > 0);
        assert!((1..=max_scale).contains(scale));
        assert_ne!(coefficient % 10, 0);
        assert!(coefficient.to_string().len() <= max_significant_digits);
        (*coefficient, *scale)
    }

    #[test]
    fn add_subtract_matches_grade_four_digit_and_place_value_bounds() {
        let mut seen_operators = HashSet::new();
        let mut seen_scales = HashSet::new();
        let mut saw_different_places = false;
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_DECIMAL_ADD_SUBTRACT,
                seed: seed.to_owned(),
                difficulty: Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems() {
                let ProblemPrompt::Arithmetic {
                    expression:
                        ArithmeticExpression::Binary {
                            operator,
                            left,
                            right,
                        },
                } = problem.prompt()
                else {
                    panic!("decimal add/subtract must be binary")
                };
                assert!(matches!(
                    operator,
                    ArithmeticOperator::Add | ArithmeticOperator::Subtract
                ));
                seen_operators.insert(*operator);
                let (_, left_scale) = assert_decimal_operand(left, 3, 3);
                let (_, right_scale) = assert_decimal_operand(right, 3, 3);
                seen_scales.insert(left_scale);
                seen_scales.insert(right_scale);
                saw_different_places |= left_scale != right_scale;
                match problem.canonical_answer() {
                    AnswerNode::Integer(value) => assert!(*value >= 0),
                    AnswerNode::ExactDecimal { coefficient, scale } => {
                        assert!(*coefficient >= 0);
                        assert!((1..=3).contains(scale));
                    }
                    _ => panic!("decimal answer must be finite decimal"),
                }
            }
        }
        assert_eq!(
            seen_operators,
            HashSet::from([ArithmeticOperator::Add, ArithmeticOperator::Subtract])
        );
        assert_eq!(seen_scales, HashSet::from([1_u32, 2_u32, 3_u32]));
        assert!(saw_different_places);
    }

    #[test]
    fn multiplication_and_division_are_independent_family_units() {
        for (theme_id, expected_operator) in [
            (
                THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
                ArithmeticOperator::Multiply,
            ),
            (THEME_ID_DECIMAL_DIVISION, ArithmeticOperator::Divide),
        ] {
            let mut saw_integer_second = false;
            let mut saw_decimal_second = false;
            for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3", "D3c5", "N6p8"] {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: theme_id,
                    seed: seed.to_owned(),
                    difficulty: Difficulty::try_from(3).unwrap(),
                    timeout_ms: None,
                    max_attempts: None,
                })
                .unwrap();
                for problem in worksheet.problems() {
                    let ProblemPrompt::Arithmetic {
                        expression:
                            ArithmeticExpression::Binary {
                                operator,
                                left,
                                right,
                            },
                    } = problem.prompt()
                    else {
                        panic!("decimal unit must be binary")
                    };
                    assert_eq!(*operator, expected_operator);
                    match right.as_ref() {
                        ArithmeticExpression::Integer { value } => {
                            saw_integer_second = true;
                            assert!((1..=9).contains(value));
                        }
                        decimal @ ArithmeticExpression::ExactDecimal { .. } => {
                            saw_decimal_second = true;
                            assert_decimal_operand(decimal, 2, 2);
                        }
                        _ => panic!("bounded decimal/integer second operand required"),
                    }
                    if expected_operator == ArithmeticOperator::Multiply {
                        assert_decimal_operand(left, 2, 2);
                    } else {
                        match problem.canonical_answer() {
                            AnswerNode::ExactDecimal { coefficient, scale } => {
                                assert!(*coefficient > 0);
                                assert!((1..=2).contains(scale));
                            }
                            AnswerNode::Integer(value) => assert!(*value > 0),
                            _ => panic!("division quotient must be exact decimal"),
                        }
                    }
                }
            }
            assert!(saw_integer_second && saw_decimal_second);
        }
    }
}
