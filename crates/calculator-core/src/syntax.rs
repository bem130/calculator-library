use alloc::{boxed::Box, string::String, vec, vec::Vec};

use crate::types::{
    BinaryOperator, ByteSpan, Constant, ExpectedToken, ExpectedTokenKind, Function,
    ImplicitMultiplicationPolicy, ParseError, ParseErrorKind, ParseSettings, PercentParsePolicy,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceExpr {
    Number {
        literal: String,
        span: ByteSpan,
    },
    Constant {
        constant: Constant,
        span: ByteSpan,
    },
    Unary {
        op: UnaryOperator,
        expr: Box<SourceExpr>,
        span: ByteSpan,
    },
    Binary {
        op: BinaryOperator,
        left: Box<SourceExpr>,
        right: Box<SourceExpr>,
        implicit: bool,
        span: ByteSpan,
    },
    Percent {
        expr: Box<SourceExpr>,
        span: ByteSpan,
    },
    Function {
        function: Function,
        argument: Box<SourceExpr>,
        base: Option<Box<SourceExpr>>,
        span: ByteSpan,
    },
}

pub(crate) struct SourceExprStats {
    pub nodes: u32,
    pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryOperator {
    Plus,
    Negate,
}

pub(crate) fn parse_source(
    source: &str,
    settings: &ParseSettings,
) -> Result<SourceExpr, ParseError> {
    let tokens = lex(source)?;
    let mut parser = Parser {
        tokens,
        position: 0,
        settings,
    };
    let expression = parser.parse_expression()?;
    if let Some(token) = parser.peek() {
        return Err(ParseError {
            kind: ParseErrorKind::UnexpectedToken,
            span: token.span,
            expected: vec![ExpectedToken {
                kind: ExpectedTokenKind::EndOfInput,
            }],
        });
    }
    Ok(expression)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Number(String),
    Constant(Constant),
    Function(Function),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    OpenParen,
    CloseParen,
    Comma,
}

fn lex(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        let ch = next_char(source, cursor);
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }

        if ch.is_ascii_digit() {
            let (literal, end) = lex_number(source, cursor)?;
            tokens.push(Token {
                kind: TokenKind::Number(literal),
                span: span(cursor, end),
            });
            cursor = end;
            continue;
        }

        if ch == '.' {
            return Err(ParseError {
                kind: ParseErrorKind::InvalidNumberLiteral,
                span: span(cursor, cursor + ch.len_utf8()),
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::Number,
                }],
            });
        }

        if ch.is_ascii_alphabetic() {
            let (identifier, end) = lex_identifier(source, cursor);
            let kind = identifier_token(&identifier).ok_or_else(|| ParseError {
                kind: ParseErrorKind::UnknownIdentifier,
                span: span(cursor, end),
                expected: Vec::new(),
            })?;
            tokens.push(Token {
                kind,
                span: span(cursor, end),
            });
            cursor = end;
            continue;
        }

        if ch == 'π' {
            let end = cursor + ch.len_utf8();
            tokens.push(Token {
                kind: TokenKind::Constant(Constant::Pi),
                span: span(cursor, end),
            });
            cursor = end;
            continue;
        }

        let kind = match ch {
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '^' => TokenKind::Caret,
            '%' => TokenKind::Percent,
            '!' => TokenKind::Bang,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            ',' => TokenKind::Comma,
            _ => {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken,
                    span: span(cursor, cursor + ch.len_utf8()),
                    expected: Vec::new(),
                });
            }
        };
        tokens.push(Token {
            kind,
            span: span(cursor, cursor + ch.len_utf8()),
        });
        cursor += ch.len_utf8();
    }

    Ok(tokens)
}

