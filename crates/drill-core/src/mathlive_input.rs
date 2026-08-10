use crate::answer::AnswerNode;
use crate::editor::ensure_capability;
use crate::error::EditorError;
use crate::model::{AnswerInputInterface, MAX_ANSWER_AST_SIZE};

/// Parse the deliberately small MathLive/LaTeX answer language accepted by
/// AutoDrill. MathLive owns editing and layout; this adapter is the only place
/// where Web editor output becomes the Rust AnswerNode authority.
pub fn parse_mathlive_answer(
    latex: &str,
    input_interface: &AnswerInputInterface,
) -> Result<AnswerNode, EditorError> {
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
}

impl<'a> Parser<'a> {
    const fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.input.len()
    }

    fn parse_expression(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        if self.at_stop(stop) {
            return Ok(AnswerNode::Empty);
        }

        let first = self.parse_value(stop)?;
        if self.at_stop(stop) {
            return Ok(first);
        }

        if !self.consume_char(',') {
            return Err(());
        }

        let mut values = vec![first];
        loop {
            values.push(self.parse_value(stop)?);
            if self.at_stop(stop) {
                break;
            }
            if !self.consume_char(',') {
                return Err(());
            }
        }
        Ok(AnswerNode::Tuple(values))
    }

    fn parse_value(&mut self, stop: Option<char>) -> Result<AnswerNode, ()> {
        let prefix = if self.consume_char('-') || self.consume_char('−') {
            Some(Prefix::Negative)
        } else if self.consume_str("\\pm") || self.consume_char('±') {
            Some(Prefix::PlusMinus)
        } else {
            None
        };

        let mut value = if self.at_stop(stop) || self.peek_char() == Some(',') {
            AnswerNode::Empty
        } else {
            self.parse_atom(stop)?
        };

        if matches!(value, AnswerNode::Integer(_) | AnswerNode::Empty) && self.starts_with("\\frac")
        {
            let fraction = self.parse_fraction()?;
            if let AnswerNode::Fraction {
                numerator,
                denominator,
            } = fraction
            {
                value = AnswerNode::MixedFraction {
                    whole: Box::new(value),
                    numerator,
                    denominator,
                };
            }
        }

        Ok(match prefix {
            Some(Prefix::Negative) => AnswerNode::Negative(Box::new(value)),
            Some(Prefix::PlusMinus) => AnswerNode::PlusMinus(Box::new(value)),
            None => value,
        })
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
            let value = self.parse_expression(Some(']'))?;
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
        let value = self.parse_expression(Some('}'))?;
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

enum Prefix {
    Negative,
    PlusMinus,
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
            ],
        }
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
}
