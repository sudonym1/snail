use pest::iterators::Pair;
use snail_ast::{Argument, AssignTarget, AugAssignOp, BinaryOp, CompareOp, Expr, IncrOp, UnaryOp};
use snail_error::ParseError;

use crate::Rule;
use crate::literal::{
    parse_dict_comp, parse_dict_literal, parse_list_comp, parse_list_literal, parse_literal,
    parse_regex_literal, parse_set_literal, parse_slice, parse_structured_accessor,
    parse_subprocess, parse_tuple_literal,
};
use crate::stmt::{parse_assign_target_ref_expr, parse_block, parse_parameters};
use crate::util::{error_with_span, expr_span, merge_span, span_from_pair};

pub fn parse_expr_pair(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr
        | Rule::aug_assign_expr
        | Rule::yield_expr
        | Rule::if_expr
        | Rule::or_expr
        | Rule::and_expr
        | Rule::not_expr
        | Rule::pipeline
        | Rule::comparison
        | Rule::sum
        | Rule::product
        | Rule::unary
        | Rule::power
        | Rule::primary
        | Rule::atom
        | Rule::try_fallback
        | Rule::try_fallback_unary
        | Rule::try_fallback_power
        | Rule::try_fallback_primary
        | Rule::compound_expr => parse_expr_rule(pair, source),
        Rule::literal => parse_literal(pair, source),
        Rule::def_expr => parse_def_expr(pair, source),
        Rule::exception_var => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span: span_from_pair(&pair, source),
        }),
        Rule::field_index_var => {
            let text = pair.as_str();
            let index = text[1..].to_string();
            Ok(Expr::FieldIndex {
                index,
                span: span_from_pair(&pair, source),
            })
        }
        Rule::injected_var => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span: span_from_pair(&pair, source),
        }),
        Rule::placeholder => Ok(Expr::Placeholder {
            span: span_from_pair(&pair, source),
        }),
        Rule::identifier => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span: span_from_pair(&pair, source),
        }),
        Rule::list_literal => parse_list_literal(pair, source),
        Rule::set_literal => parse_set_literal(pair, source),
        Rule::dict_literal => parse_dict_literal(pair, source),
        Rule::tuple_literal => parse_tuple_literal(pair, source),
        Rule::list_comp => parse_list_comp(pair, source),
        Rule::dict_comp => parse_dict_comp(pair, source),
        Rule::regex => parse_regex_literal(pair, source),
        Rule::subprocess => parse_subprocess(pair, source),
        Rule::structured_accessor => parse_structured_accessor(pair, source),
        Rule::paren_expr => parse_paren_expr(pair, source),
        _ => Err(error_with_span(
            format!("unsupported expression: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
}

fn parse_expr_rule(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr => parse_expr_rule(pair.into_inner().next().unwrap(), source),
        Rule::aug_assign_expr => parse_aug_assign_expr(pair, source),
        Rule::yield_expr => parse_yield_expr(pair, source),
        Rule::if_expr => parse_if_expr(pair, source),
        Rule::or_expr => fold_left_binary(pair, source, BinaryOp::Or),
        Rule::and_expr => fold_left_binary(pair, source, BinaryOp::And),
        Rule::not_expr => parse_not_expr(pair, source),
        Rule::pipeline => fold_left_binary(pair, source, BinaryOp::Pipeline),
        Rule::comparison => parse_comparison(pair, source),
        Rule::sum => parse_sum(pair, source),
        Rule::product => parse_product(pair, source),
        Rule::unary => parse_unary(pair, source),
        Rule::power => parse_power(pair, source),
        Rule::primary => parse_primary(pair, source),
        Rule::atom => parse_atom(pair, source),
        Rule::try_fallback => parse_expr_rule(pair.into_inner().next().unwrap(), source),
        Rule::try_fallback_unary => parse_unary(pair, source),
        Rule::try_fallback_power => parse_power(pair, source),
        Rule::try_fallback_primary => parse_primary(pair, source),
        Rule::compound_expr => parse_compound_expr(pair, source),
        Rule::paren_expr => parse_paren_expr(pair, source),
        Rule::regex => parse_regex_literal(pair, source),
        _ => Err(error_with_span(
            format!("unsupported expression: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
}

fn parse_yield_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let Some(first) = inner.next() else {
        return Ok(Expr::Yield { value: None, span });
    };
    match first.as_rule() {
        Rule::yield_from => {
            let expr_pair = first.into_inner().next().ok_or_else(|| {
                error_with_span("missing yield from expression", span.clone(), source)
            })?;
            let expr = parse_expr_pair(expr_pair, source)?;
            Ok(Expr::YieldFrom {
                expr: Box::new(expr),
                span,
            })
        }
        _ => {
            let expr = parse_expr_pair(first, source)?;
            Ok(Expr::Yield {
                value: Some(Box::new(expr)),
                span,
            })
        }
    }
}

fn parse_aug_assign_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner.next().ok_or_else(|| {
        error_with_span("missing augmented assignment target", span.clone(), source)
    })?;
    let op_pair = inner.next().ok_or_else(|| {
        error_with_span(
            "missing augmented assignment operator",
            span.clone(),
            source,
        )
    })?;
    let value_pair = inner.next().ok_or_else(|| {
        error_with_span("missing augmented assignment value", span.clone(), source)
    })?;

    let target_inner = if target_pair.as_rule() == Rule::aug_target {
        target_pair.into_inner().next().ok_or_else(|| {
            error_with_span("missing augmented assignment target", span.clone(), source)
        })?
    } else {
        target_pair
    };
    let target_expr = parse_assign_target_ref_expr(target_inner, source)?;
    let target = restricted_assign_target_from_expr(
        target_expr,
        source,
        "augmented assignment target must be a name, attribute, or index",
    )?;
    let op = parse_aug_assign_op(op_pair, source)?;
    let value = parse_expr_pair(value_pair, source)?;
    Ok(Expr::AugAssign {
        target: Box::new(target),
        op,
        value: Box::new(value),
        span,
    })
}

fn parse_if_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let body_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing if-expression body", pair_span.clone(), source))?;
    let body = parse_expr_pair(body_pair, source)?;
    let Some(test_pair) = inner.next() else {
        return Ok(body);
    };
    let test = parse_expr_pair(test_pair, source)?;
    let orelse_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing if-expression else", pair_span.clone(), source))?;
    let orelse = parse_expr_pair(orelse_pair, source)?;
    let span = merge_span(expr_span(&body), expr_span(&orelse));
    Ok(Expr::IfExpr {
        test: Box::new(test),
        body: Box::new(body),
        orelse: Box::new(orelse),
        span,
    })
}

fn fold_left_binary(pair: Pair<'_, Rule>, source: &str, op: BinaryOp) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| error_with_span("missing expression", pair_span, source))?;
    let mut expr = parse_expr_pair(first, source)?;
    for next in inner {
        let rhs = parse_expr_pair(next, source)?;
        let span = merge_span(expr_span(&expr), expr_span(&rhs));
        expr = Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(rhs),
            span,
        };
    }
    Ok(expr)
}

fn parse_not_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner().peekable();
    if inner
        .peek()
        .is_some_and(|next| next.as_rule() == Rule::not_op)
    {
        let op_pair = inner.next().unwrap();
        let operand_pair = inner.next().ok_or_else(|| {
            error_with_span(
                "missing operand for not",
                span_from_pair(&op_pair, source),
                source,
            )
        })?;
        let expr = parse_expr_pair(operand_pair, source)?;
        let span = merge_span(&span_from_pair(&op_pair, source), expr_span(&expr));
        return Ok(Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(expr),
            span,
        });
    }
    parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing comparison", pair_span, source))?,
        source,
    )
}

