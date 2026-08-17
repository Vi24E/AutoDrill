use std::sync::OnceLock;

use crate::effort::{
    arithmetic_expression_graph, calculate_graph_effort, Operation, OperationWeights,
    SolutionGraph, SolutionStep,
};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::{
    binary_expression, input_interface, mixed_number_answer, rational_answer, rational_expression,
};
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, Problem, ProblemPrompt,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup,
    FractionPresentationPolicy as FractionPresentation, SamplingLayerSpec,
    ThemeAnswerContract as AnswerContract, ThemeAnswerSchemaKind as Schema,
    ThemeInputProfile as Input, ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COMPACT_16_LAYOUT,
};

pub const THEME_ID_FRACTION_ADDITION: u32 = 9;
pub const THEME_ID_FRACTION_MULTIPLICATION: u32 = 10;
pub const THEME_ID_FRACTION_SUBTRACTION: u32 = 11;
pub const THEME_ID_FRACTION_DIVISION: u32 = 12;
pub const THEME_ID_FRACTION_INTEGER_MULTIPLICATION: u32 = 21;
pub const THEME_ID_FRACTION_INTEGER_DIVISION: u32 = 22;
pub const THEME_ID_FRACTION_SUMMARY_IMPROPER: u32 = 23;
pub const GENERATOR_REVISION_FRACTION_ADDITION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_MULTIPLICATION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_DIVISION: u32 = 6;
pub const GENERATOR_REVISION_FRACTION_INTEGER_MULTIPLICATION: u32 = 2;
pub const GENERATOR_REVISION_FRACTION_INTEGER_DIVISION: u32 = 2;
pub const GENERATOR_REVISION_FRACTION_SUMMARY_IMPROPER: u32 = 3;
pub const SKILL_ID_FRACTION_ADDITION: &str = "jp.grade5.fraction.addition";
pub const SKILL_ID_FRACTION_MULTIPLICATION: &str = "jp.grade6.fraction.multiplication";
pub const SKILL_ID_FRACTION_SUBTRACTION: &str = "jp.grade5.fraction.subtraction";
pub const SKILL_ID_FRACTION_DIVISION: &str = "jp.grade6.fraction.division";
pub const SKILL_ID_FRACTION_INTEGER_MULTIPLICATION: &str =
    "jp.grade6.fraction.integer_multiplication";
pub const SKILL_ID_FRACTION_INTEGER_DIVISION: &str = "jp.grade6.fraction.integer_division";
pub const SKILL_ID_FRACTION_SUMMARY_IMPROPER: &str = "jp.grade6.fraction.summary_improper";
pub const CURRICULUM_PATH_FRACTION_ADDITION: [&str; 3] = ["root", "小学5年生", "分数の足し算"];
pub const CURRICULUM_PATH_FRACTION_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_SUBTRACTION: [&str; 3] = ["root", "小学5年生", "分数の引き算"];
pub const CURRICULUM_PATH_FRACTION_DIVISION: [&str; 3] = ["root", "小学6年生", "分数の割り算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_DIVISION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の割り算"];
pub const CURRICULUM_PATH_FRACTION_SUMMARY_IMPROPER: [&str; 3] =
    ["root", "小学6年生", "分数総まとめ(仮分数)"];

const ADDITION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Addition];
const SUBTRACTION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Subtraction];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Division];
const SUMMARY: &[ThemeTag] = &[
    ThemeTag::Fractions,
    ThemeTag::Addition,
    ThemeTag::Subtraction,
    ThemeTag::Multiplication,
    ThemeTag::Division,
];

pub const SUMMARY_LAYERS: [SamplingLayerSpec; 4] = [
    SamplingLayerSpec {
        key: "addition",
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        key: "subtraction",
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        key: "multiplication",
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        key: "division",
        weight: 1,
        minimum: 1,
    },
];

const MIXED_PRESENTATION: Presentation =
    Presentation::STANDARD.with_fraction(FractionPresentation::MixedNumberWhenImproper);
const IMPROPER_PRESENTATION: Presentation =
    Presentation::STANDARD.with_fraction(FractionPresentation::KeepImproperFraction);
const FRACTION_ANSWER: AnswerContract = AnswerContract {
    prompt_kind: Prompt::Arithmetic,
    answer_schema_kind: Schema::Rational,
    input_profile: Input::Fraction,
};