fn lex_number(source: &str, start: usize) -> Result<(String, usize), ParseError> {
    let mut cursor = consume_ascii_digits(source, start);

    if source[cursor..].starts_with('.') {
        cursor += 1;
        let after_digits = consume_ascii_digits(source, cursor);
        if after_digits == cursor {
            return Err(ParseError {
                kind: ParseErrorKind::InvalidNumberLiteral,
                span: span(start, cursor),
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::Number,
                }],
            });
        }
        cursor = after_digits;
        if let Some(end) = consume_exponent(source, cursor)? {
            cursor = end;
        }
        return Ok((String::from(&source[start..cursor]), cursor));
    }

    if let Some(end) = consume_exponent(source, cursor)? {
        cursor = end;
    }
    Ok((String::from(&source[start..cursor]), cursor))
}

fn consume_exponent(source: &str, cursor: usize) -> Result<Option<usize>, ParseError> {
    let Some(ch) = source[cursor..].chars().next() else {
        return Ok(None);
    };
    if ch != 'e' && ch != 'E' {
        return Ok(None);
    }

    let exponent = cursor;
    let mut next = cursor + 1;
    if let Some(sign) = source[next..].chars().next() {
        if sign == '+' || sign == '-' {
            next += 1;
        }
    }
    let after_digits = consume_ascii_digits(source, next);
    if after_digits == next {
        return Err(ParseError {
            kind: ParseErrorKind::InvalidNumberLiteral,
            span: span(exponent, next),
            expected: vec![ExpectedToken {
                kind: ExpectedTokenKind::Number,
            }],
        });
    }
    Ok(Some(after_digits))
}