fn parse_comparison(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| error_with_span("missing comparison lhs", pair_span, source))?;
    let left = parse_expr_pair(first, source)?;
    let mut ops = Vec::new();
    let mut comparators = Vec::new();
    while let Some(op_pair) = inner.next() {
        let op_str = op_pair.as_str();
        let op = match op_str {
            "==" => CompareOp::Eq,
            "!=" => CompareOp::NotEq,
            "<" => CompareOp::Lt,
            "<=" => CompareOp::LtEq,
            ">" => CompareOp::Gt,
            ">=" => CompareOp::GtEq,
            "in" => CompareOp::In,
            "is" => CompareOp::Is,
            s if s.trim() == "not in" => CompareOp::NotIn,
            s if s.trim() == "is not" => CompareOp::IsNot,
            _ => {
                return Err(error_with_span(
                    format!("unknown comparison operator: {}", op_str),
                    span_from_pair(&op_pair, source),
                    source,
                ));
            }
        };
        let rhs_pair = inner.next().ok_or_else(|| {
            error_with_span(
                "missing comparison rhs",
                span_from_pair(&op_pair, source),
                source,
            )
        })?;
        ops.push(op);
        comparators.push(parse_expr_pair(rhs_pair, source)?);
    }
    if ops.len() == 1
        && matches!(ops[0], CompareOp::In | CompareOp::NotIn)
        && let [
            Expr::Regex {
                pattern,
                span: regex_span,
            },
        ] = comparators.as_slice()
    {
        let span = merge_span(expr_span(&left), regex_span);
        let regex_match = Expr::RegexMatch {
            value: Box::new(left),
            pattern: pattern.clone(),
            span: span.clone(),
        };

        return Ok(match ops[0] {
            CompareOp::In => regex_match,
            CompareOp::NotIn => Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(regex_match),
                span,
            },
            _ => unreachable!(),
        });
    }
    if ops.is_empty() {
        return Ok(left);
    }
    let span = merge_span(expr_span(&left), expr_span(comparators.last().unwrap()));
    Ok(Expr::Compare {
        left: Box::new(left),
        ops,
        comparators,
        span,
    })
}

