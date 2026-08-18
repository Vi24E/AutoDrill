use crate::answer::AnswerNode;
use crate::exact::gcd_u64;
use crate::model::{AnswerSchema, GradeResult, GradeStatus, GradeWarning};
use crate::normalize::normalize_answer;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GradeError {
    #[error("answer schema is structurally invalid")]
    InvalidAnswerSchema,
    #[error("expected canonical answer does not satisfy the answer schema")]
    ExpectedAnswerOutsideSchema,
    #[error("answer AST exceeds the current size limit")]
    AnswerAstSizeLimit,
}

impl GradeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidAnswerSchema => "invalid_answer_schema",
            Self::ExpectedAnswerOutsideSchema => "expected_answer_outside_schema",
            Self::AnswerAstSizeLimit => "answer_ast_size_limit",
        }
    }
}

pub fn grade_answer(expected: &AnswerNode, actual: &AnswerNode) -> GradeResult {
    if !expected.is_within_structural_node_limit() || !actual.is_within_structural_node_limit() {
        return GradeResult::new(
            GradeStatus::Incorrect,
            AnswerNode::NanError("answer_ast_size_limit".to_owned()),
            AnswerNode::NanError("answer_ast_size_limit".to_owned()),
            Vec::new(),
        );
    }
    grade_answer_impl(expected, actual, None)
}

pub fn grade_answer_with_schema(
    expected: &AnswerNode,
    actual: &AnswerNode,
    answer_schema: Option<&AnswerSchema>,
) -> Result<GradeResult, GradeError> {
    if !expected.is_within_structural_node_limit() || !actual.is_within_structural_node_limit() {
        return Err(GradeError::AnswerAstSizeLimit);
    }
    if let Some(schema) = answer_schema {
        if !schema.is_structurally_valid() {
            return Err(GradeError::InvalidAnswerSchema);
        }
        if !schema.accepts_canonical_answer(expected) {
            return Err(GradeError::ExpectedAnswerOutsideSchema);
        }
    }
    Ok(grade_answer_impl(expected, actual, answer_schema))
}

fn grade_answer_impl(
    expected: &AnswerNode,
    actual: &AnswerNode,
    answer_schema: Option<&AnswerSchema>,
) -> GradeResult {
    let representation_differs = expected != actual;
    let normalized_expected = normalize_answer(expected);
    let normalized_actual = normalize_answer(actual);
    let ordered_length = match answer_schema {
        Some(AnswerSchema::OrderedPair) => Some(2_usize),
        Some(AnswerSchema::OrderedTuple { length }) => Some(usize::from(*length)),
        _ => None,
    };
    let ordered_shape_valid = ordered_length.is_none_or(|length| {
        matches!(&normalized_expected, AnswerNode::Tuple(values) if values.len() == length)
            && matches!(&normalized_actual, AnswerNode::Tuple(values) if values.len() == length)
    });
    let mathematically_equal = if ordered_length.is_some() {
        ordered_shape_valid && normalized_expected == normalized_actual
    } else {
        solutions_mathematically_equal(&normalized_expected, &normalized_actual)
    };
    let status = match (&normalized_expected, &normalized_actual) {
        _ if contains_nan_error(&normalized_expected) || contains_nan_error(&normalized_actual) => {
            GradeStatus::Incorrect
        }
        (_, AnswerNode::Empty) => GradeStatus::Unanswered,
        _ if mathematically_equal => GradeStatus::Correct,
        _ => GradeStatus::Incorrect,
    };
    let mut warnings = if mathematically_equal && representation_differs {
        representation_warnings(actual)
    } else {
        Vec::new()
    };
    if ordered_length.is_none() && has_duplicate_solution(actual) {
        push_warning(&mut warnings, GradeWarning::DuplicateSolution);
    }
    if ordered_length.is_none()
        && matches!(normalized_expected, AnswerNode::Tuple(_))
        && has_embedded_plus_minus(actual)
    {
        push_warning(&mut warnings, GradeWarning::SolutionListRequired);
    }

    if mathematically_equal
        && matches!(expected, AnswerNode::MixedFraction { .. })
        && !matches!(actual, AnswerNode::MixedFraction { .. })
    {
        push_warning(&mut warnings, GradeWarning::MixedFractionFormRequired);
    }

    if mathematically_equal
        && matches!(normalized_expected, AnswerNode::Integer(_))
        && !uses_integer_display_form(actual)
    {
        push_warning(&mut warnings, GradeWarning::IntegerFormRequired);
    }

    if mathematically_equal
        && matches!(normalized_expected, AnswerNode::Fraction { .. })
        && !matches!(expected, AnswerNode::MixedFraction { .. })
        && matches!(
            answer_schema,
            Some(AnswerSchema::Rational {
                require_reduced_fraction_form: true,
                ..
            })
        )
    {
        if has_reducible_fraction(actual) {
            // Representation policy (warning => ○ or ×) is selected by the
            // client. Core grading reports mathematical correctness plus a
            // stable warning code and does not hard-code that policy.
            push_warning(&mut warnings, GradeWarning::FractionNotReduced);
        } else if !uses_simple_reduced_fraction_form(actual) {
            // Mixed fractions, exact decimals, nested fractions, roots and
            // other mathematically equivalent compatibility forms stay correct,
            // but the worksheet explicitly asks for one simple reduced fraction.
            push_warning(&mut warnings, GradeWarning::FractionFormRequired);
        }
    }

    GradeResult::new(status, normalized_expected, normalized_actual, warnings)
}