fn lex_identifier(source: &str, start: usize) -> (String, usize) {
    let mut cursor = start;
    while cursor < source.len() {
        let ch = next_char(source, cursor);
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    (String::from(&source[start..cursor]), cursor)
}

fn identifier_token(identifier: &str) -> Option<TokenKind> {
    Some(match identifier {
        "pi" => TokenKind::Constant(Constant::Pi),
        "e" => TokenKind::Constant(Constant::Euler),
        "sin" => TokenKind::Function(Function::Sin),
        "cos" => TokenKind::Function(Function::Cos),
        "tan" => TokenKind::Function(Function::Tan),
        "asin" => TokenKind::Function(Function::Asin),
        "acos" => TokenKind::Function(Function::Acos),
        "atan" => TokenKind::Function(Function::Atan),
        "sqrt" => TokenKind::Function(Function::Sqrt),
        "root" => TokenKind::Function(Function::Root),
        "exp" => TokenKind::Function(Function::Exp),
        "log" => TokenKind::Function(Function::Log),
        "ln" => TokenKind::Function(Function::Ln),
        "abs" => TokenKind::Function(Function::Abs),
        "floor" => TokenKind::Function(Function::Floor),
        "fact" => TokenKind::Function(Function::Factorial),
        "factorial" => TokenKind::Function(Function::Factorial),
        "perm" => TokenKind::Function(Function::Permutation),
        "npr" => TokenKind::Function(Function::Permutation),
        "comb" => TokenKind::Function(Function::Combination),
        "ncr" => TokenKind::Function(Function::Combination),
        "mod" => TokenKind::Function(Function::Modulo),
        "gcd" => TokenKind::Function(Function::Gcd),
        "lcm" => TokenKind::Function(Function::Lcm),
        "lcd" => TokenKind::Function(Function::Lcm),
        "sinh" => TokenKind::Function(Function::Sinh),
        "cosh" => TokenKind::Function(Function::Cosh),
        "tanh" => TokenKind::Function(Function::Tanh),
        "asinh" => TokenKind::Function(Function::Asinh),
        "acosh" => TokenKind::Function(Function::Acosh),
        "atanh" => TokenKind::Function(Function::Atanh),
        _ => return None,
    })
}

fn consume_ascii_digits(source: &str, start: usize) -> usize {
    let mut cursor = start;
    while cursor < source.len() {
        let ch = next_char(source, cursor);
        if ch.is_ascii_digit() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

struct Parser<'a> {
    tokens: Vec<Token>,
    position: usize,
    settings: &'a ParseSettings,
}

impl Parser<'_> {
    fn parse_expression(&mut self) -> Result<SourceExpr, ParseError> {
        self.parse_sum()
    }

    fn parse_sum(&mut self) -> Result<SourceExpr, ParseError> {
        let mut expr = self.parse_product()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinaryOperator::Add,
                Some(TokenKind::Minus) => BinaryOperator::Subtract,
                Some(
                    TokenKind::Number(_)
                    | TokenKind::Constant(_)
                    | TokenKind::Function(_)
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Caret
                    | TokenKind::Percent
                    | TokenKind::Bang
                    | TokenKind::OpenParen
                    | TokenKind::CloseParen
                    | TokenKind::Comma,
                )
                | None => return Ok(expr),
            };
            self.advance();
            let right = self.parse_product()?;
            expr = SourceExpr::Binary {
                span: union(expr.span(), right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
                implicit: false,
            };
        }
    }

    fn parse_product(&mut self) -> Result<SourceExpr, ParseError> {
        let mut expr = self.parse_prefix()?;
        loop {
            let Some(next) = self.peek() else {
                return Ok(expr);
            };
            let (op, implicit) = if next.kind == TokenKind::Star {
                (BinaryOperator::Multiply, false)
            } else if next.kind == TokenKind::Slash {
                (BinaryOperator::Divide, false)
            } else if starts_primary(&next.kind) {
                if self.settings.implicit_multiplication == ImplicitMultiplicationPolicy::Disabled {
                    return Err(ParseError {
                        kind: ParseErrorKind::ImplicitMultiplicationDisabled,
                        span: next.span,
                        expected: vec![ExpectedToken {
                            kind: ExpectedTokenKind::Operator,
                        }],
                    });
                }
                (BinaryOperator::Multiply, true)
            } else {
                return Ok(expr);
            };
            if !implicit {
                self.advance();
            }
            let right = self.parse_prefix()?;
            expr = SourceExpr::Binary {
                span: union(expr.span(), right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
                implicit,
            };
        }
    }

    fn parse_prefix(&mut self) -> Result<SourceExpr, ParseError> {
        let Some(token) = self.peek() else {
            return Err(self.unexpected_end());
        };
        match token.kind {
            TokenKind::Plus => {
                let start = token.span;
                self.advance();
                let expr = self.parse_prefix()?;
                Ok(SourceExpr::Unary {
                    span: union(start, expr.span()),
                    op: UnaryOperator::Plus,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Minus => {
                let start = token.span;
                self.advance();
                let expr = self.parse_prefix()?;
                Ok(SourceExpr::Unary {
                    span: union(start, expr.span()),
                    op: UnaryOperator::Negate,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Number(_)
            | TokenKind::Constant(_)
            | TokenKind::Function(_)
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Caret
            | TokenKind::Percent
            | TokenKind::Bang
            | TokenKind::OpenParen
            | TokenKind::Comma
            | TokenKind::CloseParen => self.parse_percent(),
        }
    }

    fn parse_percent(&mut self) -> Result<SourceExpr, ParseError> {
        let mut expr = self.parse_power()?;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Percent => {
                    let percent_span = token.span;
                    if self.settings.percent == PercentParsePolicy::RejectPercent {
                        return Err(ParseError {
                            kind: ParseErrorKind::PercentRejected,
                            span: percent_span,
                            expected: Vec::new(),
                        });
                    }
                    self.advance();
                    expr = SourceExpr::Percent {
                        span: union(expr.span(), percent_span),
                        expr: Box::new(expr),
                    };
                }
                TokenKind::Bang => {
                    let bang_span = token.span;
                    let span = union(expr.span(), bang_span);
                    self.advance();
                    expr = SourceExpr::Function {
                        function: Function::Factorial,
                        argument: Box::new(expr),
                        base: None,
                        span,
                    };
                }
                TokenKind::Number(_)
                | TokenKind::Constant(_)
                | TokenKind::Function(_)
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Caret
                | TokenKind::OpenParen
                | TokenKind::CloseParen
                | TokenKind::Comma => return Ok(expr),
            }
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<SourceExpr, ParseError> {
        let left = self.parse_primary()?;
        let Some(token) = self.peek() else {
            return Ok(left);
        };
        if token.kind != TokenKind::Caret {
            return Ok(left);
        }
        self.advance();
        let right = self.parse_prefix()?;
        Ok(SourceExpr::Binary {
            span: union(left.span(), right.span()),
            op: BinaryOperator::Power,
            left: Box::new(left),
            right: Box::new(right),
            implicit: false,
        })
    }

    fn parse_primary(&mut self) -> Result<SourceExpr, ParseError> {
        let Some(token) = self.take() else {
            return Err(self.unexpected_end());
        };
        match token.kind {
            TokenKind::Number(literal) => Ok(SourceExpr::Number {
                literal,
                span: token.span,
            }),
            TokenKind::Constant(constant) => Ok(SourceExpr::Constant {
                constant,
                span: token.span,
            }),
            TokenKind::Function(function) => self.parse_function(function, token.span),
            TokenKind::OpenParen => {
                let expr = self.parse_expression()?;
                let Some(close) = self.peek() else {
                    return Err(self.unexpected_end());
                };
                if close.kind != TokenKind::CloseParen {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnexpectedToken,
                        span: close.span,
                        expected: vec![ExpectedToken {
                            kind: ExpectedTokenKind::CloseParenthesis,
                        }],
                    });
                }
                let close_span = close.span;
                self.advance();
                Ok(with_span(expr, union(token.span, close_span)))
            }
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Caret
            | TokenKind::Percent
            | TokenKind::Bang
            | TokenKind::Comma
            | TokenKind::CloseParen => Err(ParseError {
                kind: ParseErrorKind::UnexpectedToken,
                span: token.span,
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::Number,
                }],
            }),
        }
    }

    fn parse_function(
        &mut self,
        function: Function,
        function_span: ByteSpan,
    ) -> Result<SourceExpr, ParseError> {
        let Some(open) = self.peek() else {
            return Err(ParseError {
                kind: ParseErrorKind::MissingFunctionParenthesis,
                span: function_span,
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::OpenParenthesis,
                }],
            });
        };
        if open.kind != TokenKind::OpenParen {
            return Err(ParseError {
                kind: ParseErrorKind::MissingFunctionParenthesis,
                span: function_span,
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::OpenParenthesis,
                }],
            });
        }
        self.advance();
        let argument = self.parse_expression()?;
        let Some(separator_or_close) = self.peek() else {
            return Err(self.unexpected_end());
        };
        let base = if separator_or_close.kind == TokenKind::Comma {
            if !function_accepts_explicit_base(function) {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken,
                    span: separator_or_close.span,
                    expected: vec![ExpectedToken {
                        kind: ExpectedTokenKind::CloseParenthesis,
                    }],
                });
            }
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            if function_requires_explicit_base(function) {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken,
                    span: separator_or_close.span,
                    expected: vec![ExpectedToken {
                        kind: ExpectedTokenKind::Comma,
                    }],
                });
            }
            None
        };
        let Some(close) = self.peek() else {
            return Err(self.unexpected_end());
        };
        if close.kind != TokenKind::CloseParen {
            return Err(ParseError {
                kind: ParseErrorKind::UnexpectedToken,
                span: close.span,
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::CloseParenthesis,
                }],
            });
        }
        let span = union(function_span, close.span);
        self.advance();
        Ok(SourceExpr::Function {
            function,
            argument: Box::new(argument),
            base,
            span,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|token| &token.kind)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get_mut(self.position)?;
        let span = token.span;
        let token = core::mem::replace(
            token,
            Token {
                kind: TokenKind::Plus,
                span,
            },
        );
        self.position += 1;
        Some(token)
    }

    fn unexpected_end(&self) -> ParseError {
        let offset = self
            .tokens
            .last()
            .map(|token| token.span.end)
            .unwrap_or_default();
        ParseError {
            kind: ParseErrorKind::UnexpectedEnd,
            span: ByteSpan {
                start: offset,
                end: offset,
            },
            expected: vec![ExpectedToken {
                kind: ExpectedTokenKind::Number,
            }],
        }
    }
}

