use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::MAX_ANSWER_AST_SIZE;
#[cfg(feature = "wire-types")]
use ts_rs::TS;

// The display-size limit and the structural-node budget are separate input
// constraints even though they currently share the same numeric maximum.
const MAX_VALIDATED_AST_NODES: usize = MAX_ANSWER_AST_SIZE;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum AnswerBinaryOperator {
    Add,
    Subtract,
    Multiply,
}

/// Exact, typed answer syntax shared by editing, grading, and every generator.
/// Mathematical values deliberately contain no binary floating-point fields.
#[derive(Default)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(
    feature = "wire-types",
    ts(tag = "type", content = "value", rename_all = "snake_case")
)]
pub enum AnswerNode {
    #[default]
    Empty,
    Integer(#[cfg_attr(feature = "wire-types", ts(type = "string"))] i64),
    ExactDecimal {
        #[cfg_attr(feature = "wire-types", ts(type = "string"))]
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

thread_local! {
    static ANSWER_DESERIALIZE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct AnswerDeserializeDepthGuard;

impl Drop for AnswerDeserializeDepthGuard {
    fn drop(&mut self) {
        ANSWER_DESERIALIZE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum RawAnswerNode {
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

impl RawAnswerNode {
    fn into_answer(self) -> AnswerNode {
        match self {
            Self::Empty => AnswerNode::Empty,
            Self::Integer(value) => AnswerNode::Integer(value),
            Self::ExactDecimal { coefficient, scale } => {
                AnswerNode::ExactDecimal { coefficient, scale }
            }
            Self::NanError(raw) => AnswerNode::NanError(raw),
            Self::Fraction {
                numerator,
                denominator,
            } => AnswerNode::Fraction {
                numerator,
                denominator,
            },
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => AnswerNode::MixedFraction {
                whole,
                numerator,
                denominator,
            },
            Self::Root { radicand, index } => AnswerNode::Root { radicand, index },
            Self::Negative(value) => AnswerNode::Negative(value),
            Self::PlusMinus(value) => AnswerNode::PlusMinus(value),
            Self::Binary {
                operator,
                left,
                right,
            } => AnswerNode::Binary {
                operator,
                left,
                right,
            },
            Self::Tuple(values) => AnswerNode::Tuple(values),
            Self::Variable(name) => AnswerNode::Variable(name),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum AnswerNodeRef<'a> {
    Empty,
    Integer(#[serde(with = "crate::exact::i64_decimal_string")] i64),
    ExactDecimal {
        #[serde(with = "crate::exact::i64_decimal_string")]
        coefficient: i64,
        scale: u32,
    },
    NanError(&'a str),
    Fraction {
        numerator: &'a AnswerNode,
        denominator: &'a AnswerNode,
    },
    MixedFraction {
        whole: &'a AnswerNode,
        numerator: &'a AnswerNode,
        denominator: &'a AnswerNode,
    },
    Root {
        radicand: &'a AnswerNode,
        index: Option<&'a AnswerNode>,
    },
    Negative(&'a AnswerNode),
    PlusMinus(&'a AnswerNode),
    Binary {
        operator: AnswerBinaryOperator,
        left: &'a AnswerNode,
        right: &'a AnswerNode,
    },
    Tuple(&'a [AnswerNode]),
    Variable(&'a str),
}

impl<'a> From<&'a AnswerNode> for AnswerNodeRef<'a> {
    fn from(value: &'a AnswerNode) -> Self {
        match value {
            AnswerNode::Empty => Self::Empty,
            AnswerNode::Integer(value) => Self::Integer(*value),
            AnswerNode::ExactDecimal { coefficient, scale } => Self::ExactDecimal {
                coefficient: *coefficient,
                scale: *scale,
            },
            AnswerNode::NanError(raw) => Self::NanError(raw),
            AnswerNode::Fraction {
                numerator,
                denominator,
            } => Self::Fraction {
                numerator,
                denominator,
            },
            AnswerNode::MixedFraction {
                whole,
                numerator,
                denominator,
            } => Self::MixedFraction {
                whole,
                numerator,
                denominator,
            },
            AnswerNode::Root { radicand, index } => Self::Root {
                radicand,
                index: index.as_deref(),
            },
            AnswerNode::Negative(value) => Self::Negative(value),
            AnswerNode::PlusMinus(value) => Self::PlusMinus(value),
            AnswerNode::Binary {
                operator,
                left,
                right,
            } => Self::Binary {
                operator: *operator,
                left,
                right,
            },
            AnswerNode::Tuple(values) => Self::Tuple(values),
            AnswerNode::Variable(name) => Self::Variable(name),
        }
    }
}

impl Serialize for AnswerNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !self.is_within_structural_node_limit() {
            return Err(S::Error::custom("answer AST exceeds structural node limit"));
        }
        AnswerNodeRef::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AnswerNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entered = ANSWER_DESERIALIZE_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= MAX_VALIDATED_AST_NODES {
                false
            } else {
                depth.set(current + 1);
                true
            }
        });
        if !entered {
            return Err(D::Error::custom(
                "answer AST exceeds structural depth limit",
            ));
        }
        let _guard = AnswerDeserializeDepthGuard;
        let answer = RawAnswerNode::deserialize(deserializer)?.into_answer();
        if !answer.is_within_structural_node_limit() {
            return Err(D::Error::custom("answer AST exceeds structural node limit"));
        }
        Ok(answer)
    }
}

impl Clone for AnswerNode {
    fn clone(&self) -> Self {
        enum Task<'a> {
            Visit(&'a AnswerNode),
            Fraction,
            MixedFraction,
            Root(bool),
            Negative,
            PlusMinus,
            Binary(AnswerBinaryOperator),
            Tuple(usize),
        }

        let mut tasks = vec![Task::Visit(self)];
        let mut values = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(node) => match node {
                    Self::Empty => values.push(Self::Empty),
                    Self::Integer(value) => values.push(Self::Integer(*value)),
                    Self::ExactDecimal { coefficient, scale } => values.push(Self::ExactDecimal {
                        coefficient: *coefficient,
                        scale: *scale,
                    }),
                    Self::NanError(raw) => values.push(Self::NanError(raw.clone())),
                    Self::Fraction {
                        numerator,
                        denominator,
                    } => {
                        tasks.push(Task::Fraction);
                        tasks.push(Task::Visit(denominator));
                        tasks.push(Task::Visit(numerator));
                    }
                    Self::MixedFraction {
                        whole,
                        numerator,
                        denominator,
                    } => {
                        tasks.push(Task::MixedFraction);
                        tasks.push(Task::Visit(denominator));
                        tasks.push(Task::Visit(numerator));
                        tasks.push(Task::Visit(whole));
                    }
                    Self::Root { radicand, index } => {
                        tasks.push(Task::Root(index.is_some()));
                        if let Some(index) = index.as_deref() {
                            tasks.push(Task::Visit(index));
                        }
                        tasks.push(Task::Visit(radicand));
                    }
                    Self::Negative(value) => {
                        tasks.push(Task::Negative);
                        tasks.push(Task::Visit(value));
                    }
                    Self::PlusMinus(value) => {
                        tasks.push(Task::PlusMinus);
                        tasks.push(Task::Visit(value));
                    }
                    Self::Binary {
                        operator,
                        left,
                        right,
                    } => {
                        tasks.push(Task::Binary(*operator));
                        tasks.push(Task::Visit(right));
                        tasks.push(Task::Visit(left));
                    }
                    Self::Tuple(items) => {
                        tasks.push(Task::Tuple(items.len()));
                        for item in items.iter().rev() {
                            tasks.push(Task::Visit(item));
                        }
                    }
                    Self::Variable(name) => values.push(Self::Variable(name.clone())),
                },
                Task::Fraction => {
                    let denominator = values
                        .pop()
                        .expect("clone task stack must contain denominator");
                    let numerator = values
                        .pop()
                        .expect("clone task stack must contain numerator");
                    values.push(Self::Fraction {
                        numerator: Box::new(numerator),
                        denominator: Box::new(denominator),
                    });
                }
                Task::MixedFraction => {
                    let denominator = values
                        .pop()
                        .expect("clone task stack must contain denominator");
                    let numerator = values
                        .pop()
                        .expect("clone task stack must contain numerator");
                    let whole = values.pop().expect("clone task stack must contain whole");
                    values.push(Self::MixedFraction {
                        whole: Box::new(whole),
                        numerator: Box::new(numerator),
                        denominator: Box::new(denominator),
                    });
                }
                Task::Root(has_index) => {
                    let index = has_index.then(|| {
                        Box::new(
                            values
                                .pop()
                                .expect("clone task stack must contain root index"),
                        )
                    });
                    let radicand = values
                        .pop()
                        .expect("clone task stack must contain radicand");
                    values.push(Self::Root {
                        radicand: Box::new(radicand),
                        index,
                    });
                }
                Task::Negative => {
                    let value = values
                        .pop()
                        .expect("clone task stack must contain unary value");
                    values.push(Self::Negative(Box::new(value)));
                }
                Task::PlusMinus => {
                    let value = values
                        .pop()
                        .expect("clone task stack must contain unary value");
                    values.push(Self::PlusMinus(Box::new(value)));
                }
                Task::Binary(operator) => {
                    let right = values
                        .pop()
                        .expect("clone task stack must contain right value");
                    let left = values
                        .pop()
                        .expect("clone task stack must contain left value");
                    values.push(Self::Binary {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    });
                }
                Task::Tuple(length) => {
                    let start = values
                        .len()
                        .checked_sub(length)
                        .expect("clone task stack must contain tuple values");
                    let tuple_values = values.split_off(start);
                    values.push(Self::Tuple(tuple_values));
                }
            }
        }
        values
            .pop()
            .expect("clone task stack must produce one root")
    }
}

impl PartialEq for AnswerNode {
    fn eq(&self, other: &Self) -> bool {
        let mut stack = vec![(self, other)];
        while let Some((left, right)) = stack.pop() {
            match (left, right) {
                (Self::Empty, Self::Empty) => {}
                (Self::Integer(left), Self::Integer(right)) if left == right => {}
                (
                    Self::ExactDecimal {
                        coefficient: lc,
                        scale: ls,
                    },
                    Self::ExactDecimal {
                        coefficient: rc,
                        scale: rs,
                    },
                ) if lc == rc && ls == rs => {}
                (Self::NanError(left), Self::NanError(right)) if left == right => {}
                (
                    Self::Fraction {
                        numerator: ln,
                        denominator: ld,
                    },
                    Self::Fraction {
                        numerator: rn,
                        denominator: rd,
                    },
                ) => {
                    stack.push((ld, rd));
                    stack.push((ln, rn));
                }
                (
                    Self::MixedFraction {
                        whole: lw,
                        numerator: ln,
                        denominator: ld,
                    },
                    Self::MixedFraction {
                        whole: rw,
                        numerator: rn,
                        denominator: rd,
                    },
                ) => {
                    stack.push((ld, rd));
                    stack.push((ln, rn));
                    stack.push((lw, rw));
                }
                (
                    Self::Root {
                        radicand: lr,
                        index: li,
                    },
                    Self::Root {
                        radicand: rr,
                        index: ri,
                    },
                ) => {
                    match (li.as_deref(), ri.as_deref()) {
                        (None, None) => {}
                        (Some(left), Some(right)) => stack.push((left, right)),
                        _ => return false,
                    }
                    stack.push((lr, rr));
                }
                (Self::Negative(left), Self::Negative(right))
                | (Self::PlusMinus(left), Self::PlusMinus(right)) => stack.push((left, right)),
                (
                    Self::Binary {
                        operator: lo,
                        left: ll,
                        right: lr,
                    },
                    Self::Binary {
                        operator: ro,
                        left: rl,
                        right: rr,
                    },
                ) if lo == ro => {
                    stack.push((lr, rr));
                    stack.push((ll, rl));
                }
                (Self::Tuple(left), Self::Tuple(right)) if left.len() == right.len() => {
                    stack.extend(left.iter().zip(right).rev());
                }
                (Self::Variable(left), Self::Variable(right)) if left == right => {}
                _ => return false,
            }
        }
        true
    }
}

impl Eq for AnswerNode {}

impl AnswerNode {
    const fn variant_rank(&self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Integer(_) => 1,
            Self::ExactDecimal { .. } => 2,
            Self::NanError(_) => 3,
            Self::Fraction { .. } => 4,
            Self::MixedFraction { .. } => 5,
            Self::Root { .. } => 6,
            Self::Negative(_) => 7,
            Self::PlusMinus(_) => 8,
            Self::Binary { .. } => 9,
            Self::Tuple(_) => 10,
            Self::Variable(_) => 11,
        }
    }
}

impl Ord for AnswerNode {
    fn cmp(&self, other: &Self) -> Ordering {
        enum Task<'a> {
            Node(&'a AnswerNode, &'a AnswerNode),
            Length(usize, usize),
        }
        let mut stack = vec![Task::Node(self, other)];
        while let Some(task) = stack.pop() {
            match task {
                Task::Length(left, right) => {
                    let ordering = left.cmp(&right);
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
                Task::Node(left, right) => {
                    let rank = left.variant_rank().cmp(&right.variant_rank());
                    if rank != Ordering::Equal {
                        return rank;
                    }
                    match (left, right) {
                        (Self::Empty, Self::Empty) => {}
                        (Self::Integer(left), Self::Integer(right)) => {
                            let ordering = left.cmp(right);
                            if ordering != Ordering::Equal {
                                return ordering;
                            }
                        }
                        (
                            Self::ExactDecimal {
                                coefficient: lc,
                                scale: ls,
                            },
                            Self::ExactDecimal {
                                coefficient: rc,
                                scale: rs,
                            },
                        ) => {
                            let ordering = lc.cmp(rc).then_with(|| ls.cmp(rs));
                            if ordering != Ordering::Equal {
                                return ordering;
                            }
                        }
                        (Self::NanError(left), Self::NanError(right))
                        | (Self::Variable(left), Self::Variable(right)) => {
                            let ordering = left.cmp(right);
                            if ordering != Ordering::Equal {
                                return ordering;
                            }
                        }
                        (
                            Self::Fraction {
                                numerator: ln,
                                denominator: ld,
                            },
                            Self::Fraction {
                                numerator: rn,
                                denominator: rd,
                            },
                        ) => {
                            stack.push(Task::Node(ld, rd));
                            stack.push(Task::Node(ln, rn));
                        }
                        (
                            Self::MixedFraction {
                                whole: lw,
                                numerator: ln,
                                denominator: ld,
                            },
                            Self::MixedFraction {
                                whole: rw,
                                numerator: rn,
                                denominator: rd,
                            },
                        ) => {
                            stack.push(Task::Node(ld, rd));
                            stack.push(Task::Node(ln, rn));
                            stack.push(Task::Node(lw, rw));
                        }
                        (
                            Self::Root {
                                radicand: lr,
                                index: li,
                            },
                            Self::Root {
                                radicand: rr,
                                index: ri,
                            },
                        ) => {
                            match (li.as_deref(), ri.as_deref()) {
                                (None, None) => {}
                                (None, Some(_)) => {
                                    stack.push(Task::Length(0, 1));
                                    stack.push(Task::Node(lr, rr));
                                    continue;
                                }
                                (Some(_), None) => {
                                    stack.push(Task::Length(1, 0));
                                    stack.push(Task::Node(lr, rr));
                                    continue;
                                }
                                (Some(left), Some(right)) => stack.push(Task::Node(left, right)),
                            }
                            stack.push(Task::Node(lr, rr));
                        }
                        (Self::Negative(left), Self::Negative(right))
                        | (Self::PlusMinus(left), Self::PlusMinus(right)) => {
                            stack.push(Task::Node(left, right));
                        }
                        (
                            Self::Binary {
                                operator: lo,
                                left: ll,
                                right: lr,
                            },
                            Self::Binary {
                                operator: ro,
                                left: rl,
                                right: rr,
                            },
                        ) => {
                            let ordering = lo.cmp(ro);
                            if ordering != Ordering::Equal {
                                return ordering;
                            }
                            stack.push(Task::Node(lr, rr));
                            stack.push(Task::Node(ll, rl));
                        }
                        (Self::Tuple(left), Self::Tuple(right)) => {
                            stack.push(Task::Length(left.len(), right.len()));
                            for (left, right) in left.iter().zip(right).rev() {
                                stack.push(Task::Node(left, right));
                            }
                        }
                        _ => unreachable!("equal variant ranks must have matching variants"),
                    }
                }
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for AnswerNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for AnswerNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_within_structural_node_limit() {
            return formatter.write_str("AnswerNode(<structural-limit-exceeded>)");
        }
        match self {
            Self::Empty => formatter.write_str("Empty"),
            Self::Integer(value) => formatter.debug_tuple("Integer").field(value).finish(),
            Self::ExactDecimal { coefficient, scale } => formatter
                .debug_struct("ExactDecimal")
                .field("coefficient", coefficient)
                .field("scale", scale)
                .finish(),
            Self::NanError(raw) => formatter.debug_tuple("NanError").field(raw).finish(),
            Self::Fraction {
                numerator,
                denominator,
            } => formatter
                .debug_struct("Fraction")
                .field("numerator", numerator)
                .field("denominator", denominator)
                .finish(),
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => formatter
                .debug_struct("MixedFraction")
                .field("whole", whole)
                .field("numerator", numerator)
                .field("denominator", denominator)
                .finish(),
            Self::Root { radicand, index } => formatter
                .debug_struct("Root")
                .field("radicand", radicand)
                .field("index", index)
                .finish(),
            Self::Negative(value) => formatter.debug_tuple("Negative").field(value).finish(),
            Self::PlusMinus(value) => formatter.debug_tuple("PlusMinus").field(value).finish(),
            Self::Binary {
                operator,
                left,
                right,
            } => formatter
                .debug_struct("Binary")
                .field("operator", operator)
                .field("left", left)
                .field("right", right)
                .finish(),
            Self::Tuple(values) => formatter.debug_tuple("Tuple").field(values).finish(),
            Self::Variable(name) => formatter.debug_tuple("Variable").field(name).finish(),
        }
    }
}

impl Drop for AnswerNode {
    fn drop(&mut self) {
        // A public recursive enum can be constructed deeper than the interactive
        // AST budget by native callers. Dismantle descendants iteratively so
        // ordinary scope exit never recursively drops a hostile tree.
        let mut pending = Vec::new();
        self.take_children_for_drop(&mut pending);
        while let Some(mut node) = pending.pop() {
            node.take_children_for_drop(&mut pending);
            // `node` now contains only shallow/empty children, so its own Drop
            // invocation at the end of this iteration is constant-stack.
        }
    }
}

impl AnswerNode {
    fn take_children_for_drop(&mut self, pending: &mut Vec<AnswerNode>) {
        match self {
            Self::Fraction {
                numerator,
                denominator,
            } => {
                pending.push(std::mem::replace(numerator.as_mut(), Self::Empty));
                pending.push(std::mem::replace(denominator.as_mut(), Self::Empty));
            }
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => {
                pending.push(std::mem::replace(whole.as_mut(), Self::Empty));
                pending.push(std::mem::replace(numerator.as_mut(), Self::Empty));
                pending.push(std::mem::replace(denominator.as_mut(), Self::Empty));
            }
            Self::Root { radicand, index } => {
                pending.push(std::mem::replace(radicand.as_mut(), Self::Empty));
                if let Some(index) = index.as_deref_mut() {
                    pending.push(std::mem::replace(index, Self::Empty));
                }
            }
            Self::Negative(value) | Self::PlusMinus(value) => {
                pending.push(std::mem::replace(value.as_mut(), Self::Empty));
            }
            Self::Binary { left, right, .. } => {
                pending.push(std::mem::replace(left.as_mut(), Self::Empty));
                pending.push(std::mem::replace(right.as_mut(), Self::Empty));
            }
            Self::Tuple(values) => pending.extend(std::mem::take(values)),
            Self::Empty
            | Self::Integer(_)
            | Self::ExactDecimal { .. }
            | Self::NanError(_)
            | Self::Variable(_) => {}
        }
    }

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

    /// Generated canonical answers may contain exact mathematical structure, but
    /// never editor-only placeholders, parse errors, or free-form variables.
    pub(crate) fn is_generated_answer(&self) -> bool {
        match self {
            Self::Empty | Self::NanError(_) | Self::Variable(_) => false,
            Self::Integer(_) | Self::ExactDecimal { .. } => true,
            Self::Fraction {
                numerator,
                denominator,
            } => numerator.is_generated_answer() && denominator.is_generated_answer(),
            Self::MixedFraction {
                whole,
                numerator,
                denominator,
            } => {
                whole.is_generated_answer()
                    && numerator.is_generated_answer()
                    && denominator.is_generated_answer()
            }
            Self::Root { radicand, index } => {
                radicand.is_generated_answer()
                    && index.as_deref().is_none_or(Self::is_generated_answer)
            }
            Self::Negative(value) | Self::PlusMinus(value) => value.is_generated_answer(),
            Self::Binary { left, right, .. } => {
                left.is_generated_answer() && right.is_generated_answer()
            }
            Self::Tuple(values) => values.iter().all(Self::is_generated_answer),
        }
    }

    /// Leaf integers count decimal digits. Composite nodes count one for the
    /// parent plus their children, so `12/42` has size `1 + 2 + 2 = 5`.
    /// Traversal is iterative so even an unvalidated external tree cannot
    /// overflow the native call stack merely by asking for its size.
    pub fn size(&self) -> usize {
        let mut total = 0_usize;
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            match node {
                Self::Empty => {}
                Self::Integer(value) => {
                    total = total.saturating_add(decimal_digit_count(value.unsigned_abs()));
                }
                Self::ExactDecimal { coefficient, scale } => {
                    total = total.saturating_add(
                        decimal_digit_count(coefficient.unsigned_abs())
                            .max((*scale as usize).saturating_add(1)),
                    );
                }
                Self::NanError(raw) => {
                    total = total.saturating_add(raw.chars().count());
                }
                Self::Fraction {
                    numerator,
                    denominator,
                } => {
                    total = total.saturating_add(1);
                    stack.push(denominator);
                    stack.push(numerator);
                }
                Self::MixedFraction {
                    whole,
                    numerator,
                    denominator,
                } => {
                    total = total.saturating_add(1);
                    stack.push(denominator);
                    stack.push(numerator);
                    stack.push(whole);
                }
                Self::Root { radicand, index } => {
                    total = total.saturating_add(1);
                    if let Some(index) = index.as_deref() {
                        stack.push(index);
                    }
                    stack.push(radicand);
                }
                Self::Negative(value) | Self::PlusMinus(value) => {
                    total = total.saturating_add(1);
                    stack.push(value);
                }
                Self::Binary { left, right, .. } => {
                    total = total.saturating_add(1);
                    stack.push(right);
                    stack.push(left);
                }
                Self::Tuple(values) => {
                    total = total.saturating_add(1);
                    stack.extend(values.iter().rev());
                }
                Self::Variable(name) => {
                    total = total.saturating_add(name.chars().count().max(1));
                }
            }
        }
        total
    }

    /// Check only the recursive structural-node budget. Semantic APIs use this
    /// guard before recursion; display-size limits remain an interactive-input
    /// concern and must not reject shallow exact values such as `i64::MIN`.
    pub(crate) fn is_within_structural_node_limit(&self) -> bool {
        let mut visited = 0_usize;
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            visited += 1;
            if visited > MAX_VALIDATED_AST_NODES {
                return false;
            }
            match node {
                Self::Fraction {
                    numerator,
                    denominator,
                } => {
                    stack.push(denominator);
                    stack.push(numerator);
                }
                Self::MixedFraction {
                    whole,
                    numerator,
                    denominator,
                } => {
                    stack.push(denominator);
                    stack.push(numerator);
                    stack.push(whole);
                }
                Self::Root { radicand, index } => {
                    if let Some(index) = index.as_deref() {
                        stack.push(index);
                    }
                    stack.push(radicand);
                }
                Self::Negative(value) | Self::PlusMinus(value) => stack.push(value),
                Self::Binary { left, right, .. } => {
                    stack.push(right);
                    stack.push(left);
                }
                Self::Tuple(values) => stack.extend(values.iter().rev()),
                Self::Empty
                | Self::Integer(_)
                | Self::ExactDecimal { .. }
                | Self::NanError(_)
                | Self::Variable(_) => {}
            }
        }
        true
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

    /// Exact integer digits used by BigNum; never reconstructs a magnitude
    /// from a display float. This is crate-internal effort support and uses an
    /// iterative traversal so callers cannot inherit recursive stack risk.
    pub(crate) fn exact_integer_magnitudes(&self, output: &mut Vec<u64>) {
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            match node {
                Self::Empty | Self::NanError(_) | Self::Variable(_) => {}
                Self::Integer(value) => output.push(value.unsigned_abs()),
                Self::ExactDecimal { coefficient, .. } => output.push(coefficient.unsigned_abs()),
                Self::Fraction {
                    numerator,
                    denominator,
                } => {
                    stack.push(denominator);
                    stack.push(numerator);
                }
                Self::MixedFraction {
                    whole,
                    numerator,
                    denominator,
                } => {
                    stack.push(denominator);
                    stack.push(numerator);
                    stack.push(whole);
                }
                Self::Root { radicand, index } => {
                    if let Some(index) = index.as_deref() {
                        stack.push(index);
                    }
                    stack.push(radicand);
                }
                Self::Negative(value) | Self::PlusMinus(value) => stack.push(value),
                Self::Binary { left, right, .. } => {
                    stack.push(right);
                    stack.push(left);
                }
                Self::Tuple(values) => stack.extend(values.iter().rev()),
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