fn parse_sum(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let mut expr = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing sum lhs", pair_span, source))?,
        source,
    )?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            _ => {
                return Err(error_with_span(
                    format!("unknown add op: {}", op_pair.as_str()),
                    span_from_pair(&op_pair, source),
                    source,
                ));
            }
        };
        let rhs = parse_expr_pair(
            inner.next().ok_or_else(|| {
                error_with_span("missing sum rhs", span_from_pair(&op_pair, source), source)
            })?,
            source,
        )?;
        let span = merge_span(expr_span(&expr), expr_span(&rhs));
        expr = Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(rhs),
            span,
        };
    }
    Ok(expr)
}

fn parse_product(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let mut expr = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing product lhs", pair_span, source))?,
        source,
    )?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "//" => BinaryOp::FloorDiv,
            "%" => BinaryOp::Mod,
            _ => {
                return Err(error_with_span(
                    format!("unknown mul op: {}", op_pair.as_str()),
                    span_from_pair(&op_pair, source),
                    source,
                ));
            }
        };
        let rhs = parse_expr_pair(
            inner.next().ok_or_else(|| {
                error_with_span(
                    "missing product rhs",
                    span_from_pair(&op_pair, source),
                    source,
                )
            })?,
            source,
        )?;
        let span = merge_span(expr_span(&expr), expr_span(&rhs));
        expr = Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(rhs),
            span,
        };
    }
    Ok(expr)
}

