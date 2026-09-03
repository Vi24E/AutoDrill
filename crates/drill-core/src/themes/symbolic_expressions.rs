use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::effort::{linear_expression_simplification_plan, EffortModel, OperationWeights};
use crate::error::GenerationError;
use crate::generator::{
    ConstructiveLayeredCandidateSource, GeneratorEntry, ProblemGenerator, SamplingStrategy,
    SelectionDedup,
};
use crate::model::{
    AnswerSchema, LinearExpression, LinearScalar, LinearVariable, Problem, ProblemPrompt,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, CurriculumUnit, DedupPolicy as Dedup, SamplingLayerSpec,
    SchoolGrade, ThemeAnswerContract as AnswerContract, ThemePresentationPolicy as Presentation,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COMPACT_16_LAYOUT,
};

pub const THEME_ID_LINEAR_EXPRESSION: u32 = 75;
pub const GENERATOR_REVISION_LINEAR_EXPRESSION: u32 = 1;
pub const SKILL_ID_LINEAR_EXPRESSION: &str = "jp.grade7.expression.linear.collect";
pub const CURRICULUM_PATH_LINEAR_EXPRESSION: [&str; 4] =
    ["root", "中学1年生", "文字を用いた式", "一次式の整理・加減"];

const CURRICULUM_UNIT_EXPRESSIONS: CurriculumUnit =
    CurriculumUnit::new("grade7-expressions", "文字を用いた式");
const EXPRESSION_TAGS: &[ThemeTag] = &[ThemeTag::Expressions];
const EXPRESSION_LAYERS: [SamplingLayerSpec; 3] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 2,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 2,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 2,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Surface {
    CollectTerms,
    AddExpressions,
    SubtractExpressions,
}

impl Surface {
    const fn from_layer(layer: usize) -> Option<Self> {
        match layer {
            0 => Some(Self::CollectTerms),
            1 => Some(Self::AddExpressions),
            2 => Some(Self::SubtractExpressions),
            _ => None,
        }
    }
}

pub(crate) static LINEAR_EXPRESSION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LINEAR_EXPRESSION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_LINEAR_EXPRESSION,
        ),
        skill_id: SKILL_ID_LINEAR_EXPRESSION,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EXPRESSION,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: EXPRESSION_TAGS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::LinearExpression,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_EXPRESSIONS);

#[derive(Debug)]
pub(crate) struct Generator;

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        &LINEAR_EXPRESSION_REGISTRATION
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        SamplingStrategy::constructive_layered(
            self,
            SelectionDedup::Deduplicate,
            1,
            self.registration().layout().problem_count(),
        )
    }
}

impl ConstructiveLayeredCandidateSource for Generator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        &EXPRESSION_LAYERS
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::LinearExpression { expression } = problem.prompt() else {
            unreachable!("symbolic-expression generator always emits linear-expression prompts");
        };
        match expression {
            LinearExpression::Add { left, right }
                if is_grouped_linear_expression(left) && is_grouped_linear_expression(right) =>
            {
                1
            }
            LinearExpression::Subtract { left, right }
                if is_grouped_linear_expression(left) && is_grouped_linear_expression(right) =>
            {
                2
            }
            _ => 0,
        }
    }

    fn draw_candidate_for_layer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        _weights: &OperationWeights,
        layer: usize,
    ) -> Result<Option<Problem>, GenerationError> {
        let Some(surface) = Surface::from_layer(layer) else {
            return Ok(None);
        };
        draw_problem(surface, rng, ordinal)
    }
}

