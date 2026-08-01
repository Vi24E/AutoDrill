use serde::{Deserialize, Serialize};

use crate::model::MAX_ANSWER_AST_SIZE;

/// Exact, typed answer syntax shared by editing, grading, and every generator.
/// Mathematical values deliberately contain no binary floating-point fields.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AnswerNode {
    #[default]
    Empty,
    Integer(#[serde(with = "crate::exact::i64_decimal_string")] i64),
    ExactDecimal {
        #[serde(with = "crate::exact::i64_decimal_string")]
        coefficient: i64,
        scale: u32,
    },
    Fraction {
        numerator: Box<AnswerNode>,
        denominator: Box<AnswerNode>,
    },
    MixedFraction {
        whole: Box<AnswerNode>,
        numerator: Box<AnswerNode>,
        denominator: Box<AnswerNode>,
    },
    Root {
        radicand: Box<AnswerNode>,
        index: Option<Box<AnswerNode>>,
    },
    Negative(Box<AnswerNode>),
    PlusMinus(Box<AnswerNode>),
    Tuple(Vec<AnswerNode>),
    Variable(String),
}

impl AnswerNode {
    pub const fn empty() -> Self {
        Self::Empty
    }

    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    pub const fn exact_decimal(coefficient: i64, scale: u32) -> Self {
        Self::ExactDecimal { coefficient, scale }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Leaf integers count decimal digits. Composite nodes count one for the
    /// parent plus their children, so `12/42` has size `1 + 2 + 2 = 5`.
    pub fn size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Integer(value) => decimal_digit_count(value.unsigned_abs()),
            Self::ExactDecimal { coefficient, scale } => {
                decimal_digit_count(coefficient.unsigned_abs()).max(*scale as usize + 1)
            }
            Self::Fraction {
                numerator,
                denominator,
            } => 1 + numerator.size() + denominator.size(),
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => 1 + whole.size() + numerator.size() + denominator.size(),
            Self::Root { radicand, index } => {
                1 + radicand.size() + index.as_deref().map_or(0, Self::size)
            }
            Self::Negative(value) | Self::PlusMinus(value) => 1 + value.size(),
            Self::Tuple(values) => 1 + values.iter().map(Self::size).sum::<usize>(),
            Self::Variable(name) => name.chars().count().max(1),
        }
    }

    pub fn is_within_size_limit(&self) -> bool {
        self.size() <= MAX_ANSWER_AST_SIZE
    }

    /// Exact integer digits used by BigNum; never reconstructs a magnitude
    /// from a display float.
    pub fn exact_integer_magnitudes(&self, output: &mut Vec<u64>) {
        match self {
            Self::Empty | Self::Variable(_) => {}
            Self::Integer(value) => output.push(value.unsigned_abs()),
            Self::ExactDecimal { coefficient, .. } => output.push(coefficient.unsigned_abs()),
            Self::Fraction {
                numerator,
                denominator,
            } => {
                numerator.exact_integer_magnitudes(output);
                denominator.exact_integer_magnitudes(output);
            }
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => {
                whole.exact_integer_magnitudes(output);
                numerator.exact_integer_magnitudes(output);
                denominator.exact_integer_magnitudes(output);
            }
            Self::Root { radicand, index } => {
                radicand.exact_integer_magnitudes(output);
                if let Some(index) = index {
                    index.exact_integer_magnitudes(output);
                }
            }
            Self::Negative(value) | Self::PlusMinus(value) => {
                value.exact_integer_magnitudes(output);
            }
            Self::Tuple(values) => {
                for value in values {
                    value.exact_integer_magnitudes(output);
                }
            }
        }
    }
}

/// Keeps the user's display/input tree separate from the canonical tree used
/// for grading and mathematical comparison.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnswerRepresentation {
    pub display: AnswerNode,
    pub normalized: AnswerNode,
}

fn decimal_digit_count(mut value: u64) -> usize {
    if value == 0 {
        return 1;
    }
    let mut digits = 0;
    while value > 0 {
        value /= 10;
        digits += 1;
    }
    digits
}