fn parse_unary(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner().peekable();
    let mut ops = Vec::new();
    while let Some(next) = inner.peek() {
        if next.as_rule() != Rule::unary_op && next.as_rule() != Rule::prefix_incr {
            break;
        }
        ops.push(inner.next().unwrap());
    }
    let base_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing unary operand", pair_span, source))?;
    let mut expr = parse_expr_pair(base_pair, source)?;
    for op_pair in ops.into_iter().rev() {
        match op_pair.as_rule() {
            Rule::unary_op => {
                let op = match op_pair.as_str() {
                    "+" => UnaryOp::Plus,
                    "-" => UnaryOp::Minus,
                    _ => {
                        return Err(error_with_span(
                            format!("unknown unary op: {}", op_pair.as_str()),
                            span_from_pair(&op_pair, source),
                            source,
                        ));
                    }
                };
                let span = merge_span(&span_from_pair(&op_pair, source), expr_span(&expr));
                expr = Expr::Unary {
                    op,
                    expr: Box::new(expr),
                    span,
                };
            }
            Rule::prefix_incr => {
                let op = match op_pair.as_str() {
                    "++" => IncrOp::Increment,
                    "--" => IncrOp::Decrement,
                    _ => {
                        return Err(error_with_span(
                            format!("unknown prefix op: {}", op_pair.as_str()),
                            span_from_pair(&op_pair, source),
                            source,
                        ));
                    }
                };
                let target_span = expr_span(&expr).clone();
                let target = restricted_assign_target_from_expr(
                    expr,
                    source,
                    "increment/decrement target must be a name, attribute, or index",
                )?;
                let span = merge_span(&span_from_pair(&op_pair, source), &target_span);
                expr = Expr::PrefixIncr {
                    op,
                    target: Box::new(target),
                    span,
                };
            }
            _ => {}
        }
    }
    Ok(expr)
}

fn parse_power(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let mut expr = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing power lhs", pair_span, source))?,
        source,
    )?;
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() != Rule::pow_op {
            continue;
        }
        let rhs = parse_expr_pair(
            inner.next().ok_or_else(|| {
                error_with_span(
                    "missing power rhs",
                    span_from_pair(&op_pair, source),
                    source,
                )
            })?,
            source,
        )?;
        let span = merge_span(expr_span(&expr), expr_span(&rhs));
        expr = Expr::Binary {
            left: Box::new(expr),
            op: BinaryOp::Pow,
            right: Box::new(rhs),
            span,
        };
    }
    Ok(expr)
}

fn parse_primary(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let atom_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing primary", pair_span, source))?;
    let mut expr = parse_expr_pair(atom_pair, source)?;
    let mut postfix_seen = false;
    for suffix in inner {
        let suffix_span = span_from_pair(&suffix, source);
        if postfix_seen {
            return Err(error_with_span(
                "postfix increment/decrement must be the final suffix",
                suffix_span,
                source,
            ));
        }
        match suffix.as_rule() {
            Rule::call => {
                let args = parse_call(suffix, source)?;
                let span = merge_span(expr_span(&expr), &suffix_span);
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span,
                };
            }
            Rule::attribute => {
                let attr = suffix
                    .into_inner()
                    .next()
                    .ok_or_else(|| {
                        error_with_span("missing attribute name", suffix_span.clone(), source)
                    })?
                    .as_str()
                    .to_string();
                let span = merge_span(expr_span(&expr), &suffix_span);
                expr = Expr::Attribute {
                    value: Box::new(expr),
                    attr,
                    span,
                };
            }
            Rule::index => {
                let mut idx_inner = suffix.into_inner();
                let index_expr = parse_slice(
                    idx_inner.next().ok_or_else(|| {
                        error_with_span("missing index expr", suffix_span.clone(), source)
                    })?,
                    source,
                )?;
                let span = merge_span(expr_span(&expr), expr_span(&index_expr));
                expr = Expr::Index {
                    value: Box::new(expr),
                    index: Box::new(index_expr),
                    span,
                };
            }
            Rule::try_suffix => {
                if matches!(
                    expr,
                    Expr::AugAssign { .. } | Expr::PrefixIncr { .. } | Expr::PostfixIncr { .. }
                ) {
                    return Err(error_with_span(
                        "compact try cannot wrap a binding expression",
                        expr_span(&expr).clone(),
                        source,
                    ));
                }
                let mut suffix_inner = suffix.into_inner();
                let fallback = suffix_inner
                    .next()
                    .map(|fallback_pair| parse_expr_pair(fallback_pair, source))
                    .transpose()?;
                let span = if let Some(ref fallback_expr) = fallback {
                    merge_span(expr_span(&expr), expr_span(fallback_expr))
                } else {
                    merge_span(expr_span(&expr), &suffix_span)
                };
                expr = Expr::TryExpr {
                    expr: Box::new(expr),
                    fallback: fallback.map(Box::new),
                    span,
                };
            }
            Rule::postfix_incr => {
                let op = match suffix.as_str() {
                    "++" => IncrOp::Increment,
                    "--" => IncrOp::Decrement,
                    _ => {
                        return Err(error_with_span(
                            format!("unknown postfix op: {}", suffix.as_str()),
                            suffix_span,
                            source,
                        ));
                    }
                };
                let target_span = expr_span(&expr).clone();
                let target = restricted_assign_target_from_expr(
                    expr,
                    source,
                    "increment/decrement target must be a name, attribute, or index",
                )?;
                let span = merge_span(&target_span, &suffix_span);
                expr = Expr::PostfixIncr {
                    op,
                    target: Box::new(target),
                    span,
                };
                postfix_seen = true;
            }
            _ => {}
        }
    }
    Ok(expr)
}

