use pest::iterators::{Pair, Pairs};
use snail_ast::{DictEntry, Expr, FStringPart, RegexPattern, SourceSpan, SubprocessKind};
use snail_error::ParseError;

use crate::Rule;
use crate::string::{
    join_fstring_text, normalize_fstring_parts, parse_fstring_parts, parse_string_or_fstring,
    unescape_regex_text,
};
use crate::util::{LineIndex, error_with_span, is_keyword_rule, span_from_pair};

pub fn parse_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, lx);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing literal", pair_span, lx))?;
    let span = span_from_pair(&inner, lx);
    match inner.as_rule() {
        Rule::number => Ok(Expr::Number {
            value: inner.as_str().to_string(),
            span,
        }),
        Rule::string => parse_string_or_fstring(inner, lx),
        Rule::boolean => Ok(Expr::Bool {
            value: inner.as_str() == "True",
            span,
        }),
        Rule::none => Ok(Expr::None { span }),
        _ => Err(error_with_span(
            format!("unsupported literal: {:?}", inner.as_rule()),
            span,
            lx,
        )),
    }
}

pub fn parse_tuple_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let elements = parse_collection_elements(pair, lx)?;
    Ok(Expr::Tuple { elements, span })
}

pub fn parse_list_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let elements = parse_collection_elements(pair, lx)?;
    Ok(Expr::List { elements, span })
}

pub fn parse_set_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let elements = parse_collection_elements(pair, lx)?;
    Ok(Expr::Set { elements, span })
}

fn parse_collection_elements(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Vec<Expr>, ParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::expr => {
                elements.push(crate::expr::parse_expr_pair(inner, lx)?);
            }
            Rule::star_element => {
                let span = span_from_pair(&inner, lx);
                let value_pair = inner
                    .into_inner()
                    .next()
                    .ok_or_else(|| error_with_span("missing starred value", span.clone(), lx))?;
                let value = crate::expr::parse_expr_pair(value_pair, lx)?;
                elements.push(Expr::Starred {
                    value: Box::new(value),
                    span,
                });
            }
            _ => {}
        }
    }
    Ok(elements)
}

pub fn parse_dict_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut entries = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::dict_entry => {
                entries.push(parse_dict_entry(inner, lx)?);
            }
            Rule::dict_unpack => {
                let entry_span = span_from_pair(&inner, lx);
                let value_pair = inner.into_inner().next().ok_or_else(|| {
                    error_with_span("missing dict unpack value", entry_span.clone(), lx)
                })?;
                let value = crate::expr::parse_expr_pair(value_pair, lx)?;
                entries.push(DictEntry::Unpack {
                    value,
                    span: entry_span,
                });
            }
            _ => {}
        }
    }
    Ok(Expr::Dict { entries, span })
}

pub fn parse_dict_entry(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<DictEntry, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let key_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict key", span.clone(), lx))?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict value", span.clone(), lx))?;
    let key = crate::expr::parse_expr_pair(key_pair, lx)?;
    let value = crate::expr::parse_expr_pair(value_pair, lx)?;
    Ok(DictEntry::KeyValue { key, value, span })
}

