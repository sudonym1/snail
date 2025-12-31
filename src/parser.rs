use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::*;
use crate::error::ParseError;

#[derive(Parser)]
#[grammar = "snail.pest"]
struct SnailParser;

pub fn parse_program(source: &str) -> Result<Program, ParseError> {
    let mut pairs = SnailParser::parse(Rule::program, source)
        .map_err(|err| parse_error_from_pest(err, source))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing program root"))?;
    let span = full_span(source);
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            stmts = parse_stmt_list(inner, source)?;
        }
    }
    Ok(Program { stmts, span })
}

fn parse_stmt_list(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        stmts.push(parse_stmt(inner, source)?);
    }
    Ok(stmts)
}

fn parse_stmt(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    match pair.as_rule() {
        Rule::if_stmt => parse_if(pair, source),
        Rule::while_stmt => parse_while(pair, source),
        Rule::for_stmt => parse_for(pair, source),
        Rule::def_stmt => parse_def(pair, source),
        Rule::class_stmt => parse_class(pair, source),
        Rule::return_stmt => parse_return(pair, source),
        Rule::break_stmt => Ok(Stmt::Break {
            span: span_from_pair(&pair, source),
        }),
        Rule::continue_stmt => Ok(Stmt::Continue {
            span: span_from_pair(&pair, source),
        }),
        Rule::pass_stmt => Ok(Stmt::Pass {
            span: span_from_pair(&pair, source),
        }),
        Rule::import_from => parse_import_from(pair, source),
        Rule::import_names => parse_import_names(pair, source),
        Rule::assign_stmt => parse_assign(pair, source),
        Rule::expr_stmt => parse_expr_stmt(pair, source),
        _ => Err(error_with_span(
            format!("unsupported statement: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
}

fn parse_block(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            return parse_stmt_list(inner, source);
        }
    }
    Ok(Vec::new())
}

fn parse_if(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let cond = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing if condition", span.clone(), source))?,
        source,
    )?;
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing if block", span.clone(), source))?,
        source,
    )?;
    let mut elifs = Vec::new();
    let mut else_body = None;
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::expr => {
                let elif_cond = parse_expr_pair(next, source)?;
                let elif_block = parse_block(
                    inner
                        .next()
                        .ok_or_else(|| error_with_span("missing elif block", span.clone(), source))?,
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
    Ok(Stmt::If {
        cond,
        body,
        elifs,
        else_body,
        span,
    })
}

fn parse_while(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let cond = parse_expr_pair(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing while condition", span.clone(), source))?,
        source,
    )?;
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing while block", span.clone(), source))?,
        source,
    )?;
    Ok(Stmt::While { cond, body, span })
}

fn parse_for(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing for target", span.clone(), source))?;
    let target = parse_assign_target(target_pair, source)?;
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
    Ok(Stmt::For {
        target,
        iter,
        body,
        span,
    })
}

fn parse_def(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| error_with_span("missing def name", span.clone(), source))?
        .as_str()
        .to_string();
    let params_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing parameter list", span.clone(), source))?;
    let params = parse_parameters(params_pair);
    let body = parse_block(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing def block", span.clone(), source))?,
        source,
    )?;
    Ok(Stmt::Def {
        name,
        params,
        body,
        span,
    })
}

fn parse_class(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
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
    Ok(Stmt::Class { name, body, span })
}

fn parse_return(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let value = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, source))
        .transpose()?;
    Ok(Stmt::Return { value, span })
}

fn parse_import_from(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let module = parse_dotted_name(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing module name", span.clone(), source))?,
    );
    let items_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing import items", span.clone(), source))?;
    let items = parse_import_items(items_pair, source)?;
    Ok(Stmt::ImportFrom {
        module,
        items,
        span,
    })
}

fn parse_import_names(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let items_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing import items", span.clone(), source))?;
    let items = parse_import_items(items_pair, source)?;
    Ok(Stmt::Import { items, span })
}

fn parse_import_items(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<ImportItem>, ParseError> {
    let mut items = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::import_item {
            items.push(parse_import_item(inner, source)?);
        }
    }
    Ok(items)
}

fn parse_import_item(pair: Pair<'_, Rule>, source: &str) -> Result<ImportItem, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let name = parse_dotted_name(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing import name", span.clone(), source))?,
    );
    let alias = inner.next().map(|pair| pair.as_str().to_string());
    Ok(ImportItem { name, alias, span })
}

