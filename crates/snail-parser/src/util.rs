use pest::iterators::Pair;
use snail_ast::{SourcePos, SourceSpan};
use snail_error::ParseError;

use crate::Rule;

/// Precomputed index mapping byte offsets to line/column positions.
/// Replaces the O(n) `line_col_from_offset` scan with an O(log n) binary search.
pub struct LineIndex<'a> {
    source: &'a str,
    /// Byte offset of the start of each line (0-indexed internally).
    /// `line_starts[0]` is always 0.
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            source,
            line_starts,
        }
    }

    /// Convert a byte offset to a 1-based (line, column) pair.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Binary search: find the last line_start <= offset
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line = line_idx + 1; // 1-based
        let col = offset - self.line_starts[line_idx] + 1; // 1-based
        (line, col)
    }

    pub fn source(&self) -> &'a str {
        self.source
    }
}

pub fn is_keyword_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::kw_if
            | Rule::kw_else
            | Rule::kw_elif
            | Rule::kw_while
            | Rule::kw_for
            | Rule::kw_in
            | Rule::kw_def
            | Rule::kw_class
            | Rule::kw_return
            | Rule::kw_break
            | Rule::kw_continue
            | Rule::kw_pass
            | Rule::kw_raise
            | Rule::kw_try
            | Rule::kw_except
            | Rule::kw_finally
            | Rule::kw_with
            | Rule::kw_assert
            | Rule::kw_del
            | Rule::kw_and
            | Rule::kw_or
            | Rule::kw_not
            | Rule::kw_import
            | Rule::kw_from
            | Rule::kw_as
            | Rule::kw_yield
            | Rule::kw_let
            | Rule::kw_awk
            | Rule::kw_xargs
            | Rule::kw_true
            | Rule::kw_false
            | Rule::kw_none
            | Rule::kw_is
    )
}

pub fn full_span(lx: &LineIndex<'_>) -> SourceSpan {
    let end_offset = lx.source().len();
    let (end_line, end_col) = lx.line_col(end_offset);
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

pub fn span_from_pair(pair: &Pair<'_, Rule>, lx: &LineIndex<'_>) -> SourceSpan {
    span_from_span(pair.as_span(), lx)
}

pub fn span_from_span(span: pest::Span<'_>, lx: &LineIndex<'_>) -> SourceSpan {
    let start_offset = span.start();
    let end_offset = span.end();
    let (start_line, start_col) = lx.line_col(start_offset);
    let (end_line, end_col) = lx.line_col(end_offset);
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

pub fn span_from_offset(start: usize, end: usize, lx: &LineIndex<'_>) -> SourceSpan {
    let (start_line, start_col) = lx.line_col(start);
    let (end_line, end_col) = lx.line_col(end);
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

pub fn error_with_span(
    message: impl Into<String>,
    span: SourceSpan,
    lx: &LineIndex<'_>,
) -> ParseError {
    let mut err = ParseError::new(message);
    err.line_text = line_text(lx.source(), span.start.line);
    err.span = Some(span);
    err
}

pub fn line_text(source: &str, line: usize) -> Option<String> {
    if line == 0 {
        return None;
    }
    source.lines().nth(line - 1).map(|s| s.to_string())
}

pub fn parse_error_from_pest(err: pest::error::Error<Rule>, lx: &LineIndex<'_>) -> ParseError {
    use pest::error::InputLocation;
    let message = err.variant.message().into_owned();
    let span = match err.location {
        InputLocation::Pos(pos) => Some(span_from_offset(pos, pos, lx)),
        InputLocation::Span((start, end)) => Some(span_from_offset(start, end, lx)),
    };
    let mut error = ParseError::new(message);
    if let Some(span) = span {
        error.line_text = line_text(lx.source(), span.start.line);
        error.span = Some(span);
    }
    error
}

pub fn parse_error_from_pest_with_offset(
    err: pest::error::Error<Rule>,
    lx: &LineIndex<'_>,
    offset: usize,
) -> ParseError {
    use pest::error::InputLocation;
    let message = err.variant.message().into_owned();
    let span = match err.location {
        InputLocation::Pos(pos) => Some(span_from_offset(offset + pos, offset + pos, lx)),
        InputLocation::Span((start, end)) => {
            Some(span_from_offset(offset + start, offset + end, lx))
        }
    };
    let mut error = ParseError::new(message);
    if let Some(span) = span {
        error.line_text = line_text(lx.source(), span.start.line);
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
        | snail_ast::Expr::Yield { span, .. }
        | snail_ast::Expr::YieldFrom { span, .. }
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
        | snail_ast::Expr::GeneratorExpr { span, .. }
        | snail_ast::Expr::Slice { span, .. }
        | snail_ast::Expr::Block { span, .. }
        | snail_ast::Expr::If { span, .. }
        | snail_ast::Expr::While { span, .. }
        | snail_ast::Expr::For { span, .. }
        | snail_ast::Expr::Def { span, .. }
        | snail_ast::Expr::Class { span, .. }
        | snail_ast::Expr::Try { span, .. }
        | snail_ast::Expr::With { span, .. }
        | snail_ast::Expr::Starred { span, .. }
        | snail_ast::Expr::Awk { span, .. }
        | snail_ast::Expr::Xargs { span, .. } => span,
    }
}
