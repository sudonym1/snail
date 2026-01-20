use pest::Parser;
use pest::iterators::Pair;
use snail_ast::{Argument, Expr, FStringPart, SourceSpan, StringDelimiter};
use snail_error::ParseError;

use crate::util::{
    error_with_span, parse_error_from_pest_with_offset, span_from_offset, span_from_pair,
};
use crate::{Rule, SnailParser};

pub fn parse_string_or_fstring(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let parsed = parse_string_literal(pair)?;

    // Raw strings should not have f-string interpolation
    if parsed.raw {
        return Ok(Expr::String {
            value: parsed.content,
            raw: true,
            bytes: parsed.bytes,
            delimiter: parsed.delimiter,
            span,
        });
    }

    let parts = parse_fstring_parts(&parsed.content, parsed.content_offset, source)?;
    let has_expr = parts
        .iter()
        .any(|part| matches!(part, FStringPart::Expr(_)));
    if has_expr {
        let parts = normalize_string_parts(parts, parsed.raw)?;
        Ok(Expr::FString {
            parts,
            bytes: parsed.bytes,
            span,
        })
    } else {
        let value = join_fstring_text(parts);
        Ok(Expr::String {
            value,
            raw: parsed.raw,
            bytes: parsed.bytes,
            delimiter: parsed.delimiter,
            span,
        })
    }
}

pub struct ParsedStringLiteral {
    pub content: String,
    pub raw: bool,
    pub bytes: bool,
    pub delimiter: StringDelimiter,
    pub content_offset: usize,
}

pub fn parse_string_literal(pair: Pair<'_, Rule>) -> Result<ParsedStringLiteral, ParseError> {
    let value = pair.as_str();
    let span = pair.as_span();
    // Parse prefix - check longer prefixes first
    let (raw, bytes, rest, prefix_len) = if let Some(stripped) = value.strip_prefix("br") {
        (true, true, stripped, 2usize)
    } else if let Some(stripped) = value.strip_prefix("rb") {
        (true, true, stripped, 2usize)
    } else if let Some(stripped) = value.strip_prefix('b') {
        (false, true, stripped, 1usize)
    } else if let Some(stripped) = value.strip_prefix('r') {
        (true, false, stripped, 1usize)
    } else {
        (false, false, value, 0usize)
    };
    let (delimiter, open, close) = if rest.starts_with("\"\"\"") {
        (StringDelimiter::TripleDouble, "\"\"\"", "\"\"\"")
    } else if rest.starts_with("'''") {
        (StringDelimiter::TripleSingle, "'''", "'''")
    } else if rest.starts_with('"') {
        (StringDelimiter::Double, "\"", "\"")
    } else {
        (StringDelimiter::Single, "'", "'")
    };
    let content = if rest.len() >= open.len() + close.len() {
        &rest[open.len()..rest.len() - close.len()]
    } else {
        ""
    };
    let content_offset = span.start() + prefix_len + open.len();
    Ok(ParsedStringLiteral {
        content: content.to_string(),
        raw,
        bytes,
        delimiter,
        content_offset,
    })
}

pub fn parse_fstring_parts(
    content: &str,
    content_offset: usize,
    source: &str,
) -> Result<Vec<FStringPart>, ParseError> {
    let bytes = content.as_bytes();
    let mut parts = Vec::new();
    let mut text_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    i += 2;
                    continue;
                }
                if text_start < i {
                    parts.push(FStringPart::Text(content[text_start..i].to_string()));
                }
                let expr_start = i + 1;
                let expr_end = find_fstring_expr_end(content, expr_start).ok_or_else(|| {
                    error_with_span(
                        "unterminated f-string expression",
                        span_from_offset(content_offset + i, content_offset + i + 1, source),
                        source,
                    )
                })?;
                let expr_text = &content[expr_start..expr_end];
                if expr_text.trim().is_empty() {
                    return Err(error_with_span(
                        "empty f-string expression",
                        span_from_offset(content_offset + i, content_offset + expr_end + 1, source),
                        source,
                    ));
                }
                let expr = parse_inline_expr(expr_text, content_offset + expr_start, source)?;
                parts.push(FStringPart::Expr(Box::new(expr)));
                i = expr_end + 1;
                text_start = i;
            }
            b'}' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                    i += 2;
                    continue;
                }
                return Err(error_with_span(
                    "unmatched '}' in f-string",
                    span_from_offset(content_offset + i, content_offset + i + 1, source),
                    source,
                ));
            }
            _ => i += 1,
        }
    }
    if text_start < bytes.len() {
        parts.push(FStringPart::Text(content[text_start..].to_string()));
    }
    for part in parts.iter_mut() {
        if let FStringPart::Text(text) = part {
            *text = text.replace("{{", "{").replace("}}", "}");
        }
    }
    Ok(parts)
}

