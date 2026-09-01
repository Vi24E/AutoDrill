//! Exact mathematical semantics for generated problem validation.
//!
//! Generator code may choose how to construct a candidate, but this module is the
//! independent domain authority for deciding whether a canonical answer actually
//! solves the prompt. `Problem::generated` calls this boundary before a `Problem`
//! can enter the core aggregate.

use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::exact::ExactRational;
use crate::exact_value::rational_from_answer;
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, LiarStatement, MiniSudokuGrid,
    ProblemPrompt, QuadraticEquationForm, RationalCoefficient, MINI_SUDOKU_CELL_COUNT,
    MINI_SUDOKU_GRID_SPEC, MINI_SUDOKU_SIDE,
};
use crate::theme::ThemeAnswerContract;

fn rational_from_coefficient(value: RationalCoefficient) -> Option<ExactRational> {
    ExactRational::new(
        i128::from(value.numerator()),
        i128::from(value.denominator()),
    )
}

fn rational_from_expression(expression: &ArithmeticExpression) -> Option<ExactRational> {
    match expression {
        ArithmeticExpression::Integer { value } => Some(ExactRational::from_integer(*value)),
        ArithmeticExpression::Rational { value } => rational_from_coefficient(*value),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => {
            ExactRational::new(i128::from(*coefficient), 10_i128.checked_pow(*scale)?)
        }
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = rational_from_expression(left)?;
            let right = rational_from_expression(right)?;
            match operator {
                ArithmeticOperator::Add => left.add(right),
                ArithmeticOperator::Subtract => left.subtract(right),
                ArithmeticOperator::Multiply => left.multiply(right),
                ArithmeticOperator::Divide => left.divide(right),
            }
        }
    }
}

fn rational_to_coefficient(value: ExactRational) -> Option<RationalCoefficient> {
    RationalCoefficient::new(
        i64::try_from(value.numerator()).ok()?,
        i64::try_from(value.denominator()).ok()?,
    )
}

pub(crate) fn evaluate_expression(
    expression: &ArithmeticExpression,
) -> Option<RationalCoefficient> {
    rational_to_coefficient(rational_from_expression(expression)?)
}

pub(crate) fn prompt_accepts_canonical_answer(
    contract: ThemeAnswerContract,
    prompt: &ProblemPrompt,
    schema: &AnswerSchema,
    answer: &AnswerNode,
) -> bool {
    match prompt {
        ProblemPrompt::Addition { left, right } => {
            rational_from_answer(answer)
                == ExactRational::new(i128::from(*left) + i128::from(*right), 1)
        }
        ProblemPrompt::Arithmetic { expression } => {
            if contract == ThemeAnswerContract::ArithmeticIntegerDivision
                || matches!(schema, AnswerSchema::OrderedPair)
            {
                arithmetic_integer_division_answer_is_correct(expression, answer)
            } else {
                rational_from_expression(expression).is_some_and(|expected| {
                    rational_from_answer(answer).is_some_and(|actual| actual == expected)
                })
            }
        }
        ProblemPrompt::ColumnArithmetic {
            operator,
            left,
            right,
        } => column_answer_is_correct(contract, schema, *operator, left, right, answer),
        ProblemPrompt::LinearEquation { a, b, c, d, .. } => {
            linear_answer_is_correct(*a, *b, *c, *d, answer)
        }
        ProblemPrompt::QuadraticEquation { form, a, b, c } => {
            quadratic_answer_is_correct(*form, *a, *b, *c, answer)
        }
        ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } => {
            simultaneous_answer_is_correct(*a, *b, *c, *d, *e, *f, answer)
        }
        ProblemPrompt::LiarPuzzle {
            people_count,
            statements,
        } => liar_answer_is_correct(*people_count, statements, answer),
        ProblemPrompt::MiniSudoku { givens } => mini_sudoku_answer_is_correct(givens, answer),
    }
}

