use crate::answer::AnswerNode;
use crate::model::{ArithmeticExpression, ArithmeticOperator, RationalCoefficient};
use crate::rng::DeterministicRng;

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
    if value.denominator() == 1 {
        AnswerNode::Integer(value.numerator())
    } else {
        AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(value.numerator())),
            denominator: Box::new(AnswerNode::Integer(value.denominator())),
        }
    }
}

pub(crate) fn mixed_number_answer(value: RationalCoefficient) -> AnswerNode {
    if value.denominator() == 1 || value.numerator() < value.denominator() {
        return rational_answer(value);
    }
    let whole = value.numerator() / value.denominator();
    let numerator = value.numerator() % value.denominator();
    if numerator == 0 {
        AnswerNode::Integer(whole)
    } else {
        AnswerNode::MixedFraction {
            whole: Box::new(AnswerNode::Integer(whole)),
            numerator: Box::new(AnswerNode::Integer(numerator)),
            denominator: Box::new(AnswerNode::Integer(value.denominator())),
        }
    }
}

pub(crate) fn exact_decimal_rational(coefficient: i64, scale: u32) -> Option<RationalCoefficient> {
    let denominator = 10_i64.checked_pow(scale)?;
    RationalCoefficient::new(coefficient, denominator)
}

pub(crate) fn rational_less_than(left: RationalCoefficient, right: RationalCoefficient) -> bool {
    i128::from(left.numerator()) * i128::from(right.denominator())
        < i128::from(right.numerator()) * i128::from(left.denominator())
}

pub(crate) fn rational_to_exact_decimal_answer(
    value: RationalCoefficient,
    max_scale: u32,
) -> Option<AnswerNode> {
    if value.is_integer() {
        return Some(AnswerNode::Integer(value.numerator()));
    }
    let mut denominator = value.denominator();
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
        if power % value.denominator() == 0 {
            let coefficient = value.numerator().checked_mul(power / value.denominator())?;
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

pub(crate) fn draw_decimal_coefficient_with_significant_digits(
    rng: &mut DeterministicRng,
    significant_digits: u32,
) -> Option<i64> {
    if significant_digits == 0 {
        return None;
    }
    let lower = if significant_digits == 1 {
        1_i64
    } else {
        10_i64.checked_pow(significant_digits - 1)?
    };
    let upper = 10_i64.checked_pow(significant_digits)?.checked_sub(1)?;
    let width = upper.checked_sub(lower)?.checked_add(1)?;
    let width = u64::try_from(width).ok()?;
    loop {
        let offset = i64::try_from(rng.next_bounded(width)).ok()?;
        let candidate = lower.checked_add(offset)?;
        if candidate % 10 != 0 {
            break Some(candidate);
        }
    }
}

pub(crate) fn draw_decimal_coefficient(
    rng: &mut DeterministicRng,
    max_significant_digits: u32,
) -> Option<i64> {
    if max_significant_digits == 0 {
        return None;
    }
    let significant_digits = 1 + rng.next_bounded(u64::from(max_significant_digits)) as u32;
    draw_decimal_coefficient_with_significant_digits(rng, significant_digits)
}

pub(crate) fn draw_decimal_operand_with_significant_digits(
    rng: &mut DeterministicRng,
    significant_digits: u32,
    max_scale: u32,
) -> Option<(i64, u32)> {
    if max_scale == 0 {
        return None;
    }
    let coefficient = draw_decimal_coefficient_with_significant_digits(rng, significant_digits)?;
    let scale = 1 + rng.next_bounded(u64::from(max_scale)) as u32;
    Some((coefficient, scale))
}

pub(crate) fn draw_decimal_operand(
    rng: &mut DeterministicRng,
    max_significant_digits: u32,
    max_scale: u32,
) -> Option<(i64, u32)> {
    if max_scale == 0 {
        return None;
    }
    let coefficient = draw_decimal_coefficient(rng, max_significant_digits)?;
    let scale = 1 + rng.next_bounded(u64::from(max_scale)) as u32;
    Some((coefficient, scale))
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

pub(crate) fn draw_signed_integer(rng: &mut DeterministicRng, max_abs: i64) -> Option<i64> {
    let upper = u64::try_from(max_abs).ok().filter(|value| *value > 0)?;
    let magnitude = i64::try_from(1 + rng.next_bounded(upper)).ok()?;
    Some(if rng.next_bounded(2) == 0 {
        magnitude
    } else {
        -magnitude
    })
}

pub(crate) fn ensure_negative_term(rng: &mut DeterministicRng, values: &mut [i64]) -> Option<()> {
    if values.is_empty() {
        return None;
    }
    if values.iter().all(|value| *value > 0) {
        let index = rng.next_bounded(values.len() as u64) as usize;
        values[index] = -values[index];
    }
    Some(())
}

pub(crate) fn evaluate_expression(
    expression: &ArithmeticExpression,
) -> Option<RationalCoefficient> {
    crate::semantics::evaluate_expression(expression)
}

pub(crate) fn draw_bounded_rational_arithmetic_ast(
    rng: &mut DeterministicRng,
    values: &[i64],
) -> Option<ArithmeticExpression> {
    if values.is_empty() {
        return None;
    }
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
    (value.numerator().unsigned_abs() <= 729 && value.denominator() <= 81).then_some(expression)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_helpers_reject_empty_or_zero_bounds_without_reaching_rng_panics() {
        let mut rng = DeterministicRng::from_seed("helper-bounds");
        assert!(draw_decimal_coefficient(&mut rng, 0).is_none());
        assert!(draw_decimal_coefficient(&mut rng, u32::MAX).is_none());
        assert!(draw_decimal_operand(&mut rng, 0, 2).is_none());
        assert!(draw_decimal_operand(&mut rng, 2, 0).is_none());
        assert!(draw_decimal_coefficient_with_significant_digits(&mut rng, 0).is_none());
        assert!(draw_decimal_operand_with_significant_digits(&mut rng, 2, 0).is_none());
        for _ in 0..32 {
            let one_digit = draw_decimal_coefficient_with_significant_digits(&mut rng, 1).unwrap();
            let two_digit = draw_decimal_coefficient_with_significant_digits(&mut rng, 2).unwrap();
            assert!((1..=9).contains(&one_digit));
            assert!((10..=99).contains(&two_digit));
            assert_ne!(two_digit % 10, 0);
        }
        assert!(draw_signed_integer(&mut rng, 0).is_none());
        assert!(draw_signed_integer(&mut rng, -1).is_none());

        let mut empty = [];
        assert!(ensure_negative_term(&mut rng, &mut empty).is_none());
        assert!(draw_bounded_rational_arithmetic_ast(&mut rng, &[]).is_none());
    }
}
