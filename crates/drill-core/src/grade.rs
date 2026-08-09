use crate::answer::AnswerNode;
use crate::model::{AnswerSchema, GradeResult, GradeStatus, GradeWarning};
use crate::normalize::normalize_answer;

pub fn grade_answer(expected: &AnswerNode, actual: &AnswerNode) -> GradeResult {
    grade_answer_with_schema(expected, actual, None)
}

pub fn grade_answer_with_schema(
    expected: &AnswerNode,
    actual: &AnswerNode,
    answer_schema: Option<&AnswerSchema>,
) -> GradeResult {
    let representation_differs = expected != actual;
    let normalized_expected = normalize_answer(expected);
    let normalized_actual = normalize_answer(actual);
    let mathematically_equal = normalized_expected == normalized_actual;
    let mut status = match (&normalized_expected, &normalized_actual) {
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

    if mathematically_equal
        && matches!(normalized_expected, AnswerNode::Integer(_))
        && !uses_integer_display_form(actual)
    {
        push_warning(&mut warnings, GradeWarning::IntegerFormRequired);
    }

    if mathematically_equal
        && matches!(normalized_expected, AnswerNode::Fraction { .. })
        && matches!(
            answer_schema,
            Some(AnswerSchema::Rational {
                require_reduced_fraction_form: true,
                ..
            })
        )
    {
        if has_reducible_fraction(actual) {
            // An unreduced ordinary fraction violates the explicit answer
            // format and is intentionally incorrect even when numerically equal.
            status = GradeStatus::Incorrect;
            push_warning(&mut warnings, GradeWarning::FractionNotReduced);
        } else if !uses_simple_reduced_fraction_form(actual) {
            // Mixed fractions, exact decimals, nested fractions, roots and
            // other mathematically equivalent compatibility forms stay correct,
            // but the worksheet explicitly asks for one simple reduced fraction.
            push_warning(&mut warnings, GradeWarning::FractionFormRequired);
        }
    }

    let is_correct = matches!(status, GradeStatus::Correct);
    GradeResult {
        status,
        is_correct,
        expected: normalized_expected,
        actual: normalized_actual,
        warnings,
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
                if *right > 0 && integer_gcd(left.unsigned_abs(), right.unsigned_abs()) == 1
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
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_negative),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::ExactDecimal { .. }
        | AnswerNode::NanError(_)
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
        AnswerNode::Tuple(values) => values.iter().any(has_redundant_decimal),
        AnswerNode::Empty
        | AnswerNode::Integer(_)
        | AnswerNode::NanError(_)
        | AnswerNode::Variable(_) => false,
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
