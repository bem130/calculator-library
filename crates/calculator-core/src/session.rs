use alloc::string::String;

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
            let mut next = state.clone();
            next.source.push(char::from(b'0' + digit));
            next.cursor_utf8 = next.source.len() as u32;
            Ok(SessionReduction {
                state: next,
                command: SessionCommand::None,
            })
        }
        InputAction::Digit(_) => Err(InputError {
            kind: InputErrorKind::InvalidDigit,
        }),
        _ => Ok(SessionReduction {
            state: state.clone(),
            command: SessionCommand::None,
        }),
    }
}

pub fn apply_calculation_result(
    state: &InputState,
    result: Result<CalculationOutcome, CalculatorError>,
) -> InputState {
    let mut next = state.clone();
    next.display = match result {
        Ok(CalculationOutcome::Complete(calculation)) => SessionDisplay::Result {
            calculation: alloc::boxed::Box::new(calculation),
        },
        Ok(CalculationOutcome::Partial { calculation, .. }) => SessionDisplay::Result {
            calculation: alloc::boxed::Box::new(calculation),
        },
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
}