fn parse_call(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Argument>, ParseError> {
    let mut args = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::argument {
            args.push(parse_argument(inner, source)?);
        }
    }
    Ok(args)
}

fn parse_argument(pair: Pair<'_, Rule>, source: &str) -> Result<Argument, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| error_with_span("missing argument", span.clone(), source))?;
    match first.as_rule() {
        Rule::kw_argument => {
            let mut kw_inner = first.into_inner();
            let name = kw_inner
                .next()
                .ok_or_else(|| error_with_span("missing keyword argument", span.clone(), source))?
                .as_str()
                .to_string();
            let value_pair = kw_inner.next().ok_or_else(|| {
                error_with_span("missing keyword argument value", span.clone(), source)
            })?;
            let value = parse_expr_pair(value_pair, source)?;
            Ok(Argument::Keyword { name, value, span })
        }
        Rule::star_arg => {
            let value_pair = first
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing *arg value", span.clone(), source))?;
            let value = parse_expr_pair(value_pair, source)?;
            Ok(Argument::Star { value, span })
        }
        Rule::kw_star_arg => {
            let value_pair = first
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing **arg value", span.clone(), source))?;
            let value = parse_expr_pair(value_pair, source)?;
            Ok(Argument::KwStar { value, span })
        }
        _ => {
            let value = parse_expr_pair(first, source)?;
            Ok(Argument::Positional { value, span })
        }
    }
}

fn parse_compound_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut expressions = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            expressions.push(parse_expr_pair(inner, source)?);
        }
    }

    if expressions.is_empty() {
        return Err(error_with_span(
            "compound expression requires at least one expression",
            span,
            source,
        ));
    }

    Ok(Expr::Compound { expressions, span })
}

fn parse_paren_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let inner = pair.into_inner().next().ok_or_else(|| {
        error_with_span("missing expression in parentheses", span.clone(), source)
    })?;
    let expr = parse_expr_pair(inner, source)?;
    Ok(Expr::Paren {
        expr: Box::new(expr),
        span,
    })
}

fn parse_def_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let (params, body_pair) = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::parameters => {
            let params = parse_parameters(pair, source)?;
            let body_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing def body", span.clone(), source))?;
            (params, body_pair)
        }
        Some(pair) if pair.as_rule() == Rule::block => (Vec::new(), pair),
        Some(_) | None => {
            return Err(error_with_span("missing def body", span.clone(), source));
        }
    };
    let body = parse_block(body_pair, source)?;
    Ok(Expr::Lambda { params, body, span })
}