impl SourceExpr {
    pub(crate) fn stats(&self) -> Option<SourceExprStats> {
        let mut stack = vec![(self, 1_u32)];
        let mut nodes = 0_u32;
        let mut depth = 0_u32;

        while let Some((expr, current_depth)) = stack.pop() {
            nodes = nodes.checked_add(1)?;
            depth = depth.max(current_depth);
            let child_depth = current_depth.checked_add(1)?;
            match expr {
                Self::Number { .. } | Self::Constant { .. } => {}
                Self::Unary { expr, .. } | Self::Percent { expr, .. } => {
                    stack.push((expr, child_depth));
                }
                Self::Binary { left, right, .. } => {
                    stack.push((left, child_depth));
                    stack.push((right, child_depth));
                }
                Self::Function { argument, base, .. } => {
                    stack.push((argument, child_depth));
                    if let Some(base) = base {
                        stack.push((base, child_depth));
                    }
                }
            }
        }

        Some(SourceExprStats { nodes, depth })
    }

    fn span(&self) -> ByteSpan {
        match self {
            Self::Number { span, .. }
            | Self::Constant { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Percent { span, .. }
            | Self::Function { span, .. } => *span,
        }
    }
}

fn starts_primary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Number(_)
            | TokenKind::Constant(_)
            | TokenKind::Function(_)
            | TokenKind::OpenParen
    )
}

