use alloc::{
    format,
    string::{String, ToString},
};

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
        InputAction::Percent => percent(state, policy),
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

fn percent(state: &InputState, policy: &InputPolicy) -> Result<SessionReduction, InputError> {
    match policy.percent_policy {
        PercentPolicy::ExpressionPercent => edit_source(state, "%"),
        PercentPolicy::CalculatorPercent => calculator_percent(state),
    }
}

fn calculator_percent(state: &InputState) -> Result<SessionReduction, InputError> {
    let mut next = editable_state(state);
    let (start, end) = selected_or_cursor_span(&next)?;
    if start != end {
        next.source.replace_range(start..end, "%");
        next.cursor_utf8 = (start + 1) as u32;
        next.selection_utf8 = OptionalTextSpan::None;
        return Ok(no_command(next));
    }

    if let Some(rewrite) = calculator_percent_rewrite(&next.source, start) {
        next.source
            .replace_range(rewrite.start..rewrite.end, &rewrite.text);
        next.cursor_utf8 = (rewrite.start + rewrite.text.len()) as u32;
    } else {
        next.source.replace_range(start..start, "%");
        next.cursor_utf8 = (start + 1) as u32;
    }
    next.selection_utf8 = OptionalTextSpan::None;
    Ok(no_command(next))
}

struct PercentRewrite {
    start: usize,
    end: usize,
    text: String,
}

fn calculator_percent_rewrite(source: &str, cursor: usize) -> Option<PercentRewrite> {
    let operand_end = trim_end_whitespace(source, cursor);
    let container_start = containing_expression_start(source, operand_end);
    let operator_start = find_additive_operator(source, container_start, operand_end)?;
    let rhs_start = trim_start_whitespace(source, operator_start + 1, operand_end);
    if rhs_start >= operand_end {
        return None;
    }

    let base = source[container_start..operator_start].trim();
    let rhs = source[rhs_start..operand_end].trim();
    if base.is_empty() || rhs.is_empty() {
        return None;
    }

    Some(PercentRewrite {
        start: rhs_start,
        end: operand_end,
        text: format!("(({})*({})/100)", base, rhs),
    })
}

fn containing_expression_start(source: &str, end: usize) -> usize {
    let mut cursor = end;
    let mut depth = 0_u32;
    while let Some((index, ch)) = previous_char(source, cursor) {
        match ch {
            ')' => depth += 1,
            '(' if depth == 0 => return index + ch.len_utf8(),
            '(' => depth -= 1,
            _ => {}
        }
        cursor = index;
    }
    0
}

fn find_additive_operator(source: &str, start: usize, end: usize) -> Option<usize> {
    let mut cursor = end;
    let mut depth = 0_u32;
    while cursor > start {
        let (index, ch) = previous_char(source, cursor)?;
        match ch {
            ')' => depth += 1,
            '(' if depth > 0 => depth -= 1,
            '+' | '-' if depth == 0 && is_binary_additive_operator(source, index, start, end) => {
                return Some(index);
            }
            _ => {}
        }
        cursor = index;
    }
    None
}

fn is_binary_additive_operator(source: &str, index: usize, start: usize, end: usize) -> bool {
    if is_number_exponent_sign(source, index) {
        return false;
    }
    let Some((previous_index, previous)) = previous_non_whitespace(source, index) else {
        return false;
    };
    if previous_index < start {
        return false;
    }
    if matches!(previous, '(' | '+' | '-' | '*' | '/' | '^') {
        return false;
    }
    next_non_whitespace(source, index + 1).is_some_and(|(next_index, _)| next_index < end)
}

fn is_number_exponent_sign(source: &str, index: usize) -> bool {
    let Some((exponent_index, exponent)) = previous_char(source, index) else {
        return false;
    };
    if exponent != 'e' && exponent != 'E' {
        return false;
    }
    let Some((_, before_exponent)) = previous_char(source, exponent_index) else {
        return false;
    };
    let Some((_, after_sign)) = next_char(source, index + 1) else {
        return false;
    };
    before_exponent.is_ascii_digit() && after_sign.is_ascii_digit()
}

fn trim_start_whitespace(source: &str, start: usize, end: usize) -> usize {
    let mut cursor = start;
    while cursor < end {
        let ch = source[cursor..].chars().next().expect("cursor is in range");
        if !ch.is_whitespace() {
            return cursor;
        }
        cursor += ch.len_utf8();
    }
    end
}

fn trim_end_whitespace(source: &str, end: usize) -> usize {
    let mut cursor = end;
    while let Some((index, ch)) = previous_char(source, cursor) {
        if !ch.is_whitespace() {
            return cursor;
        }
        cursor = index;
    }
    0
}

