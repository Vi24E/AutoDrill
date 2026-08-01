use crate::answer::AnswerNode;
use crate::model::{GradeResult, GradeStatus, GradeWarning};
use crate::normalize::normalize_answer;

pub fn grade_answer(expected: &AnswerNode, actual: &AnswerNode) -> GradeResult {
    let representation_differs = expected != actual;
    let normalized_expected = normalize_answer(expected);
    let normalized_actual = normalize_answer(actual);
    let status = match normalized_actual {
        AnswerNode::Empty => GradeStatus::Unanswered,
        _ if normalized_expected == normalized_actual => GradeStatus::Correct,
        _ => GradeStatus::Incorrect,
    };
    let is_correct = matches!(status, GradeStatus::Correct);
    let warnings = if is_correct && representation_differs {
        representation_warnings(actual)
    } else {
        Vec::new()
    };
    GradeResult {
        status,
        is_correct,
        expected: normalized_expected,
        actual: normalized_actual,
        warnings,
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
                    if *right != 0 && integer_gcd(left.unsigned_abs(), right.unsigned_abs()) > 1
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
                    if *right != 0 && integer_gcd(left.unsigned_abs(), right.unsigned_abs()) > 1
            ) || has_reducible_fraction(whole)
                || has_reducible_fraction(numerator)
                || has_reducible_fraction(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            has_reducible_fraction(radicand) || index.as_deref().is_some_and(has_reducible_fraction)
        }
        AnswerNode::Negative(value) | AnswerNode::PlusMinus(value) => has_reducible_fraction(value),
        AnswerNode::Tuple(values) => values.iter().any(has_reducible_fraction),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
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
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_negative),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::Variable(_) => false,
    }
}

fn starts_negative(answer: &AnswerNode) -> bool {
    match normalize_answer(answer) {
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
        | AnswerNode::Tuple(_)
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
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_decimal),
        AnswerNode::Empty | AnswerNode::Integer(_) | AnswerNode::Variable(_) => false,
    }
}

fn integer_gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
