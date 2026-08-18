use crate::answer::AnswerNode;
use crate::effort::{
    arithmetic_expression_plan, one_digit_addition_plan, one_digit_subtraction_plan,
    two_digit_addition_plan, EffortModel, OperationPlan, OperationWeights,
};
use crate::error::GenerationError;
use crate::generator::{
    BootstrapDedup, GeneratorEntry, ProblemGenerator, RandomCandidateSource, SamplingStrategy,
};
use crate::generator_support::{
    binary_expression, draw_bounded_rational_arithmetic_ast, draw_signed_integer,
    ensure_negative_term, evaluate_expression, integer_expression, rational_answer,
};
use crate::model::{AnswerSchema, ArithmeticOperator, Problem, ProblemPrompt};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SchoolGrade,
    ThemeAnswerContract as AnswerContract, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemeRegistration, ThemeRegistrationSpec, ThemeTag,
    STANDARD_20_LAYOUT,
};
use crate::themes::{division_table, multiplication_table};

pub const DEFAULT_PROBLEM_COUNT: usize = STANDARD_20_LAYOUT.problem_count();
pub const DEFAULT_COLUMNS: usize = STANDARD_20_LAYOUT.columns();
pub const DEFAULT_ROWS: usize = STANDARD_20_LAYOUT.rows();
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
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ONE_DIGIT_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ONE_DIGIT_ADDITION,
        ),
        skill_id: SKILL_ID,
        curriculum_path: &CURRICULUM_PATH,
        grade: Some(SchoolGrade::Elementary1),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::AdditionInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const ONE_DIGIT_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ONE_DIGIT_SUBTRACTION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION,
        ),
        skill_id: SKILL_ID_ONE_DIGIT_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION,
        grade: Some(SchoolGrade::Elementary1),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const TWO_DIGIT_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_TWO_DIGIT_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_TWO_DIGIT_ADDITION,
        ),
        skill_id: SKILL_ID_TWO_DIGIT_ADDITION,
        curriculum_path: &CURRICULUM_PATH_TWO_DIGIT_ADDITION,
        grade: Some(SchoolGrade::Elementary2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const MULTIPLICATION_TABLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_MULTIPLICATION_TABLE),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_MULTIPLICATION_TABLE,
        ),
        skill_id: SKILL_ID_MULTIPLICATION_TABLE,
        curriculum_path: &CURRICULUM_PATH_MULTIPLICATION_TABLE,
        grade: Some(SchoolGrade::Elementary2),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const DIVISION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DIVISION_1),
        generator_revision: crate::theme::GeneratorRevision::new(GENERATOR_REVISION_DIVISION_1),
        skill_id: SKILL_ID_DIVISION_1,
        curriculum_path: &CURRICULUM_PATH_DIVISION_1,
        grade: Some(SchoolGrade::Elementary3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const SIGNED_ARITHMETIC_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_ARITHMETIC_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_ARITHMETIC_1,
        ),
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_1,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_1,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);

