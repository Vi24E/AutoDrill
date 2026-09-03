use crate::answer::AnswerNode;
use crate::error::EditorError;
use crate::model::{AnswerInputInterface, EditorStructure};
use crate::theme::ThemeInputProfile;

/// Resolve the canonical input capability for a theme profile.
///
/// Theme registration owns the semantic profile; this module is the single
/// projection from that profile to the problem-level wire capability.
pub(crate) fn input_interface(profile: ThemeInputProfile) -> AnswerInputInterface {
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
        ThemeInputProfile::LinearExpression => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![
                EditorStructure::Negative,
                EditorStructure::Arithmetic,
                EditorStructure::Variable,
            ],
        },
        ThemeInputProfile::JuniorHighFull => AnswerInputInterface::StructuredMath {
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
        },
        ThemeInputProfile::TupleOnly => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Tuple],
        },
        ThemeInputProfile::DecimalTuple => AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Decimal, EditorStructure::Tuple],
        },
        ThemeInputProfile::DigitGrid(spec) => AnswerInputInterface::DigitGrid {
            min_digit: spec.min_digit(),
            max_digit: spec.max_digit(),
            cell_count: spec.cell_count(),
        },
    }
}

/// Validate that an already-parsed answer uses only capabilities exposed by the
/// selected theme. Caret/selection state is deliberately absent: MathLive owns
/// editing state, while Rust owns mathematical input validation.
pub(crate) fn ensure_capability(
    answer: &AnswerNode,
    input_interface: &AnswerInputInterface,
) -> Result<(), EditorError> {
    if let AnswerInputInterface::DigitGrid {
        min_digit,
        max_digit,
        cell_count,
    } = input_interface
    {
        return match answer {
            AnswerNode::Empty => Ok(()),
            AnswerNode::Tuple(values) if values.len() == usize::from(*cell_count) => {
                if values.iter().all(|value| match value {
                    AnswerNode::Empty => true,
                    AnswerNode::Integer(value) => {
                        i64::from(*min_digit) <= *value && *value <= i64::from(*max_digit)
                    }
                    _ => false,
                }) {
                    Ok(())
                } else {
                    Err(EditorError::InputInterfaceViolation)
                }
            }
            _ => Err(EditorError::InputInterfaceViolation),
        };
    }

    match answer {
        AnswerNode::Empty | AnswerNode::NanError(_) => Ok(()),
        AnswerNode::Integer(value) => {
            if *value < 0 {
                ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            }
            Ok(())
        }
        AnswerNode::ExactDecimal { coefficient, .. } => {
            ensure_structure_allowed(input_interface, EditorStructure::Decimal)?;
            if *coefficient < 0 {
                ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            }
            Ok(())
        }
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => {
            ensure_structure_allowed(input_interface, EditorStructure::Fraction)?;
            ensure_capability(numerator, input_interface)?;
            ensure_capability(denominator, input_interface)
        }
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            ensure_structure_allowed(input_interface, EditorStructure::MixedFraction)?;
            ensure_capability(whole, input_interface)?;
            ensure_capability(numerator, input_interface)?;
            ensure_capability(denominator, input_interface)
        }
        AnswerNode::Root { radicand, index } => {
            ensure_structure_allowed(input_interface, EditorStructure::Root)?;
            ensure_capability(radicand, input_interface)?;
            if let Some(index) = index {
                ensure_capability(index, input_interface)?;
            }
            Ok(())
        }
        AnswerNode::Negative(value) => {
            ensure_structure_allowed(input_interface, EditorStructure::Negative)?;
            ensure_capability(value, input_interface)
        }
        AnswerNode::PlusMinus(value) => {
            ensure_structure_allowed(input_interface, EditorStructure::PlusMinus)?;
            ensure_capability(value, input_interface)
        }
        AnswerNode::Binary { left, right, .. } => {
            ensure_structure_allowed(input_interface, EditorStructure::Arithmetic)?;
            ensure_capability(left, input_interface)?;
            ensure_capability(right, input_interface)
        }
        AnswerNode::Tuple(values) => {
            ensure_structure_allowed(input_interface, EditorStructure::Tuple)?;
            for value in values {
                ensure_capability(value, input_interface)?;
            }
            Ok(())
        }
        AnswerNode::Variable(_) => {
            ensure_structure_allowed(input_interface, EditorStructure::Variable)
        }
    }
}

fn ensure_structure_allowed(
    input_interface: &AnswerInputInterface,
    structure: EditorStructure,
) -> Result<(), EditorError> {
    if input_interface.allows_structure(structure) {
        Ok(())
    } else {
        Err(EditorError::StructureNotAllowed { structure })
    }
}
