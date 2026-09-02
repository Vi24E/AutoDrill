use super::*;

#[derive(Default)]
struct LargeSampleCounters {
    problem_count: usize,
    operator_counts: [usize; 4],
    expression_operator_count: usize,
    expression_depth_sum: usize,
    expression_depth_max: usize,
    literal_digit_sum: usize,
    rational_denominator_sum: u64,
    rational_literal_count: usize,
    different_fraction_denominator_pairs: usize,
    fraction_binary_pairs: usize,
    decimal_scale_sum: u64,
    decimal_literal_count: usize,
    equation_coefficient_abs_sum: u64,
    equation_coefficient_count: usize,
    fractional_equation_coefficients: usize,
    liar_people_sum: usize,
    liar_problem_count: usize,
    liar_statement_sum: usize,
    sudoku_blank_sum: usize,
    sudoku_problem_count: usize,
    zero_answers: usize,
    one_answers: usize,
    negative_answers: usize,
    scalar_answers: usize,
    repeated_binary_operands: usize,
    commutative_pairs: usize,
    commutative_left_greater: usize,
    max_abs_numerator: u64,
    max_denominator: u64,
    max_answer_magnitude: f64,
    scalar_answer_abs_sum: f64,
    scalar_answer_denominator_sum: u64,
    carry_borrow_events: usize,
}

fn audit_seed(mut index: usize) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    index += 1;
    let mut seed = String::from("Q");
    while index > 0 {
        seed.push(ALPHABET[index % ALPHABET.len()] as char);
        index /= ALPHABET.len();
    }
    seed
}

fn decimal_digit_count(value: i64) -> usize {
    if value == 0 {
        1
    } else {
        value.unsigned_abs().ilog10() as usize + 1
    }
}

fn rational_denominator(expression: &ArithmeticExpression) -> Option<i64> {
    match expression {
        ArithmeticExpression::Rational { value } => Some(value.denominator()),
        _ => None,
    }
}

fn scaled_integer(expression: &ArithmeticExpression) -> Option<(i128, u32)> {
    match expression {
        ArithmeticExpression::Integer { value } => Some((i128::from(*value), 0)),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => {
            Some((i128::from(*coefficient), *scale))
        }
        _ => None,
    }
}

fn pow10_i128(exponent: u32) -> Option<i128> {
    10_i128.checked_pow(exponent)
}

fn aligned_nonnegative_pair(
    left: &ArithmeticExpression,
    right: &ArithmeticExpression,
) -> Option<(u128, u128)> {
    let (left_value, left_scale) = scaled_integer(left)?;
    let (right_value, right_scale) = scaled_integer(right)?;
    if left_value < 0 || right_value < 0 {
        return None;
    }
    let scale = left_scale.max(right_scale);
    let left = left_value.checked_mul(pow10_i128(scale - left_scale)?)?;
    let right = right_value.checked_mul(pow10_i128(scale - right_scale)?)?;
    Some((u128::try_from(left).ok()?, u128::try_from(right).ok()?))
}

fn carry_events(mut left: u128, mut right: u128) -> usize {
    let mut carry = 0_u128;
    let mut count = 0_usize;
    while left > 0 || right > 0 || carry > 0 {
        let sum = left % 10 + right % 10 + carry;
        carry = u128::from(sum >= 10);
        count += usize::from(carry > 0);
        left /= 10;
        right /= 10;
    }
    count
}

fn borrow_events(mut left: u128, mut right: u128) -> usize {
    let mut borrow = 0_u128;
    let mut count = 0_usize;
    while left > 0 || right > 0 {
        let left_digit = left % 10;
        let right_digit = right % 10 + borrow;
        if left_digit < right_digit {
            borrow = 1;
            count += 1;
        } else {
            borrow = 0;
        }
        left /= 10;
        right /= 10;
    }
    count
}

fn observe_carry_borrow(
    operator: ArithmeticOperator,
    left: &ArithmeticExpression,
    right: &ArithmeticExpression,
    counters: &mut LargeSampleCounters,
) {
    let Some((left, right)) = aligned_nonnegative_pair(left, right) else {
        return;
    };
    counters.carry_borrow_events += match operator {
        ArithmeticOperator::Add => carry_events(left, right),
        ArithmeticOperator::Subtract if left >= right => borrow_events(left, right),
        ArithmeticOperator::Subtract
        | ArithmeticOperator::Multiply
        | ArithmeticOperator::Divide => 0,
    };
}