pub const SIGNED_ARITHMETIC_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_ARITHMETIC_2),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_ARITHMETIC_2,
        ),
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_2,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_2,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedRational,
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
        draw_problem(self.registration, self.mode, rng, ordinal, weights).transpose()
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
) -> Option<Result<Problem, GenerationError>> {
    if mode_matches_addition(mode) {
        let (left, right) = rng.next_ordered_pair();
        return Some(one_digit_addition_problem(id, left, right, weights));
    }

    let (expression, answer, operation_plan, answer_schema, theme_specific_effort) = match mode {
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
                one_digit_subtraction_plan(a as u8, b as u8)?,
                AnswerSchema::Integer { min: 1, max: 9 },
                None,
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
                two_digit_addition_plan(a as u8, b as u8)?,
                AnswerSchema::Integer { min: 20, max: 198 },
                None,
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
                OperationPlan::default(),
                AnswerSchema::Integer { min: 1, max: 81 },
                Some(multiplication_table::effort(c as u8)),
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
                division_table::operation_plan(dividend as u8),
                AnswerSchema::Integer { min: 1, max: 9 },
                None,
            )
        }
        Mode::SignedArithmetic1 => {
            let term_count = 2 + rng.next_bounded(3) as usize;
            let mut terms = (0..term_count)
                .map(|_| draw_signed_integer(rng, 15))
                .collect::<Option<Vec<_>>>()?;
            ensure_negative_term(rng, &mut terms)?;
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
            let answer = AnswerNode::Integer(value.numerator());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                plan,
                AnswerSchema::Integer { min: -60, max: 60 },
                None,
            )
        }
        Mode::SignedArithmetic2 => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let mut values = (0..leaf_count)
                .map(|_| draw_signed_integer(rng, 9))
                .collect::<Option<Vec<_>>>()?;
            ensure_negative_term(rng, &mut values)?;
            let expression = draw_bounded_rational_arithmetic_ast(rng, &values)?;
            let value = evaluate_expression(&expression)?;
            if value.numerator().unsigned_abs() > 200 || value.denominator() > 36 {
                return None;
            }
            let answer = rational_answer(value);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                plan,
                AnswerSchema::Rational {
                    max_abs_numerator: 200,
                    max_denominator: 36,
                    require_reduced_fraction_form: true,
                },
                None,
            )
        }
    };
    let effort_model = match theme_specific_effort {
        Some(value) => EffortModel::theme_specific(value)?,
        None => EffortModel::operations(operation_plan),
    };
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::Arithmetic { expression },
            answer_schema,
            answer,
            effort_model,
        )
        .map_err(GenerationError::from),
    )
}

