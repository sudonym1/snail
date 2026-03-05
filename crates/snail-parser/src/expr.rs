use pest::iterators::Pair;
use snail_ast::{
    Argument, AssignTarget, AugAssignOp, BinaryOp, CompareOp, Condition, ExceptHandler, Expr,
    IncrOp, Stmt, UnaryOp,
};
use snail_error::ParseError;

use crate::Rule;
use crate::literal::{
    parse_dict_comp, parse_dict_literal, parse_list_comp, parse_list_literal, parse_literal,
    parse_regex_literal, parse_set_literal, parse_slice, parse_structured_accessor,
    parse_subprocess, parse_tuple_literal,
};
use crate::stmt::{
    parse_assign_target_list, parse_assign_target_ref_expr, parse_block, parse_condition,
    parse_except_clause, parse_parameters, parse_pattern_action, parse_stmt, parse_with_items,
};
use crate::util::{error_with_span, expr_span, merge_span, span_from_pair};

const COMPACT_TRY_EXCEPTION_VAR: &str = "__snail_compact_exc";
const COMPACT_TRY_NO_FALLBACK_HELPER: &str = "__snail_compact_try_no_fallback";

pub fn parse_expr_pair(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr
        | Rule::aug_assign_expr
        | Rule::yield_expr
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
        | Rule::try_fallback_primary => parse_expr_rule(pair, source),
        Rule::literal => parse_literal(pair, source),
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
        Rule::block => parse_block_expr(pair, source),
        Rule::if_expr => parse_if_expr(pair, source),
        Rule::while_expr => parse_while_expr(pair, source),
        Rule::for_expr => parse_for_expr(pair, source),
        Rule::def_expr => parse_def_expr(pair, source),
        Rule::class_expr => parse_class_expr(pair, source),
        Rule::try_expr => parse_try_expr(pair, source),
        Rule::with_expr => parse_with_expr(pair, source),
        Rule::awk_expr => parse_awk_expr(pair, source),
        Rule::xargs_expr => parse_xargs_expr(pair, source),
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
        let op = match op_pair.as_rule() {
            Rule::comp_op => {
                let sub: Vec<_> = op_pair.clone().into_inner().collect();
                match sub.as_slice() {
                    [kw] if kw.as_rule() == Rule::comp_kw_in => CompareOp::In,
                    [kw] if kw.as_rule() == Rule::comp_kw_is => CompareOp::Is,
                    [not_kw, in_kw]
                        if not_kw.as_rule() == Rule::comp_kw_not
                            && in_kw.as_rule() == Rule::comp_kw_in =>
                    {
                        CompareOp::NotIn
                    }
                    [is_kw, not_kw]
                        if is_kw.as_rule() == Rule::comp_kw_is
                            && not_kw.as_rule() == Rule::comp_kw_not =>
                    {
                        CompareOp::IsNot
                    }
                    _ => {
                        // Symbolic operators have no inner rules
                        let op_text = op_pair.as_str().trim();
                        match op_text {
                            "==" => CompareOp::Eq,
                            "!=" => CompareOp::NotEq,
                            "<" => CompareOp::Lt,
                            "<=" => CompareOp::LtEq,
                            ">" => CompareOp::Gt,
                            ">=" => CompareOp::GtEq,
                            _ => {
                                return Err(error_with_span(
                                    format!("unknown comparison operator: {}", op_text),
                                    span_from_pair(&op_pair, source),
                                    source,
                                ));
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(error_with_span(
                    format!("expected comp_op, got {:?}", op_pair.as_rule()),
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
            Rule::try_suffix => {
                expr = parse_compact_try_suffix(expr, suffix, source)?;
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
            _ => {
                expr = apply_attr_index_suffix(expr, suffix, source)?;
            }
        }
    }
    Ok(expr)
}

pub(crate) fn parse_compact_try_suffix(
    expr: Expr,
    suffix: Pair<'_, Rule>,
    source: &str,
) -> Result<Expr, ParseError> {
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

    let suffix_span = span_from_pair(&suffix, source);
    let mut suffix_inner = suffix.into_inner();
    let fallback = suffix_inner
        .next()
        .map(|fallback_pair| parse_expr_pair(fallback_pair, source))
        .transpose()?
        .map(replace_compact_try_exception_var);
    let span = if let Some(ref fallback_expr) = fallback {
        merge_span(expr_span(&expr), expr_span(fallback_expr))
    } else {
        merge_span(expr_span(&expr), &suffix_span)
    };

    let body_expr = unwrap_compound_parens(expr);
    let body_span = expr_span(&body_expr).clone();
    let body = vec![Stmt::Expr {
        value: body_expr,
        semicolon_terminated: false,
        span: body_span.clone(),
    }];
    let handler_value = fallback.unwrap_or_else(|| Expr::Call {
        func: Box::new(Expr::Name {
            name: COMPACT_TRY_NO_FALLBACK_HELPER.to_string(),
            span: span.clone(),
        }),
        args: vec![Argument::Positional {
            value: Expr::Name {
                name: COMPACT_TRY_EXCEPTION_VAR.to_string(),
                span: span.clone(),
            },
            span: span.clone(),
        }],
        span: span.clone(),
    });
    let handler_body = vec![Stmt::Expr {
        value: handler_value,
        semicolon_terminated: false,
        span: span.clone(),
    }];

    Ok(Expr::Try {
        body,
        handlers: vec![ExceptHandler {
            type_name: Some(Expr::Name {
                name: "Exception".to_string(),
                span: span.clone(),
            }),
            name: Some(COMPACT_TRY_EXCEPTION_VAR.to_string()),
            body: handler_body,
            span: span.clone(),
        }],
        else_body: None,
        finally_body: None,
        span,
    })
}

fn unwrap_compound_parens(expr: Expr) -> Expr {
    match expr {
        Expr::Paren { expr: inner, .. } if is_compound_expr(inner.as_ref()) => {
            unwrap_compound_parens(*inner)
        }
        other => other,
    }
}

fn is_compound_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Block { .. }
            | Expr::If { .. }
            | Expr::While { .. }
            | Expr::For { .. }
            | Expr::Try { .. }
            | Expr::With { .. }
            | Expr::Awk { .. }
            | Expr::Xargs { .. }
    )
}

fn replace_compact_try_exception_var(expr: Expr) -> Expr {
    match expr {
        Expr::Name { name, span } if name == "$e" => Expr::Name {
            name: COMPACT_TRY_EXCEPTION_VAR.to_string(),
            span,
        },
        Expr::FString { parts, bytes, span } => Expr::FString {
            parts: replace_compact_try_exception_var_in_fstring_parts(parts),
            bytes,
            span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op,
            expr: Box::new(replace_compact_try_exception_var(*expr)),
            span,
        },
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(replace_compact_try_exception_var(*left)),
            op,
            right: Box::new(replace_compact_try_exception_var(*right)),
            span,
        },
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Expr::Compare {
            left: Box::new(replace_compact_try_exception_var(*left)),
            ops,
            comparators: comparators
                .into_iter()
                .map(replace_compact_try_exception_var)
                .collect(),
            span,
        },
        Expr::Yield { value, span } => Expr::Yield {
            value: value.map(|value| Box::new(replace_compact_try_exception_var(*value))),
            span,
        },
        Expr::YieldFrom { expr, span } => Expr::YieldFrom {
            expr: Box::new(replace_compact_try_exception_var(*expr)),
            span,
        },
        Expr::Regex { pattern, span } => Expr::Regex {
            pattern: replace_compact_try_exception_var_in_regex(pattern),
            span,
        },
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Expr::RegexMatch {
            value: Box::new(replace_compact_try_exception_var(*value)),
            pattern: replace_compact_try_exception_var_in_regex(pattern),
            span,
        },
        Expr::Subprocess { kind, parts, span } => Expr::Subprocess {
            kind,
            parts: replace_compact_try_exception_var_in_fstring_parts(parts),
            span,
        },
        Expr::Call { func, args, span } => Expr::Call {
            func: Box::new(replace_compact_try_exception_var(*func)),
            args: args
                .into_iter()
                .map(replace_compact_try_exception_var_in_arg)
                .collect(),
            span,
        },
        Expr::Attribute { value, attr, span } => Expr::Attribute {
            value: Box::new(replace_compact_try_exception_var(*value)),
            attr,
            span,
        },
        Expr::Index { value, index, span } => Expr::Index {
            value: Box::new(replace_compact_try_exception_var(*value)),
            index: Box::new(replace_compact_try_exception_var(*index)),
            span,
        },
        Expr::Paren { expr, span } => Expr::Paren {
            expr: Box::new(replace_compact_try_exception_var(*expr)),
            span,
        },
        Expr::List { elements, span } => Expr::List {
            elements: elements
                .into_iter()
                .map(replace_compact_try_exception_var)
                .collect(),
            span,
        },
        Expr::Tuple { elements, span } => Expr::Tuple {
            elements: elements
                .into_iter()
                .map(replace_compact_try_exception_var)
                .collect(),
            span,
        },
        Expr::Set { elements, span } => Expr::Set {
            elements: elements
                .into_iter()
                .map(replace_compact_try_exception_var)
                .collect(),
            span,
        },
        Expr::Dict { entries, span } => Expr::Dict {
            entries: entries
                .into_iter()
                .map(|(key, value)| {
                    (
                        replace_compact_try_exception_var(key),
                        replace_compact_try_exception_var(value),
                    )
                })
                .collect(),
            span,
        },
        Expr::Slice { start, end, span } => Expr::Slice {
            start: start.map(|start| Box::new(replace_compact_try_exception_var(*start))),
            end: end.map(|end| Box::new(replace_compact_try_exception_var(*end))),
            span,
        },
        other => other,
    }
}

fn replace_compact_try_exception_var_in_arg(arg: Argument) -> Argument {
    match arg {
        Argument::Positional { value, span } => Argument::Positional {
            value: replace_compact_try_exception_var(value),
            span,
        },
        Argument::Keyword { name, value, span } => Argument::Keyword {
            name,
            value: replace_compact_try_exception_var(value),
            span,
        },
        Argument::Star { value, span } => Argument::Star {
            value: replace_compact_try_exception_var(value),
            span,
        },
        Argument::KwStar { value, span } => Argument::KwStar {
            value: replace_compact_try_exception_var(value),
            span,
        },
    }
}

fn replace_compact_try_exception_var_in_fstring_parts(
    parts: Vec<snail_ast::FStringPart>,
) -> Vec<snail_ast::FStringPart> {
    parts
        .into_iter()
        .map(|part| match part {
            snail_ast::FStringPart::Text(text) => snail_ast::FStringPart::Text(text),
            snail_ast::FStringPart::Expr(expr) => {
                snail_ast::FStringPart::Expr(snail_ast::FStringExpr {
                    expr: Box::new(replace_compact_try_exception_var(*expr.expr)),
                    conversion: expr.conversion,
                    format_spec: expr
                        .format_spec
                        .map(replace_compact_try_exception_var_in_fstring_parts),
                })
            }
        })
        .collect()
}

fn replace_compact_try_exception_var_in_regex(
    pattern: snail_ast::RegexPattern,
) -> snail_ast::RegexPattern {
    match pattern {
        snail_ast::RegexPattern::Literal(text) => snail_ast::RegexPattern::Literal(text),
        snail_ast::RegexPattern::Interpolated(parts) => snail_ast::RegexPattern::Interpolated(
            replace_compact_try_exception_var_in_fstring_parts(parts),
        ),
    }
}

pub(crate) fn apply_attr_index_suffix(
    expr: Expr,
    suffix: Pair<'_, Rule>,
    source: &str,
) -> Result<Expr, ParseError> {
    let suffix_span = span_from_pair(&suffix, source);
    match suffix.as_rule() {
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
            Ok(Expr::Attribute {
                value: Box::new(expr),
                attr,
                span,
            })
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
            Ok(Expr::Index {
                value: Box::new(expr),
                index: Box::new(index_expr),
                span,
            })
        }
        _ => Ok(expr),
    }
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

pub(crate) fn parse_argument(pair: Pair<'_, Rule>, source: &str) -> Result<Argument, ParseError> {
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

fn parse_atom(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let inner_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing atom", pair_span.clone(), source))?;
    match inner_pair.as_rule() {
        Rule::literal => parse_literal(inner_pair, source),
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
        Rule::paren_expr => parse_paren_expr(inner_pair, source),
        Rule::block => parse_block_expr(inner_pair, source),
        Rule::if_expr => parse_if_expr(inner_pair, source),
        Rule::while_expr => parse_while_expr(inner_pair, source),
        Rule::for_expr => parse_for_expr(inner_pair, source),
        Rule::def_expr => parse_def_expr(inner_pair, source),
        Rule::class_expr => parse_class_expr(inner_pair, source),
        Rule::try_expr => parse_try_expr(inner_pair, source),
        Rule::with_expr => parse_with_expr(inner_pair, source),
        Rule::awk_expr => parse_awk_expr(inner_pair, source),
        Rule::xargs_expr => parse_xargs_expr(inner_pair, source),
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

// --- Compound expression parsers ---

fn parse_block_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let stmts = parse_block(pair, source)?;
    Ok(Expr::Block { stmts, span })
}

fn parse_if_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let cond_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing if condition", span.clone(), source))?;
    let cond = parse_condition(cond_pair, source)?;
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing if body", span.clone(), source))?,
        source,
    )?;
    let mut elifs = Vec::new();
    let mut else_body = None;
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::if_cond => {
                let elif_cond = parse_condition(next, source)?;
                let elif_block = parse_block(
                    inner.next().ok_or_else(|| {
                        error_with_span("missing elif block", span.clone(), source)
                    })?,
                    source,
                )?;
                elifs.push((elif_cond, elif_block));
            }
            Rule::block => {
                else_body = Some(parse_block(next, source)?);
            }
            _ => {}
        }
    }
    Ok(Expr::If {
        cond,
        body,
        elifs,
        else_body,
        span,
    })
}

fn parse_while_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| error_with_span("missing while body", span.clone(), source))?;
    let (cond, body, else_body) = if first.as_rule() == Rule::block {
        // Unconditional while: `while { ... }` desugars to `while True { ... }`
        let body = parse_block(first, source)?;
        let cond = Condition::Expr(Box::new(Expr::Bool {
            value: true,
            span: span.clone(),
        }));
        (cond, body, None)
    } else {
        let cond = parse_condition(first, source)?;
        let body = parse_block(
            inner
                .next()
                .ok_or_else(|| error_with_span("missing while block", span.clone(), source))?,
            source,
        )?;
        let else_body = inner
            .next()
            .map(|pair| parse_block(pair, source))
            .transpose()?;
        (cond, body, else_body)
    };
    Ok(Expr::While {
        cond,
        body,
        else_body,
        span,
    })
}