fn observe_expression(expression: &ArithmeticExpression, counters: &mut LargeSampleCounters) {
    let mut stack = vec![(expression, 1_usize)];
    while let Some((node, depth)) = stack.pop() {
        counters.expression_depth_max = counters.expression_depth_max.max(depth);
        match node {
            ArithmeticExpression::Integer { value } => {
                counters.literal_digit_sum += decimal_digit_count(*value);
            }
            ArithmeticExpression::ExactDecimal { coefficient, scale } => {
                counters.literal_digit_sum += decimal_digit_count(*coefficient);
                counters.decimal_scale_sum += u64::from(*scale);
                counters.decimal_literal_count += 1;
            }
            ArithmeticExpression::Rational { value } => {
                counters.literal_digit_sum += decimal_digit_count(value.numerator())
                    + decimal_digit_count(value.denominator());
                counters.rational_denominator_sum += value.denominator().unsigned_abs();
                counters.rational_literal_count += 1;
            }
            ArithmeticExpression::Binary {
                operator,
                left,
                right,
            } => {
                counters.expression_operator_count += 1;
                counters.operator_counts[*operator as usize] += 1;
                observe_carry_borrow(*operator, left, right, counters);
                if left == right {
                    counters.repeated_binary_operands += 1;
                }
                if matches!(
                    operator,
                    ArithmeticOperator::Add | ArithmeticOperator::Multiply
                ) {
                    counters.commutative_pairs += 1;
                    if right < left {
                        counters.commutative_left_greater += 1;
                    }
                }
                if let (Some(left_denominator), Some(right_denominator)) =
                    (rational_denominator(left), rational_denominator(right))
                {
                    counters.fraction_binary_pairs += 1;
                    counters.different_fraction_denominator_pairs +=
                        usize::from(left_denominator != right_denominator);
                }
                stack.push((right, depth + 1));
                stack.push((left, depth + 1));
            }
        }
    }
}

fn observe_coefficient(
    value: &crate::model::RationalCoefficient,
    counters: &mut LargeSampleCounters,
) {
    counters.equation_coefficient_abs_sum += value.numerator().unsigned_abs();
    counters.equation_coefficient_count += 1;
    counters.fractional_equation_coefficients += usize::from(value.denominator() != 1);
}