fn parse_dotted_name(pair: Pair<'_, Rule>) -> Vec<String> {
    pair.into_inner().map(|part| part.as_str().to_string()).collect()
}

fn parse_assign(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), source))?;
    let targets = vec![parse_assign_target(target_pair, source)?];
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment value", span.clone(), source))?;
    let value = parse_expr_pair(value_pair, source)?;
    Ok(Stmt::Assign {
        targets,
        value,
        span,
    })
}

fn parse_assign_target(pair: Pair<'_, Rule>, source: &str) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::assign_target => {
            let mut inner = pair.into_inner();
            let name_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing assignment name", span.clone(), source))?;
            let name = name_pair.as_str().to_string();
            if inner.next().is_some() {
                return Err(error_with_span(
                    "assignment target must be a name for now",
                    span,
                    source,
                ));
            }
            Ok(AssignTarget::Name { name, span })
        }
        Rule::identifier => Ok(AssignTarget::Name {
            name: pair.as_str().to_string(),
            span,
        }),
        _ => Err(error_with_span(
            format!("unsupported assignment target: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

fn parse_expr_stmt(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let expr_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing expression", span.clone(), source))?;
    let value = parse_expr_pair(expr_pair, source)?;
    Ok(Stmt::Expr { value, span })
}

fn parse_parameters(pair: Pair<'_, Rule>) -> Vec<String> {
    pair.into_inner().map(|part| part.as_str().to_string()).collect()
}

fn parse_expr_pair(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr | Rule::or_expr | Rule::and_expr | Rule::not_expr | Rule::comparison | Rule::sum
        | Rule::product | Rule::unary | Rule::power | Rule::primary | Rule::atom => {
            parse_expr_rule(pair, source)
        }
        Rule::literal => parse_literal(pair, source),
        Rule::identifier => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span: span_from_pair(&pair, source),
        }),
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
        Rule::or_expr => fold_left_binary(pair, source, BinaryOp::Or),
        Rule::and_expr => fold_left_binary(pair, source, BinaryOp::And),
        Rule::not_expr => parse_not_expr(pair, source),
        Rule::comparison => parse_comparison(pair, source),
        Rule::sum => parse_sum(pair, source),
        Rule::product => parse_product(pair, source),
        Rule::unary => parse_unary(pair, source),
        Rule::power => parse_power(pair, source),
        Rule::primary => parse_primary(pair, source),
        Rule::atom => parse_atom(pair, source),
        _ => Err(error_with_span(
            format!("unsupported expression: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
}

fn fold_left_binary(
    pair: Pair<'_, Rule>,
    source: &str,
    op: BinaryOp,
) -> Result<Expr, ParseError> {
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
    if let Some(next) = inner.peek() {
        if next.as_rule() == Rule::not_op {
            let op_pair = inner.next().unwrap();
            let operand_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing operand for not", span_from_pair(&op_pair, source), source))?;
            let expr = parse_expr_pair(operand_pair, source)?;
            let span = merge_span(&span_from_pair(&op_pair, source), expr_span(&expr));
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
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
        let op = match op_pair.as_str() {
            "==" => CompareOp::Eq,
            "!=" => CompareOp::NotEq,
            "<" => CompareOp::Lt,
            "<=" => CompareOp::LtEq,
            ">" => CompareOp::Gt,
            ">=" => CompareOp::GtEq,
            "in" => CompareOp::In,
            "is" => CompareOp::Is,
            _ => {
                return Err(error_with_span(
                    format!("unknown comparison operator: {}", op_pair.as_str()),
                    span_from_pair(&op_pair, source),
                    source,
                ))
            }
        };
        let rhs_pair = inner
            .next()
            .ok_or_else(|| error_with_span("missing comparison rhs", span_from_pair(&op_pair, source), source))?;
        ops.push(op);
        comparators.push(parse_expr_pair(rhs_pair, source)?);
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
                ))
            }
        };
        let rhs = parse_expr_pair(
            inner
                .next()
                .ok_or_else(|| error_with_span("missing sum rhs", span_from_pair(&op_pair, source), source))?,
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
                ))
            }
        };
        let rhs = parse_expr_pair(
            inner
                .next()
                .ok_or_else(|| error_with_span("missing product rhs", span_from_pair(&op_pair, source), source))?,
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
        if next.as_rule() != Rule::unary_op {
            break;
        }
        ops.push(inner.next().unwrap());
    }
    let base_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing unary operand", pair_span, source))?;
    let mut expr = parse_expr_pair(base_pair, source)?;
    for op_pair in ops.into_iter().rev() {
        let op = match op_pair.as_str() {
            "+" => UnaryOp::Plus,
            "-" => UnaryOp::Minus,
            _ => {
                return Err(error_with_span(
                    format!("unknown unary op: {}", op_pair.as_str()),
                    span_from_pair(&op_pair, source),
                    source,
                ))
            }
        };
        let span = merge_span(&span_from_pair(&op_pair, source), expr_span(&expr));
        expr = Expr::Unary {
            op,
            expr: Box::new(expr),
            span,
        };
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
            inner
                .next()
                .ok_or_else(|| error_with_span("missing power rhs", span_from_pair(&op_pair, source), source))?,
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
    for suffix in inner {
        let suffix_span = span_from_pair(&suffix, source);
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
                    .ok_or_else(|| error_with_span("missing attribute name", suffix_span.clone(), source))?
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
                let index_expr = parse_expr_pair(
                    idx_inner
                        .next()
                        .ok_or_else(|| error_with_span("missing index expr", suffix_span.clone(), source))?,
                    source,
                )?;
                let span = merge_span(expr_span(&expr), expr_span(&index_expr));
                expr = Expr::Index {
                    value: Box::new(expr),
                    index: Box::new(index_expr),
                    span,
                };
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
    let second = inner.next();
    if let Some(value_pair) = second {
        let value = parse_expr_pair(value_pair, source)?;
        Ok(Argument {
            name: Some(first.as_str().to_string()),
            value,
            span,
        })
    } else {
        let value = parse_expr_pair(first, source)?;
        Ok(Argument {
            name: None,
            value,
            span,
        })
    }
}

fn parse_atom(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let pair_span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let inner_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing atom", pair_span.clone(), source))?;
    match inner_pair.as_rule() {
        Rule::literal => parse_literal(inner_pair, source),
        Rule::identifier => Ok(Expr::Name {
            name: inner_pair.as_str().to_string(),
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::expr => {
            let expr = parse_expr_pair(inner_pair, source)?;
            Ok(Expr::Paren {
                expr: Box::new(expr),
                span: pair_span,
            })
        }
        _ => Err(error_with_span(
            format!("unsupported atom: {:?}", inner_pair.as_rule()),
            span_from_pair(&inner_pair, source),
            source,
        )),
    }
}

fn parse_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
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
        Rule::string => Ok(Expr::String {
            value: strip_quotes(inner.as_str()),
            span,
        }),
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

fn strip_quotes(value: &str) -> String {
    if value.len() >= 2 {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn expr_span(expr: &Expr) -> &SourceSpan {
    match expr {
        Expr::Name { span, .. }
        | Expr::Number { span, .. }
        | Expr::String { span, .. }
        | Expr::Bool { span, .. }
        | Expr::None { span }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Compare { span, .. }
        | Expr::Call { span, .. }
        | Expr::Attribute { span, .. }
        | Expr::Index { span, .. }
        | Expr::Paren { span, .. } => span,
    }
}

fn full_span(source: &str) -> SourceSpan {
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

fn span_from_pair(pair: &Pair<'_, Rule>, source: &str) -> SourceSpan {
    span_from_span(pair.as_span(), source)
}

fn span_from_span(span: pest::Span<'_>, source: &str) -> SourceSpan {
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

fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start: left.start.clone(),
        end: right.end.clone(),
    }
}

fn error_with_span(message: impl Into<String>, span: SourceSpan, source: &str) -> ParseError {
    let mut err = ParseError::new(message);
    err.line_text = line_text(source, span.start.line);
    err.span = Some(span);
    err
}

fn line_text(source: &str, line: usize) -> Option<String> {
    if line == 0 {
        return None;
    }
    source.lines().nth(line - 1).map(|s| s.to_string())
}

fn line_col_from_offset(source: &str, offset: usize) -> (usize, usize) {
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

fn parse_error_from_pest(err: pest::error::Error<Rule>, source: &str) -> ParseError {
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

fn span_from_offset(start: usize, end: usize, source: &str) -> SourceSpan {
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
