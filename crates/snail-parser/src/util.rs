use pest::iterators::Pair;
use snail_ast::{SourcePos, SourceSpan};
use snail_error::ParseError;

use crate::Rule;

pub fn full_span(source: &str) -> SourceSpan {
    let end_offset = source.len();
    let (end_line, end_col) = line_col_from_offset(source, end_offset);
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line: 1,
            column: 1,
        },
        end: SourcePos {
            offset: end_offset,
            line: end_line,
            column: end_col,
        },
    }
}

pub fn span_from_pair(pair: &Pair<'_, Rule>, source: &str) -> SourceSpan {
    span_from_span(pair.as_span(), source)
}

pub fn span_from_span(span: pest::Span<'_>, source: &str) -> SourceSpan {
    let start_offset = span.start();
    let end_offset = span.end();
    let (start_line, start_col) = line_col_from_offset(source, start_offset);
    let (end_line, end_col) = line_col_from_offset(source, end_offset);
    SourceSpan {
        start: SourcePos {
            offset: start_offset,
            line: start_line,
            column: start_col,
        },
        end: SourcePos {
            offset: end_offset,
            line: end_line,
            column: end_col,
        },
    }
}

pub fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start: left.start.clone(),
        end: right.end.clone(),
    }
}

pub fn span_from_offset(start: usize, end: usize, source: &str) -> SourceSpan {
    let (start_line, start_col) = line_col_from_offset(source, start);
    let (end_line, end_col) = line_col_from_offset(source, end);
    SourceSpan {
        start: SourcePos {
            offset: start,
            line: start_line,
            column: start_col,
        },
        end: SourcePos {
            offset: end,
            line: end_line,
            column: end_col,
        },
    }
}

pub fn error_with_span(message: impl Into<String>, span: SourceSpan, source: &str) -> ParseError {
    let mut err = ParseError::new(message);
    err.line_text = line_text(source, span.start.line);
    err.span = Some(span);
    err
}

pub fn line_text(source: &str, line: usize) -> Option<String> {
    if line == 0 {
        return None;
    }
    source.lines().nth(line - 1).map(|s| s.to_string())
}

pub fn line_col_from_offset(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn parse_error_from_pest(err: pest::error::Error<Rule>, source: &str) -> ParseError {
    use pest::error::InputLocation;
    let message = err.to_string();
    let span = match err.location {
        InputLocation::Pos(pos) => Some(span_from_offset(pos, pos, source)),
        InputLocation::Span((start, end)) => Some(span_from_offset(start, end, source)),
    };
    let mut error = ParseError::new(message);
    if let Some(span) = span {
        error.line_text = line_text(source, span.start.line);
        error.span = Some(span);
    }
    error
}

pub fn parse_error_from_pest_with_offset(
    err: pest::error::Error<Rule>,
    source: &str,
    offset: usize,
) -> ParseError {
    use pest::error::InputLocation;
    let message = err.to_string();
    let span = match err.location {
        InputLocation::Pos(pos) => Some(span_from_offset(offset + pos, offset + pos, source)),
        InputLocation::Span((start, end)) => {
            Some(span_from_offset(offset + start, offset + end, source))
        }
    };
    let mut error = ParseError::new(message);
    if let Some(span) = span {
        error.line_text = line_text(source, span.start.line);
        error.span = Some(span);
    }
    error
}

pub fn expr_span(expr: &snail_ast::Expr) -> &SourceSpan {
    match expr {
        snail_ast::Expr::Name { span, .. }
        | snail_ast::Expr::Placeholder { span, .. }
        | snail_ast::Expr::Number { span, .. }
        | snail_ast::Expr::String { span, .. }
        | snail_ast::Expr::FString { span, .. }
        | snail_ast::Expr::Bool { span, .. }
        | snail_ast::Expr::None { span }
        | snail_ast::Expr::Unary { span, .. }
        | snail_ast::Expr::Binary { span, .. }
        | snail_ast::Expr::AugAssign { span, .. }
        | snail_ast::Expr::PrefixIncr { span, .. }
        | snail_ast::Expr::PostfixIncr { span, .. }
        | snail_ast::Expr::Compare { span, .. }
        | snail_ast::Expr::IfExpr { span, .. }
        | snail_ast::Expr::TryExpr { span, .. }
        | snail_ast::Expr::Yield { span, .. }
        | snail_ast::Expr::YieldFrom { span, .. }
        | snail_ast::Expr::Lambda { span, .. }
        | snail_ast::Expr::Compound { span, .. }
        | snail_ast::Expr::Regex { span, .. }
        | snail_ast::Expr::RegexMatch { span, .. }
        | snail_ast::Expr::Subprocess { span, .. }
        | snail_ast::Expr::StructuredAccessor { span, .. }
        | snail_ast::Expr::Call { span, .. }
        | snail_ast::Expr::Attribute { span, .. }
        | snail_ast::Expr::Index { span, .. }
        | snail_ast::Expr::Paren { span, .. }
        | snail_ast::Expr::FieldIndex { span, .. }
        | snail_ast::Expr::List { span, .. }
        | snail_ast::Expr::Tuple { span, .. }
        | snail_ast::Expr::Set { span, .. }
        | snail_ast::Expr::Dict { span, .. }
        | snail_ast::Expr::ListComp { span, .. }
        | snail_ast::Expr::DictComp { span, .. }
        | snail_ast::Expr::Slice { span, .. } => span,
    }
}