fn with_span(expr: SourceExpr, span: ByteSpan) -> SourceExpr {
    match expr {
        SourceExpr::Number { literal, .. } => SourceExpr::Number { literal, span },
        SourceExpr::Constant { constant, .. } => SourceExpr::Constant { constant, span },
        SourceExpr::Unary { op, expr, .. } => SourceExpr::Unary { op, expr, span },
        SourceExpr::Binary {
            op,
            left,
            right,
            implicit,
            ..
        } => SourceExpr::Binary {
            op,
            left,
            right,
            implicit,
            span,
        },
        SourceExpr::Percent { expr, .. } => SourceExpr::Percent { expr, span },
        SourceExpr::Function {
            function,
            argument,
            base,
            ..
        } => SourceExpr::Function {
            function,
            argument,
            base,
            span,
        },
    }
}

fn function_accepts_explicit_base(function: Function) -> bool {
    matches!(
        function,
        Function::Exp
            | Function::Log
            | Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
    )
}

fn function_requires_explicit_base(function: Function) -> bool {
    matches!(
        function,
        Function::Log
            | Function::Root
            | Function::Permutation
            | Function::Combination
            | Function::Modulo
            | Function::Gcd
            | Function::Lcm
    )
}

fn next_char(source: &str, cursor: usize) -> char {
    source[cursor..]
        .chars()
        .next()
        .expect("cursor must be inside source")
}

fn span(start: usize, end: usize) -> ByteSpan {
    ByteSpan {
        start: start as u32,
        end: end as u32,
    }
}

