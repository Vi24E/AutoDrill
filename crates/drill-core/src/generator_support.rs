use crate::answer::AnswerNode;
use crate::model::{
    AnswerInputInterface, ArithmeticExpression, ArithmeticOperator, EditorStructure,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::theme::ThemeInputProfile;

pub(crate) fn input_interface(profile: ThemeInputProfile) -> AnswerInputInterface {
    let junior_high_full = || AnswerInputInterface::StructuredMath {
        allowed_structures: vec![
            EditorStructure::Fraction,
            EditorStructure::MixedFraction,
            EditorStructure::Decimal,
            EditorStructure::Root,
            EditorStructure::Negative,
            EditorStructure::PlusMinus,
            EditorStructure::Tuple,
            EditorStructure::Arithmetic,
        ],
    };
    match profile {
        ThemeInputProfile::SimplePositive => AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        },
        ThemeInputProfile::SimpleSigned => AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: true,
        },
        ThemeInputProfile::SimpleDecimal => AnswerInputInterface::SimpleNumeric {
            allow_decimal: true,
            allow_negative: false,
        },
        ThemeInputProfile::Fraction => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![
                EditorStructure::Fraction,
                EditorStructure::MixedFraction,
                EditorStructure::Decimal,
            ],
        },
        ThemeInputProfile::ImproperFraction => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Fraction, EditorStructure::Decimal],
        },
        ThemeInputProfile::SignedRational => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Fraction, EditorStructure::Negative],
        },
        ThemeInputProfile::LinearEquation => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![
                EditorStructure::Fraction,
                EditorStructure::MixedFraction,
                EditorStructure::Decimal,
                EditorStructure::Root,
                EditorStructure::Negative,
                EditorStructure::PlusMinus,
                EditorStructure::Tuple,
            ],
        },
        ThemeInputProfile::QuadraticEquation => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![
                EditorStructure::Fraction,
                EditorStructure::Root,
                EditorStructure::Negative,
                EditorStructure::PlusMinus,
                EditorStructure::Tuple,
                EditorStructure::Arithmetic,
            ],
        },
        ThemeInputProfile::SimultaneousEquation => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Negative, EditorStructure::Tuple],
        },
        ThemeInputProfile::JuniorHighFull => junior_high_full(),
        ThemeInputProfile::TupleOnly => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Tuple],
        },
    }
}

pub(crate) fn integer_expression(value: i64) -> ArithmeticExpression {
    ArithmeticExpression::Integer { value }
}

pub(crate) fn rational_expression(value: RationalCoefficient) -> ArithmeticExpression {
    ArithmeticExpression::Rational { value }
}

pub(crate) fn exact_decimal_expression(coefficient: i64, scale: u32) -> ArithmeticExpression {
    ArithmeticExpression::ExactDecimal { coefficient, scale }
}

pub(crate) fn binary_expression(
    operator: ArithmeticOperator,
    left: ArithmeticExpression,
    right: ArithmeticExpression,
) -> ArithmeticExpression {
    ArithmeticExpression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

pub(crate) fn rational_answer(value: RationalCoefficient) -> AnswerNode {
    if value.denominator == 1 {
        AnswerNode::Integer(value.numerator)
    } else {
        AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(value.numerator)),
            denominator: Box::new(AnswerNode::Integer(value.denominator)),
        }
    }
}

pub(crate) fn mixed_number_answer(value: RationalCoefficient) -> AnswerNode {
    if value.denominator == 1 || value.numerator < value.denominator {
        return rational_answer(value);
    }
    let whole = value.numerator / value.denominator;
    let numerator = value.numerator % value.denominator;
    if numerator == 0 {
        AnswerNode::Integer(whole)
    } else {
        AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(whole)),
            numerator: Box::new(AnswerNode::Integer(numerator)),
            denominator: Box::new(AnswerNode::Integer(value.denominator)),
        }
    }
}

pub(crate) fn exact_decimal_rational(coefficient: i64, scale: u32) -> Option<RationalCoefficient> {
    let denominator = 10_i64.checked_pow(scale)?;
    RationalCoefficient::new(coefficient, denominator)
}

pub(crate) fn rational_less_than(left: RationalCoefficient, right: RationalCoefficient) -> bool {
    i128::from(left.numerator) * i128::from(right.denominator)
        < i128::from(right.numerator) * i128::from(left.denominator)
}

pub(crate) fn rational_to_exact_decimal_answer(
    value: RationalCoefficient,
    max_scale: u32,
) -> Option<AnswerNode> {
    if value.is_integer() {
        return Some(AnswerNode::Integer(value.numerator));
    }
    let mut denominator = value.denominator;
    while denominator % 2 == 0 {
        denominator /= 2;
    }
    while denominator % 5 == 0 {
        denominator /= 5;
    }
    if denominator != 1 {
        return None;
    }
    for scale in 1..=max_scale {
        let power = 10_i64.checked_pow(scale)?;
        if power % value.denominator == 0 {
            let coefficient = value.numerator.checked_mul(power / value.denominator)?;
            return Some(AnswerNode::ExactDecimal { coefficient, scale });
        }
    }
    None
}