fn arithmetic_integer_division_answer_is_correct(
    expression: &ArithmeticExpression,
    answer: &AnswerNode,
) -> bool {
    let ArithmeticExpression::Binary {
        operator: ArithmeticOperator::Divide,
        left,
        right,
    } = expression
    else {
        return false;
    };
    let (Some(dividend), Some(divisor)) = (
        rational_from_expression(left),
        rational_from_expression(right),
    ) else {
        return false;
    };
    if !dividend.is_integer() || !divisor.is_integer() || divisor.numerator() <= 0 {
        return false;
    }
    let AnswerNode::Tuple(values) = answer else {
        return false;
    };
    if values.len() != 2 {
        return false;
    }
    let (Some(quotient), Some(remainder)) = (
        rational_from_answer(&values[0]),
        rational_from_answer(&values[1]),
    ) else {
        return false;
    };
    if !quotient.is_integer() || !remainder.is_integer() {
        return false;
    }
    let divisor_value = divisor.numerator();
    let quotient_value = quotient.numerator();
    let remainder_value = remainder.numerator();
    remainder_value >= 0
        && remainder_value < divisor_value
        && dividend.numerator() == divisor_value * quotient_value + remainder_value
}

fn column_answer_is_correct(
    contract: ThemeAnswerContract,
    schema: &AnswerSchema,
    operator: ArithmeticOperator,
    left: &ArithmeticExpression,
    right: &ArithmeticExpression,
    answer: &AnswerNode,
) -> bool {
    let Some(left) = rational_from_expression(left) else {
        return false;
    };
    let Some(right) = rational_from_expression(right) else {
        return false;
    };

    if contract == ThemeAnswerContract::ColumnIntegerDivision
        || matches!(schema, AnswerSchema::OrderedPair)
    {
        if operator != ArithmeticOperator::Divide || !left.is_integer() || !right.is_integer() {
            return false;
        }
        let AnswerNode::Tuple(values) = answer else {
            return false;
        };
        if values.len() != 2 || right.is_zero() {
            return false;
        }
        let (Some(quotient), Some(remainder)) = (
            rational_from_answer(&values[0]),
            rational_from_answer(&values[1]),
        ) else {
            return false;
        };
        if !quotient.is_integer() || !remainder.is_integer() || remainder.numerator() < 0 {
            return false;
        }
        let divisor_abs = right.numerator().unsigned_abs();
        if remainder.numerator().unsigned_abs() >= divisor_abs {
            return false;
        }
        let Some(reconstructed) = right
            .multiply(quotient)
            .and_then(|value| value.add(remainder))
        else {
            return false;
        };
        return reconstructed == left;
    }

    let expected = match operator {
        ArithmeticOperator::Add => left.add(right),
        ArithmeticOperator::Subtract => left.subtract(right),
        ArithmeticOperator::Multiply => left.multiply(right),
        ArithmeticOperator::Divide => left.divide(right),
    };
    expected.is_some_and(|expected| {
        rational_from_answer(answer).is_some_and(|actual| actual == expected)
    })
}

fn linear_answer_is_correct(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    d: RationalCoefficient,
    answer: &AnswerNode,
) -> bool {
    let Some(x) = rational_from_answer(answer) else {
        return false;
    };
    let (Some(a), Some(b), Some(c), Some(d)) = (
        rational_from_coefficient(a),
        rational_from_coefficient(b),
        rational_from_coefficient(c),
        rational_from_coefficient(d),
    ) else {
        return false;
    };
    if a == c {
        return false;
    }
    let Some(left) = a.multiply(x).and_then(|value| value.add(b)) else {
        return false;
    };
    let Some(right) = c.multiply(x).and_then(|value| value.add(d)) else {
        return false;
    };
    left == right
}

