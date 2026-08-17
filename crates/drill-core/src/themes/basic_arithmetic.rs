use crate::answer::AnswerNode;
use crate::effort::{
    arithmetic_expression_graph, calculate_graph_effort, one_digit_addition_graph,
    one_digit_subtraction_graph, two_digit_addition_graph, OperationWeights,
};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::{
    binary_expression, draw_bounded_rational_arithmetic_ast, draw_signed_integer,
    ensure_negative_term, evaluate_expression, input_interface, integer_expression,
    rational_answer,
};
use crate::model::{AnswerSchema, ArithmeticOperator, Problem, ProblemPrompt};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, ThemeAnswerContract as AnswerContract,
    ThemeAnswerSchemaKind as Schema, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt, ThemeRegistration,
    ThemeRegistrationSpec, ThemeTag, STANDARD_20_LAYOUT,
};
use crate::themes::{division_table, multiplication_table};

pub const DEFAULT_PROBLEM_COUNT: usize = STANDARD_20_LAYOUT.problem_count;
pub const DEFAULT_COLUMNS: usize = STANDARD_20_LAYOUT.columns;
pub const DEFAULT_ROWS: usize = STANDARD_20_LAYOUT.rows;
pub const MIN_OPERAND: u8 = 1;
pub const MAX_OPERAND: u8 = 9;
pub const MIN_ANSWER: u8 = 1;
pub const MAX_ANSWER: u8 = 18;
pub const THEME_ID_ONE_DIGIT_ADDITION: u32 = 1;
pub const THEME_ID_ONE_DIGIT_SUBTRACTION: u32 = 4;
pub const THEME_ID_TWO_DIGIT_ADDITION: u32 = 5;
pub const THEME_ID_MULTIPLICATION_TABLE: u32 = 6;
pub const THEME_ID_SIGNED_ARITHMETIC_1: u32 = 7;
pub const THEME_ID_SIGNED_ARITHMETIC_2: u32 = 8;
pub const THEME_ID_DIVISION_1: u32 = 13;
pub const GENERATOR_REVISION_ONE_DIGIT_ADDITION: u32 = 5;
pub const GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION: u32 = 3;
pub const GENERATOR_REVISION_TWO_DIGIT_ADDITION: u32 = 3;
pub const GENERATOR_REVISION_MULTIPLICATION_TABLE: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_1: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_2: u32 = 3;
pub const GENERATOR_REVISION_DIVISION_1: u32 = 3;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const SKILL_ID_ONE_DIGIT_SUBTRACTION: &str = "jp.grade1.subtraction.one_digit";
pub const SKILL_ID_TWO_DIGIT_ADDITION: &str = "jp.grade2.addition.two_digit";
pub const SKILL_ID_MULTIPLICATION_TABLE: &str = "jp.grade2.multiplication.table";
pub const SKILL_ID_SIGNED_ARITHMETIC_1: &str = "jp.grade7.signed.arithmetic.1";
pub const SKILL_ID_SIGNED_ARITHMETIC_2: &str = "jp.grade7.signed.arithmetic.2";
pub const SKILL_ID_DIVISION_1: &str = "jp.grade3.division.table.1";
pub const CURRICULUM_PATH: [&str; 3] = ["root", "小学1年生", "一桁の足し算"];
pub const CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION: [&str; 3] = ["root", "小学1年生", "一桁の引き算"];
pub const CURRICULUM_PATH_TWO_DIGIT_ADDITION: [&str; 3] = ["root", "小学2年生", "二桁の足し算"];
pub const CURRICULUM_PATH_MULTIPLICATION_TABLE: [&str; 3] = ["root", "小学2年生", "九九"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_1: [&str; 3] = ["root", "中学1年生", "負の数の計算(1)"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_2: [&str; 3] = ["root", "中学1年生", "負の数の計算(2)"];
pub const CURRICULUM_PATH_DIVISION_1: [&str; 3] = ["root", "小学3年生", "割り算(1)"];

const ADDITION: &[ThemeTag] = &[ThemeTag::Addition];
const SUBTRACTION: &[ThemeTag] = &[ThemeTag::Subtraction];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Division];
const NEGATIVE_NUMBERS: &[ThemeTag] = &[ThemeTag::NegativeNumbers];

pub const ONE_DIGIT_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
        generator_revision: GENERATOR_REVISION_ONE_DIGIT_ADDITION,
        skill_id: SKILL_ID,
        curriculum_path: &CURRICULUM_PATH,
        grade: Some(1),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Addition,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimplePositive,
        },
        layout: STANDARD_20_LAYOUT,
    });