pub fn parse_list_comp(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let element = parse_required_comp_expr(&mut inner, &span, lx, "missing list comp expr")?;
    let (target, iter, ifs) =
        parse_required_comp_for(&mut inner, &span, lx, "missing list comp for")?;
    Ok(Expr::ListComp {
        element: Box::new(element),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

pub fn parse_dict_comp(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let key = parse_required_comp_expr(&mut inner, &span, lx, "missing dict comp key")?;
    let value = parse_required_comp_expr(&mut inner, &span, lx, "missing dict comp value")?;
    let (target, iter, ifs) =
        parse_required_comp_for(&mut inner, &span, lx, "missing dict comp for")?;
    Ok(Expr::DictComp {
        key: Box::new(key),
        value: Box::new(value),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

fn parse_required_comp_expr(
    inner: &mut Pairs<'_, Rule>,
    span: &SourceSpan,
    lx: &LineIndex<'_>,
    missing_message: &str,
) -> Result<Expr, ParseError> {
    let expr_pair = inner
        .next()
        .ok_or_else(|| error_with_span(missing_message, span.clone(), lx))?;
    crate::expr::parse_expr_pair(expr_pair, lx)
}

fn parse_required_comp_for(
    inner: &mut Pairs<'_, Rule>,
    span: &SourceSpan,
    lx: &LineIndex<'_>,
    missing_message: &str,
) -> Result<(String, Expr, Vec<Expr>), ParseError> {
    let comp_pair = inner
        .next()
        .ok_or_else(|| error_with_span(missing_message, span.clone(), lx))?;
    parse_comp_for(comp_pair, lx)
}

pub fn parse_comp_for(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<(String, Expr, Vec<Expr>), ParseError> {
    let pair_span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing comp target", pair_span.clone(), lx))?;
    let iter_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing comp iter", pair_span.clone(), lx))?;
    let target = target_pair.as_str().to_string();
    let iter = crate::expr::parse_expr_pair(iter_pair, lx)?;
    let mut ifs = Vec::new();
    for next in inner {
        if next.as_rule() == Rule::comp_if {
            let cond = next
                .into_inner()
                .find(|p| !is_keyword_rule(p.as_rule()))
                .ok_or_else(|| {
                    error_with_span("missing comp if condition", pair_span.clone(), lx)
                })?;
            ifs.push(crate::expr::parse_expr_pair(cond, lx)?);
        }
    }
    Ok((target, iter, ifs))
}

pub fn parse_generator_expr(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let element =
        parse_required_comp_expr(&mut inner, &span, lx, "missing generator expr element")?;
    let (target, iter, ifs) =
        parse_required_comp_for(&mut inner, &span, lx, "missing generator expr for")?;
    Ok(Expr::GeneratorExpr {
        element: Box::new(element),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

pub fn parse_call_genexpr(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let element = parse_required_comp_expr(&mut inner, &span, lx, "missing call genexpr element")?;
    let (target, iter, ifs) =
        parse_required_comp_for(&mut inner, &span, lx, "missing call genexpr for")?;
    Ok(Expr::GeneratorExpr {
        element: Box::new(element),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

pub fn parse_regex_literal(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let text = pair.as_str();
    let (content, content_offset) = if text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        let offset = pair.as_span().start() + 1;
        (inner, offset)
    } else {
        ("", pair.as_span().start())
    };
    let parts = parse_fstring_parts(content, content_offset, lx)?;
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

pub fn parse_subprocess(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::subprocess => {
            let inner_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing subprocess body", span.clone(), lx))?;
            parse_subprocess(inner_pair, lx)
        }
        Rule::subprocess_capture | Rule::subprocess_status => {
            let kind = if pair.as_rule() == Rule::subprocess_capture {
                SubprocessKind::Capture
            } else {
                SubprocessKind::Status
            };
            let text = pair.as_str();
            let prefix_len = 2usize;
            let content_end = text.len().saturating_sub(1);
            let content = text.get(prefix_len..content_end).unwrap_or("");
            let content_offset = pair.as_span().start() + prefix_len;
            let parts = parse_fstring_parts(content, content_offset, lx)?;
            if parts.is_empty() {
                return Err(error_with_span("missing subprocess command", span, lx));
            }
            Ok(Expr::Subprocess { kind, parts, span })
        }
        _ => Err(error_with_span(
            format!("unsupported subprocess: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}

pub fn parse_structured_accessor(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let body_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing structured query body", span.clone(), lx))?;
    let query = body_pair.as_str().to_string();
    Ok(Expr::StructuredAccessor { query, span })
}

pub fn parse_slice(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::slice => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing slice expression", span.clone(), lx))?;
            parse_slice(inner, lx)
        }
        Rule::slice_expr => {
            let mut start = None;
            let mut end = None;
            for part in pair.into_inner() {
                match part.as_rule() {
                    Rule::slice_start => {
                        let expr_pair = part.into_inner().next().ok_or_else(|| {
                            error_with_span("missing slice start", span.clone(), lx)
                        })?;
                        start = Some(crate::expr::parse_expr_pair(expr_pair, lx)?);
                    }
                    Rule::slice_end => {
                        let expr_pair = part.into_inner().next().ok_or_else(|| {
                            error_with_span("missing slice end", span.clone(), lx)
                        })?;
                        end = Some(crate::expr::parse_expr_pair(expr_pair, lx)?);
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
        Rule::expr => crate::expr::parse_expr_pair(pair, lx),
        _ => Err(error_with_span(
            format!("unsupported slice: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}
