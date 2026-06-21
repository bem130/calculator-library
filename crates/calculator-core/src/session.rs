use alloc::string::{String, ToString};

use crate::types::*;

pub fn reduce_input(
    state: &InputState,
    action: InputAction,
    policy: &InputPolicy,
) -> Result<SessionReduction, InputError> {
    match action {
        InputAction::Evaluate => Ok(SessionReduction {
            state: state.clone(),
            command: SessionCommand::Calculate {
                source: state.source.clone(),
                request: policy.calculation_request.clone(),
            },
        }),
        InputAction::Digit(digit) if digit <= 9 => {
            edit_source(state, &char::from(b'0' + digit).to_string())
        }
        InputAction::Digit(_) => Err(InputError {
            kind: InputErrorKind::InvalidDigit,
        }),
        InputAction::DecimalPoint => edit_source(state, "."),
        InputAction::Constant(constant) => edit_source(state, constant_source(constant)),
        InputAction::Ans => edit_source(state, "Ans"),
        InputAction::Function(function) => edit_source(state, function_source(function)),
        InputAction::BinaryOperator(operator) => {
            edit_source(state, binary_operator_source(operator))
        }
        InputAction::Percent => edit_source(state, "%"),
        InputAction::OpenParenthesis => edit_source(state, "("),
        InputAction::CloseParenthesis => edit_source(state, ")"),
        InputAction::DeleteBackward => delete_backward(state),
        InputAction::ClearEntry => {
            let mut next = state.clone();
            next.source.clear();
            next.cursor_utf8 = 0;
            next.selection_utf8 = OptionalTextSpan::None;
            next.display = SessionDisplay::Editing;
            Ok(no_command(next))
        }
        InputAction::ClearAll => {
            let mut next = state.clone();
            next.source.clear();
            next.cursor_utf8 = 0;
            next.selection_utf8 = OptionalTextSpan::None;
            next.display = SessionDisplay::Editing;
            next.has_ans = false;
            Ok(no_command(next))
        }
        InputAction::MemoryClear => {
            let mut next = state.clone();
            next.has_memory = false;
            Ok(no_command(next))
        }
        InputAction::MemoryRecall if !state.has_memory => Err(InputError {
            kind: InputErrorKind::MemoryEmpty,
        }),
        InputAction::MemoryRecall => edit_source(state, "M"),
        InputAction::MemoryAdd | InputAction::MemorySubtract => {
            let mut next = state.clone();
            next.has_memory = true;
            Ok(no_command(next))
        }
    }
}

pub fn apply_calculation_result(
    state: &InputState,
    result: Result<CalculationOutcome, CalculatorError>,
) -> InputState {
    let mut next = state.clone();
    next.display = match result {
        Ok(CalculationOutcome::Complete(calculation)) => {
            next.has_ans = true;
            SessionDisplay::Result {
                calculation: alloc::boxed::Box::new(calculation),
            }
        }
        Ok(CalculationOutcome::Partial { calculation, .. }) => {
            next.has_ans = true;
            SessionDisplay::Result {
                calculation: alloc::boxed::Box::new(calculation),
            }
        }
        Err(error) => SessionDisplay::Error { error },
    };
    next
}

impl InputState {
    pub fn empty() -> Self {
        Self {
            source: String::new(),
            cursor_utf8: 0,
            selection_utf8: OptionalTextSpan::None,
            has_ans: false,
            has_memory: false,
            display: SessionDisplay::Editing,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn cursor_utf8(&self) -> u32 {
        self.cursor_utf8
    }

    pub fn selection_utf8(&self) -> &OptionalTextSpan {
        &self.selection_utf8
    }

    pub fn has_ans(&self) -> bool {
        self.has_ans
    }

    pub fn has_memory(&self) -> bool {
        self.has_memory
    }

    pub fn display(&self) -> &SessionDisplay {
        &self.display
    }
}

fn edit_source(state: &InputState, text: &str) -> Result<SessionReduction, InputError> {
    let mut next = editable_state(state);
    let (start, end) = selected_or_cursor_span(&next)?;
    next.source.replace_range(start..end, text);
    next.cursor_utf8 = (start + text.len()) as u32;
    next.selection_utf8 = OptionalTextSpan::None;
    Ok(no_command(next))
}

fn delete_backward(state: &InputState) -> Result<SessionReduction, InputError> {
    let mut next = editable_state(state);
    let (start, end) = selected_or_cursor_span(&next)?;
    if start != end {
        next.source.replace_range(start..end, "");
        next.cursor_utf8 = start as u32;
        next.selection_utf8 = OptionalTextSpan::None;
        return Ok(no_command(next));
    }
    if start == 0 {
        return Ok(no_command(next));
    }
    let previous = next.source[..start]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .ok_or(InputError {
            kind: InputErrorKind::InvalidCursor,
        })?;
    next.source.replace_range(previous..start, "");
    next.cursor_utf8 = previous as u32;
    Ok(no_command(next))
}

fn editable_state(state: &InputState) -> InputState {
    let mut next = state.clone();
    if !matches!(next.display, SessionDisplay::Editing) {
        next.source.clear();
        next.cursor_utf8 = 0;
        next.selection_utf8 = OptionalTextSpan::None;
    }
    next.display = SessionDisplay::Editing;
    next
}

fn selected_or_cursor_span(state: &InputState) -> Result<(usize, usize), InputError> {
    let len = state.source.len();
    match state.selection_utf8 {
        OptionalTextSpan::None => {
            let cursor = state.cursor_utf8 as usize;
            if cursor > len || !state.source.is_char_boundary(cursor) {
                return Err(InputError {
                    kind: InputErrorKind::InvalidCursor,
                });
            }
            Ok((cursor, cursor))
        }
        OptionalTextSpan::Some(span) => {
            let start = span.start as usize;
            let end = span.end as usize;
            if start > end
                || end > len
                || !state.source.is_char_boundary(start)
                || !state.source.is_char_boundary(end)
            {
                return Err(InputError {
                    kind: InputErrorKind::SelectionOutOfBounds,
                });
            }
            Ok((start, end))
        }
    }
}

fn no_command(state: InputState) -> SessionReduction {
    SessionReduction {
        state,
        command: SessionCommand::None,
    }
}

fn constant_source(constant: Constant) -> &'static str {
    match constant {
        Constant::Pi => "pi",
        Constant::Euler => "e",
    }
}

fn function_source(function: Function) -> &'static str {
    match function {
        Function::Sin => "sin(",
        Function::Cos => "cos(",
        Function::Tan => "tan(",
        Function::Asin => "asin(",
        Function::Acos => "acos(",
        Function::Atan => "atan(",
        Function::Sqrt => "sqrt(",
        Function::Exp => "exp(",
        Function::Log => "log(",
    }
}