pub fn parse_inline_expr(
    expr_text: &str,
    expr_offset: usize,
    source: &str,
) -> Result<Expr, ParseError> {
    let mut pairs = SnailParser::parse(Rule::expr, expr_text)
        .map_err(|err| parse_error_from_pest_with_offset(err, source, expr_offset))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing f-string expression"))?;
    let mut expr = crate::expr::parse_expr_pair(pair, expr_text)?;
    shift_expr_spans(&mut expr, expr_offset, source);
    Ok(expr)
}

pub fn find_fstring_expr_end(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'r' | b'b' => {
                // Check for string prefix combinations: r, b, rb, br
                if let Some(next) = bytes.get(i + 1) {
                    if *next == b'\'' || *next == b'"' {
                        // r"..." or b"..."
                        if let Some(end) = skip_string_literal(bytes, i) {
                            i = end;
                            continue;
                        } else {
                            return None;
                        }
                    } else if (*next == b'r' || *next == b'b') && bytes[i] != *next {
                        // Could be rb"..." or br"..."
                        if let Some(third) = bytes.get(i + 2)
                            && (*third == b'\'' || *third == b'"')
                        {
                            if let Some(end) = skip_string_literal(bytes, i) {
                                i = end;
                                continue;
                            } else {
                                return None;
                            }
                        }
                    }
                }
                i += 1;
            }
            b'\'' | b'"' => {
                if let Some(end) = skip_string_literal(bytes, i) {
                    i = end;
                } else {
                    return None;
                }
            }
            b'(' => {
                paren += 1;
                i += 1;
            }
            b')' => {
                paren = paren.saturating_sub(1);
                i += 1;
            }
            b'[' => {
                bracket += 1;
                i += 1;
            }
            b']' => {
                bracket = bracket.saturating_sub(1);
                i += 1;
            }
            b'{' => {
                brace += 1;
                i += 1;
            }
            b'}' => {
                if paren == 0 && bracket == 0 && brace == 0 {
                    return Some(i);
                }
                brace = brace.saturating_sub(1);
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

pub fn skip_string_literal(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    // Handle prefixes: br, rb, b, r (check longer prefixes first)
    let raw = if bytes.get(i..i + 2) == Some(b"br") || bytes.get(i..i + 2) == Some(b"rb") {
        i += 2;
        true
    } else if bytes.get(i) == Some(&b'b') {
        i += 1;
        false
    } else if bytes.get(i) == Some(&b'r') {
        i += 1;
        true
    } else {
        false
    };
    let quote = *bytes.get(i)?;
    let (delim_len, delim) = if bytes.get(i..i + 3) == Some(&[quote, quote, quote]) {
        (3usize, vec![quote, quote, quote])
    } else {
        (1usize, vec![quote])
    };
    i += delim_len;
    while i < bytes.len() {
        if bytes.get(i..i + delim_len) == Some(delim.as_slice()) {
            return Some(i + delim_len);
        }
        if !raw && bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        i += 1;
    }
    None
}

pub fn normalize_string_parts(
    parts: Vec<FStringPart>,
    raw: bool,
) -> Result<Vec<FStringPart>, ParseError> {
    if raw {
        return Ok(parts);
    }
    let mut normalized = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => {
                normalized.push(FStringPart::Text(unescape_string_text(&text)));
            }
            FStringPart::Expr(expr) => normalized.push(FStringPart::Expr(expr)),
        }
    }
    Ok(normalized)
}