fn parse_for_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing for target", span.clone(), source))?;
    let target = parse_assign_target_list(target_pair, source)?;
    let iter = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing for iterator", span.clone(), source))?,
        source,
    )?;
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing for block", span.clone(), source))?,
        source,
    )?;
    let else_body = inner
        .next()
        .map(|pair| parse_block(pair, source))
        .transpose()?;
    Ok(Expr::For {
        target,
        iter: Box::new(iter),
        body,
        else_body,
        span,
    })
}

fn parse_def_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| error_with_span("missing def block", span.clone(), source))?;
    let (name, params, body_pair) = match first.as_rule() {
        Rule::identifier => {
            let name = Some(first.as_str().to_string());
            match inner.next() {
                Some(pair) if pair.as_rule() == Rule::parameters => {
                    let params = parse_parameters(pair, source)?;
                    let body_pair = inner.next().ok_or_else(|| {
                        error_with_span("missing def block", span.clone(), source)
                    })?;
                    (name, params, body_pair)
                }
                Some(pair) if pair.as_rule() == Rule::block => (name, Vec::new(), pair),
                Some(_) | None => {
                    return Err(error_with_span("missing def block", span.clone(), source));
                }
            }
        }
        Rule::parameters => {
            let params = parse_parameters(first, source)?;
            let body_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing def block", span.clone(), source))?;
            (None, params, body_pair)
        }
        Rule::block => (None, Vec::new(), first),
        _ => {
            return Err(error_with_span("missing def block", span.clone(), source));
        }
    };
    let body = parse_block(body_pair, source)?;
    Ok(Expr::Def {
        name,
        params,
        body,
        span,
    })
}

