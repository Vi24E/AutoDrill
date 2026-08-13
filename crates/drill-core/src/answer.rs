use serde::{Deserialize, Serialize};

use crate::model::MAX_ANSWER_AST_SIZE;

// The display-size limit and the structural-node budget are separate input
// constraints even though they currently share the same numeric maximum.
const MAX_VALIDATED_AST_NODES: usize = MAX_ANSWER_AST_SIZE;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerBinaryOperator {
    Add,
    Subtract,
    Multiply,
}

/// Exact, typed answer syntax shared by editing, grading, and every generator.
/// Mathematical values deliberately contain no binary floating-point fields.
#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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
    NanError(String),
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
    Binary {
        operator: AnswerBinaryOperator,
        left: Box<AnswerNode>,
        right: Box<AnswerNode>,
    },
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
        self.size_capped(usize::MAX)
    }

    /// Check the input AST limits without traversing more than 19 nodes.
    /// Display size keeps its documented rules, while structural nodes are
    /// budgeted separately so Empty children still consume validation budget.
    /// Exact-decimal scale is treated as a claimed size, never as a loop or
    /// allocation count.
    pub fn is_within_size_limit(&self) -> bool {
        let mut visited_nodes = 0;
        self.bounded_input_size(&mut visited_nodes, MAX_ANSWER_AST_SIZE)
            .is_some()
    }

    fn bounded_input_size(
        &self,
        visited_nodes: &mut usize,
        display_remaining: usize,
    ) -> Option<usize> {
        *visited_nodes = visited_nodes.saturating_add(1);
        if *visited_nodes > MAX_VALIDATED_AST_NODES {
            return None;
        }

        let display_size = match self {
            Self::Empty => 0,
            Self::Integer(value) => decimal_digit_count(value.unsigned_abs()),
            Self::ExactDecimal { coefficient, scale } => {
                decimal_digit_count(coefficient.unsigned_abs())
                    .max((*scale as usize).saturating_add(1))
            }
            Self::NanError(raw) => {
                let count = raw
                    .chars()
                    .take(display_remaining.saturating_add(1))
                    .count();
                if count > display_remaining {
                    return None;
                }
                count
            }
            Self::Fraction {
                numerator,
                denominator,
            } => {
                let mut remaining = display_remaining.checked_sub(1)?;
                let numerator_size = numerator.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(numerator_size)?;
                let denominator_size = denominator.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(denominator_size)?;
                display_remaining - remaining
            }
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => {
                let mut remaining = display_remaining.checked_sub(1)?;
                let whole_size = whole.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(whole_size)?;
                let numerator_size = numerator.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(numerator_size)?;
                let denominator_size = denominator.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(denominator_size)?;
                display_remaining - remaining
            }
            Self::Root { radicand, index } => {
                let mut remaining = display_remaining.checked_sub(1)?;
                let radicand_size = radicand.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(radicand_size)?;
                if let Some(index) = index {
                    let index_size = index.bounded_input_size(visited_nodes, remaining)?;
                    remaining = remaining.checked_sub(index_size)?;
                }
                display_remaining - remaining
            }
            Self::Negative(value) | Self::PlusMinus(value) => {
                let remaining = display_remaining.checked_sub(1)?;
                let child_size = value.bounded_input_size(visited_nodes, remaining)?;
                display_remaining - remaining.checked_sub(child_size)?
            }
            Self::Binary { left, right, .. } => {
                let mut remaining = display_remaining.checked_sub(1)?;
                let left_size = left.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(left_size)?;
                let right_size = right.bounded_input_size(visited_nodes, remaining)?;
                remaining = remaining.checked_sub(right_size)?;
                display_remaining - remaining
            }
            Self::Tuple(values) => {
                let mut remaining = display_remaining.checked_sub(1)?;
                for value in values {
                    let child_size = value.bounded_input_size(visited_nodes, remaining)?;
                    remaining = remaining.checked_sub(child_size)?;
                }
                display_remaining - remaining
            }
            Self::Variable(name) => {
                let count = name
                    .chars()
                    .take(display_remaining.saturating_add(1))
                    .count();
                if count > display_remaining {
                    return None;
                }
                count.max(1)
            }
        };

        if display_size <= display_remaining {
            Some(display_size)
        } else {
            None
        }
    }

    fn size_capped(&self, cap: usize) -> usize {
        if cap == 0 {
            return 0;
        }
        match self {
            Self::Empty => 0,
            Self::Integer(value) => decimal_digit_count(value.unsigned_abs()).min(cap),
            Self::ExactDecimal { coefficient, scale } => {
                decimal_digit_count(coefficient.unsigned_abs())
                    .max((*scale as usize).saturating_add(1))
                    .min(cap)
            }
            Self::NanError(raw) => raw.chars().take(cap).count().min(cap),
            Self::Fraction {
                numerator,
                denominator,
            } => {
                let mut total = 1usize;
                total = total
                    .saturating_add(numerator.size_capped(cap - total))
                    .min(cap);
                if total == cap {
                    return total;
                }
                total
                    .saturating_add(denominator.size_capped(cap - total))
                    .min(cap)
            }
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => {
                let mut total = 1usize;
                total = total
                    .saturating_add(whole.size_capped(cap - total))
                    .min(cap);
                if total == cap {
                    return total;
                }
                total = total
                    .saturating_add(numerator.size_capped(cap - total))
                    .min(cap);
                if total == cap {
                    return total;
                }
                total
                    .saturating_add(denominator.size_capped(cap - total))
                    .min(cap)
            }
            Self::Root { radicand, index } => {
                let mut total = 1usize;
                total = total
                    .saturating_add(radicand.size_capped(cap - total))
                    .min(cap);
                if total == cap {
                    return total;
                }
                if let Some(index) = index {
                    total = total
                        .saturating_add(index.size_capped(cap - total))
                        .min(cap);
                }
                total
            }
            Self::Negative(value) | Self::PlusMinus(value) => {
                1usize.saturating_add(value.size_capped(cap - 1)).min(cap)
            }
            Self::Binary { left, right, .. } => {
                let mut total = 1usize;
                total = total.saturating_add(left.size_capped(cap - total)).min(cap);
                if total == cap {
                    return total;
                }
                total
                    .saturating_add(right.size_capped(cap - total))
                    .min(cap)
            }
            Self::Tuple(values) => {
                let mut total = 1usize;
                for value in values {
                    if total == cap {
                        return total;
                    }
                    total = total
                        .saturating_add(value.size_capped(cap - total))
                        .min(cap);
                }
                total
            }
            Self::Variable(name) => name.chars().take(cap).count().max(1).min(cap),
        }
    }

    /// Exact integer digits used by BigNum; never reconstructs a magnitude
    /// from a display float.
    pub fn exact_integer_magnitudes(&self, output: &mut Vec<u64>) {
        match self {
            Self::Empty | Self::NanError(_) | Self::Variable(_) => {}
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
            Self::Binary { left, right, .. } => {
                left.exact_integer_magnitudes(output);
                right.exact_integer_magnitudes(output);
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