pub(crate) fn rational_to_arithmetic_expression(
    value: RationalCoefficient,
    max_scale: u32,
) -> Option<ArithmeticExpression> {
    match rational_to_exact_decimal_answer(value, max_scale)? {
        AnswerNode::Integer(value) => Some(integer_expression(value)),
        AnswerNode::ExactDecimal { coefficient, scale } => {
            Some(exact_decimal_expression(coefficient, scale))
        }
        _ => None,
    }
}

pub(crate) fn draw_decimal_coefficient(
    rng: &mut DeterministicRng,
    max_significant_digits: u32,
) -> i64 {
    debug_assert!(max_significant_digits >= 1);
    let significant_digits = 1 + rng.next_bounded(u64::from(max_significant_digits)) as u32;
    let lower = if significant_digits == 1 {
        1_i64
    } else {
        10_i64.pow(significant_digits - 1)
    };
    let upper = 10_i64.pow(significant_digits) - 1;
    loop {
        let candidate = lower + rng.next_bounded((upper - lower + 1) as u64) as i64;
        if candidate % 10 != 0 {
            break candidate;
        }
    }
}

pub(crate) fn draw_decimal_operand(
    rng: &mut DeterministicRng,
    max_significant_digits: u32,
    max_scale: u32,
) -> (i64, u32) {
    debug_assert!(max_scale >= 1);
    let coefficient = draw_decimal_coefficient(rng, max_significant_digits);
    let scale = 1 + rng.next_bounded(u64::from(max_scale)) as u32;
    (coefficient, scale)
}

pub(crate) fn arithmetic_leaf_significant_digits(
    expression: &ArithmeticExpression,
) -> Option<usize> {
    let magnitude = match expression {
        ArithmeticExpression::Integer { value } => value.unsigned_abs(),
        ArithmeticExpression::ExactDecimal { coefficient, .. } => coefficient.unsigned_abs(),
        _ => return None,
    };
    Some(magnitude.to_string().len())
}

pub(crate) fn arithmetic_leaf_column_grid_cells(
    expression: &ArithmeticExpression,
) -> Option<usize> {
    match expression {
        ArithmeticExpression::Integer { value } => Some(value.unsigned_abs().to_string().len()),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => Some(
            coefficient
                .unsigned_abs()
                .to_string()
                .len()
                .max(*scale as usize + 1),
        ),
        _ => None,
    }
}

pub(crate) fn draw_signed_integer(rng: &mut DeterministicRng, max_abs: i64) -> i64 {
    let magnitude = 1 + rng.next_bounded(max_abs as u64) as i64;
    if rng.next_bounded(2) == 0 {
        magnitude
    } else {
        -magnitude
    }
}

pub(crate) fn ensure_negative_term(rng: &mut DeterministicRng, values: &mut [i64]) {
    if values.iter().all(|value| *value > 0) {
        let index = rng.next_bounded(values.len() as u64) as usize;
        values[index] = -values[index];
    }
}

pub(crate) fn evaluate_expression(
    expression: &ArithmeticExpression,
) -> Option<RationalCoefficient> {
    match expression {
        ArithmeticExpression::Integer { value } => RationalCoefficient::new(*value, 1),
        ArithmeticExpression::Rational { value } => Some(*value),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => {
            exact_decimal_rational(*coefficient, *scale)
        }
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_expression(left)?;
            let right = evaluate_expression(right)?;
            match operator {
                ArithmeticOperator::Add => left.checked_add(right),
                ArithmeticOperator::Subtract => left.subtract(right),
                ArithmeticOperator::Multiply => left.multiply(right),
                ArithmeticOperator::Divide => left.divide(right),
            }
        }
    }
}

pub(crate) fn draw_bounded_rational_arithmetic_ast(
    rng: &mut DeterministicRng,
    values: &[i64],
) -> Option<ArithmeticExpression> {
    if values.len() == 1 {
        return Some(integer_expression(values[0]));
    }
    let split = 1 + rng.next_bounded((values.len() - 1) as u64) as usize;
    let left = draw_bounded_rational_arithmetic_ast(rng, &values[..split])?;
    let right = draw_bounded_rational_arithmetic_ast(rng, &values[split..])?;
    let operator = match rng.next_bounded(4) {
        0 => ArithmeticOperator::Add,
        1 => ArithmeticOperator::Subtract,
        2 => ArithmeticOperator::Multiply,
        _ => ArithmeticOperator::Divide,
    };
    let expression = binary_expression(operator, left, right);
    let value = evaluate_expression(&expression)?;
    (value.numerator.unsigned_abs() <= 729 && value.denominator <= 81).then_some(expression)
}