pub(crate) fn one_digit_addition_problem(
    id: u32,
    left: u8,
    right: u8,
    _weights: &OperationWeights,
) -> Result<Problem, GenerationError> {
    if !(MIN_OPERAND..=MAX_OPERAND).contains(&left) || !(MIN_OPERAND..=MAX_OPERAND).contains(&right)
    {
        return Err(GenerationError::InvalidGeneratedProblem {
            reason:
                "one-digit addition generator received an operand outside its registered domain",
        });
    }
    let answer = left + right;
    let effort_model = EffortModel::operations(one_digit_addition_plan(left, right).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "one-digit addition effort operands are outside the modeled domain",
        },
    )?);
    Problem::generated(
        &ONE_DIGIT_ADDITION_REGISTRATION,
        id,
        ProblemPrompt::Addition { left, right },
        AnswerSchema::Integer {
            min: i64::from(MIN_ANSWER),
            max: i64::from(MAX_ANSWER),
        },
        AnswerNode::Integer(i64::from(answer)),
        effort_model,
    )
    .map_err(GenerationError::from)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf_count(expression: &crate::model::ArithmeticExpression) -> usize {
        match expression {
            crate::model::ArithmeticExpression::Binary { left, right, .. } => {
                leaf_count(left) + leaf_count(right)
            }
            _ => 1,
        }
    }

    fn operators(
        expression: &crate::model::ArithmeticExpression,
        output: &mut Vec<ArithmeticOperator>,
    ) {
        if let crate::model::ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } = expression
        {
            output.push(*operator);
            operators(left, output);
            operators(right, output);
        }
    }

    fn draw_accepted(
        registration: &'static ThemeRegistration,
        mode: Mode,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Problem {
        for _ in 0..10_000 {
            if let Some(problem) = draw_problem(registration, mode, rng, ordinal, weights) {
                return problem.expect("generated candidate must satisfy its problem contract");
            }
        }
        panic!("local basic-arithmetic generator failed to produce a candidate");
    }

    #[test]
    fn basic_arithmetic_modes_follow_their_local_domains() {
        let weights = OperationWeights::default();
        let mut rng = DeterministicRng::from_seed("BasicDomainA1");
        let cases = [
            (
                &ONE_DIGIT_SUBTRACTION_REGISTRATION,
                Mode::OneDigitSubtraction,
            ),
            (&TWO_DIGIT_ADDITION_REGISTRATION, Mode::TwoDigitAddition),
            (
                &MULTIPLICATION_TABLE_REGISTRATION,
                Mode::MultiplicationTable,
            ),
            (&SIGNED_ARITHMETIC_1_REGISTRATION, Mode::SignedArithmetic1),
            (&SIGNED_ARITHMETIC_2_REGISTRATION, Mode::SignedArithmetic2),
            (&DIVISION_1_REGISTRATION, Mode::DivisionTable),
        ];

        for (registration, mode) in cases {
            for ordinal in 1..=128 {
                let problem = draw_accepted(registration, mode, &mut rng, ordinal, &weights);
                let ProblemPrompt::Arithmetic { expression } = problem.prompt() else {
                    panic!("basic arithmetic mode returned non-arithmetic prompt");
                };
                let value =
                    evaluate_expression(expression).expect("generated expression evaluates");
                assert_eq!(
                    crate::normalize::normalize_answer(&rational_answer(value)),
                    crate::normalize::normalize_answer(problem.canonical_answer())
                );
                match mode {
                    Mode::OneDigitSubtraction => {
                        let crate::model::ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Subtract,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("subtraction shape")
                        };
                        let (
                            crate::model::ArithmeticExpression::Integer { value: left },
                            crate::model::ArithmeticExpression::Integer { value: right },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer operands")
                        };
                        assert!((1..=18).contains(left));
                        assert!((1..=9).contains(right));
                        assert!((1..=9).contains(&value.numerator()));
                    }
                    Mode::TwoDigitAddition => {
                        let crate::model::ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Add,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("addition shape")
                        };
                        let (
                            crate::model::ArithmeticExpression::Integer { value: left },
                            crate::model::ArithmeticExpression::Integer { value: right },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer operands")
                        };
                        assert!((10..=99).contains(left));
                        assert!((10..=99).contains(right));
                    }
                    Mode::MultiplicationTable => {
                        let crate::model::ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Multiply,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("multiplication shape")
                        };
                        let (
                            crate::model::ArithmeticExpression::Integer { value: left },
                            crate::model::ArithmeticExpression::Integer { value: right },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer operands")
                        };
                        assert!((1..=9).contains(left) && (1..=9).contains(right));
                        let expected = (*left * *right) as f64;
                        assert_eq!(problem.theme_specific_effort(), Some(expected.log10()));
                        assert!(problem.operation_plan().is_none());
                    }
                    Mode::SignedArithmetic1 => {
                        assert!((2..=4).contains(&leaf_count(expression)));
                        let mut found = Vec::new();
                        operators(expression, &mut found);
                        assert!(found.iter().all(|operator| matches!(
                            operator,
                            ArithmeticOperator::Add | ArithmeticOperator::Subtract
                        )));
                    }
                    Mode::SignedArithmetic2 => {
                        assert!((2..=4).contains(&leaf_count(expression)));
                        assert!(value.numerator().unsigned_abs() <= 200);
                        assert!(value.denominator() <= 36);
                    }
                    Mode::DivisionTable => {
                        let crate::model::ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Divide,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("division shape")
                        };
                        let (
                            crate::model::ArithmeticExpression::Integer { value: dividend },
                            crate::model::ArithmeticExpression::Integer { value: divisor },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer operands")
                        };
                        let quotient = problem.canonical_answer().as_integer().unwrap();
                        assert!((1..=81).contains(dividend));
                        assert!((1..=9).contains(divisor));
                        assert!((1..=9).contains(&quotient));
                        assert_eq!(*dividend, *divisor * quotient);
                    }
                    Mode::OneDigitAddition => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn signed_arithmetic_two_samples_all_four_operators() {
        let weights = OperationWeights::default();
        let mut rng = DeterministicRng::from_seed("SignedOpsB2");
        let mut seen = [false; 4];
        for ordinal in 1..=512 {
            let problem = draw_accepted(
                &SIGNED_ARITHMETIC_2_REGISTRATION,
                Mode::SignedArithmetic2,
                &mut rng,
                ordinal,
                &weights,
            );
            let ProblemPrompt::Arithmetic { expression } = problem.prompt() else {
                unreachable!()
            };
            let mut found = Vec::new();
            operators(expression, &mut found);
            for operator in found {
                seen[match operator {
                    ArithmeticOperator::Add => 0,
                    ArithmeticOperator::Subtract => 1,
                    ArithmeticOperator::Multiply => 2,
                    ArithmeticOperator::Divide => 3,
                }] = true;
            }
        }
        assert!(
            seen.into_iter().all(|value| value),
            "operator coverage collapsed: {seen:?}"
        );
    }
}