pub const FRACTION_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_ADDITION,
        generator_revision: GENERATOR_REVISION_FRACTION_ADDITION,
        skill_id: SKILL_ID_FRACTION_ADDITION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_ADDITION,
        grade: Some(5),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_SUBTRACTION,
        generator_revision: GENERATOR_REVISION_FRACTION_SUBTRACTION,
        skill_id: SKILL_ID_FRACTION_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUBTRACTION,
        grade: Some(5),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_MULTIPLICATION,
        generator_revision: GENERATOR_REVISION_FRACTION_MULTIPLICATION,
        skill_id: SKILL_ID_FRACTION_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_MULTIPLICATION,
        grade: Some(6),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_DIVISION,
        generator_revision: GENERATOR_REVISION_FRACTION_DIVISION,
        skill_id: SKILL_ID_FRACTION_DIVISION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_DIVISION,
        grade: Some(6),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_INTEGER_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_INTEGER_MULTIPLICATION,
        generator_revision: GENERATOR_REVISION_FRACTION_INTEGER_MULTIPLICATION,
        skill_id: SKILL_ID_FRACTION_INTEGER_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_INTEGER_MULTIPLICATION,
        grade: Some(6),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_INTEGER_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_INTEGER_DIVISION,
        generator_revision: GENERATOR_REVISION_FRACTION_INTEGER_DIVISION,
        skill_id: SKILL_ID_FRACTION_INTEGER_DIVISION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_INTEGER_DIVISION,
        grade: Some(6),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_SUMMARY_IMPROPER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_FRACTION_SUMMARY_IMPROPER,
        generator_revision: GENERATOR_REVISION_FRACTION_SUMMARY_IMPROPER,
        skill_id: SKILL_ID_FRACTION_SUMMARY_IMPROPER,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUMMARY_IMPROPER,
        grade: Some(6),
        tags: SUMMARY,
        safety: Safety::NonNegativeOnly,
        presentation: IMPROPER_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    IntegerMultiplication,
    IntegerDivision,
    SummaryImproper,
}

