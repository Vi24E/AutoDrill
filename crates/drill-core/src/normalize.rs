use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::exact::ExactRational;

fn exact_rational_into_answer(value: ExactRational) -> Option<AnswerNode> {
    let numerator = i64::try_from(value.numerator()).ok()?;
    let denominator = i64::try_from(value.denominator()).ok()?;
    if denominator == 1 {
        Some(AnswerNode::Integer(numerator))
    } else {
        Some(AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(numerator)),
            denominator: Box::new(AnswerNode::Integer(denominator)),
        })
    }
}

/// Return a canonical tree while preserving the caller's display tree outside
/// this function. Exact numeric nodes normalize to a reduced rational value;
/// normalization never passes mathematical values through binary float.
///
/// Raw `AnswerNode` is a public recursive wire/domain syntax type, so native
/// callers can construct trees deeper than the interactive contract. Reject
/// those before entering recursive semantic normalization.
pub fn normalize_answer(answer: &AnswerNode) -> AnswerNode {
    if !answer.is_within_structural_node_limit() {
        return AnswerNode::NanError("answer_ast_size_limit".to_owned());
    }
    normalize_answer_bounded(answer)
}

fn normalize_answer_bounded(answer: &AnswerNode) -> AnswerNode {
    if let Some(normalized) =
        crate::exact_value::rational_from_answer(answer).and_then(exact_rational_into_answer)
    {
        return normalized;
    }

    match answer {
        AnswerNode::Empty => AnswerNode::Empty,
        AnswerNode::Integer(value) => AnswerNode::Integer(*value),
        AnswerNode::ExactDecimal { coefficient, scale } => {
            // This fallback is reached only when an external scale is too
            // large for the bounded exact conversion. Still remove decimal
            // trailing zeroes without using Float. A zero coefficient is
            // canonical zero immediately; otherwise the loop is bounded by
            // the coefficient's at-most-19 decimal digits, not by `scale`.
            let mut coefficient = *coefficient;
            let mut scale = *scale;
            if coefficient == 0 {
                return AnswerNode::Integer(0);
            }
            while scale > 0 && coefficient % 10 == 0 {
                coefficient /= 10;
                scale -= 1;
            }
            if scale == 0 {
                AnswerNode::Integer(coefficient)
            } else {
                AnswerNode::ExactDecimal { coefficient, scale }
            }
        }
        AnswerNode::NanError(raw) => AnswerNode::NanError(raw.clone()),
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => AnswerNode::Fraction {
            numerator: Box::new(normalize_answer_bounded(numerator)),
            denominator: Box::new(normalize_answer_bounded(denominator)),
        },
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => AnswerNode::MixedFraction {
            whole: Box::new(normalize_answer_bounded(whole)),
            numerator: Box::new(normalize_answer_bounded(numerator)),
            denominator: Box::new(normalize_answer_bounded(denominator)),
        },
        AnswerNode::Root { radicand, index } => AnswerNode::Root {
            radicand: Box::new(normalize_answer_bounded(radicand)),
            index: index.as_deref().map(normalize_answer_bounded).map(Box::new),
        },
        AnswerNode::Negative(value) => {
            let normalized = normalize_answer_bounded(value);
            match &normalized {
                AnswerNode::Integer(value) => value.checked_neg().map_or_else(
                    || AnswerNode::Negative(Box::new(AnswerNode::Integer(*value))),
                    AnswerNode::Integer,
                ),
                AnswerNode::Negative(inner) => inner.as_ref().clone(),
                _ => AnswerNode::Negative(Box::new(normalized)),
            }
        }
        AnswerNode::PlusMinus(value) => {
            let normalized = normalize_answer_bounded(value);
            match &normalized {
                AnswerNode::PlusMinus(inner) => {
                    AnswerNode::PlusMinus(Box::new(inner.as_ref().clone()))
                }
                _ => AnswerNode::PlusMinus(Box::new(normalized)),
            }
        }
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = normalize_answer_bounded(left);
            let right = normalize_answer_bounded(right);
            match (operator, &left, &right) {
                (AnswerBinaryOperator::Add, AnswerNode::Integer(0), _) => right,
                (AnswerBinaryOperator::Add, _, AnswerNode::Integer(0)) => left,
                (AnswerBinaryOperator::Subtract, _, AnswerNode::Integer(0)) => left,
                (AnswerBinaryOperator::Multiply, AnswerNode::Integer(0), _)
                | (AnswerBinaryOperator::Multiply, _, AnswerNode::Integer(0)) => {
                    AnswerNode::Integer(0)
                }
                (AnswerBinaryOperator::Multiply, AnswerNode::Integer(1), _) => right,
                (AnswerBinaryOperator::Multiply, _, AnswerNode::Integer(1)) => left,
                _ => AnswerNode::Binary {
                    operator: *operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            }
        }
        AnswerNode::Tuple(values) => {
            AnswerNode::Tuple(values.iter().map(normalize_answer_bounded).collect())
        }
        AnswerNode::Variable(name) => AnswerNode::Variable(name.clone()),
    }
}