fn simultaneous_answer_is_correct(
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    e: i64,
    f: i64,
    answer: &AnswerNode,
) -> bool {
    let AnswerNode::Tuple(values) = answer else {
        return false;
    };
    if values.len() != 2 {
        return false;
    }
    let (Some(x), Some(y)) = (
        rational_from_answer(&values[0]),
        rational_from_answer(&values[1]),
    ) else {
        return false;
    };
    let (a, b, c, d, e, f) = (
        ExactRational::from_integer(a),
        ExactRational::from_integer(b),
        ExactRational::from_integer(c),
        ExactRational::from_integer(d),
        ExactRational::from_integer(e),
        ExactRational::from_integer(f),
    );
    let Some(determinant) = a
        .multiply(e)
        .and_then(|ae| b.multiply(d).and_then(|bd| ae.subtract(bd)))
    else {
        return false;
    };
    if determinant.is_zero() {
        return false;
    }
    let first = a
        .multiply(x)
        .and_then(|value| b.multiply(y).and_then(|by| value.add(by)));
    let second = d
        .multiply(x)
        .and_then(|value| e.multiply(y).and_then(|ey| value.add(ey)));
    first == Some(c) && second == Some(f)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QuadraticNumber {
    rational: ExactRational,
    radical_coefficient: ExactRational,
    radicand: Option<ExactRational>,
}

impl QuadraticNumber {
    fn rational(value: ExactRational) -> Self {
        Self {
            rational: value,
            radical_coefficient: ExactRational::zero(),
            radicand: None,
        }
    }

    fn from_answer(answer: &AnswerNode) -> Option<Self> {
        if let Some(value) = rational_from_answer(answer) {
            return Some(Self::rational(value));
        }
        match answer {
            AnswerNode::Root { radicand, index } => {
                if let Some(index) = index {
                    if rational_from_answer(index)? != ExactRational::new(2, 1)? {
                        return None;
                    }
                }
                let radicand = rational_from_answer(radicand)?;
                if radicand.sign() != std::cmp::Ordering::Greater {
                    return None;
                }
                Some(Self {
                    rational: ExactRational::zero(),
                    radical_coefficient: ExactRational::one(),
                    radicand: Some(radicand),
                })
            }
            AnswerNode::Negative(value) => Self::from_answer(value)?.negate(),
            AnswerNode::Fraction {
                numerator,
                denominator,
            } => {
                let numerator = Self::from_answer(numerator)?;
                let denominator = Self::from_answer(denominator)?;
                if !denominator.radical_coefficient.is_zero() || denominator.rational.is_zero() {
                    return None;
                }
                numerator.divide_rational(denominator.rational)
            }
            AnswerNode::Binary {
                operator,
                left,
                right,
            } => {
                let left = Self::from_answer(left)?;
                let right = Self::from_answer(right)?;
                match operator {
                    AnswerBinaryOperator::Add => left.add(right),
                    AnswerBinaryOperator::Subtract => left.subtract(right),
                    AnswerBinaryOperator::Multiply => left.multiply(right),
                }
            }
            AnswerNode::Empty
            | AnswerNode::Integer(_)
            | AnswerNode::ExactDecimal { .. }
            | AnswerNode::NanError(_)
            | AnswerNode::MixedFraction { .. }
            | AnswerNode::PlusMinus(_)
            | AnswerNode::Tuple(_)
            | AnswerNode::Variable(_) => None,
        }
    }

    fn normalized(mut self) -> Self {
        if self.radical_coefficient.is_zero() {
            self.radicand = None;
        }
        self
    }

    fn compatible_radicand(self, other: Self) -> Option<Option<ExactRational>> {
        let left_has_radical = !self.radical_coefficient.is_zero();
        let right_has_radical = !other.radical_coefficient.is_zero();
        match (left_has_radical, right_has_radical) {
            (false, false) => Some(None),
            (true, false) => Some(self.radicand),
            (false, true) => Some(other.radicand),
            (true, true) if self.radicand == other.radicand => Some(self.radicand),
            _ => None,
        }
    }

    fn add(self, other: Self) -> Option<Self> {
        Some(
            Self {
                rational: self.rational.add(other.rational)?,
                radical_coefficient: self.radical_coefficient.add(other.radical_coefficient)?,
                radicand: self.compatible_radicand(other)?,
            }
            .normalized(),
        )
    }

    fn subtract(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn negate(self) -> Option<Self> {
        Some(
            Self {
                rational: self.rational.negate()?,
                radical_coefficient: self.radical_coefficient.negate()?,
                radicand: self.radicand,
            }
            .normalized(),
        )
    }

    fn multiply(self, other: Self) -> Option<Self> {
        let radicand = self.compatible_radicand(other)?;
        let rational_product = self.rational.multiply(other.rational)?;
        let radical_square = match radicand {
            Some(radicand) => self
                .radical_coefficient
                .multiply(other.radical_coefficient)?
                .multiply(radicand)?,
            None => ExactRational::zero(),
        };
        let radical_coefficient = self
            .rational
            .multiply(other.radical_coefficient)?
            .add(self.radical_coefficient.multiply(other.rational)?)?;
        Some(
            Self {
                rational: rational_product.add(radical_square)?,
                radical_coefficient,
                radicand,
            }
            .normalized(),
        )
    }

    fn multiply_rational(self, scalar: ExactRational) -> Option<Self> {
        Some(
            Self {
                rational: self.rational.multiply(scalar)?,
                radical_coefficient: self.radical_coefficient.multiply(scalar)?,
                radicand: self.radicand,
            }
            .normalized(),
        )
    }

    fn divide_rational(self, scalar: ExactRational) -> Option<Self> {
        Some(
            Self {
                rational: self.rational.divide(scalar)?,
                radical_coefficient: self.radical_coefficient.divide(scalar)?,
                radicand: self.radicand,
            }
            .normalized(),
        )
    }

    fn is_zero(self) -> bool {
        self.rational.is_zero() && self.radical_coefficient.is_zero()
    }
}

fn quadratic_answer_is_correct(
    form: QuadraticEquationForm,
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    answer: &AnswerNode,
) -> bool {
    let (Some(a), Some(b), Some(c)) = (
        rational_from_coefficient(a),
        rational_from_coefficient(b),
        rational_from_coefficient(c),
    ) else {
        return false;
    };
    let (leading, linear, constant) = match form {
        QuadraticEquationForm::SquareEqualsConstant => {
            let Some(constant) = c.negate() else {
                return false;
            };
            (a, ExactRational::zero(), constant)
        }
        QuadraticEquationForm::SquarePlusConstantZero => (a, ExactRational::zero(), c),
        QuadraticEquationForm::FactoredScale => {
            if a.is_zero() {
                return false;
            }
            (ExactRational::one(), b, c)
        }
        QuadraticEquationForm::Standard => (a, b, c),
    };
    if leading.is_zero() {
        return false;
    }

    let Some(discriminant) = linear.multiply(linear).and_then(|square| {
        leading
            .multiply(constant)
            .and_then(|product| ExactRational::new(4, 1)?.multiply(product))
            .and_then(|four_ac| square.subtract(four_ac))
    }) else {
        return false;
    };
    let expected_roots = match discriminant.sign() {
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Greater => 2,
    };

    let Some(solutions) = crate::grade::solution_set(answer) else {
        return false;
    };
    if solutions.len() != expected_roots || solutions.windows(2).any(|pair| pair[0] == pair[1]) {
        return false;
    }

    solutions.into_iter().all(|solution| {
        let Some(x) = QuadraticNumber::from_answer(&solution) else {
            return false;
        };
        let Some(value) = x
            .multiply(x)
            .and_then(|square| square.multiply_rational(leading))
            .and_then(|value| {
                x.multiply_rational(linear)
                    .and_then(|linear_term| value.add(linear_term))
            })
            .and_then(|value| value.add(QuadraticNumber::rational(constant)))
        else {
            return false;
        };
        value.is_zero()
    })
}

pub(crate) fn liar_solutions(
    people_count: crate::model::PeopleCount,
    statements: &[LiarStatement],
) -> Vec<u32> {
    let mut solutions = Vec::new();
    for mask in 0_u32..(1_u32 << people_count.value()) {
        let valid = statements.iter().enumerate().all(|(speaker, statement)| {
            let speaker_is_liar = ((mask >> speaker) & 1) == 1;
            statement.is_true_for_mask(mask) == !speaker_is_liar
        });
        if valid {
            solutions.push(mask);
        }
    }
    solutions
}

fn liar_answer_is_correct(
    people_count: crate::model::PeopleCount,
    statements: &[LiarStatement],
    answer: &AnswerNode,
) -> bool {
    let AnswerNode::Tuple(values) = answer else {
        return false;
    };
    let Some(mask) = values.iter().try_fold(0_u32, |mask, value| {
        let AnswerNode::Integer(person) = value else {
            return None;
        };
        let person = u32::try_from(*person).ok()?;
        let bit = person.checked_sub(1)?;
        Some(mask | (1_u32 << bit))
    }) else {
        return false;
    };
    matches!(liar_solutions(people_count, statements).as_slice(), [solution] if *solution == mask)
}

pub(crate) fn mini_sudoku_solutions(
    board: [u8; MINI_SUDOKU_CELL_COUNT],
    limit: usize,
) -> Vec<[u8; MINI_SUDOKU_CELL_COUNT]> {
    if !mini_sudoku_board_is_consistent(&board) {
        return Vec::new();
    }
    let mut solutions = Vec::new();
    enumerate_mini_sudoku_solutions(board, 0, &mut solutions, limit);
    solutions
}

fn mini_sudoku_board_is_consistent(board: &[u8; MINI_SUDOKU_CELL_COUNT]) -> bool {
    (0..MINI_SUDOKU_CELL_COUNT).all(|index| {
        let digit = board[index];
        if digit == 0 {
            return true;
        }
        if !(MINI_SUDOKU_GRID_SPEC.min_digit()..=MINI_SUDOKU_GRID_SPEC.max_digit()).contains(&digit)
        {
            return false;
        }
        let mut without_current = *board;
        without_current[index] = 0;
        mini_sudoku_can_place(&without_current, index, digit)
    })
}

fn enumerate_mini_sudoku_solutions(
    mut board: [u8; MINI_SUDOKU_CELL_COUNT],
    start: usize,
    solutions: &mut Vec<[u8; MINI_SUDOKU_CELL_COUNT]>,
    limit: usize,
) {
    if solutions.len() >= limit {
        return;
    }
    let Some(index) = (start..MINI_SUDOKU_CELL_COUNT).find(|&index| board[index] == 0) else {
        solutions.push(board);
        return;
    };
    for digit in MINI_SUDOKU_GRID_SPEC.min_digit()..=MINI_SUDOKU_GRID_SPEC.max_digit() {
        if mini_sudoku_can_place(&board, index, digit) {
            board[index] = digit;
            enumerate_mini_sudoku_solutions(board, index + 1, solutions, limit);
            board[index] = 0;
            if solutions.len() >= limit {
                return;
            }
        }
    }
}

fn mini_sudoku_can_place(board: &[u8; MINI_SUDOKU_CELL_COUNT], index: usize, digit: u8) -> bool {
    let row = index / MINI_SUDOKU_SIDE;
    let column = index % MINI_SUDOKU_SIDE;
    if (0..MINI_SUDOKU_SIDE).any(|offset| board[row * MINI_SUDOKU_SIDE + offset] == digit) {
        return false;
    }
    if (0..MINI_SUDOKU_SIDE).any(|offset| board[offset * MINI_SUDOKU_SIDE + column] == digit) {
        return false;
    }
    let block_row = (row / 2) * 2;
    let block_column = (column / 2) * 2;
    !(0..2).any(|dr| {
        (0..2).any(|dc| board[(block_row + dr) * MINI_SUDOKU_SIDE + block_column + dc] == digit)
    })
}

fn mini_sudoku_answer_is_correct(givens: &MiniSudokuGrid, answer: &AnswerNode) -> bool {
    let AnswerNode::Tuple(values) = answer else {
        return false;
    };
    if values.len() != MINI_SUDOKU_CELL_COUNT {
        return false;
    }
    let mut solved = [0_u8; MINI_SUDOKU_CELL_COUNT];
    for (index, value) in values.iter().enumerate() {
        let AnswerNode::Integer(digit) = value else {
            return false;
        };
        let Ok(digit) = u8::try_from(*digit) else {
            return false;
        };
        if !(MINI_SUDOKU_GRID_SPEC.min_digit()..=MINI_SUDOKU_GRID_SPEC.max_digit()).contains(&digit)
        {
            return false;
        }
        if givens[index].is_some_and(|given| given != digit) {
            return false;
        }
        solved[index] = digit;
    }

    let givens_board = std::array::from_fn(|index| givens[index].unwrap_or(0));
    matches!(mini_sudoku_solutions(givens_board, 2).as_slice(), [solution] if *solution == solved)
}