fn union(left: ByteSpan, right: ByteSpan) -> ByteSpan {
    ByteSpan {
        start: left.start,
        end: right.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BinaryOperator, ImplicitMultiplicationPolicy, ParseSettings, PercentParsePolicy,
    };

    fn parse_ok(source: &str) -> SourceExpr {
        parse_source(source, &ParseSettings::default()).expect(source)
    }

    fn parse_err(source: &str) -> ParseErrorKind {
        parse_source(source, &ParseSettings::default())
            .expect_err(source)
            .kind
    }

    #[test]
    fn power_is_right_associative() {
        let expr = parse_ok("2^3^2");
        let SourceExpr::Binary {
            op: BinaryOperator::Power,
            right,
            ..
        } = expr
        else {
            panic!("expected power");
        };
        assert!(matches!(
            *right,
            SourceExpr::Binary {
                op: BinaryOperator::Power,
                ..
            }
        ));
    }

    #[test]
    fn unary_minus_has_lower_precedence_than_power() {
        let expr = parse_ok("-2^2");
        let SourceExpr::Unary {
            op: UnaryOperator::Negate,
            expr,
            ..
        } = expr
        else {
            panic!("expected unary minus");
        };
        assert!(matches!(
            *expr,
            SourceExpr::Binary {
                op: BinaryOperator::Power,
                ..
            }
        ));
    }

    #[test]
    fn power_exponent_accepts_unary_minus() {
        let expr = parse_ok("2^-3");
        let SourceExpr::Binary {
            op: BinaryOperator::Power,
            right,
            ..
        } = expr
        else {
            panic!("expected power");
        };
        assert!(matches!(
            *right,
            SourceExpr::Unary {
                op: UnaryOperator::Negate,
                ..
            }
        ));
    }

    #[test]
    fn implicit_multiplication_matches_division_precedence() {
        let expr = parse_ok("2/3π");
        let SourceExpr::Binary {
            op: BinaryOperator::Multiply,
            implicit: true,
            left,
            ..
        } = expr
        else {
            panic!("expected implicit multiply");
        };
        assert!(matches!(
            *left,
            SourceExpr::Binary {
                op: BinaryOperator::Divide,
                ..
            }
        ));
    }

    #[test]
    fn decimal_literals_require_digits_on_both_sides() {
        assert_eq!(parse_err(".5"), ParseErrorKind::InvalidNumberLiteral);
        assert_eq!(parse_err("1."), ParseErrorKind::InvalidNumberLiteral);
        parse_ok("1.2e-3");
        parse_ok("1e+2");
    }

    #[test]
    fn function_call_requires_parentheses() {
        assert_eq!(
            parse_err("sin 30"),
            ParseErrorKind::MissingFunctionParenthesis
        );
        parse_ok("sin(30)");
    }

    #[test]
    fn consumed_primary_tokens_preserve_success_and_error_contracts() {
        parse_ok("2sin((3+4))");

        for (source, offset) in [("(1", 2), ("sin(1", 5), ("1+", 2)] {
            assert_eq!(
                parse_source(source, &ParseSettings::default()).expect_err(source),
                ParseError {
                    kind: ParseErrorKind::UnexpectedEnd,
                    span: ByteSpan {
                        start: offset,
                        end: offset,
                    },
                    expected: vec![ExpectedToken {
                        kind: ExpectedTokenKind::Number,
                    }],
                }
            );
        }

        assert_eq!(
            parse_source("sin 1", &ParseSettings::default()).expect_err("sin 1"),
            ParseError {
                kind: ParseErrorKind::MissingFunctionParenthesis,
                span: ByteSpan { start: 0, end: 3 },
                expected: vec![ExpectedToken {
                    kind: ExpectedTokenKind::OpenParenthesis,
                }],
            }
        );
    }

    #[test]
    fn reserved_non_numbers_are_rejected() {
        assert_eq!(parse_err("nan"), ParseErrorKind::UnknownIdentifier);
        assert_eq!(parse_err("undefined"), ParseErrorKind::UnknownIdentifier);
        assert_eq!(parse_err("null"), ParseErrorKind::UnknownIdentifier);
    }

    #[test]
    fn percent_policy_can_reject_percent_token() {
        let settings = ParseSettings {
            percent: PercentParsePolicy::RejectPercent,
            ..ParseSettings::default()
        };
        assert_eq!(
            parse_source("50%", &settings)
                .expect_err("percent should be rejected")
                .kind,
            ParseErrorKind::PercentRejected
        );
    }

    #[test]
    fn implicit_multiplication_can_be_disabled() {
        let settings = ParseSettings {
            implicit_multiplication: ImplicitMultiplicationPolicy::Disabled,
            ..ParseSettings::default()
        };
        assert_eq!(
            parse_source("2π", &settings)
                .expect_err("implicit multiplication should be rejected")
                .kind,
            ParseErrorKind::ImplicitMultiplicationDisabled
        );
    }
}