pub const ONE_DIGIT_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_ONE_DIGIT_SUBTRACTION,
        generator_revision: GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION,
        skill_id: SKILL_ID_ONE_DIGIT_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION,
        grade: Some(1),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimplePositive,
        },
        layout: STANDARD_20_LAYOUT,
    });

pub const TWO_DIGIT_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_TWO_DIGIT_ADDITION,
        generator_revision: GENERATOR_REVISION_TWO_DIGIT_ADDITION,
        skill_id: SKILL_ID_TWO_DIGIT_ADDITION,
        curriculum_path: &CURRICULUM_PATH_TWO_DIGIT_ADDITION,
        grade: Some(2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimplePositive,
        },
        layout: STANDARD_20_LAYOUT,
    });

pub const MULTIPLICATION_TABLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_MULTIPLICATION_TABLE,
        generator_revision: GENERATOR_REVISION_MULTIPLICATION_TABLE,
        skill_id: SKILL_ID_MULTIPLICATION_TABLE,
        curriculum_path: &CURRICULUM_PATH_MULTIPLICATION_TABLE,
        grade: Some(2),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimplePositive,
        },
        layout: STANDARD_20_LAYOUT,
    });

pub const DIVISION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_DIVISION_1,
        generator_revision: GENERATOR_REVISION_DIVISION_1,
        skill_id: SKILL_ID_DIVISION_1,
        curriculum_path: &CURRICULUM_PATH_DIVISION_1,
        grade: Some(3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimplePositive,
        },
        layout: STANDARD_20_LAYOUT,
    });

pub const SIGNED_ARITHMETIC_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_SIGNED_ARITHMETIC_1,
        generator_revision: GENERATOR_REVISION_SIGNED_ARITHMETIC_1,
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_1,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_1,
        grade: Some(7),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::SimpleSigned,
        },
        layout: STANDARD_20_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);

pub const SIGNED_ARITHMETIC_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_SIGNED_ARITHMETIC_2,
        generator_revision: GENERATOR_REVISION_SIGNED_ARITHMETIC_2,
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_2,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_2,
        grade: Some(7),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::Arithmetic,
            answer_schema_kind: Schema::Rational,
            input_profile: Input::SignedRational,
        },
        layout: STANDARD_20_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);

#[derive(Clone, Copy, Debug)]
enum Mode {
    OneDigitAddition,
    OneDigitSubtraction,
    TwoDigitAddition,
    MultiplicationTable,
    DivisionTable,
    SignedArithmetic1,
    SignedArithmetic2,
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

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        draw_problem(self.registration, self.mode, rng, ordinal, weights)
    }
}

macro_rules! generator {
    ($name:ident, $registration:ident, $mode:ident) => {
        pub(crate) static $name: Generator = Generator {
            registration: &$registration,
            mode: Mode::$mode,
        };
    };
}

generator!(
    ONE_DIGIT_ADDITION_GENERATOR,
    ONE_DIGIT_ADDITION_REGISTRATION,
    OneDigitAddition
);
generator!(
    ONE_DIGIT_SUBTRACTION_GENERATOR,
    ONE_DIGIT_SUBTRACTION_REGISTRATION,
    OneDigitSubtraction
);
generator!(
    TWO_DIGIT_ADDITION_GENERATOR,
    TWO_DIGIT_ADDITION_REGISTRATION,
    TwoDigitAddition
);
generator!(
    MULTIPLICATION_TABLE_GENERATOR,
    MULTIPLICATION_TABLE_REGISTRATION,
    MultiplicationTable
);
generator!(DIVISION_1_GENERATOR, DIVISION_1_REGISTRATION, DivisionTable);
generator!(
    SIGNED_ARITHMETIC_1_GENERATOR,
    SIGNED_ARITHMETIC_1_REGISTRATION,
    SignedArithmetic1
);
generator!(
    SIGNED_ARITHMETIC_2_GENERATOR,
    SIGNED_ARITHMETIC_2_REGISTRATION,
    SignedArithmetic2
);