const MAX_SOLUTION_BRANCHES: usize = 4;

fn solutions_mathematically_equal(left: &AnswerNode, right: &AnswerNode) -> bool {
    let left = canonicalize_solution_order(left);
    let right = canonicalize_solution_order(right);
    if left == right {
        return true;
    }
    let one_is_tuple = matches!(left, AnswerNode::Tuple(_)) ^ matches!(right, AnswerNode::Tuple(_));
    if one_is_tuple {
        let non_tuple = if matches!(left, AnswerNode::Tuple(_)) {
            &right
        } else {
            &left
        };
        if !contains_plus_minus(non_tuple) {
            return false;
        }
    }
    match (solution_set(&left), solution_set(&right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn solution_set(answer: &AnswerNode) -> Option<Vec<AnswerNode>> {
    let mut values = Vec::new();
    match answer {
        AnswerNode::Tuple(items) => {
            for item in items {
                values.extend(expand_plus_minus(item)?);
                if values.len() > MAX_SOLUTION_BRANCHES {
                    return None;
                }
            }
        }
        _ => values.extend(expand_plus_minus(answer)?),
    }
    values = values
        .into_iter()
        .map(|value| canonicalize_algebraic_signs(&normalize_answer(&value)))
        .map(|value| normalize_answer(&value))
        .collect();
    values.sort();
    Some(values)
}

fn expand_plus_minus(answer: &AnswerNode) -> Option<Vec<AnswerNode>> {
    match answer {
        AnswerNode::PlusMinus(value) => {
            let expanded = expand_plus_minus(value)?;
            let mut values = Vec::with_capacity(expanded.len().saturating_mul(2));
            for value in expanded {
                values.push(value.clone());
                values.push(AnswerNode::Negative(Box::new(value)));
                if values.len() > MAX_SOLUTION_BRANCHES {
                    return None;
                }
            }
            Some(values)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => combine_branches(
            expand_plus_minus(numerator)?,
            expand_plus_minus(denominator)?,
            |numerator, denominator| AnswerNode::Fraction {
                numerator: Box::new(numerator),
                denominator: Box::new(denominator),
            },
        ),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            let wholes = expand_plus_minus(whole)?;
            let numerators = expand_plus_minus(numerator)?;
            let denominators = expand_plus_minus(denominator)?;
            let mut values = Vec::new();
            for whole in wholes {
                for numerator in &numerators {
                    for denominator in &denominators {
                        values.push(AnswerNode::MixedFraction {
                            whole: Box::new(whole.clone()),
                            numerator: Box::new(numerator.clone()),
                            denominator: Box::new(denominator.clone()),
                        });
                        if values.len() > MAX_SOLUTION_BRANCHES {
                            return None;
                        }
                    }
                }
            }
            Some(values)
        }
        AnswerNode::Root { radicand, index } => {
            let radicands = expand_plus_minus(radicand)?;
            let indices = match index {
                Some(index) => expand_plus_minus(index)?,
                None => vec![AnswerNode::Empty],
            };
            let mut values = Vec::new();
            for radicand in radicands {
                for index in &indices {
                    values.push(AnswerNode::Root {
                        radicand: Box::new(radicand.clone()),
                        index: (!matches!(index, AnswerNode::Empty))
                            .then(|| Box::new(index.clone())),
                    });
                    if values.len() > MAX_SOLUTION_BRANCHES {
                        return None;
                    }
                }
            }
            Some(values)
        }
        AnswerNode::Negative(value) => Some(
            expand_plus_minus(value)?
                .into_iter()
                .map(|value| AnswerNode::Negative(Box::new(value)))
                .collect(),
        ),
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left_values = expand_plus_minus(left)?;
            let right_values = expand_plus_minus(right)?;
            combine_branches(left_values, right_values, |left, right| {
                AnswerNode::Binary {
                    operator: *operator,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            })
        }
        AnswerNode::Tuple(values) => {
            let mut expanded_items = Vec::new();
            for value in values {
                expanded_items.extend(expand_plus_minus(value)?);
                if expanded_items.len() > MAX_SOLUTION_BRANCHES {
                    return None;
                }
            }
            Some(vec![AnswerNode::Tuple(expanded_items)])
        }
        _ => Some(vec![answer.clone()]),
    }
}

fn combine_branches<F>(
    left: Vec<AnswerNode>,
    right: Vec<AnswerNode>,
    mut build: F,
) -> Option<Vec<AnswerNode>>
where
    F: FnMut(AnswerNode, AnswerNode) -> AnswerNode,
{
    let mut values = Vec::new();
    for left in left {
        for right in &right {
            values.push(build(left.clone(), right.clone()));
            if values.len() > MAX_SOLUTION_BRANCHES {
                return None;
            }
        }
    }
    Some(values)
}

fn canonicalize_algebraic_signs(answer: &AnswerNode) -> AnswerNode {
    match answer {
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = canonicalize_algebraic_signs(left);
            let right = canonicalize_algebraic_signs(right);
            match (operator, &right) {
                (crate::answer::AnswerBinaryOperator::Add, AnswerNode::Negative(value)) => {
                    AnswerNode::Binary {
                        operator: crate::answer::AnswerBinaryOperator::Subtract,
                        left: Box::new(left),
                        right: Box::new(value.as_ref().clone()),
                    }
                }
                (crate::answer::AnswerBinaryOperator::Subtract, AnswerNode::Negative(value)) => {
                    AnswerNode::Binary {
                        operator: crate::answer::AnswerBinaryOperator::Add,
                        left: Box::new(left),
                        right: Box::new(value.as_ref().clone()),
                    }
                }
                (operator, _) => AnswerNode::Binary {
                    operator: *operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            }
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => AnswerNode::Fraction {
            numerator: Box::new(canonicalize_algebraic_signs(numerator)),
            denominator: Box::new(canonicalize_algebraic_signs(denominator)),
        },
        AnswerNode::Root { radicand, index } => AnswerNode::Root {
            radicand: Box::new(canonicalize_algebraic_signs(radicand)),
            index: index
                .as_deref()
                .map(canonicalize_algebraic_signs)
                .map(Box::new),
        },
        AnswerNode::Negative(value) => {
            AnswerNode::Negative(Box::new(canonicalize_algebraic_signs(value)))
        }
        _ => answer.clone(),
    }
}

fn canonicalize_solution_order(answer: &AnswerNode) -> AnswerNode {
    match answer {
        AnswerNode::Tuple(values) => {
            let mut values: Vec<_> = values.iter().map(canonicalize_solution_order).collect();
            values.sort();
            AnswerNode::Tuple(values)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => AnswerNode::Fraction {
            numerator: Box::new(canonicalize_solution_order(numerator)),
            denominator: Box::new(canonicalize_solution_order(denominator)),
        },
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => AnswerNode::MixedFraction {
            whole: Box::new(canonicalize_solution_order(whole)),
            numerator: Box::new(canonicalize_solution_order(numerator)),
            denominator: Box::new(canonicalize_solution_order(denominator)),
        },
        AnswerNode::Root { radicand, index } => AnswerNode::Root {
            radicand: Box::new(canonicalize_solution_order(radicand)),
            index: index
                .as_deref()
                .map(canonicalize_solution_order)
                .map(Box::new),
        },
        AnswerNode::Negative(value) => {
            AnswerNode::Negative(Box::new(canonicalize_solution_order(value)))
        }
        AnswerNode::PlusMinus(value) => {
            AnswerNode::PlusMinus(Box::new(canonicalize_solution_order(value)))
        }
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => AnswerNode::Binary {
            operator: *operator,
            left: Box::new(canonicalize_solution_order(left)),
            right: Box::new(canonicalize_solution_order(right)),
        },
        _ => answer.clone(),
    }
}

fn push_warning(warnings: &mut Vec<GradeWarning>, warning: GradeWarning) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn uses_integer_display_form(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::Integer(_) => true,
        AnswerNode::Negative(value) => matches!(value.as_ref(), AnswerNode::Integer(0..)),
        _ => false,
    }
}

fn uses_simple_reduced_fraction_form(answer: &AnswerNode) -> bool {
    fn simple_fraction(value: &AnswerNode) -> bool {
        let AnswerNode::Fraction {
            numerator,
            denominator,
        } = value
        else {
            return false;
        };
        matches!(
            (numerator.as_ref(), denominator.as_ref()),
            (AnswerNode::Integer(left), AnswerNode::Integer(right))
                if *right > 0 && gcd_u64(left.unsigned_abs(), right.unsigned_abs()) == 1
        )
    }
    simple_fraction(answer)
        || matches!(answer, AnswerNode::Negative(value) if simple_fraction(value))
}

fn contains_nan_error(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::NanError(_) => true,
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => contains_nan_error(numerator) || contains_nan_error(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            contains_nan_error(whole)
                || contains_nan_error(numerator)
                || contains_nan_error(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            contains_nan_error(radicand) || index.as_deref().is_some_and(contains_nan_error)
        }
        AnswerNode::Negative(value) | AnswerNode::PlusMinus(value) => contains_nan_error(value),
        AnswerNode::Binary { left, right, .. } => {
            contains_nan_error(left) || contains_nan_error(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(contains_nan_error),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::Variable(_) => false,
    }
}

fn representation_warnings(answer: &AnswerNode) -> Vec<GradeWarning> {
    let mut warnings = Vec::new();
    if has_reducible_fraction(answer) {
        warnings.push(GradeWarning::FractionNotReduced);
    }
    if has_redundant_negative(answer) {
        warnings.push(GradeWarning::RedundantNegative);
    }
    if has_redundant_plus_minus(answer) {
        warnings.push(GradeWarning::RedundantPlusMinus);
    }
    if has_redundant_decimal(answer) {
        warnings.push(GradeWarning::RedundantDecimal);
    }
    warnings
}

fn has_reducible_fraction(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => {
            matches!(
                (numerator.as_ref(), denominator.as_ref()),
                (AnswerNode::Integer(left), AnswerNode::Integer(right))
                    if *right != 0 && gcd_u64(left.unsigned_abs(), right.unsigned_abs()) > 1
            ) || has_reducible_fraction(numerator)
                || has_reducible_fraction(denominator)
        }
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            matches!(
                (numerator.as_ref(), denominator.as_ref()),
                (AnswerNode::Integer(left), AnswerNode::Integer(right))
                    if *right != 0 && gcd_u64(left.unsigned_abs(), right.unsigned_abs()) > 1
            ) || has_reducible_fraction(whole)
                || has_reducible_fraction(numerator)
                || has_reducible_fraction(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            has_reducible_fraction(radicand) || index.as_deref().is_some_and(has_reducible_fraction)
        }
        AnswerNode::Negative(value) | AnswerNode::PlusMinus(value) => has_reducible_fraction(value),
        AnswerNode::Binary { left, right, .. } => {
            has_reducible_fraction(left) || has_reducible_fraction(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(has_reducible_fraction),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn has_redundant_negative(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::Negative(value) => starts_negative(value) || has_redundant_negative(value),
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => has_redundant_negative(numerator) || has_redundant_negative(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            has_redundant_negative(whole)
                || has_redundant_negative(numerator)
                || has_redundant_negative(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            has_redundant_negative(radicand) || index.as_deref().is_some_and(has_redundant_negative)
        }
        AnswerNode::PlusMinus(value) => has_redundant_negative(value),
        AnswerNode::Binary { left, right, .. } => {
            has_redundant_negative(left) || has_redundant_negative(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_negative),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn starts_negative(answer: &AnswerNode) -> bool {
    let normalized = normalize_answer(answer);
    match &normalized {
        AnswerNode::Negative(_) | AnswerNode::Integer(i64::MIN..=-1) => true,
        AnswerNode::ExactDecimal {
            coefficient: i64::MIN..=-1,
            ..
        } => true,
        AnswerNode::Fraction { numerator, .. } => {
            matches!(numerator.as_ref(), AnswerNode::Integer(i64::MIN..=-1))
        }
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::MixedFraction { .. }
        | AnswerNode::Root { .. }
        | AnswerNode::PlusMinus(_)
        | AnswerNode::Binary { .. }
        | AnswerNode::Tuple(_)
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn has_redundant_plus_minus(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::PlusMinus(value) => {
            matches!(value.as_ref(), AnswerNode::PlusMinus(_)) || has_redundant_plus_minus(value)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => has_redundant_plus_minus(numerator) || has_redundant_plus_minus(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            has_redundant_plus_minus(whole)
                || has_redundant_plus_minus(numerator)
                || has_redundant_plus_minus(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            has_redundant_plus_minus(radicand)
                || index.as_deref().is_some_and(has_redundant_plus_minus)
        }
        AnswerNode::Negative(value) => has_redundant_plus_minus(value),
        AnswerNode::Binary { left, right, .. } => {
            has_redundant_plus_minus(left) || has_redundant_plus_minus(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_plus_minus),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn has_duplicate_solution(answer: &AnswerNode) -> bool {
    let AnswerNode::Tuple(values) = answer else {
        return false;
    };
    let mut normalized: Vec<_> = values.iter().map(normalize_answer).collect();
    normalized.sort();
    normalized.windows(2).any(|pair| pair[0] == pair[1])
}

fn has_embedded_plus_minus(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::PlusMinus(_) => false,
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => contains_plus_minus(numerator) || contains_plus_minus(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            contains_plus_minus(whole)
                || contains_plus_minus(numerator)
                || contains_plus_minus(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            contains_plus_minus(radicand) || index.as_deref().is_some_and(contains_plus_minus)
        }
        AnswerNode::Negative(value) => contains_plus_minus(value),
        AnswerNode::Binary { left, right, .. } => {
            contains_plus_minus(left) || contains_plus_minus(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(contains_plus_minus),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn contains_plus_minus(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::PlusMinus(_) => true,
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => contains_plus_minus(numerator) || contains_plus_minus(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            contains_plus_minus(whole)
                || contains_plus_minus(numerator)
                || contains_plus_minus(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            contains_plus_minus(radicand) || index.as_deref().is_some_and(contains_plus_minus)
        }
        AnswerNode::Negative(value) => contains_plus_minus(value),
        AnswerNode::Binary { left, right, .. } => {
            contains_plus_minus(left) || contains_plus_minus(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(contains_plus_minus),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

fn has_redundant_decimal(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::ExactDecimal { coefficient, scale } => {
            *scale == 0 || (*scale > 0 && coefficient % 10 == 0)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => has_redundant_decimal(numerator) || has_redundant_decimal(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            has_redundant_decimal(whole)
                || has_redundant_decimal(numerator)
                || has_redundant_decimal(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            has_redundant_decimal(radicand) || index.as_deref().is_some_and(has_redundant_decimal)
        }
        AnswerNode::Negative(value) | AnswerNode::PlusMinus(value) => has_redundant_decimal(value),
        AnswerNode::Binary { left, right, .. } => {
            has_redundant_decimal(left) || has_redundant_decimal(right)
        }
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_decimal),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grade_answer_with_schema(
        expected: &AnswerNode,
        actual: &AnswerNode,
        answer_schema: Option<&AnswerSchema>,
    ) -> GradeResult {
        super::grade_answer_with_schema(expected, actual, answer_schema)
            .expect("test answer schema must be valid")
    }

    #[test]
    fn grading_rejects_invalid_schema_contracts_before_comparison() {
        let expected = AnswerNode::Integer(2);
        assert_eq!(
            super::grade_answer_with_schema(
                &expected,
                &expected,
                Some(&AnswerSchema::Integer { min: 3, max: 1 }),
            ),
            Err(GradeError::InvalidAnswerSchema)
        );
        assert_eq!(
            super::grade_answer_with_schema(
                &expected,
                &expected,
                Some(&AnswerSchema::Integer { min: 3, max: 10 }),
            ),
            Err(GradeError::ExpectedAnswerOutsideSchema)
        );
    }

    #[test]
    fn ordered_pair_schema_preserves_coordinate_order_and_allows_equal_coordinates() {
        let expected = AnswerNode::Tuple(vec![AnswerNode::Integer(2), AnswerNode::Integer(-3)]);
        let correct =
            grade_answer_with_schema(&expected, &expected, Some(&AnswerSchema::OrderedPair));
        assert_eq!(correct.status(), GradeStatus::Correct);

        let swapped = AnswerNode::Tuple(vec![AnswerNode::Integer(-3), AnswerNode::Integer(2)]);
        let wrong = grade_answer_with_schema(&expected, &swapped, Some(&AnswerSchema::OrderedPair));
        assert_eq!(wrong.status(), GradeStatus::Incorrect);

        let equal_coordinates =
            AnswerNode::Tuple(vec![AnswerNode::Integer(2), AnswerNode::Integer(2)]);
        let equal_result = grade_answer_with_schema(
            &equal_coordinates,
            &equal_coordinates,
            Some(&AnswerSchema::OrderedPair),
        );
        assert_eq!(equal_result.status(), GradeStatus::Correct);
        assert!(!equal_result
            .warnings()
            .contains(&GradeWarning::DuplicateSolution));
    }

    #[test]
    fn ordered_tuple_schema_preserves_full_order_and_length() {
        let expected = AnswerNode::Tuple(vec![
            AnswerNode::Integer(1),
            AnswerNode::Integer(2),
            AnswerNode::Integer(3),
            AnswerNode::Integer(4),
        ]);
        let schema = AnswerSchema::OrderedTuple { length: 4 };
        assert!(grade_answer_with_schema(&expected, &expected, Some(&schema)).is_correct());
        let swapped = AnswerNode::Tuple(vec![
            AnswerNode::Integer(2),
            AnswerNode::Integer(1),
            AnswerNode::Integer(3),
            AnswerNode::Integer(4),
        ]);
        assert!(!grade_answer_with_schema(&expected, &swapped, Some(&schema)).is_correct());
        let short = AnswerNode::Tuple(vec![AnswerNode::Integer(1), AnswerNode::Integer(2)]);
        assert!(!grade_answer_with_schema(&expected, &short, Some(&schema)).is_correct());
    }

    #[test]
    fn embedded_plus_minus_expands_to_exact_solution_set() {
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: crate::answer::AnswerBinaryOperator::Add,
                left: Box::new(AnswerNode::Integer(2)),
                right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(4)))),
            }),
            denominator: Box::new(AnswerNode::Integer(3)),
        };
        let actual = AnswerNode::Tuple(vec![
            AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(-2)),
                denominator: Box::new(AnswerNode::Integer(3)),
            },
            AnswerNode::Integer(2),
        ]);
        assert!(grade_answer(&expected, &actual).is_correct());
        assert!(grade_answer(&expected, &expected).is_correct());
    }

    #[test]
    fn embedded_plus_minus_preserves_exact_radical_semantics() {
        let root_five = AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(5)),
            index: None,
        };
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: crate::answer::AnswerBinaryOperator::Add,
                left: Box::new(AnswerNode::Integer(1)),
                right: Box::new(AnswerNode::PlusMinus(Box::new(root_five.clone()))),
            }),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let minus = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: crate::answer::AnswerBinaryOperator::Subtract,
                left: Box::new(AnswerNode::Integer(1)),
                right: Box::new(root_five.clone()),
            }),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let plus = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: crate::answer::AnswerBinaryOperator::Add,
                left: Box::new(AnswerNode::Integer(1)),
                right: Box::new(root_five),
            }),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        assert!(grade_answer(&expected, &AnswerNode::Tuple(vec![minus, plus])).is_correct());
    }

    #[test]
    fn mixed_fraction_is_preferred_but_improper_equivalent_stays_mathematically_correct() {
        let expected = AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(1)),
            numerator: Box::new(AnswerNode::Integer(1)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let actual = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(3)),
            denominator: Box::new(AnswerNode::Integer(2)),
        };
        let schema = AnswerSchema::Rational {
            max_abs_numerator: 10,
            max_denominator: 10,
            require_reduced_fraction_form: true,
        };
        let result = grade_answer_with_schema(&expected, &actual, Some(&schema));
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::MixedFractionFormRequired));
        assert!(!result
            .warnings()
            .contains(&GradeWarning::FractionFormRequired));
    }
}