fn draw_problem(
    surface: Surface,
    rng: &mut DeterministicRng,
    ordinal: u32,
) -> Result<Option<Problem>, GenerationError> {
    let left_coefficient = draw_nonzero_signed(rng);
    let right_coefficient = draw_nonzero_signed(rng);
    let left_constant = draw_nonzero_signed(rng);
    let right_constant = draw_nonzero_signed(rng);

    let (coefficient, constant, expression, plan) = match surface {
        Surface::CollectTerms => (
            left_coefficient.checked_add(right_coefficient),
            left_constant.checked_add(right_constant),
            four_term_expression(
                left_coefficient,
                left_constant,
                right_coefficient,
                right_constant,
            ),
            linear_expression_simplification_plan(
                left_coefficient,
                right_coefficient,
                left_constant,
                right_constant,
            ),
        ),
        Surface::AddExpressions => (
            left_coefficient.checked_add(right_coefficient),
            left_constant.checked_add(right_constant),
            combine_linear_expressions(
                false,
                left_coefficient,
                left_constant,
                right_coefficient,
                right_constant,
            ),
            linear_expression_simplification_plan(
                left_coefficient,
                right_coefficient,
                left_constant,
                right_constant,
            ),
        ),
        Surface::SubtractExpressions => (
            left_coefficient.checked_sub(right_coefficient),
            left_constant.checked_sub(right_constant),
            combine_linear_expressions(
                true,
                left_coefficient,
                left_constant,
                right_coefficient,
                right_constant,
            ),
            crate::effort::linear_expression_subtraction_plan(
                left_coefficient,
                right_coefficient,
                left_constant,
                right_constant,
            ),
        ),
    };
    let (Some(coefficient), Some(constant)) = (coefficient, constant) else {
        return Ok(None);
    };
    // Zero-elimination is a useful later archetype, but this first drill keeps
    // the canonical answer visibly linear with a nonzero constant so the new
    // answer surface is exercised on every problem.
    if coefficient == 0 || constant == 0 {
        return Ok(None);
    }

    let answer = collected_answer(coefficient, constant);
    let problem = Problem::generated(
        &LINEAR_EXPRESSION_REGISTRATION,
        ordinal,
        ProblemPrompt::LinearExpression { expression },
        AnswerSchema::LinearExpression {
            variable: LinearVariable::X,
            require_collected_form: true,
        },
        answer,
        EffortModel::operations(plan),
    )
    .map_err(GenerationError::from)?;
    Ok(Some(problem))
}

