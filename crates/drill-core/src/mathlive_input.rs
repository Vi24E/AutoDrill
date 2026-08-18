use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::error::EditorError;
use crate::input::ensure_capability;
use crate::model::{AnswerInputInterface, MAX_ANSWER_AST_SIZE};

const MAX_MATHLIVE_LATEX_BYTES: usize = 4096;

/// Parse the deliberately small MathLive/LaTeX answer language accepted by
/// AutoDrill. MathLive owns editing and layout; this adapter is the only place
/// where Web editor output becomes the Rust AnswerNode authority.
pub fn parse_mathlive_answer(
    latex: &str,
    input_interface: &AnswerInputInterface,
) -> Result<AnswerNode, EditorError> {
    if !input_interface.is_structurally_valid() {
        return Err(EditorError::InputInterfaceViolation);
    }
    if latex_exceeds_parse_budget(latex) {
        return Err(EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        });
    }
    let compact = compact_mathlive_latex(latex);
    let mut parser = Parser::new(&compact);
    let answer = match parser.parse_expression(None) {
        Ok(answer) if parser.is_eof() => answer,
        _ => AnswerNode::NanError(latex.to_owned()),
    };

    if !answer.is_within_size_limit() {
        return Err(EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        });
    }
    ensure_capability(&answer, input_interface)?;
    Ok(answer)
}

