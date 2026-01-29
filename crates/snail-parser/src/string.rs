use pest::Parser;
use pest::iterators::Pair;
use snail_ast::{
    Argument, AssignTarget, Condition, ExceptHandler, Expr, FStringConversion, FStringExpr,
    FStringPart, ImportFromItems, ImportItem, Parameter, SourceSpan, Stmt, StringDelimiter,
    WithItem,
};
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
                let expr = parse_fstring_expr(expr_text, content_offset + expr_start, source)?;
                parts.push(FStringPart::Expr(expr));
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

    // Check that the entire expression text was consumed
    let consumed_end = pair.as_span().end();
    if consumed_end < expr_text.len() {
        let unconsumed_start = expr_offset + consumed_end;
        let unconsumed_end = expr_offset + expr_text.len();
        let unconsumed_text = &expr_text[consumed_end..];
        return Err(error_with_span(
            format!(
                "unexpected characters in f-string expression: {:?}",
                unconsumed_text
            ),
            span_from_offset(unconsumed_start, unconsumed_end, source),
            source,
        ));
    }

    let mut expr = crate::expr::parse_expr_pair(pair, expr_text)?;
    shift_expr_spans(&mut expr, expr_offset, source);
    Ok(expr)
}

pub fn parse_fstring_expr(
    expr_text: &str,
    expr_offset: usize,
    source: &str,
) -> Result<FStringExpr, ParseError> {
    let (expr_text, expr_offset, conversion, format_spec) =
        split_fstring_expr(expr_text, expr_offset, source)?;
    let expr = parse_inline_expr(expr_text, expr_offset, source)?;
    let format_spec = match format_spec {
        Some((spec_text, spec_offset)) => {
            Some(parse_fstring_parts(spec_text, spec_offset, source)?)
        }
        None => None,
    };
    Ok(FStringExpr {
        expr: Box::new(expr),
        conversion,
        format_spec,
    })
}

