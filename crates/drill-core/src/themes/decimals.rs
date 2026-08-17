use crate::effort::{arithmetic_expression_graph, calculate_graph_effort, OperationWeights};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::{
    arithmetic_leaf_significant_digits, binary_expression, draw_decimal_operand,
    exact_decimal_expression, exact_decimal_rational, input_interface, integer_expression,
    rational_less_than, rational_to_arithmetic_expression, rational_to_exact_decimal_answer,
};
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, Problem, ProblemPrompt,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SamplingLayerSpec,
    ThemeAnswerContract as AnswerContract, ThemeAnswerSchemaKind as Schema,
    ThemeInputProfile as Input, ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt,
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
        key: "addition",
        weight: 1,
        minimum: 0,
    },
    SamplingLayerSpec {
        key: "subtraction",
        weight: 1,
        minimum: 0,
    },
];

pub const DECIMAL_ADD_SUBTRACT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_DECIMAL_ADD_SUBTRACT,
        generator_revision: GENERATOR_REVISION_DECIMAL_ADD_SUBTRACT,
        skill_id: SKILL_ID_DECIMAL_ADD_SUBTRACT,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_ADD_SUBTRACT,
        grade: Some(4),
        tags: ADD_SUBTRACT,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Decimal,
            input_profile: Input::SimpleDecimal,
        },
        layout: STANDARD_20_LAYOUT,
    });
pub const DECIMAL_MULTIPLY_DIVIDE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
        generator_revision: GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE,
        skill_id: SKILL_ID_DECIMAL_MULTIPLY_DIVIDE,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE,
        grade: Some(5),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Decimal,
            input_profile: Input::SimpleDecimal,
        },
        layout: STANDARD_20_LAYOUT,
    });
pub const DECIMAL_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_DECIMAL_DIVISION,
        generator_revision: GENERATOR_REVISION_DECIMAL_DIVISION,
        skill_id: SKILL_ID_DECIMAL_DIVISION,
        curriculum_path: &CURRICULUM_PATH_DECIMAL_DIVISION,
        grade: Some(5),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Decimal,
            input_profile: Input::SimpleDecimal,
        },
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
    fn sampling_layers(&self) -> Option<&'static [SamplingLayerSpec]> {
        (self.mode == Mode::AddSubtract).then_some(&ADD_SUBTRACT_LAYERS)
    }
    fn sampling_layer(&self, problem: &Problem) -> Option<usize> {
        if self.mode != Mode::AddSubtract {
            return None;
        }
        let ProblemPrompt::Arithmetic {
            expression: ArithmeticExpression::Binary { operator, .. },
        } = &problem.prompt
        else {
            return None;
        };
        match operator {
            ArithmeticOperator::Add => Some(0),
            ArithmeticOperator::Subtract => Some(1),
            _ => None,
        }
    }
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        draw_problem_v1(self.registration, self.mode, rng, ordinal, weights)
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
    weights: &OperationWeights,
) -> Option<Problem> {
    let (expression, answer, solution_graph, max_scale) = match mode {
        Mode::AddSubtract => {
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3);
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (expression, answer, graph, 3)
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
                    let (left_coefficient, left_scale) = draw_decimal_operand(rng, 2, 2);
                    let left_value = exact_decimal_rational(left_coefficient, left_scale)?;
                    let (right_expression, right_value) = if use_integer_second_operand {
                        let value = 1_i64 + rng.next_bounded(9) as i64;
                        (
                            integer_expression(value),
                            RationalCoefficient::new(value, 1)?,
                        )
                    } else {
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2);
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
                    let (quotient_coefficient, quotient_scale) = draw_decimal_operand(rng, 2, 2);
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
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (expression, answer, graph, 4)
        }
    };
    let effort = calculate_graph_effort(&solution_graph, weights);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: registration.numeric_theme_id,
        prompt: ProblemPrompt::Arithmetic { expression },
        input_interface: input_interface(Input::SimpleDecimal),
        answer_schema: AnswerSchema::Decimal { max_scale },
        canonical_answer: answer,
        worked_solution: None,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 3] = [
    GeneratorEntry::current(&ADD_SUBTRACT_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DIVISION_GENERATOR),
];
