//! Neutral projection from answer syntax to exact rational values.
//!
//! Normalization and semantic validation both need to interpret rational AnswerNode
//! shapes. Keeping that traversal here avoids coupling semantics to normalization
//! while leaving arithmetic/canonicalization itself in `exact.rs`.

use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::exact::ExactRational;
use crate::model::RationalCoefficient;

pub(crate) fn rational_from_answer(answer: &AnswerNode) -> Option<ExactRational> {
    match answer {
        AnswerNode::Integer(value) => Some(ExactRational::from_integer(*value)),
        AnswerNode::ExactDecimal { coefficient, scale } => {
            ExactRational::new(i128::from(*coefficient), 10_i128.checked_pow(*scale)?)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => rational_from_answer(numerator)?.divide(rational_from_answer(denominator)?),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => rational_from_answer(whole)?
            .add(rational_from_answer(numerator)?.divide(rational_from_answer(denominator)?)?),
        AnswerNode::Negative(value) => rational_from_answer(value)?.negate(),
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = rational_from_answer(left)?;
            let right = rational_from_answer(right)?;
            match operator {
                AnswerBinaryOperator::Add => left.add(right),
                AnswerBinaryOperator::Subtract => left.subtract(right),
                AnswerBinaryOperator::Multiply => left.multiply(right),
            }
        }
        AnswerNode::Root {
            radicand,
            index: None,
        } => rational_from_answer(radicand)?.square_root(),
        AnswerNode::Empty
        | AnswerNode::NanError(_)
        | AnswerNode::Root { .. }
        | AnswerNode::PlusMinus(_)
        | AnswerNode::Tuple(_)
        | AnswerNode::Variable(_) => None,
    }
}

pub(crate) fn rational_parts_from_answer(answer: &AnswerNode) -> Option<(i128, i128)> {
    let value = rational_from_answer(answer)?;
    Some((value.numerator(), value.denominator()))
}

/// Project an exact numeric answer into the bounded coefficient type used by equation prompts.
/// The AnswerNode traversal and rational canonicalization remain owned by this neutral layer.
pub(crate) fn rational_coefficient_from_answer(answer: &AnswerNode) -> Option<RationalCoefficient> {
    let value = rational_from_answer(answer)?;
    RationalCoefficient::new(
        i64::try_from(value.numerator()).ok()?,
        i64::try_from(value.denominator()).ok()?,
    )
}