fn observe_problem(problem: &Problem, counters: &mut LargeSampleCounters) {
    counters.problem_count += 1;
    let depth_before = counters.expression_depth_max;
    match problem.prompt() {
        ProblemPrompt::Addition { left, right } => {
            counters.literal_digit_sum += decimal_digit_count(i64::from(*left));
            counters.literal_digit_sum += decimal_digit_count(i64::from(*right));
            counters.carry_borrow_events += carry_events(u128::from(*left), u128::from(*right));
        }
        ProblemPrompt::Arithmetic { expression } => observe_expression(expression, counters),
        ProblemPrompt::ColumnArithmetic {
            operator,
            left,
            right,
        } => {
            counters.operator_counts[*operator as usize] += 1;
            observe_carry_borrow(*operator, left, right, counters);
            if left == right {
                counters.repeated_binary_operands += 1;
            }
            if matches!(
                operator,
                ArithmeticOperator::Add | ArithmeticOperator::Multiply
            ) {
                counters.commutative_pairs += 1;
                if right < left {
                    counters.commutative_left_greater += 1;
                }
            }
            observe_expression(left, counters);
            observe_expression(right, counters);
        }
        ProblemPrompt::LinearEquation { left, right } => {
            if let (Some((a, left_y, b)), Some((c, right_y, d))) = (
                crate::semantics::normalize_linear_expression(left),
                crate::semantics::normalize_linear_expression(right),
            ) {
                debug_assert!(left_y.is_zero() && right_y.is_zero());
                for coefficient in [&a, &b, &c, &d] {
                    observe_coefficient(coefficient, counters);
                }
            }
        }
        ProblemPrompt::QuadraticEquation { a, b, c, .. } => {
            for coefficient in [a, b, c] {
                observe_coefficient(coefficient, counters);
            }
        }
        ProblemPrompt::SimultaneousEquation { equations, .. } => {
            for equation in equations {
                if let Some((x, y, rhs)) = crate::semantics::normalize_linear_equation(equation) {
                    for coefficient in [x, y, rhs] {
                        observe_coefficient(&coefficient, counters);
                    }
                }
            }
        }
        ProblemPrompt::LiarPuzzle {
            people_count,
            statements,
        } => {
            counters.liar_people_sum += usize::from(people_count.value());
            counters.liar_statement_sum += statements.len();
            counters.liar_problem_count += 1;
        }
        ProblemPrompt::MiniSudoku { givens } => {
            counters.sudoku_blank_sum += (0..crate::model::MINI_SUDOKU_CELL_COUNT)
                .filter(|&index| givens[index].is_none())
                .count();
            counters.sudoku_problem_count += 1;
        }
    }
    counters.expression_depth_sum += counters.expression_depth_max.saturating_sub(depth_before);

    let normalized = crate::normalize::normalize_answer(problem.canonical_answer());
    let scalar = match &normalized {
        AnswerNode::Integer(value) => Some((*value, 1_i64)),
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => match (&**numerator, &**denominator) {
            (AnswerNode::Integer(numerator), AnswerNode::Integer(denominator)) => {
                Some((*numerator, *denominator))
            }
            _ => None,
        },
        _ => None,
    };
    if let Some((numerator, denominator)) = scalar {
        counters.scalar_answers += 1;
        counters.zero_answers += usize::from(numerator == 0);
        counters.one_answers += usize::from(numerator == denominator);
        counters.negative_answers += usize::from(numerator < 0);
        counters.max_abs_numerator = counters.max_abs_numerator.max(numerator.unsigned_abs());
        counters.max_denominator = counters.max_denominator.max(denominator.unsigned_abs());
        let magnitude = (numerator as f64 / denominator as f64).abs();
        counters.max_answer_magnitude = counters.max_answer_magnitude.max(magnitude);
        counters.scalar_answer_abs_sum += magnitude;
        counters.scalar_answer_denominator_sum += denominator.unsigned_abs();
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn structural_summary(counters: &LargeSampleCounters) -> String {
    let n = counters.problem_count.max(1) as f64;
    let rational_denominator_mean = if counters.rational_literal_count == 0 {
        0.0
    } else {
        counters.rational_denominator_sum as f64 / counters.rational_literal_count as f64
    };
    let decimal_scale_mean = if counters.decimal_literal_count == 0 {
        0.0
    } else {
        counters.decimal_scale_sum as f64 / counters.decimal_literal_count as f64
    };
    let coefficient_abs_mean = if counters.equation_coefficient_count == 0 {
        0.0
    } else {
        counters.equation_coefficient_abs_sum as f64 / counters.equation_coefficient_count as f64
    };
    let liar_people_mean = if counters.liar_problem_count == 0 {
        0.0
    } else {
        counters.liar_people_sum as f64 / counters.liar_problem_count as f64
    };
    let sudoku_blanks_mean = if counters.sudoku_problem_count == 0 {
        0.0
    } else {
        counters.sudoku_blank_sum as f64 / counters.sudoku_problem_count as f64
    };
    format!(
        "ops/p={:.2} digits/p={:.2} carry_borrow/p={:.2} rational_den={:.2} diff_den={:.2} repeat={:.3} decimal_scale={:.2} coeff_abs={:.2} frac_coeff={:.2} liar_people={:.2} sudoku_blanks={:.2} answer_abs={:.2} answer_den={:.2} zero={:.3} one={:.3} neg={:.3}",
        counters.expression_operator_count as f64 / n,
        counters.literal_digit_sum as f64 / n,
        counters.carry_borrow_events as f64 / n,
        rational_denominator_mean,
        ratio(counters.different_fraction_denominator_pairs, counters.fraction_binary_pairs),
        ratio(counters.repeated_binary_operands, counters.problem_count),
        decimal_scale_mean,
        coefficient_abs_mean,
        ratio(counters.fractional_equation_coefficients, counters.equation_coefficient_count),
        liar_people_mean,
        sudoku_blanks_mean,
        if counters.scalar_answers == 0 { 0.0 } else { counters.scalar_answer_abs_sum / counters.scalar_answers as f64 },
        if counters.scalar_answers == 0 { 0.0 } else { counters.scalar_answer_denominator_sum as f64 / counters.scalar_answers as f64 },
        ratio(counters.zero_answers, counters.scalar_answers),
        ratio(counters.one_answers, counters.scalar_answers),
        ratio(counters.negative_answers, counters.scalar_answers),
    )
}

#[test]
#[ignore = "large-sample autonomous QA; run explicitly"]
fn autonomous_large_sample_registry_qa() {
    const SEED_COUNT: usize = 100;
    const REGENERATION_SEEDS: usize = 3;

    let registrations = crate::registry::active_registrations().unwrap();
    let mut total_worksheets = 0_usize;
    let mut total_problems = 0_usize;
    let mut generation_failures = Vec::new();
    let mut invariant_violations = Vec::new();
    let mut total_layered_worksheets = 0_usize;

    println!(
        "AUTONOMOUS_QA_BEGIN themes={} difficulties=4 seeds_per_difficulty={SEED_COUNT}",
        registrations.len()
    );

    for registration in registrations {
        let generator = registered_generator(
            registration.numeric_theme_id(),
            registration.generator_revision(),
        )
        .unwrap()
        .expect("active registration must have a current generator");
        let strategy = generator.sampling_strategy().unwrap();
        let layer_contract = strategy.layers().map(|layers| {
            (
                layers,
                layered_quotas(layers, registration.layout().problem_count()),
            )
        });
        let mut counters_by_difficulty: [LargeSampleCounters; 4] =
            std::array::from_fn(|_| LargeSampleCounters::default());

        for difficulty_value in 1_u8..=4 {
            let difficulty = crate::identity::Difficulty::try_from(difficulty_value).unwrap();
            for seed_index in 0..SEED_COUNT {
                let seed = audit_seed(seed_index);
                let request = GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: registration.numeric_theme_id(),
                    seed: seed.clone(),
                    difficulty,
                    timeout_ms: Some(15_000),
                    max_attempts: Some(50_000),
                };
                let worksheet = match generate_worksheet_request(&request) {
                    Ok(worksheet) => worksheet,
                    Err(error) => {
                        generation_failures.push(format!(
                            "theme={} difficulty={} seed={} error={}",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed,
                            error
                        ));
                        continue;
                    }
                };
                total_worksheets += 1;
                total_problems += worksheet.problems().len();

                if worksheet.problems().len() != registration.layout().problem_count() {
                    invariant_violations.push(format!(
                        "theme={} difficulty={} seed={} problem_count={} expected={}",
                        registration.numeric_theme_id(),
                        difficulty_value,
                        seed,
                        worksheet.problems().len(),
                        registration.layout().problem_count()
                    ));
                }

                let mut prompt_set = std::collections::BTreeSet::new();
                let mut key_set = std::collections::BTreeSet::new();
                for problem in worksheet.problems() {
                    if !prompt_set.insert(problem.prompt()) {
                        invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} duplicate_prompt",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        ));
                    }
                    if !key_set.insert(problem_key(registration, problem)) {
                        invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} duplicate_problem_key",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        ));
                    }
                    if !problem_allowed_by_curriculum(registration, problem) {
                        invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} curriculum_violation",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        ));
                    }
                    let effort = problem.effort();
                    if !effort.is_finite() || effort < 0.0 {
                        invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} invalid_effort={effort}",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        ));
                    }
                    observe_problem(
                        problem,
                        &mut counters_by_difficulty[(difficulty_value - 1) as usize],
                    );
                }

                if difficulty_value <= 2
                    && !worksheet
                        .problems()
                        .windows(2)
                        .all(|pair| pair[0].effort() <= pair[1].effort())
                {
                    invariant_violations.push(format!(
                        "theme={} difficulty={} seed={} lost_effort_order",
                        registration.numeric_theme_id(),
                        difficulty_value,
                        seed
                    ));
                }

                if let Some((layers, expected)) = &layer_contract {
                    total_layered_worksheets += 1;
                    let mut counts = vec![0_usize; layers.specs().len()];
                    for problem in worksheet.problems() {
                        match strategy.layer_of(problem) {
                            Ok(Some(layer)) => counts[layer.value()] += 1,
                            other => invariant_violations.push(format!(
                                "theme={} difficulty={} seed={} invalid_layer={other:?}",
                                registration.numeric_theme_id(),
                                difficulty_value,
                                seed
                            )),
                        }
                    }
                    if counts != *expected {
                        invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} layer_counts={counts:?} expected={expected:?}",
                            registration.numeric_theme_id(), difficulty_value, seed
                        ));
                    }
                }

                if seed_index < REGENERATION_SEEDS {
                    match generate_worksheet_request(&request) {
                        Ok(second) if second == worksheet => {}
                        Ok(_) => invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} nondeterministic_request",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        )),
                        Err(error) => invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} second_generation_error={error}",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        )),
                    }
                    match generate_identity_with_clock(
                        worksheet.identity(),
                        &GenerationConfig::default(),
                        &SystemClock::new(),
                    ) {
                        Ok(regenerated) if regenerated == worksheet => {}
                        Ok(_) => invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} regeneration_mismatch",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        )),
                        Err(error) => invariant_violations.push(format!(
                            "theme={} difficulty={} seed={} regeneration_error={error}",
                            registration.numeric_theme_id(),
                            difficulty_value,
                            seed
                        )),
                    }
                }
            }
        }

        let summaries = counters_by_difficulty
            .iter()
            .enumerate()
            .map(|(difficulty, counters)| {
                format!("d{} {}", difficulty + 1, structural_summary(counters))
            })
            .collect::<Vec<_>>();
        let aggregate = counters_by_difficulty.iter().fold(
            LargeSampleCounters::default(),
            |mut total, counters| {
                total.problem_count += counters.problem_count;
                for index in 0..4 {
                    total.operator_counts[index] += counters.operator_counts[index];
                }
                total.expression_operator_count += counters.expression_operator_count;
                total.expression_depth_sum += counters.expression_depth_sum;
                total.expression_depth_max = total
                    .expression_depth_max
                    .max(counters.expression_depth_max);
                total.literal_digit_sum += counters.literal_digit_sum;
                total.rational_denominator_sum += counters.rational_denominator_sum;
                total.rational_literal_count += counters.rational_literal_count;
                total.different_fraction_denominator_pairs +=
                    counters.different_fraction_denominator_pairs;
                total.fraction_binary_pairs += counters.fraction_binary_pairs;
                total.decimal_scale_sum += counters.decimal_scale_sum;
                total.decimal_literal_count += counters.decimal_literal_count;
                total.equation_coefficient_abs_sum += counters.equation_coefficient_abs_sum;
                total.equation_coefficient_count += counters.equation_coefficient_count;
                total.fractional_equation_coefficients += counters.fractional_equation_coefficients;
                total.liar_people_sum += counters.liar_people_sum;
                total.liar_problem_count += counters.liar_problem_count;
                total.liar_statement_sum += counters.liar_statement_sum;
                total.sudoku_blank_sum += counters.sudoku_blank_sum;
                total.sudoku_problem_count += counters.sudoku_problem_count;
                total.zero_answers += counters.zero_answers;
                total.one_answers += counters.one_answers;
                total.negative_answers += counters.negative_answers;
                total.scalar_answers += counters.scalar_answers;
                total.repeated_binary_operands += counters.repeated_binary_operands;
                total.commutative_pairs += counters.commutative_pairs;
                total.commutative_left_greater += counters.commutative_left_greater;
                total.max_abs_numerator = total.max_abs_numerator.max(counters.max_abs_numerator);
                total.max_denominator = total.max_denominator.max(counters.max_denominator);
                total.max_answer_magnitude = total
                    .max_answer_magnitude
                    .max(counters.max_answer_magnitude);
                total.scalar_answer_abs_sum += counters.scalar_answer_abs_sum;
                total.scalar_answer_denominator_sum += counters.scalar_answer_denominator_sum;
                total.carry_borrow_events += counters.carry_borrow_events;
                total
            },
        );

        println!(
            "AUTONOMOUS_QA_THEME id={} revision={} problems={} {}; operators add={} sub={} mul={} div={}; repeated_operands={} commutative_left_greater={}/{}; max_answer_num={} max_answer_den={} max_answer_abs={:.3}; layered={}",
            registration.numeric_theme_id(),
            registration.generator_revision(),
            aggregate.problem_count,
            summaries.join(" | "),
            aggregate.operator_counts[ArithmeticOperator::Add as usize],
            aggregate.operator_counts[ArithmeticOperator::Subtract as usize],
            aggregate.operator_counts[ArithmeticOperator::Multiply as usize],
            aggregate.operator_counts[ArithmeticOperator::Divide as usize],
            aggregate.repeated_binary_operands,
            aggregate.commutative_left_greater,
            aggregate.commutative_pairs,
            aggregate.max_abs_numerator,
            aggregate.max_denominator,
            aggregate.max_answer_magnitude,
            layer_contract.is_some(),
        );
    }

    println!(
        "AUTONOMOUS_QA_END worksheets={} problems={} generation_failures={} invariant_violations={} layered_worksheets={}",
        total_worksheets,
        total_problems,
        generation_failures.len(),
        invariant_violations.len(),
        total_layered_worksheets,
    );
    for failure in generation_failures.iter().take(20) {
        println!("AUTONOMOUS_QA_GENERATION_FAILURE {failure}");
    }
    for violation in invariant_violations.iter().take(20) {
        println!("AUTONOMOUS_QA_INVARIANT_VIOLATION {violation}");
    }
    assert!(
        generation_failures.is_empty(),
        "large-sample generation failures"
    );
    assert!(
        invariant_violations.is_empty(),
        "large-sample invariant violations"
    );
}