fn latex_exceeds_parse_budget(input: &str) -> bool {
    if input.len() > MAX_MATHLIVE_LATEX_BYTES {
        return true;
    }

    // The AnswerNode limit is also a safe parser-depth budget. Reject deeply
    // nested MathLive structures before entering the recursive-descent parser,
    // so malformed/pasted input cannot reach the native/WASM stack limit.
    let mut depth = 0usize;
    for ch in input.chars() {
        match ch {
            '{' | '[' => {
                depth = depth.saturating_add(1);
                if depth > MAX_ANSWER_AST_SIZE {
                    return true;
                }
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    ["\\frac", "\\sqrt", "\\placeholder"]
        .iter()
        .map(|command| input.match_indices(command).count())
        .sum::<usize>()
        > MAX_ANSWER_AST_SIZE
}

fn compact_mathlive_latex(input: &str) -> String {
    input
        .replace("\\left", "")
        .replace("\\right", "")
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    nesting_depth: usize,
}

impl<'a> Parser<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            nesting_depth: 0,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.input.len()
    }

    fn with_nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> Result<T, ()>) -> Result<T, ()> {
        if self.nesting_depth >= MAX_ANSWER_AST_SIZE {
            return Err(());
        }
        self.nesting_depth += 1;
        let result = parse(self);
        self.nesting_depth -= 1;
        result
    }

    fn parse_expression(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        if self.at_stop(stop) {
            return Ok(AnswerNode::Empty);
        }
        let first = self.parse_additive(stop)?;
        if self.at_stop(stop) {
            return Ok(first);
        }
        if !self.consume_char(',') {
            return Err(());
        }
        let mut values = vec![first];
        loop {
            values.push(self.parse_additive(stop)?);
            if self.at_stop(stop) {
                break;
            }
            if !self.consume_char(',') {
                return Err(());
            }
        }
        Ok(AnswerNode::Tuple(values))
    }

    fn parse_additive(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        let mut left = self.parse_multiplicative(stop)?;
        loop {
            if self.at_stop(stop) || self.peek_char() == Some(',') {
                return Ok(left);
            }
            let operator = if self.consume_char('+') {
                Some(AnswerBinaryOperator::Add)
            } else if self.consume_char('-') || self.consume_char('−') {
                Some(AnswerBinaryOperator::Subtract)
            } else if self.consume_str("\\pm") || self.consume_char('±') {
                let right = self.parse_multiplicative(stop)?;
                left = AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Add,
                    left: Box::new(left),
                    right: Box::new(AnswerNode::PlusMinus(Box::new(right))),
                };
                continue;
            } else {
                return Ok(left);
            };
            let right = self.parse_multiplicative(stop)?;
            left = AnswerNode::Binary {
                operator: operator.expect("additive operator"),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_multiplicative(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        let mut left = self.parse_unary(stop)?;
        loop {
            if self.at_stop(stop)
                || self.peek_char() == Some(',')
                || self.peek_char() == Some('+')
                || self.peek_char() == Some('-')
                || self.peek_char() == Some('−')
                || self.starts_with("\\pm")
                || self.peek_char() == Some('±')
            {
                return Ok(left);
            }

            // Preserve the established mixed-number spelling 1\frac{1}{2}.
            if matches!(left, AnswerNode::Integer(_) | AnswerNode::Empty)
                && self.starts_with("\\frac")
            {
                let fraction = self.parse_fraction()?;
                let AnswerNode::Fraction {
                    numerator,
                    denominator,
                } = &fraction
                else {
                    unreachable!();
                };
                left = AnswerNode::MixedFraction {
                    whole: Box::new(left),
                    numerator: Box::new(numerator.as_ref().clone()),
                    denominator: Box::new(denominator.as_ref().clone()),
                };
                continue;
            }

            let explicit = self.consume_str("\\times")
                || self.consume_str("\\cdot")
                || self.consume_char('*')
                || self.consume_char('×');
            if !explicit && !self.starts_atom() {
                return Ok(left);
            }
            let right = self.parse_unary(stop)?;
            left = AnswerNode::Binary {
                operator: AnswerBinaryOperator::Multiply,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_unary(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        if self.consume_char('-') || self.consume_char('−') {
            let value = self.with_nested(|parser| parser.parse_unary(stop))?;
            return Ok(AnswerNode::Negative(Box::new(value)));
        }
        if self.consume_str("\\pm") || self.consume_char('±') {
            let value = self.with_nested(|parser| parser.parse_unary(stop))?;
            return Ok(AnswerNode::PlusMinus(Box::new(value)));
        }
        if self.at_stop(stop) || self.peek_char() == Some(',') {
            return Ok(AnswerNode::Empty);
        }
        self.parse_atom(stop)
    }

    fn starts_atom(&self) -> bool {
        self.starts_with("\\frac")
            || self.starts_with("\\sqrt")
            || self.starts_with("\\placeholder")
            || matches!(self.peek_char(), Some('{') | Some('('))
            || self.peek_char().is_some_and(|ch| ch.is_ascii_digit())
    }

    fn parse_atom(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        if self.starts_with("\\frac") {
            return self.parse_fraction();
        }
        if self.starts_with("\\sqrt") {
            return self.parse_root();
        }
        if self.starts_with("\\placeholder") {
            return self.parse_placeholder();
        }
        if self.peek_char() == Some('{') {
            return self.parse_group();
        }
        if self.consume_char('(') {
            let value = self.with_nested(|parser| parser.parse_expression(Some(')')))?;
            if !self.consume_char(')') {
                return Err(());
            }
            return Ok(value);
        }
        if self.at_stop(stop) {
            return Ok(AnswerNode::Empty);
        }
        self.parse_number()
    }

    fn parse_fraction(&mut self) -> Result<AnswerNode, ()> {
        if !self.consume_str("\\frac") {
            return Err(());
        }
        let numerator = self.parse_required_argument()?;
        let denominator = self.parse_required_argument()?;
        Ok(AnswerNode::Fraction {
            numerator: Box::new(numerator),
            denominator: Box::new(denominator),
        })
    }

    fn parse_root(&mut self) -> Result<AnswerNode, ()> {
        if !self.consume_str("\\sqrt") {
            return Err(());
        }
        let index = if self.consume_char('[') {
            let value = self.with_nested(|parser| parser.parse_expression(Some(']')))?;
            if !self.consume_char(']') {
                return Err(());
            }
            if value.is_empty() {
                None
            } else {
                Some(Box::new(value))
            }
        } else {
            None
        };
        let radicand = self.parse_required_argument()?;
        Ok(AnswerNode::Root {
            radicand: Box::new(radicand),
            index,
        })
    }

    fn parse_required_argument(&mut self) -> Result<AnswerNode, ()> {
        if self.peek_char() == Some('{') {
            return self.parse_group();
        }
        if self.starts_with("\\frac")
            || self.starts_with("\\sqrt")
            || self.starts_with("\\placeholder")
        {
            return self.parse_atom(None);
        }
        let ch = self.peek_char().ok_or(())?;
        if ch.is_ascii_digit() {
            self.bump_char();
            return Ok(AnswerNode::Integer(ch.to_digit(10).ok_or(())?.into()));
        }
        Err(())
    }

    fn parse_placeholder(&mut self) -> Result<AnswerNode, ()> {
        if !self.consume_str("\\placeholder") {
            return Err(());
        }
        if self.consume_char('[') {
            while let Some(ch) = self.peek_char() {
                self.bump_char();
                if ch == ']' {
                    break;
                }
            }
            if !self.input[..self.pos].ends_with(']') {
                return Err(());
            }
        }
        self.parse_group()
    }

    fn parse_group(&mut self) -> Result<AnswerNode, ()> {
        if !self.consume_char('{') {
            return Err(());
        }
        let value = self.with_nested(|parser| parser.parse_expression(Some('}')))?;
        if !self.consume_char('}') {
            return Err(());
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<AnswerNode, ()> {
        let start = self.pos;
        let mut decimal_seen = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.bump_char();
            } else if ch == '.' && !decimal_seen {
                decimal_seen = true;
                self.bump_char();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(());
        }
        let text = &self.input[start..self.pos];
        if decimal_seen {
            let (whole, fraction) = text.split_once('.').ok_or(())?;
            let whole = if whole.is_empty() { "0" } else { whole };
            let coefficient_text = format!("{whole}{fraction}");
            let coefficient = coefficient_text.parse::<i64>().map_err(|_| ())?;
            let scale = fraction.len().try_into().map_err(|_| ())?;
            Ok(AnswerNode::ExactDecimal { coefficient, scale })
        } else {
            Ok(AnswerNode::Integer(text.parse::<i64>().map_err(|_| ())?))
        }
    }

    fn at_stop(&self, stop: Option<char>) -> bool {
        stop.is_some_and(|stop| self.peek_char() == Some(stop)) || self.is_eof()
    }

    fn starts_with(&self, text: &str) -> bool {
        self.input[self.pos..].starts_with(text)
    }

    fn consume_str(&mut self, text: &str) -> bool {
        if self.starts_with(text) {
            self.pos += text.len();
            true
        } else {
            false
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.bump_char();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump_char(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EditorStructure;

    fn structured() -> AnswerInputInterface {
        AnswerInputInterface::StructuredMath {
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
        }
    }

    #[test]
    fn rejects_structurally_invalid_input_interface_before_parsing() {
        let invalid = AnswerInputInterface::DigitGrid {
            min_digit: 4,
            max_digit: 1,
            cell_count: 0,
        };
        assert_eq!(
            parse_mathlive_answer("1", &invalid),
            Err(EditorError::InputInterfaceViolation)
        );
    }

    #[test]
    fn parses_representative_mathlive_values() {
        let interface = structured();
        let cases = [
            ("12", AnswerNode::Integer(12)),
            (
                "-12",
                AnswerNode::Negative(Box::new(AnswerNode::Integer(12))),
            ),
            ("1.25", AnswerNode::exact_decimal(125, 2)),
            (
                "\\frac{3}{4}",
                AnswerNode::Fraction {
                    numerator: Box::new(AnswerNode::Integer(3)),
                    denominator: Box::new(AnswerNode::Integer(4)),
                },
            ),
            (
                "\\frac72",
                AnswerNode::Fraction {
                    numerator: Box::new(AnswerNode::Integer(7)),
                    denominator: Box::new(AnswerNode::Integer(2)),
                },
            ),
            (
                "\\frac{11}{1}",
                AnswerNode::Fraction {
                    numerator: Box::new(AnswerNode::Integer(11)),
                    denominator: Box::new(AnswerNode::Integer(1)),
                },
            ),
            (
                "\\sqrt2",
                AnswerNode::Root {
                    radicand: Box::new(AnswerNode::Integer(2)),
                    index: None,
                },
            ),
            (
                "1\\frac{1}{2}",
                AnswerNode::MixedFraction {
                    whole: Box::new(AnswerNode::Integer(1)),
                    numerator: Box::new(AnswerNode::Integer(1)),
                    denominator: Box::new(AnswerNode::Integer(2)),
                },
            ),
            (
                "\\sqrt{\\frac{1}{2}}",
                AnswerNode::Root {
                    radicand: Box::new(AnswerNode::Fraction {
                        numerator: Box::new(AnswerNode::Integer(1)),
                        denominator: Box::new(AnswerNode::Integer(2)),
                    }),
                    index: None,
                },
            ),
            (
                "\\pm\\sqrt{2}",
                AnswerNode::PlusMinus(Box::new(AnswerNode::Root {
                    radicand: Box::new(AnswerNode::Integer(2)),
                    index: None,
                })),
            ),
            (
                "1,2",
                AnswerNode::Tuple(vec![AnswerNode::Integer(1), AnswerNode::Integer(2)]),
            ),
        ];

        for (latex, expected) in cases {
            assert_eq!(parse_mathlive_answer(latex, &interface).unwrap(), expected);
        }
    }

    #[test]
    fn distinguishes_prefix_negative_from_infix_subtraction_unambiguously() {
        let interface = structured();
        let cases = [
            ("-3", AnswerNode::Negative(Box::new(AnswerNode::Integer(3)))),
            (
                "5-3",
                AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Subtract,
                    left: Box::new(AnswerNode::Integer(5)),
                    right: Box::new(AnswerNode::Integer(3)),
                },
            ),
            (
                "5--3",
                AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Subtract,
                    left: Box::new(AnswerNode::Integer(5)),
                    right: Box::new(AnswerNode::Negative(Box::new(AnswerNode::Integer(3)))),
                },
            ),
            (
                "-3-2",
                AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Subtract,
                    left: Box::new(AnswerNode::Negative(Box::new(AnswerNode::Integer(3)))),
                    right: Box::new(AnswerNode::Integer(2)),
                },
            ),
            (
                "3+-2",
                AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Add,
                    left: Box::new(AnswerNode::Integer(3)),
                    right: Box::new(AnswerNode::Negative(Box::new(AnswerNode::Integer(2)))),
                },
            ),
        ];
        for (latex, expected) in cases {
            assert_eq!(
                parse_mathlive_answer(latex, &interface).unwrap(),
                expected,
                "{latex}"
            );
        }
    }

    #[test]
    fn parses_quadratic_formula_answer_with_plus_minus_root_and_fraction() {
        let interface = structured();
        let parsed = parse_mathlive_answer(r"\frac{-3\pm2\sqrt{5}}{4}", &interface).unwrap();
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: AnswerBinaryOperator::Add,
                left: Box::new(AnswerNode::Negative(Box::new(AnswerNode::Integer(3)))),
                right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Multiply,
                    left: Box::new(AnswerNode::Integer(2)),
                    right: Box::new(AnswerNode::Root {
                        radicand: Box::new(AnswerNode::Integer(5)),
                        index: None,
                    }),
                }))),
            }),
            denominator: Box::new(AnswerNode::Integer(4)),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn accepts_nested_root_plus_minus_expression_within_the_ast_budget() {
        let interface = structured();
        let parsed = parse_mathlive_answer(r"\sqrt{57\pm\sqrt{99}}{42}", &interface).unwrap();
        let expected = AnswerNode::Binary {
            operator: AnswerBinaryOperator::Multiply,
            left: Box::new(AnswerNode::Root {
                radicand: Box::new(AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Add,
                    left: Box::new(AnswerNode::Integer(57)),
                    right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Root {
                        radicand: Box::new(AnswerNode::Integer(99)),
                        index: None,
                    }))),
                }),
                index: None,
            }),
            right: Box::new(AnswerNode::Integer(42)),
        };
        assert_eq!(parsed, expected);
        assert!(parsed.size() <= MAX_ANSWER_AST_SIZE);
    }

    #[test]
    fn quadratic_formula_mathlive_input_grades_equal_to_canonical_answer() {
        let interface = structured();
        let actual = parse_mathlive_answer(r"\frac{-3\pm2\sqrt{5}}{4}", &interface).unwrap();
        let expected = AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Binary {
                operator: AnswerBinaryOperator::Add,
                left: Box::new(AnswerNode::Integer(-3)),
                right: Box::new(AnswerNode::PlusMinus(Box::new(AnswerNode::Binary {
                    operator: AnswerBinaryOperator::Multiply,
                    left: Box::new(AnswerNode::Integer(2)),
                    right: Box::new(AnswerNode::Root {
                        radicand: Box::new(AnswerNode::Integer(5)),
                        index: None,
                    }),
                }))),
            }),
            denominator: Box::new(AnswerNode::Integer(4)),
        };
        assert!(crate::grade::grade_answer(&expected, &actual).is_correct());
    }

    #[test]
    fn grades_requested_redundant_and_multi_solution_mathlive_forms() {
        use crate::grade::grade_answer;
        use crate::model::GradeWarning;

        let interface = structured();
        let parse = |latex: &str| parse_mathlive_answer(latex, &interface).unwrap();

        let result = grade_answer(&AnswerNode::Integer(2), &parse("--2"));
        assert!(result.is_correct());
        assert!(result.warnings().contains(&GradeWarning::RedundantNegative));

        let plus_minus_two = AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(2)));
        let result = grade_answer(&plus_minus_two, &parse(r"\pm\pm2"));
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::RedundantPlusMinus));

        assert!(grade_answer(&plus_minus_two, &parse("2,-2")).is_correct());
        assert!(grade_answer(&plus_minus_two, &parse(r"\pm2")).is_correct());

        let offset_roots = AnswerNode::Tuple(vec![AnswerNode::Integer(-2), AnswerNode::Integer(6)]);
        let result = grade_answer(&offset_roots, &parse(r"2\pm4"));
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::SolutionListRequired));
        assert!(grade_answer(&offset_roots, &parse("-2,6")).is_correct());

        let result = grade_answer(&AnswerNode::Integer(2), &parse("2,2"));
        assert!(!result.is_correct());
        assert!(result.warnings().contains(&GradeWarning::DuplicateSolution));
        assert!(grade_answer(&AnswerNode::Integer(2), &parse("2")).is_correct());

        let result = grade_answer(&AnswerNode::Integer(4), &parse(r"\sqrt{16}"));
        assert!(result.is_correct());
        assert!(result
            .warnings()
            .contains(&GradeWarning::IntegerFormRequired));
    }

    #[test]
    fn preserves_mathlive_placeholders_as_empty_nodes() {
        let interface = structured();
        assert_eq!(
            parse_mathlive_answer("\\frac{\\placeholder{}}{2}", &interface).unwrap(),
            AnswerNode::Fraction {
                numerator: Box::new(AnswerNode::Empty),
                denominator: Box::new(AnswerNode::Integer(2)),
            }
        );
        assert_eq!(
            parse_mathlive_answer(
                "\\placeholder{}\\frac{\\placeholder{}}{\\placeholder{}}",
                &interface,
            )
            .unwrap(),
            AnswerNode::MixedFraction {
                whole: Box::new(AnswerNode::Empty),
                numerator: Box::new(AnswerNode::Empty),
                denominator: Box::new(AnswerNode::Empty),
            }
        );
    }

    #[test]
    fn rejects_pathological_latex_before_recursive_parsing() {
        let interface = structured();
        let deeply_nested = format!(
            "{}1{}",
            "\\sqrt{".repeat(MAX_ANSWER_AST_SIZE + 20),
            "}".repeat(MAX_ANSWER_AST_SIZE + 20)
        );
        assert!(matches!(
            parse_mathlive_answer(&deeply_nested, &interface),
            Err(EditorError::AnswerSizeLimit { .. })
        ));

        let oversized = "1".repeat(MAX_MATHLIVE_LATEX_BYTES + 1);
        assert!(matches!(
            parse_mathlive_answer(&oversized, &interface),
            Err(EditorError::AnswerSizeLimit { .. })
        ));

        let unary_chain = format!("{}1", "-".repeat(MAX_ANSWER_AST_SIZE + 20));
        assert!(matches!(
            parse_mathlive_answer(&unary_chain, &interface),
            Err(EditorError::AnswerSizeLimit { .. })
        ));

        let parenthesized = format!(
            "{}1{}",
            "(".repeat(MAX_ANSWER_AST_SIZE + 20),
            ")".repeat(MAX_ANSWER_AST_SIZE + 20)
        );
        assert!(matches!(
            parse_mathlive_answer(&parenthesized, &interface),
            Err(EditorError::AnswerSizeLimit { .. })
        ));
    }
}