#[derive(Clone, Copy, Debug)]
struct FractionEntry {
    operator: ArithmeticOperator,
    left: RationalCoefficient,
    right: RationalCoefficient,
    result: RationalCoefficient,
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
        (self.mode == Mode::SummaryImproper).then_some(&SUMMARY_LAYERS)
    }

    fn sampling_layer(&self, problem: &Problem) -> Option<usize> {
        if self.mode != Mode::SummaryImproper {
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
            ArithmeticOperator::Multiply => Some(2),
            ArithmeticOperator::Divide => Some(3),
        }
    }

    fn bootstrap_layer_multiplier(&self) -> usize {
        1
    }

    fn deduplicate_bootstrap_pool(&self) -> bool {
        true
    }

    fn constructive_layer_sampling(&self) -> bool {
        self.mode == Mode::SummaryImproper
    }

    fn draw_candidate_for_layer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        layer: usize,
    ) -> Option<Problem> {
        if !self.constructive_layer_sampling() {
            return None;
        }
        let entry = draw_summary_entry_for_layer(layer, rng)?;
        Some(build_problem(
            self.registration,
            self.mode,
            ordinal,
            weights,
            entry,
        ))
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        let entry = draw_constructive_entry(self.mode, rng)?;
        Some(build_problem(
            self.registration,
            self.mode,
            ordinal,
            weights,
            entry,
        ))
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

generator!(ADDITION_GENERATOR, FRACTION_ADDITION_REGISTRATION, Addition);
generator!(
    SUBTRACTION_GENERATOR,
    FRACTION_SUBTRACTION_REGISTRATION,
    Subtraction
);
generator!(
    MULTIPLICATION_GENERATOR,
    FRACTION_MULTIPLICATION_REGISTRATION,
    Multiplication
);
generator!(DIVISION_GENERATOR, FRACTION_DIVISION_REGISTRATION, Division);
generator!(
    INTEGER_MULTIPLICATION_GENERATOR,
    FRACTION_INTEGER_MULTIPLICATION_REGISTRATION,
    IntegerMultiplication
);
generator!(
    INTEGER_DIVISION_GENERATOR,
    FRACTION_INTEGER_DIVISION_REGISTRATION,
    IntegerDivision
);
generator!(
    SUMMARY_IMPROPER_GENERATOR,
    FRACTION_SUMMARY_IMPROPER_REGISTRATION,
    SummaryImproper
);

fn draw_pair_from_domain(
    rng: &mut DeterministicRng,
    domain: &[RationalCoefficient],
    unordered: bool,
) -> Option<(RationalCoefficient, RationalCoefficient)> {
    if domain.is_empty() {
        return None;
    }
    if !unordered {
        let left = domain[rng.next_bounded(domain.len() as u64) as usize];
        let right = domain[rng.next_bounded(domain.len() as u64) as usize];
        return Some((left, right));
    }

    // Uniformly sample one pair from the upper triangle i <= j. Sampling two
    // independent operands and sorting would double the weight of off-diagonal
    // candidates, so use a triangular index instead.
    let pair_count = domain.len().checked_mul(domain.len() + 1)? / 2;
    let mut index = rng.next_bounded(pair_count as u64) as usize;
    for left_index in 0..domain.len() {
        let row_len = domain.len() - left_index;
        if index < row_len {
            return Some((domain[left_index], domain[left_index + index]));
        }
        index -= row_len;
    }
    None
}

fn small_positive_integer(value: RationalCoefficient) -> bool {
    value.denominator == 1 && (1..=9).contains(&value.numerator)
}

fn draw_standard_fraction_entry(mode: Mode, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    let operator = operator_for_mode(mode)?;
    let unordered = matches!(
        operator,
        ArithmeticOperator::Add | ArithmeticOperator::Multiply
    );
    let (left, right) = draw_pair_from_domain(rng, operand_domain_v1(), unordered)?;
    let result = match operator {
        ArithmeticOperator::Add => left.checked_add(right),
        ArithmeticOperator::Subtract => left.subtract(right),
        ArithmeticOperator::Multiply => left.multiply(right),
        ArithmeticOperator::Divide => left.divide(right),
    }?;
    if result.numerator <= 0 {
        return None;
    }
    let allowed = match mode {
        Mode::Addition => {
            result.denominator > 1 && result.numerator <= 65 && result.denominator <= 72
        }
        Mode::Subtraction | Mode::Multiplication => operand_domain_v1().contains(&result),
        Mode::Division => operand_domain_v1().contains(&result) || small_positive_integer(result),
        _ => return None,
    };
    allowed.then_some(FractionEntry {
        operator,
        left,
        right,
        result,
    })
}

fn draw_integer_fraction_entry(mode: Mode, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    let fraction = operand_domain_v1()[rng.next_bounded(operand_domain_v1().len() as u64) as usize];
    let integer = RationalCoefficient::new(1 + rng.next_bounded(9) as i64, 1)?;
    let operator = operator_for_mode(mode)?;
    let (left, right) = match operator {
        ArithmeticOperator::Multiply => (fraction, integer),
        ArithmeticOperator::Divide if rng.next_bounded(2) == 0 => (fraction, integer),
        ArithmeticOperator::Divide => (integer, fraction),
        _ => return None,
    };
    let result = match operator {
        ArithmeticOperator::Multiply => left.multiply(right),
        ArithmeticOperator::Divide => left.divide(right),
        _ => return None,
    }?;
    let allowed = result.numerator > 0
        && (operand_domain_v1().contains(&result) || small_positive_integer(result));
    allowed.then_some(FractionEntry {
        operator,
        left,
        right,
        result,
    })
}

fn summary_operand_domain_v1() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = operand_domain_v1().to_vec();
        values.extend((1_i64..=9).map(|value| RationalCoefficient::new(value, 1).unwrap()));
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn draw_summary_entry_for_layer(layer: usize, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    match layer {
        0 => draw_standard_fraction_entry(Mode::Addition, rng),
        1 => draw_standard_fraction_entry(Mode::Subtraction, rng),
        2 | 3 => {
            let operator = if layer == 2 {
                ArithmeticOperator::Multiply
            } else {
                ArithmeticOperator::Divide
            };
            let operands = summary_operand_domain_v1();
            let (left, right) =
                draw_pair_from_domain(rng, operands, operator == ArithmeticOperator::Multiply)?;
            if left.is_integer() && right.is_integer() {
                return None;
            }
            let result = match operator {
                ArithmeticOperator::Multiply => left.multiply(right),
                ArithmeticOperator::Divide => left.divide(right),
                _ => unreachable!(),
            }?;
            (result.numerator > 0 && result.numerator <= 200 && result.denominator <= 196)
                .then_some(FractionEntry {
                    operator,
                    left,
                    right,
                    result,
                })
        }
        _ => None,
    }
}

fn draw_summary_entry(rng: &mut DeterministicRng) -> Option<FractionEntry> {
    draw_summary_entry_for_layer(rng.next_bounded(4) as usize, rng)
}

fn draw_constructive_entry(mode: Mode, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    match mode {
        Mode::Addition | Mode::Subtraction | Mode::Multiplication | Mode::Division => {
            draw_standard_fraction_entry(mode, rng)
        }
        Mode::IntegerMultiplication | Mode::IntegerDivision => {
            draw_integer_fraction_entry(mode, rng)
        }
        Mode::SummaryImproper => draw_summary_entry(rng),
    }
}

// Canonical operand domain for the current fraction curriculum.
pub(crate) fn operand_domain_v1() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = Vec::new();
        for denominator in 2_i64..=14 {
            for numerator in 1_i64..=(15 - denominator) {
                let Some(value) = RationalCoefficient::new(numerator, denominator) else {
                    continue;
                };
                if value.denominator == 1 {
                    continue;
                }
                values.push(value);
            }
        }
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn operator_for_mode(mode: Mode) -> Option<ArithmeticOperator> {
    match mode {
        Mode::Addition => Some(ArithmeticOperator::Add),
        Mode::Subtraction => Some(ArithmeticOperator::Subtract),
        Mode::Multiplication | Mode::IntegerMultiplication => Some(ArithmeticOperator::Multiply),
        Mode::Division | Mode::IntegerDivision => Some(ArithmeticOperator::Divide),
        Mode::SummaryImproper => None,
    }
}

fn build_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    id: u32,
    weights: &OperationWeights,
    entry: FractionEntry,
) -> Problem {
    let FractionEntry {
        operator,
        left,
        right,
        result,
    } = entry;
    let expression = binary_expression(
        operator,
        rational_expression(left),
        rational_expression(right),
    );
    let answer = if mode == Mode::SummaryImproper {
        rational_answer(result)
    } else {
        mixed_number_answer(result)
    };
    let self_division = operator == ArithmeticOperator::Divide && left == right;
    let solution_graph = if self_division {
        SolutionGraph {
            steps: vec![SolutionStep {
                id: 1,
                operation: Operation::FractionSelfDivision,
                depends_on: vec![],
            }],
        }
    } else {
        arithmetic_expression_graph(&expression, &rational_answer(result))
            .expect("accepted fraction-domain expression must have an effort graph")
    };
    let effort = calculate_graph_effort(&solution_graph, weights);
    let (max_abs_numerator, max_denominator) = match mode {
        Mode::Addition => (65, 72),
        Mode::Subtraction
        | Mode::Multiplication
        | Mode::Division
        | Mode::IntegerMultiplication
        | Mode::IntegerDivision => (13, 14),
        Mode::SummaryImproper => (200, 196),
    };
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: registration.numeric_theme_id,
        prompt: ProblemPrompt::Arithmetic { expression },
        input_interface: input_interface(registration.answer_contract.input_profile),
        answer_schema: AnswerSchema::Rational {
            max_abs_numerator,
            max_denominator,
            require_reduced_fraction_form: true,
        },
        canonical_answer: answer,
        worked_solution: None,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    }
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 7] = [
    GeneratorEntry::current(&ADDITION_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DIVISION_GENERATOR),
    GeneratorEntry::current(&INTEGER_MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&INTEGER_DIVISION_GENERATOR),
    GeneratorEntry::current(&SUMMARY_IMPROPER_GENERATOR),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::OperationKind;

    #[test]
    fn fraction_self_division_uses_one_unit_primitive() {
        let value = RationalCoefficient::new(2, 3).unwrap();
        let one = RationalCoefficient::new(1, 1).unwrap();
        let problem = build_problem(
            &FRACTION_DIVISION_REGISTRATION,
            Mode::Division,
            1,
            &OperationWeights::default(),
            FractionEntry {
                operator: ArithmeticOperator::Divide,
                left: value,
                right: value,
                result: one,
            },
        );
        assert_eq!(problem.effort, 1.0);
        assert_eq!(
            problem
                .operation_vector
                .get(OperationKind::FractionSelfDivision),
            1.0
        );
    }
}
