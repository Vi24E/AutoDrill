use crate::answer::{AnswerBinaryOperator, AnswerNode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExactRational {
    numerator: i128,
    denominator: i128,
}

impl ExactRational {
    fn new(numerator: i128, denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let (numerator, denominator) = if denominator < 0 {
            (numerator.checked_neg()?, denominator.checked_neg()?)
        } else {
            (numerator, denominator)
        };
        if numerator == 0 {
            return Some(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Self::new(
            left.checked_add(right)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn subtract(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn divide(self, other: Self) -> Option<Self> {
        if other.numerator == 0 {
            return None;
        }
        Self::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }

    fn negate(self) -> Option<Self> {
        Self::new(self.numerator.checked_neg()?, self.denominator)
    }

    fn square_root(self) -> Option<Self> {
        if self.numerator < 0 {
            return None;
        }
        let numerator = exact_square_root(self.numerator as u128)?;
        let denominator = exact_square_root(self.denominator as u128)?;
        Self::new(
            i128::try_from(numerator).ok()?,
            i128::try_from(denominator).ok()?,
        )
    }

    fn into_answer(self) -> Option<AnswerNode> {
        let numerator = i64::try_from(self.numerator).ok()?;
        let denominator = i64::try_from(self.denominator).ok()?;
        if denominator == 1 {
            Some(AnswerNode::Integer(numerator))
        } else {
            Some(AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Integer(numerator)),
                denominator: Box::new(AnswerNode::Integer(denominator)),
            })
        }
    }
}

/// Return a canonical tree while preserving the caller's display tree outside
/// this function. Exact numeric nodes normalize to a reduced rational value;
/// normalization never passes mathematical values through binary float.
pub fn normalize_answer(answer: &AnswerNode) -> AnswerNode {
    if let Some(normalized) = exact_rational(answer).and_then(ExactRational::into_answer) {
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
            numerator: Box::new(normalize_answer(numerator)),
            denominator: Box::new(normalize_answer(denominator)),
        },
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => AnswerNode::MixedFraction {
            whole: Box::new(normalize_answer(whole)),
            numerator: Box::new(normalize_answer(numerator)),
            denominator: Box::new(normalize_answer(denominator)),
        },
        AnswerNode::Root { radicand, index } => AnswerNode::Root {
            radicand: Box::new(normalize_answer(radicand)),
            index: index.as_deref().map(normalize_answer).map(Box::new),
        },
        AnswerNode::Negative(value) => match normalize_answer(value) {
            AnswerNode::Integer(value) => value.checked_neg().map_or_else(
                || AnswerNode::Negative(Box::new(AnswerNode::Integer(value))),
                AnswerNode::Integer,
            ),
            AnswerNode::Negative(inner) => *inner,
            value => AnswerNode::Negative(Box::new(value)),
        },
        AnswerNode::PlusMinus(value) => match normalize_answer(value) {
            AnswerNode::PlusMinus(inner) => AnswerNode::PlusMinus(inner),
            value => AnswerNode::PlusMinus(Box::new(value)),
        },
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = normalize_answer(left);
            let right = normalize_answer(right);
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
            AnswerNode::Tuple(values.iter().map(normalize_answer).collect())
        }
        AnswerNode::Variable(name) => AnswerNode::Variable(name.clone()),
    }
}

fn exact_rational(answer: &AnswerNode) -> Option<ExactRational> {
    match answer {
        AnswerNode::Integer(value) => ExactRational::new(i128::from(*value), 1),
        AnswerNode::ExactDecimal { coefficient, scale } => {
            let denominator = 10_i128.checked_pow(*scale)?;
            ExactRational::new(i128::from(*coefficient), denominator)
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => exact_rational(numerator)?.divide(exact_rational(denominator)?),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => exact_rational(whole)?
            .add(exact_rational(numerator)?.divide(exact_rational(denominator)?)?),
        AnswerNode::Negative(value) => exact_rational(value)?.negate(),
        AnswerNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = exact_rational(left)?;
            let right = exact_rational(right)?;
            match operator {
                AnswerBinaryOperator::Add => left.add(right),
                AnswerBinaryOperator::Subtract => left.subtract(right),
                AnswerBinaryOperator::Multiply => left.multiply(right),
            }
        }
        AnswerNode::Root {
            radicand,
            index: None,
        } => exact_rational(radicand)?.square_root(),
        AnswerNode::Empty
        | AnswerNode::NanError(_)
        | AnswerNode::Root { .. }
        | AnswerNode::PlusMinus(_)
        | AnswerNode::Tuple(_)
        | AnswerNode::Variable(_) => None,
    }
}

fn exact_square_root(value: u128) -> Option<u128> {
    if value < 2 {
        return Some(value);
    }
    let mut x = value;
    let mut next = value / 2 + 1;
    while next < x {
        x = next;
        next = (x + value / x) / 2;
    }
    x.checked_mul(x)
        .filter(|square| *square == value)
        .map(|_| x)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