fn parse_class_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| error_with_span("missing class name", span.clone(), source))?
        .as_str()
        .to_string();
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing class block", span.clone(), source))?,
        source,
    )?;
    Ok(Expr::Class { name, body, span })
}

fn parse_try_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let body_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing try block", span.clone(), source))?;
    let body = parse_block(body_pair, source)?;
    let mut handlers = Vec::new();
    let mut else_body = None;
    let mut finally_body = None;

    for next in inner {
        match next.as_rule() {
            Rule::except_clause => handlers.push(parse_except_clause(next, source)?),
            Rule::else_clause => {
                let block = next
                    .into_inner()
                    .next()
                    .ok_or_else(|| error_with_span("missing else block", span.clone(), source))?;
                else_body = Some(parse_block(block, source)?);
            }
            Rule::finally_clause => {
                let block = next.into_inner().next().ok_or_else(|| {
                    error_with_span("missing finally block", span.clone(), source)
                })?;
                finally_body = Some(parse_block(block, source)?);
            }
            _ => {}
        }
    }

    if handlers.is_empty() && finally_body.is_none() {
        return Err(error_with_span(
            "try must have at least one except clause or a finally block",
            span,
            source,
        ));
    }

    Ok(Expr::Try {
        body,
        handlers,
        else_body,
        finally_body,
        span,
    })
}