fn binary_operator_source(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Power => "^",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reduce_sequence(actions: &[InputAction]) -> SessionReduction {
        let policy = InputPolicy::default();
        let mut state = InputState::empty();
        let mut reduction = no_command(state.clone());
        for action in actions {
            reduction =
                reduce_input(&state, action.clone(), &policy).expect("action should reduce");
            state = reduction.state.clone();
        }
        reduction
    }

    #[test]
    fn action_sequence_returns_calculate_command_without_evaluating() {
        let reduction = reduce_sequence(&[
            InputAction::Digit(1),
            InputAction::BinaryOperator(BinaryOperator::Add),
            InputAction::Digit(2),
            InputAction::Evaluate,
        ]);
        assert_eq!(
            reduction.command,
            SessionCommand::Calculate {
                source: String::from("1+2"),
                request: CalculationRequest::default(),
            }
        );
    }

    #[test]
    fn function_and_parenthesis_actions_edit_source() {
        let reduction = reduce_sequence(&[
            InputAction::Function(Function::Sqrt),
            InputAction::Digit(2),
            InputAction::CloseParenthesis,
            InputAction::BinaryOperator(BinaryOperator::Multiply),
            InputAction::Constant(Constant::Pi),
        ]);
        assert_eq!(reduction.state.source, "sqrt(2)*pi");
    }

    #[test]
    fn delete_backward_removes_previous_utf8_scalar() {
        let state = InputState {
            source: String::from("π2"),
            cursor_utf8: "π".len() as u32,
            selection_utf8: OptionalTextSpan::None,
            has_ans: false,
            has_memory: false,
            display: SessionDisplay::Editing,
        };
        let reduction = reduce_input(&state, InputAction::DeleteBackward, &InputPolicy::default())
            .expect("delete should succeed");
        assert_eq!(reduction.state.source, "2");
        assert_eq!(reduction.state.cursor_utf8, 0);
    }

    #[test]
    fn clear_entry_preserves_ans_and_memory_but_clear_all_drops_ans() {
        let state = InputState {
            source: String::from("12"),
            cursor_utf8: 2,
            selection_utf8: OptionalTextSpan::None,
            has_ans: true,
            has_memory: true,
            display: SessionDisplay::Editing,
        };

        let entry = reduce_input(&state, InputAction::ClearEntry, &InputPolicy::default())
            .expect("clear entry should succeed")
            .state;
        assert!(entry.source.is_empty());
        assert!(entry.has_ans);
        assert!(entry.has_memory);

        let all = reduce_input(&state, InputAction::ClearAll, &InputPolicy::default())
            .expect("clear all should succeed")
            .state;
        assert!(all.source.is_empty());
        assert!(!all.has_ans);
        assert!(all.has_memory);
    }

    #[test]
    fn successful_calculation_updates_ans_but_error_does_not() {
        let state = InputState::empty();
        let calculation = Calculation {
            exact: ExactOutput::Omitted,
            scientific: ScientificOutput::Omitted,
            enclosure: EnclosureOutput::Omitted,
            metadata: CalculationMetadata {
                exact_representation: ExactRepresentationKind::Integer,
                simplification_status: SimplificationStatus::FullySimplifiedWithinLimits,
                semantic_settings: SemanticSettings::default(),
                methods: alloc::vec![],
                internal_precision_bits: 0,
                refinement_rounds: 0,
                confirmed_significant_digits: 0,
                assurance: AssuranceLevel::Exact,
                protocol_version: ProtocolVersion::CURRENT,
            },
        };

        let success =
            apply_calculation_result(&state, Ok(CalculationOutcome::Complete(calculation)));
        assert!(success.has_ans);

        let error = apply_calculation_result(
            &state,
            Err(CalculatorError::Domain(DomainError {
                kind: DomainErrorKind::DivisionByZero,
                span: None,
            })),
        );
        assert!(!error.has_ans);
    }
}