fn parse_atom(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let inner_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing atom", pair_span.clone(), source))?;
    match inner_pair.as_rule() {
        Rule::literal => parse_literal(inner_pair, source),
        Rule::compound_expr => parse_compound_expr(inner_pair, source),
        Rule::exception_var => Ok(Expr::Name {
            name: inner_pair.as_str().to_string(),
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::field_index_var => {
            let text = inner_pair.as_str();
            let index = text[1..].to_string();
            Ok(Expr::FieldIndex {
                index,
                span: span_from_pair(&inner_pair, source),
            })
        }
        Rule::injected_var => Ok(Expr::Name {
            name: inner_pair.as_str().to_string(),
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::placeholder => Ok(Expr::Placeholder {
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::identifier => Ok(Expr::Name {
            name: inner_pair.as_str().to_string(),
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::list_literal => parse_list_literal(inner_pair, source),
        Rule::set_literal => parse_set_literal(inner_pair, source),
        Rule::dict_literal => parse_dict_literal(inner_pair, source),
        Rule::tuple_literal => parse_tuple_literal(inner_pair, source),
        Rule::list_comp => parse_list_comp(inner_pair, source),
        Rule::dict_comp => parse_dict_comp(inner_pair, source),
        Rule::regex => parse_regex_literal(inner_pair, source),
        Rule::subprocess => parse_subprocess(inner_pair, source),
        Rule::def_expr => parse_def_expr(inner_pair, source),
        Rule::paren_expr => parse_paren_expr(inner_pair, source),
        _ => Err(error_with_span(
            format!("unsupported atom: {:?}", inner_pair.as_rule()),
            span_from_pair(&inner_pair, source),
            source,
        )),
    }
}

pub fn assign_target_from_expr(expr: Expr, source: &str) -> Result<AssignTarget, ParseError> {
    match expr {
        Expr::Name { name, span } => Ok(AssignTarget::Name { name, span }),
        Expr::Attribute { value, attr, span } => Ok(AssignTarget::Attribute { value, attr, span }),
        Expr::Index { value, index, span } => Ok(AssignTarget::Index { value, index, span }),
        Expr::List { elements, span } => {
            let elements = elements
                .into_iter()
                .map(|element| assign_target_from_expr(element, source))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssignTarget::List { elements, span })
        }
        Expr::Tuple { elements, span } => {
            let elements = elements
                .into_iter()
                .map(|element| assign_target_from_expr(element, source))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssignTarget::Tuple { elements, span })
        }
        Expr::Paren { expr, .. } => assign_target_from_expr(*expr, source),
        other => {
            let span = expr_span(&other).clone();
            Err(error_with_span(
                format!("unsupported assignment target: {:?}", other),
                span,
                source,
            ))
        }
    }
}

fn restricted_assign_target_from_expr(
    expr: Expr,
    source: &str,
    message: &str,
) -> Result<AssignTarget, ParseError> {
    let span = expr_span(&expr).clone();
    let target = assign_target_from_expr(expr, source)?;
    match target {
        AssignTarget::Name { .. } | AssignTarget::Attribute { .. } | AssignTarget::Index { .. } => {
            Ok(target)
        }
        _ => Err(error_with_span(message, span, source)),
    }
}

fn parse_aug_assign_op(pair: Pair<'_, Rule>, source: &str) -> Result<AugAssignOp, ParseError> {
    let op = match pair.as_str() {
        "+=" => AugAssignOp::Add,
        "-=" => AugAssignOp::Sub,
        "*=" => AugAssignOp::Mul,
        "/=" => AugAssignOp::Div,
        "//=" => AugAssignOp::FloorDiv,
        "%=" => AugAssignOp::Mod,
        "**=" => AugAssignOp::Pow,
        _ => {
            return Err(error_with_span(
                format!("unknown augmented assignment operator: {}", pair.as_str()),
                span_from_pair(&pair, source),
                source,
            ));
        }
    };
    Ok(op)
}