pub fn unescape_string_text(text: &str) -> String {
    unescape_text(text, false)
}

pub fn unescape_regex_text(text: &str) -> String {
    unescape_text(text, true)
}

pub fn unescape_text(text: &str, escape_slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('\\') => out.push('\\'),
            Some('/') if escape_slash => out.push('/'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

pub fn join_fstring_text(parts: Vec<FStringPart>) -> String {
    let mut text = String::new();
    for part in parts {
        if let FStringPart::Text(value) = part {
            text.push_str(&value);
        }
    }
    text
}

pub fn shift_expr_spans(expr: &mut Expr, offset: usize, source: &str) {
    match expr {
        Expr::Name { span, .. }
        | Expr::Placeholder { span, .. }
        | Expr::Number { span, .. }
        | Expr::String { span, .. }
        | Expr::Bool { span, .. }
        | Expr::None { span }
        | Expr::Subprocess { span, .. }
        | Expr::StructuredAccessor { span, .. }
        | Expr::FieldIndex { span, .. }
        | Expr::List { span, .. }
        | Expr::Tuple { span, .. }
        | Expr::Dict { span, .. }
        | Expr::Slice { span, .. } => {
            *span = shift_span(span, offset, source);
        }
        Expr::FString { parts, span, .. } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part {
                    shift_expr_spans(expr, offset, source);
                }
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Regex { pattern, span } => {
            if let snail_ast::RegexPattern::Interpolated(parts) = pattern {
                for part in parts {
                    if let FStringPart::Expr(expr) = part {
                        shift_expr_spans(expr, offset, source);
                    }
                }
            }
            *span = shift_span(span, offset, source);
        }
        Expr::RegexMatch { value, span, .. } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Unary { expr, span, .. } => {
            shift_expr_spans(expr, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Binary {
            left, right, span, ..
        } => {
            shift_expr_spans(left, offset, source);
            shift_expr_spans(right, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Compare {
            left,
            comparators,
            span,
            ..
        } => {
            shift_expr_spans(left, offset, source);
            for expr in comparators {
                shift_expr_spans(expr, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => {
            shift_expr_spans(test, offset, source);
            shift_expr_spans(body, offset, source);
            shift_expr_spans(orelse, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => {
            shift_expr_spans(expr, offset, source);
            if let Some(fallback) = fallback {
                shift_expr_spans(fallback, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Compound { expressions, span } => {
            for expr in expressions {
                shift_expr_spans(expr, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Call { func, args, span } => {
            shift_expr_spans(func, offset, source);
            for arg in args {
                shift_argument_spans(arg, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Attribute { value, span, .. } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Index { value, index, span } => {
            shift_expr_spans(value, offset, source);
            shift_expr_spans(index, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Paren { expr, span } => {
            shift_expr_spans(expr, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::ListComp {
            element,
            iter,
            ifs,
            span,
            ..
        } => {
            shift_expr_spans(element, offset, source);
            shift_expr_spans(iter, offset, source);
            for cond in ifs {
                shift_expr_spans(cond, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            span,
            ..
        } => {
            shift_expr_spans(key, offset, source);
            shift_expr_spans(value, offset, source);
            shift_expr_spans(iter, offset, source);
            for cond in ifs {
                shift_expr_spans(cond, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
    }
}

fn shift_argument_spans(arg: &mut Argument, offset: usize, source: &str) {
    match arg {
        Argument::Positional { value, span } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Argument::Keyword { value, span, .. } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Argument::Star { value, span } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Argument::KwStar { value, span } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
    }
}

fn shift_span(span: &SourceSpan, offset: usize, source: &str) -> SourceSpan {
    span_from_offset(span.start.offset + offset, span.end.offset + offset, source)
}