fn parse_with_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut items = Vec::new();
    let mut body = None;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::with_items => items.extend(parse_with_items(inner, source)?),
            Rule::block => body = Some(parse_block(inner, source)?),
            _ => {}
        }
    }
    let body = body.ok_or_else(|| error_with_span("missing with block", span.clone(), source))?;
    if items.is_empty() {
        return Err(error_with_span("missing with items", span, source));
    }
    Ok(Expr::With { items, body, span })
}

fn parse_awk_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut sources = Vec::new();
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::awk_source => {
                for arg_pair in inner.into_inner() {
                    if arg_pair.as_rule() == Rule::argument {
                        sources.push(parse_argument(arg_pair, source)?);
                    }
                }
            }
            Rule::awk_body => {
                for entry in inner.into_inner() {
                    match entry.as_rule() {
                        Rule::pattern_action => {
                            body.push(parse_pattern_action(entry, source)?);
                        }
                        _ => {
                            body.push(parse_stmt(entry, source)?);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Expr::Awk {
        sources,
        body,
        span,
    })
}

fn parse_xargs_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut sources = Vec::new();
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::xargs_source => {
                for arg_pair in inner.into_inner() {
                    if arg_pair.as_rule() == Rule::argument {
                        sources.push(parse_argument(arg_pair, source)?);
                    }
                }
            }
            Rule::block => {
                body = parse_block(inner, source)?;
            }
            _ => {}
        }
    }

    Ok(Expr::Xargs {
        sources,
        body,
        span,
    })
}
