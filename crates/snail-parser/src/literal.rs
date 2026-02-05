use pest::iterators::Pair;
use snail_ast::{Expr, FStringPart, RegexPattern, SourceSpan, SubprocessKind, SubprocessPart};
use snail_error::ParseError;

use crate::Rule;
use crate::string::{
    join_fstring_text, normalize_fstring_parts, parse_fstring_parts, parse_string_or_fstring,
    unescape_regex_text,
};
use crate::util::{error_with_span, span_from_pair};

pub fn parse_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing literal", pair_span, source))?;
    let span = span_from_pair(&inner, source);
    match inner.as_rule() {
        Rule::number => Ok(Expr::Number {
            value: inner.as_str().to_string(),
            span,
        }),
        Rule::string => parse_string_or_fstring(inner, source),
        Rule::boolean => Ok(Expr::Bool {
            value: inner.as_str() == "True",
            span,
        }),
        Rule::none => Ok(Expr::None { span }),
        _ => Err(error_with_span(
            format!("unsupported literal: {:?}", inner.as_rule()),
            span,
            source,
        )),
    }
}

pub fn parse_tuple_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(crate::expr::parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::Tuple { elements, span })
}

pub fn parse_list_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(crate::expr::parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::List { elements, span })
}

pub fn parse_set_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(crate::expr::parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::Set { elements, span })
}

pub fn parse_dict_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut entries = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::dict_entry {
            entries.push(parse_dict_entry(inner, source)?);
        }
    }
    Ok(Expr::Dict { entries, span })
}

pub fn parse_dict_entry(pair: Pair<'_, Rule>, source: &str) -> Result<(Expr, Expr), ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let key_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict key", span.clone(), source))?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict value", span.clone(), source))?;
    let key = crate::expr::parse_expr_pair(key_pair, source)?;
    let value = crate::expr::parse_expr_pair(value_pair, source)?;
    Ok((key, value))
}

pub fn parse_list_comp(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let element_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing list comp expr", span.clone(), source))?;
    let comp_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing list comp for", span.clone(), source))?;
    let element = crate::expr::parse_expr_pair(element_pair, source)?;
    let (target, iter, ifs) = parse_comp_for(comp_pair, source)?;
    Ok(Expr::ListComp {
        element: Box::new(element),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

pub fn parse_dict_comp(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let key_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict comp key", span.clone(), source))?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict comp value", span.clone(), source))?;
    let comp_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict comp for", span.clone(), source))?;
    let key = crate::expr::parse_expr_pair(key_pair, source)?;
    let value = crate::expr::parse_expr_pair(value_pair, source)?;
    let (target, iter, ifs) = parse_comp_for(comp_pair, source)?;
    Ok(Expr::DictComp {
        key: Box::new(key),
        value: Box::new(value),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

pub fn parse_comp_for(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<(String, Expr, Vec<Expr>), ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing comp target", pair_span.clone(), source))?;
    let iter_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing comp iter", pair_span.clone(), source))?;
    let target = target_pair.as_str().to_string();
    let iter = crate::expr::parse_expr_pair(iter_pair, source)?;
    let mut ifs = Vec::new();
    for next in inner {
        if next.as_rule() == Rule::comp_if {
            let mut if_inner = next.into_inner();
            let cond = if_inner.next().ok_or_else(|| {
                error_with_span("missing comp if condition", pair_span.clone(), source)
            })?;
            ifs.push(crate::expr::parse_expr_pair(cond, source)?);
        }
    }
    Ok((target, iter, ifs))
}

pub fn parse_regex_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let text = pair.as_str();
    let (content, content_offset) = if text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        let offset = pair.as_span().start() + 1;
        (inner, offset)
    } else {
        ("", pair.as_span().start())
    };
    let parts = parse_fstring_parts(content, content_offset, source)?;
    let has_expr = parts
        .iter()
        .any(|part| matches!(part, FStringPart::Expr(_)));
    if has_expr {
        let parts = normalize_regex_parts(parts)?;
        Ok(Expr::Regex {
            pattern: RegexPattern::Interpolated(parts),
            span,
        })
    } else {
        let mut text = join_fstring_text(parts);
        text = normalize_regex_text(&text);
        Ok(Expr::Regex {
            pattern: RegexPattern::Literal(text),
            span,
        })
    }
}

pub fn normalize_regex_parts(parts: Vec<FStringPart>) -> Result<Vec<FStringPart>, ParseError> {
    Ok(normalize_fstring_parts(parts, unescape_regex_text))
}

pub fn normalize_regex_text(text: &str) -> String {
    text.replace("\\/", "/")
}

pub fn parse_subprocess(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::subprocess => {
            let inner_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing subprocess body", span.clone(), source))?;
            parse_subprocess(inner_pair, source)
        }
        Rule::subprocess_capture | Rule::subprocess_status => {
            let kind = if pair.as_rule() == Rule::subprocess_capture {
                SubprocessKind::Capture
            } else {
                SubprocessKind::Status
            };
            let body_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing subprocess body", span.clone(), source))?;
            let parts = parse_subprocess_body(body_pair, source, span.clone())?;
            Ok(Expr::Subprocess { kind, parts, span })
        }
        _ => Err(error_with_span(
            format!("unsupported subprocess: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

pub fn parse_subprocess_body(
    pair: Pair<'_, Rule>,
    source: &str,
    span: SourceSpan,
) -> Result<Vec<SubprocessPart>, ParseError> {
    let content = pair.as_str();
    let content_offset = pair.as_span().start();
    let fstring_parts = parse_fstring_parts(content, content_offset, source)?;
    let parts = fstring_parts
        .into_iter()
        .map(|part| match part {
            FStringPart::Text(text) => SubprocessPart::Text(text),
            FStringPart::Expr(expr) => SubprocessPart::Expr(expr),
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return Err(error_with_span("missing subprocess command", span, source));
    }
    Ok(parts)
}

pub fn parse_structured_accessor(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let body_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing structured query body", span.clone(), source))?;
    let query = body_pair.as_str().to_string();
    Ok(Expr::StructuredAccessor { query, span })
}

pub fn parse_slice(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::slice => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing slice expression", span.clone(), source))?;
            parse_slice(inner, source)
        }
        Rule::slice_expr => {
            let mut start = None;
            let mut end = None;
            for part in pair.into_inner() {
                match part.as_rule() {
                    Rule::slice_start => {
                        let expr_pair = part.into_inner().next().ok_or_else(|| {
                            error_with_span("missing slice start", span.clone(), source)
                        })?;
                        start = Some(crate::expr::parse_expr_pair(expr_pair, source)?);
                    }
                    Rule::slice_end => {
                        let expr_pair = part.into_inner().next().ok_or_else(|| {
                            error_with_span("missing slice end", span.clone(), source)
                        })?;
                        end = Some(crate::expr::parse_expr_pair(expr_pair, source)?);
                    }
                    _ => {}
                }
            }
            Ok(Expr::Slice {
                start: start.map(Box::new),
                end: end.map(Box::new),
                span,
            })
        }
        Rule::expr => crate::expr::parse_expr_pair(pair, source),
        _ => Err(error_with_span(
            format!("unsupported slice: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}