fn draw_nonzero_signed(rng: &mut DeterministicRng) -> i64 {
    let magnitude = 1 + rng.next_bounded(9) as i64;
    if rng.next_bounded(2) == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn x_term(magnitude: i64) -> LinearExpression {
    LinearExpression::Scale {
        factor: LinearScalar::Integer { value: magnitude },
        expression: Box::new(LinearExpression::Variable {
            variable: LinearVariable::X,
        }),
    }
}

fn constant(value: i64) -> LinearExpression {
    LinearExpression::Constant {
        value: LinearScalar::Integer { value },
    }
}

fn append_signed(
    expression: LinearExpression,
    signed_value: i64,
    term: impl FnOnce(i64) -> LinearExpression,
) -> LinearExpression {
    let magnitude = signed_value.abs();
    if signed_value > 0 {
        LinearExpression::Add {
            left: Box::new(expression),
            right: Box::new(term(magnitude)),
        }
    } else {
        LinearExpression::Subtract {
            left: Box::new(expression),
            right: Box::new(term(magnitude)),
        }
    }
}

fn signed_first_x(coefficient: i64) -> LinearExpression {
    if coefficient > 0 {
        x_term(coefficient)
    } else {
        LinearExpression::Scale {
            factor: LinearScalar::Integer { value: coefficient },
            expression: Box::new(LinearExpression::Variable {
                variable: LinearVariable::X,
            }),
        }
    }
}

fn four_term_expression(
    left_coefficient: i64,
    left_constant: i64,
    right_coefficient: i64,
    right_constant: i64,
) -> LinearExpression {
    let expression = signed_first_x(left_coefficient);
    let expression = append_signed(expression, left_constant, constant);
    let expression = append_signed(expression, right_coefficient, x_term);
    append_signed(expression, right_constant, constant)
}

fn linear_pair_expression(coefficient: i64, constant_value: i64) -> LinearExpression {
    append_signed(signed_first_x(coefficient), constant_value, constant)
}

fn grouped_linear_expression(expression: LinearExpression) -> LinearExpression {
    LinearExpression::Group {
        expression: Box::new(expression),
    }
}

fn combine_linear_expressions(
    subtract: bool,
    left_coefficient: i64,
    left_constant: i64,
    right_coefficient: i64,
    right_constant: i64,
) -> LinearExpression {
    let left = grouped_linear_expression(linear_pair_expression(left_coefficient, left_constant));
    let right =
        grouped_linear_expression(linear_pair_expression(right_coefficient, right_constant));
    if subtract {
        LinearExpression::Subtract {
            left: Box::new(left),
            right: Box::new(right),
        }
    } else {
        LinearExpression::Add {
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

fn is_grouped_linear_expression(expression: &LinearExpression) -> bool {
    matches!(expression, LinearExpression::Group { .. })
}

fn answer_variable_term(coefficient: i64) -> AnswerNode {
    let variable = AnswerNode::Variable("x".to_owned());
    match coefficient {
        1 => variable,
        -1 => AnswerNode::Negative(Box::new(variable)),
        value if value < 0 => AnswerNode::Negative(Box::new(AnswerNode::Binary {
            operator: AnswerBinaryOperator::Multiply,
            left: Box::new(AnswerNode::Integer(-value)),
            right: Box::new(variable),
        })),
        value => AnswerNode::Binary {
            operator: AnswerBinaryOperator::Multiply,
            left: Box::new(AnswerNode::Integer(value)),
            right: Box::new(variable),
        },
    }
}

fn collected_answer(coefficient: i64, constant: i64) -> AnswerNode {
    let variable_term = answer_variable_term(coefficient);
    if constant > 0 {
        AnswerNode::Binary {
            operator: AnswerBinaryOperator::Add,
            left: Box::new(variable_term),
            right: Box::new(AnswerNode::Integer(constant)),
        }
    } else {
        AnswerNode::Binary {
            operator: AnswerBinaryOperator::Subtract,
            left: Box::new(variable_term),
            right: Box::new(AnswerNode::Integer(-constant)),
        }
    }
}

pub(crate) static LINEAR_EXPRESSION_GENERATOR: Generator = Generator;

pub(crate) const GENERATORS: &[GeneratorEntry] =
    &[GeneratorEntry::current(&LINEAR_EXPRESSION_GENERATOR)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_worksheet_request;
    use crate::grade::grade_answer_with_schema;
    use crate::model::GenerateWorksheetRequest;
    use crate::model::GradeWarning;
    use crate::Difficulty;

    #[test]
    fn generated_linear_expression_uses_dedicated_symbolic_contract() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest::new(
            THEME_ID_LINEAR_EXPRESSION,
            "Ab3Z",
            Difficulty::try_from(2).unwrap(),
        ))
        .unwrap();
        assert_eq!(
            worksheet.problems().len(),
            COMPACT_16_LAYOUT.problem_count()
        );
        for problem in worksheet.problems() {
            assert!(matches!(
                problem.prompt(),
                ProblemPrompt::LinearExpression { .. }
            ));
            assert_eq!(
                problem.answer_schema(),
                &AnswerSchema::LinearExpression {
                    variable: LinearVariable::X,
                    require_collected_form: true,
                }
            );
            assert!(crate::semantics::answer_is_collected_linear_form(
                problem.canonical_answer(),
                LinearVariable::X
            ));
        }
    }

    #[test]
    fn every_difficulty_includes_collect_add_and_subtract_surfaces() {
        for difficulty in 1..=4 {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest::new(
                THEME_ID_LINEAR_EXPRESSION,
                "ExprA7",
                Difficulty::try_from(difficulty).unwrap(),
            ))
            .unwrap();
            let mut counts = [0_usize; 3];
            for problem in worksheet.problems() {
                counts[LINEAR_EXPRESSION_GENERATOR.layer_of(problem)] += 1;
            }
            assert!(
                counts.iter().all(|count| *count >= 2),
                "difficulty {difficulty}: {counts:?}"
            );
        }
    }

    #[test]
    fn collected_schema_allows_a_zero_constant_answer_without_allowing_redundant_zero_terms() {
        let schema = AnswerSchema::LinearExpression {
            variable: LinearVariable::X,
            require_collected_form: true,
        };
        assert!(schema.accepts_canonical_answer(&AnswerNode::Integer(0)));
        let redundant = AnswerNode::Binary {
            operator: AnswerBinaryOperator::Add,
            left: Box::new(AnswerNode::Variable("x".to_owned())),
            right: Box::new(AnswerNode::Integer(0)),
        };
        assert!(!crate::semantics::answer_is_collected_linear_form(
            &redundant,
            LinearVariable::X
        ));
    }

    #[test]
    fn equivalent_but_uncollected_expression_is_reported_separately() {
        let expected = collected_answer(5, 3);
        let actual = AnswerNode::Binary {
            operator: AnswerBinaryOperator::Add,
            left: Box::new(AnswerNode::Binary {
                operator: AnswerBinaryOperator::Add,
                left: Box::new(answer_variable_term(2)),
                right: Box::new(answer_variable_term(3)),
            }),
            right: Box::new(AnswerNode::Integer(3)),
        };
        let schema = AnswerSchema::LinearExpression {
            variable: LinearVariable::X,
            require_collected_form: true,
        };
        let result = grade_answer_with_schema(&expected, &actual, Some(&schema)).unwrap();
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::ExpressionNotSimplified));

        let wrong = collected_answer(4, 3);
        let result = grade_answer_with_schema(&expected, &wrong, Some(&schema)).unwrap();
        assert!(!result.is_correct());
    }
}