type FStringExprParts<'a> = (&'a str, usize, FStringConversion, Option<(&'a str, usize)>);

fn split_fstring_expr<'a>(
    expr_text: &'a str,
    expr_offset: usize,
    source: &str,
) -> Result<FStringExprParts<'a>, ParseError> {
    let bytes = expr_text.as_bytes();
    let mut i = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut conversion = FStringConversion::None;
    let mut expr_end = expr_text.len();
    let mut format_spec: Option<(&'a str, usize)> = None;
    let mut conversion_pos: Option<usize> = None;

    while i < bytes.len() {
        match bytes[i] {
            b'r' | b'b' => {
                if let Some(next) = bytes.get(i + 1) {
                    if *next == b'\'' || *next == b'"' {
                        if let Some(end) = skip_string_literal(bytes, i) {
                            i = end;
                            continue;
                        } else {
                            return Err(error_with_span(
                                "unterminated string in f-string expression",
                                span_from_offset(expr_offset + i, expr_offset + i + 1, source),
                                source,
                            ));
                        }
                    } else if (*next == b'r' || *next == b'b')
                        && bytes[i] != *next
                        && let Some(third) = bytes.get(i + 2)
                        && (*third == b'\'' || *third == b'"')
                    {
                        if let Some(end) = skip_string_literal(bytes, i) {
                            i = end;
                            continue;
                        } else {
                            return Err(error_with_span(
                                "unterminated string in f-string expression",
                                span_from_offset(expr_offset + i, expr_offset + i + 1, source),
                                source,
                            ));
                        }
                    }
                }
                i += 1;
            }
            b'\'' | b'"' => {
                if let Some(end) = skip_string_literal(bytes, i) {
                    i = end;
                } else {
                    return Err(error_with_span(
                        "unterminated string in f-string expression",
                        span_from_offset(expr_offset + i, expr_offset + i + 1, source),
                        source,
                    ));
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
                brace = brace.saturating_sub(1);
                i += 1;
            }
            b'!' if paren == 0 && bracket == 0 && brace == 0 => {
                if bytes.get(i + 1) == Some(&b'=') {
                    i += 2;
                    continue;
                }
                let conv_char = bytes.get(i + 1).copied();
                let parsed = match conv_char {
                    Some(b'r') => FStringConversion::Repr,
                    Some(b's') => FStringConversion::Str,
                    Some(b'a') => FStringConversion::Ascii,
                    _ => {
                        return Err(error_with_span(
                            "invalid f-string conversion (expected !r, !s, or !a)",
                            span_from_offset(
                                expr_offset + i,
                                expr_offset + (i + 1).min(expr_text.len()),
                                source,
                            ),
                            source,
                        ));
                    }
                };
                conversion = parsed;
                conversion_pos = Some(i);
                expr_end = i;
                break;
            }
            b':' if paren == 0 && bracket == 0 && brace == 0 => {
                expr_end = i;
                format_spec = Some((&expr_text[i + 1..], expr_offset + i + 1));
                i = bytes.len();
            }
            _ => i += 1,
        }
    }

    if let Some(conv_pos) = conversion_pos {
        let tail = &expr_text[conv_pos + 2..];
        let tail_offset = expr_offset + conv_pos + 2;
        let trimmed_tail = tail.trim_start();
        let trim_start = tail.len() - trimmed_tail.len();
        if trimmed_tail.is_empty() {
            // No format spec.
        } else if let Some(stripped) = trimmed_tail.strip_prefix(':') {
            let spec_offset = tail_offset + trim_start + 1;
            format_spec = Some((stripped, spec_offset));
        } else {
            return Err(error_with_span(
                "unexpected characters after f-string conversion",
                span_from_offset(
                    tail_offset + trim_start,
                    expr_offset + expr_text.len(),
                    source,
                ),
                source,
            ));
        }
    }

    let expr_slice = &expr_text[..expr_end];
    let trim_start = expr_slice.len() - expr_slice.trim_start().len();
    let trim_end = expr_slice.len() - expr_slice.trim_end().len();
    let trimmed_len = expr_slice.len().saturating_sub(trim_start + trim_end);
    if trimmed_len == 0 {
        return Err(error_with_span(
            "empty f-string expression",
            span_from_offset(expr_offset, expr_offset + expr_text.len(), source),
            source,
        ));
    }
    let trimmed_expr = &expr_slice[trim_start..trim_start + trimmed_len];
    let trimmed_offset = expr_offset + trim_start;
    Ok((trimmed_expr, trimmed_offset, conversion, format_spec))
}

pub fn find_fstring_expr_end(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 1usize;
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
                if paren == 0 && bracket == 0 && brace == 1 {
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
    Ok(normalize_fstring_parts(parts, unescape_string_text))
}

pub fn normalize_fstring_parts(
    parts: Vec<FStringPart>,
    unescape: fn(&str) -> String,
) -> Vec<FStringPart> {
    parts
        .into_iter()
        .map(|part| match part {
            FStringPart::Text(text) => FStringPart::Text(unescape(&text)),
            FStringPart::Expr(expr) => FStringPart::Expr(normalize_fstring_expr(expr, unescape)),
        })
        .collect()
}

fn normalize_fstring_expr(expr: FStringExpr, unescape: fn(&str) -> String) -> FStringExpr {
    let format_spec = expr
        .format_spec
        .map(|parts| normalize_fstring_parts(parts, unescape));
    FStringExpr {
        expr: expr.expr,
        conversion: expr.conversion,
        format_spec,
    }
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
        | Expr::Set { span, .. }
        | Expr::Dict { span, .. }
        | Expr::Slice { span, .. } => {
            *span = shift_span(span, offset, source);
        }
        Expr::FString { parts, span, .. } => {
            for part in parts {
                shift_fstring_part_spans(part, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Regex { pattern, span } => {
            if let snail_ast::RegexPattern::Interpolated(parts) = pattern {
                for part in parts {
                    shift_fstring_part_spans(part, offset, source);
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
        Expr::AugAssign {
            target,
            value,
            span,
            ..
        } => {
            shift_assign_target_spans(target, offset, source);
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::PrefixIncr { target, span, .. } | Expr::PostfixIncr { target, span, .. } => {
            shift_assign_target_spans(target, offset, source);
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
        Expr::Yield { value, span } => {
            if let Some(value) = value {
                shift_expr_spans(value, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Expr::YieldFrom { expr, span } => {
            shift_expr_spans(expr, offset, source);
            *span = shift_span(span, offset, source);
        }
        Expr::Lambda { params, body, span } => {
            for param in params {
                shift_param_spans(param, offset, source);
            }
            for stmt in body {
                shift_stmt_spans(stmt, offset, source);
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

fn shift_fstring_part_spans(part: &mut FStringPart, offset: usize, source: &str) {
    if let FStringPart::Expr(expr) = part {
        shift_expr_spans(&mut expr.expr, offset, source);
        if let Some(spec) = &mut expr.format_spec {
            for spec_part in spec {
                shift_fstring_part_spans(spec_part, offset, source);
            }
        }
    }
}

fn shift_block_spans(block: &mut [Stmt], offset: usize, source: &str) {
    for stmt in block {
        shift_stmt_spans(stmt, offset, source);
    }
}

fn shift_stmt_spans(stmt: &mut Stmt, offset: usize, source: &str) {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => {
            shift_condition_spans(cond, offset, source);
            shift_block_spans(body, offset, source);
            for (cond, block) in elifs {
                shift_condition_spans(cond, offset, source);
                shift_block_spans(block, offset, source);
            }
            if let Some(block) = else_body {
                shift_block_spans(block, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => {
            shift_condition_spans(cond, offset, source);
            shift_block_spans(body, offset, source);
            if let Some(block) = else_body {
                shift_block_spans(block, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => {
            shift_assign_target_spans(target, offset, source);
            shift_expr_spans(iter, offset, source);
            shift_block_spans(body, offset, source);
            if let Some(block) = else_body {
                shift_block_spans(block, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Def {
            params, body, span, ..
        } => {
            for param in params {
                shift_param_spans(param, offset, source);
            }
            shift_block_spans(body, offset, source);
            *span = shift_span(span, offset, source);
        }
        Stmt::Class { body, span, .. } => {
            shift_block_spans(body, offset, source);
            *span = shift_span(span, offset, source);
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => {
            shift_block_spans(body, offset, source);
            for handler in handlers {
                shift_except_handler_spans(handler, offset, source);
            }
            if let Some(block) = else_body {
                shift_block_spans(block, offset, source);
            }
            if let Some(block) = finally_body {
                shift_block_spans(block, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::With { items, body, span } => {
            for item in items {
                shift_with_item_spans(item, offset, source);
            }
            shift_block_spans(body, offset, source);
            *span = shift_span(span, offset, source);
        }
        Stmt::Return { value, span } => {
            if let Some(value) = value {
                shift_expr_spans(value, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Raise { value, from, span } => {
            if let Some(value) = value {
                shift_expr_spans(value, offset, source);
            }
            if let Some(from) = from {
                shift_expr_spans(from, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Assert {
            test,
            message,
            span,
        } => {
            shift_expr_spans(test, offset, source);
            if let Some(message) = message {
                shift_expr_spans(message, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Delete { targets, span } => {
            for target in targets {
                shift_assign_target_spans(target, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Break { span } | Stmt::Continue { span } | Stmt::Pass { span } => {
            *span = shift_span(span, offset, source);
        }
        Stmt::Import { items, span } => {
            for item in items {
                shift_import_item_spans(item, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::ImportFrom { items, span, .. } => {
            match items {
                ImportFromItems::Names(names) => {
                    for item in names {
                        shift_import_item_spans(item, offset, source);
                    }
                }
                ImportFromItems::Star { span: star_span } => {
                    *star_span = shift_span(star_span, offset, source);
                }
            }
            *span = shift_span(span, offset, source);
        }
        Stmt::Assign {
            targets,
            value,
            span,
        } => {
            for target in targets {
                shift_assign_target_spans(target, offset, source);
            }
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        Stmt::Expr { value, span, .. } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
    }
}

fn shift_condition_spans(cond: &mut Condition, offset: usize, source: &str) {
    match cond {
        Condition::Expr(expr) => shift_expr_spans(expr, offset, source),
        Condition::Let {
            target,
            value,
            guard,
            span,
        } => {
            shift_assign_target_spans(target, offset, source);
            shift_expr_spans(value, offset, source);
            if let Some(guard) = guard {
                shift_expr_spans(guard, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
    }
}

fn shift_with_item_spans(item: &mut WithItem, offset: usize, source: &str) {
    shift_expr_spans(&mut item.context, offset, source);
    if let Some(target) = &mut item.target {
        shift_assign_target_spans(target, offset, source);
    }
    item.span = shift_span(&item.span, offset, source);
}

fn shift_except_handler_spans(handler: &mut ExceptHandler, offset: usize, source: &str) {
    if let Some(type_name) = &mut handler.type_name {
        shift_expr_spans(type_name, offset, source);
    }
    shift_block_spans(&mut handler.body, offset, source);
    handler.span = shift_span(&handler.span, offset, source);
}

fn shift_import_item_spans(item: &mut ImportItem, offset: usize, source: &str) {
    item.span = shift_span(&item.span, offset, source);
}

fn shift_param_spans(param: &mut Parameter, offset: usize, source: &str) {
    match param {
        Parameter::Regular { default, span, .. } => {
            if let Some(default) = default {
                shift_expr_spans(default, offset, source);
            }
            *span = shift_span(span, offset, source);
        }
        Parameter::VarArgs { span, .. } | Parameter::KwArgs { span, .. } => {
            *span = shift_span(span, offset, source);
        }
    }
}

fn shift_assign_target_spans(target: &mut AssignTarget, offset: usize, source: &str) {
    match target {
        AssignTarget::Name { span, .. } => {
            *span = shift_span(span, offset, source);
        }
        AssignTarget::Attribute { value, span, .. } => {
            shift_expr_spans(value, offset, source);
            *span = shift_span(span, offset, source);
        }
        AssignTarget::Index { value, index, span } => {
            shift_expr_spans(value, offset, source);
            shift_expr_spans(index, offset, source);
            *span = shift_span(span, offset, source);
        }
        AssignTarget::Starred { target, span } => {
            shift_assign_target_spans(target, offset, source);
            *span = shift_span(span, offset, source);
        }
        AssignTarget::Tuple { elements, span } | AssignTarget::List { elements, span } => {
            for element in elements {
                shift_assign_target_spans(element, offset, source);
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