fn previous_non_whitespace(source: &str, cursor: usize) -> Option<(usize, char)> {
    let mut current = cursor;
    while let Some((index, ch)) = previous_char(source, current) {
        if !ch.is_whitespace() {
            return Some((index, ch));
        }
        current = index;
    }
    None
}

fn next_non_whitespace(source: &str, cursor: usize) -> Option<(usize, char)> {
    let mut current = cursor;
    while let Some((index, ch)) = next_char(source, current) {
        if !ch.is_whitespace() {
            return Some((index, ch));
        }
        current = index + ch.len_utf8();
    }
    None
}

fn previous_char(source: &str, cursor: usize) -> Option<(usize, char)> {
    source[..cursor].char_indices().last()
}

fn next_char(source: &str, cursor: usize) -> Option<(usize, char)> {
    source[cursor..]
        .char_indices()
        .next()
        .map(|(offset, ch)| (cursor + offset, ch))
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
        reduce_sequence_with_policy(actions, InputPolicy::default())
    }

    fn reduce_sequence_with_percent_policy(
        actions: &[InputAction],
        percent_policy: PercentPolicy,
    ) -> SessionReduction {
        reduce_sequence_with_policy(
            actions,
            InputPolicy {
                percent_policy,
                ..InputPolicy::default()
            },
        )
    }

    fn reduce_sequence_with_policy(
        actions: &[InputAction],
        policy: InputPolicy,
    ) -> SessionReduction {
        let mut state = InputState::empty();
        let mut reduction = no_command(state.clone());
        for action in actions {
            reduction =
                reduce_input(&state, action.clone(), &policy).expect("action should reduce");
            state = reduction.state.clone();
        }
        reduction
    }

    fn exact_plain_text(source: &str) -> String {
        let request = CalculationRequest {
            scientific_output: ScientificOutputRequest::Omit,
            enclosure_output: EnclosureOutputRequest::Omit,
            ..CalculationRequest::default()
        };
        let mut context = EvaluationContext::default();
        let outcome =
            crate::calculate(source, &request, &mut context).expect("source should calculate");
        let CalculationOutcome::Complete(calculation) = outcome else {
            panic!("expected complete calculation");
        };
        let ExactOutput::Included(exact) = calculation.exact else {
            panic!("expected exact output");
        };
        exact.plain_text
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
    fn expression_percent_keeps_string_api_meaning() {
        let reduction = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Add),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::ExpressionPercent,
        );
        let SessionCommand::Calculate { source, .. } = reduction.command else {
            panic!("expected calculate command");
        };
        assert_eq!(source, "100+10%");
        assert_eq!(exact_plain_text(&source), "1001/10");
    }

    #[test]
    fn calculator_percent_uses_additive_left_operand() {
        let addition = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Add),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate {
            source: addition, ..
        } = addition.command
        else {
            panic!("expected calculate command");
        };
        assert_eq!(addition, "100+((100)*(10)/100)");
        assert_eq!(exact_plain_text(&addition), "110");

        let subtraction = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Subtract),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate {
            source: subtraction,
            ..
        } = subtraction.command
        else {
            panic!("expected calculate command");
        };
        assert_eq!(subtraction, "100-((100)*(10)/100)");
        assert_eq!(exact_plain_text(&subtraction), "90");
    }

    #[test]
    fn calculator_percent_keeps_product_and_repeated_percent_semantics() {
        let product = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Multiply),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate {
            source: product, ..
        } = product.command
        else {
            panic!("expected calculate command");
        };
        assert_eq!(product, "100*10%");
        assert_eq!(exact_plain_text(&product), "10");

        let division = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Divide),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate {
            source: division, ..
        } = division.command
        else {
            panic!("expected calculate command");
        };
        assert_eq!(division, "100/10%");
        assert_eq!(exact_plain_text(&division), "1000");

        let repeated = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(5),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::Percent,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate {
            source: repeated, ..
        } = repeated.command
        else {
            panic!("expected calculate command");
        };
        assert_eq!(repeated, "50%%");
        assert_eq!(exact_plain_text(&repeated), "1/200");
    }

    #[test]
    fn calculator_percent_uses_nearest_parenthesized_additive_context() {
        let reduction = reduce_sequence_with_percent_policy(
            &[
                InputAction::Digit(2),
                InputAction::BinaryOperator(BinaryOperator::Multiply),
                InputAction::OpenParenthesis,
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Digit(0),
                InputAction::BinaryOperator(BinaryOperator::Add),
                InputAction::Digit(1),
                InputAction::Digit(0),
                InputAction::Percent,
                InputAction::CloseParenthesis,
                InputAction::Evaluate,
            ],
            PercentPolicy::CalculatorPercent,
        );
        let SessionCommand::Calculate { source, .. } = reduction.command else {
            panic!("expected calculate command");
        };
        assert_eq!(source, "2*(100+((100)*(10)/100))");
        assert_eq!(exact_plain_text(&source), "220");
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