fn draw_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    if mode_matches_addition(mode) {
        let (left, right) = rng.next_ordered_pair();
        return Some(one_digit_addition_problem(id, left, right, weights));
    }

    let (expression, answer, solution_graph, answer_schema, input_profile) = match mode {
        Mode::OneDigitAddition => unreachable!(),
        Mode::OneDigitSubtraction => {
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = 1_i64 + rng.next_bounded(9) as i64;
            let a = b + c;
            let expression = binary_expression(
                ArithmeticOperator::Subtract,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                one_digit_subtraction_graph(a as u8, b as u8),
                AnswerSchema::Integer { min: 1, max: 9 },
                Input::SimplePositive,
            )
        }
        Mode::TwoDigitAddition => {
            let a = 10_i64 + rng.next_bounded(90) as i64;
            let b = 10_i64 + rng.next_bounded(90) as i64;
            let c = a + b;
            let expression = binary_expression(
                ArithmeticOperator::Add,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                two_digit_addition_graph(a as u8, b as u8),
                AnswerSchema::Integer { min: 20, max: 198 },
                Input::SimplePositive,
            )
        }
        Mode::MultiplicationTable => {
            let a = 1_i64 + rng.next_bounded(9) as i64;
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = a * b;
            let expression = binary_expression(
                ArithmeticOperator::Multiply,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                multiplication_table::solution_graph(c as u8),
                AnswerSchema::Integer { min: 1, max: 81 },
                Input::SimplePositive,
            )
        }
        Mode::DivisionTable => {
            let divisor = 1_i64 + rng.next_bounded(9) as i64;
            let quotient = 1_i64 + rng.next_bounded(9) as i64;
            let dividend = divisor * quotient;
            let expression = binary_expression(
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
            );
            (
                expression,
                AnswerNode::Integer(quotient),
                division_table::solution_graph(dividend as u8),
                AnswerSchema::Integer { min: 1, max: 9 },
                Input::SimplePositive,
            )
        }
        Mode::SignedArithmetic1 => {
            let term_count = 2 + rng.next_bounded(3) as usize;
            let mut terms = (0..term_count)
                .map(|_| draw_signed_integer(rng, 15))
                .collect::<Vec<_>>();
            ensure_negative_term(rng, &mut terms);
            let mut expression = integer_expression(terms[0]);
            for term in terms.iter().skip(1) {
                let operator = if rng.next_bounded(2) == 0 {
                    ArithmeticOperator::Add
                } else {
                    ArithmeticOperator::Subtract
                };
                expression = binary_expression(operator, expression, integer_expression(*term));
            }
            let value = evaluate_expression(&expression)?;
            if !value.is_integer() {
                return None;
            }
            let answer = AnswerNode::Integer(value.numerator);
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                AnswerSchema::Integer { min: -60, max: 60 },
                Input::SimpleSigned,
            )
        }
        Mode::SignedArithmetic2 => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let mut values = (0..leaf_count)
                .map(|_| draw_signed_integer(rng, 9))
                .collect::<Vec<_>>();
            ensure_negative_term(rng, &mut values);
            let expression = draw_bounded_rational_arithmetic_ast(rng, &values)?;
            let value = evaluate_expression(&expression)?;
            if value.numerator.unsigned_abs() > 200 || value.denominator > 36 {
                return None;
            }
            let answer = rational_answer(value);
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                AnswerSchema::Rational {
                    max_abs_numerator: 200,
                    max_denominator: 36,
                    require_reduced_fraction_form: true,
                },
                Input::SignedRational,
            )
        }
    };
    let effort = calculate_graph_effort(&solution_graph, weights);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: registration.numeric_theme_id,
        prompt: ProblemPrompt::Arithmetic { expression },
        input_interface: input_interface(input_profile),
        answer_schema,
        canonical_answer: answer,
        worked_solution: None,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

pub(crate) fn one_digit_addition_problem(
    id: u32,
    left: u8,
    right: u8,
    weights: &OperationWeights,
) -> Problem {
    debug_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&left));
    debug_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&right));
    let answer = left + right;
    let solution_graph = one_digit_addition_graph(left, right);
    let effort = calculate_graph_effort(&solution_graph, weights);
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
        prompt: ProblemPrompt::Addition { left, right },
        input_interface: input_interface(Input::SimplePositive),
        answer_schema: AnswerSchema::Integer {
            min: i64::from(MIN_ANSWER),
            max: i64::from(MAX_ANSWER),
        },
        canonical_answer: AnswerNode::Integer(i64::from(answer)),
        worked_solution: None,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    }
}

const fn mode_matches_addition(mode: Mode) -> bool {
    matches!(mode, Mode::OneDigitAddition)
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 7] = [
    GeneratorEntry::current(&ONE_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&ONE_DIGIT_SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&TWO_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_1_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_2_GENERATOR),
    GeneratorEntry::current(&DIVISION_1_GENERATOR),
];
