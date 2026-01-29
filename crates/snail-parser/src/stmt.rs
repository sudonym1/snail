use pest::iterators::Pair;
use snail_ast::{
    AssignTarget, Condition, ExceptHandler, Expr, ImportFromItems, ImportItem, Parameter, Stmt,
    WithItem,
};
use snail_error::ParseError;

use crate::Rule;
use crate::expr::{assign_target_from_expr, parse_expr_pair};
use crate::util::{error_with_span, expr_span, merge_span, span_from_pair};

pub fn parse_stmt_list(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        stmts.push(parse_stmt(inner, source)?);
    }
    Ok(stmts)
}

pub fn parse_stmt(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    match pair.as_rule() {
        Rule::if_stmt => parse_if(pair, source),
        Rule::while_stmt => parse_while(pair, source),
        Rule::for_stmt => parse_for(pair, source),
        Rule::def_stmt => parse_def(pair, source),
        Rule::class_stmt => parse_class(pair, source),
        Rule::try_stmt => parse_try(pair, source),
        Rule::with_stmt => parse_with(pair, source),
        Rule::return_stmt => parse_return(pair, source),
        Rule::raise_stmt => parse_raise(pair, source),
        Rule::assert_stmt => parse_assert(pair, source),
        Rule::del_stmt => parse_del(pair, source),
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

pub fn parse_block(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            return parse_stmt_list(inner, source);
        }
    }
    Ok(Vec::new())
}

fn parse_condition(pair: Pair<'_, Rule>, source: &str) -> Result<Condition, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::if_cond | Rule::while_cond => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing condition", span.clone(), source))?;
            parse_condition(inner, source)
        }
        Rule::let_cond => parse_let_condition(pair, source),
        Rule::expr => Ok(Condition::Expr(Box::new(parse_expr_pair(pair, source)?))),
        _ => Err(error_with_span(
            format!("unsupported condition: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

fn parse_let_condition(pair: Pair<'_, Rule>, source: &str) -> Result<Condition, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing let target", span.clone(), source))?;
    let target = parse_assign_target_list(target_pair, source)?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing let value", span.clone(), source))?;
    let value = parse_expr_pair(value_pair, source)?;
    let guard = inner
        .next()
        .map(|guard_pair| {
            let mut guard_inner = guard_pair.into_inner();
            let expr_pair = guard_inner
                .next()
                .ok_or_else(|| error_with_span("missing let guard", span.clone(), source))?;
            parse_expr_pair(expr_pair, source)
        })
        .transpose()?;
    Ok(Condition::Let {
        target: Box::new(target),
        value: Box::new(value),
        guard: guard.map(Box::new),
        span,
    })
}

fn parse_if(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let cond = parse_condition(
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
    let cond = parse_condition(
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
    let else_body = inner
        .next()
        .map(|pair| parse_block(pair, source))
        .transpose()?;
    Ok(Stmt::While {
        cond,
        body,
        else_body,
        span,
    })
}

fn parse_for(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
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
    Ok(Stmt::For {
        target,
        iter,
        body,
        else_body,
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
    let (params, body_pair) = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::parameters => {
            let params = parse_parameters(pair, source)?;
            let body_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing def block", span.clone(), source))?;
            (params, body_pair)
        }
        Some(pair) if pair.as_rule() == Rule::block => (Vec::new(), pair),
        Some(_) | None => {
            return Err(error_with_span("missing def block", span.clone(), source));
        }
    };
    let body = parse_block(body_pair, source)?;
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

fn parse_raise(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let value = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, source))
        .transpose()?;
    let from = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, source))
        .transpose()?;
    if value.is_none() && from.is_some() {
        return Err(error_with_span(
            "raise from requires an exception value",
            span,
            source,
        ));
    }
    Ok(Stmt::Raise { value, from, span })
}

fn parse_assert(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let test_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assert condition", span.clone(), source))?;
    let test = parse_expr_pair(test_pair, source)?;
    let message = inner
        .next()
        .map(|message_pair| parse_expr_pair(message_pair, source))
        .transpose()?;
    Ok(Stmt::Assert {
        test,
        message,
        span,
    })
}

fn parse_del(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut targets = Vec::new();
    for inner in pair.into_inner() {
        targets.push(parse_assign_target(inner, source)?);
    }
    if targets.is_empty() {
        return Err(error_with_span("missing del target", span, source));
    }
    Ok(Stmt::Delete { targets, span })
}

fn parse_try(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner().peekable();
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

    Ok(Stmt::Try {
        body,
        handlers,
        else_body,
        finally_body,
        span,
    })
}

fn parse_with(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
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
    Ok(Stmt::With { items, body, span })
}

fn parse_with_items(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<WithItem>, ParseError> {
    let mut items = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::with_item {
            items.push(parse_with_item(inner, source)?);
        }
    }
    Ok(items)
}

fn parse_with_item(pair: Pair<'_, Rule>, source: &str) -> Result<WithItem, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let context_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing with context", span.clone(), source))?;
    let context = parse_expr_pair(context_pair, source)?;
    let target = inner
        .next()
        .map(|target_pair| parse_assign_target(target_pair, source))
        .transpose()?;
    Ok(WithItem {
        context,
        target,
        span,
    })
}

fn parse_except_clause(pair: Pair<'_, Rule>, source: &str) -> Result<ExceptHandler, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner().peekable();
    let mut type_name = None;
    let mut name = None;
    let mut body = None;

    #[allow(clippy::while_let_on_iterator)]
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::expr => {
                type_name = Some(parse_expr_pair(next, source)?);
                if let Some(candidate) = inner.peek()
                    && candidate.as_rule() == Rule::identifier
                {
                    let alias_pair = inner.next().unwrap();
                    name = Some(alias_pair.as_str().to_string());
                }
            }
            Rule::identifier => {
                name = Some(next.as_str().to_string());
            }
            Rule::block => {
                body = Some(parse_block(next, source)?);
            }
            _ => {}
        }
    }

    let body = body.ok_or_else(|| error_with_span("missing except block", span.clone(), source))?;
    if type_name.is_none() && name.is_some() {
        return Err(error_with_span(
            "except alias requires an exception type",
            span,
            source,
        ));
    }
    Ok(ExceptHandler {
        type_name,
        name,
        body,
        span,
    })
}

fn parse_import_from(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let module_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing module name", span.clone(), source))?;
    let (level, module) = parse_import_from_module(module_pair, source)?;
    let items_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing import items", span.clone(), source))?;
    let items = parse_import_from_items(items_pair, source)?;
    Ok(Stmt::ImportFrom {
        level,
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

fn parse_import_from_module(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<(usize, Option<Vec<String>>), ParseError> {
    match pair.as_rule() {
        Rule::import_from_module => {
            let span = span_from_pair(&pair, source);
            let mut inner = pair.into_inner();
            let module_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing module name", span, source))?;
            parse_import_from_module(module_pair, source)
        }
        Rule::relative_module => {
            let span = span_from_pair(&pair, source);
            let mut inner = pair.into_inner();
            let dots_pair = inner.next().ok_or_else(|| {
                error_with_span("missing relative import dots", span.clone(), source)
            })?;
            let level = dots_pair.as_str().chars().filter(|ch| *ch == '.').count();
            let module = inner.next().map(parse_dotted_name);
            Ok((level, module))
        }
        Rule::dotted_name => Ok((0, Some(parse_dotted_name(pair)))),
        _ => Err(error_with_span(
            format!("unsupported import module: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
}

fn parse_import_from_items(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<ImportFromItems, ParseError> {
    match pair.as_rule() {
        Rule::import_from_items => {
            let span = span_from_pair(&pair, source);
            let mut inner = pair.into_inner();
            let items_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing import items", span, source))?;
            parse_import_from_items(items_pair, source)
        }
        Rule::import_star => Ok(ImportFromItems::Star {
            span: span_from_pair(&pair, source),
        }),
        Rule::import_items | Rule::import_items_multiline => {
            Ok(ImportFromItems::Names(parse_import_items(pair, source)?))
        }
        Rule::import_paren_items => {
            let span = span_from_pair(&pair, source);
            let mut inner = pair.into_inner();
            let items_pair = inner
                .find(|inner| {
                    matches!(
                        inner.as_rule(),
                        Rule::import_items | Rule::import_items_multiline
                    )
                })
                .ok_or_else(|| error_with_span("missing import items", span, source))?;
            Ok(ImportFromItems::Names(parse_import_items(
                items_pair, source,
            )?))
        }
        _ => Err(error_with_span(
            format!("unsupported import items: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
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
    pair.into_inner()
        .map(|part| part.as_str().to_string())
        .collect()
}

fn parse_assign(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), source))?;
    let targets = vec![parse_assign_target_list(target_pair, source)?];
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

pub fn parse_assign_target_list(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::assign_target_list => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                error_with_span("missing assignment target", span.clone(), source)
            })?;
            parse_assign_target_list(inner, source)
        }
        Rule::assign_target_tuple => parse_assign_target_tuple(pair, source),
        Rule::assign_target => parse_assign_target(pair, source),
        Rule::assign_target_ref
        | Rule::assign_list
        | Rule::assign_tuple
        | Rule::assign_target_atom
        | Rule::identifier => parse_assign_target(pair, source),
        _ => Err(error_with_span(
            format!("unsupported assignment target list: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

fn parse_assign_target_tuple(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        elements.push(parse_assign_target_item(inner, source)?);
    }
    Ok(AssignTarget::Tuple { elements, span })
}

fn parse_assign_target_ref(pair: Pair<'_, Rule>, source: &str) -> Result<AssignTarget, ParseError> {
    let expr = parse_assign_target_ref_expr(pair, source)?;
    assign_target_from_expr(expr, source)
}

pub(crate) fn parse_assign_target_ref_expr(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let atom_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), source))?;
    let mut expr = parse_assign_target_atom_expr(atom_pair, source)?;
    for suffix in inner {
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
                expr = Expr::Attribute {
                    value: Box::new(expr),
                    attr,
                    span,
                };
            }
            Rule::index => {
                let mut idx_inner = suffix.into_inner();
                let index_expr = crate::literal::parse_slice(
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
            _ => {}
        }
    }
    Ok(expr)
}

fn parse_assign_target_atom_expr(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::assign_target_atom => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                error_with_span("missing assignment target", span.clone(), source)
            })?;
            parse_assign_target_atom_expr(inner, source)
        }
        Rule::identifier => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span,
        }),
        Rule::assign_target_ref => parse_assign_target_ref_expr(pair, source),
        _ => Err(error_with_span(
            format!("unsupported assignment target: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

fn parse_assign_list(pair: Pair<'_, Rule>, source: &str) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::assign_target_items {
            for item in inner.into_inner() {
                elements.push(parse_assign_target_item(item, source)?);
            }
        }
    }
    Ok(AssignTarget::List { elements, span })
}

fn parse_assign_tuple(pair: Pair<'_, Rule>, source: &str) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), source))?;
    let tuple = parse_assign_target_tuple(inner, source)?;
    if let AssignTarget::Tuple { elements, .. } = tuple {
        Ok(AssignTarget::Tuple { elements, span })
    } else {
        Err(error_with_span(
            "invalid tuple assignment target",
            span,
            source,
        ))
    }
}

pub fn parse_assign_target(pair: Pair<'_, Rule>, source: &str) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::assign_target => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                error_with_span("missing assignment target", span.clone(), source)
            })?;
            parse_assign_target(inner, source)
        }
        Rule::assign_target_star => parse_assign_target_star(pair, source),
        Rule::assign_target_ref => parse_assign_target_ref(pair, source),
        Rule::assign_list => parse_assign_list(pair, source),
        Rule::assign_tuple => parse_assign_tuple(pair, source),
        Rule::assign_target_atom => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                error_with_span("missing assignment target", span.clone(), source)
            })?;
            parse_assign_target(inner, source)
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

fn parse_assign_target_item(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::assign_target_item => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                error_with_span("missing assignment target", span.clone(), source)
            })?;
            parse_assign_target_item(inner, source)
        }
        Rule::assign_target_star => parse_assign_target_star(pair, source),
        _ => parse_assign_target(pair, source),
    }
}

fn parse_assign_target_star(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, source);
    let inner = pair.into_inner().next().ok_or_else(|| {
        error_with_span("missing starred assignment target", span.clone(), source)
    })?;
    let target = parse_assign_target(inner, source)?;
    Ok(AssignTarget::Starred {
        target: Box::new(target),
        span,
    })
}

fn parse_expr_stmt(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let expr_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing expression", span.clone(), source))?;
    let value = parse_expr_pair(expr_pair, source)?;

    // Check if there's a semicolon immediately after this expression statement
    let semicolon_terminated = check_trailing_semicolon(source, span.end.offset);

    Ok(Stmt::Expr {
        value,
        semicolon_terminated,
        span,
    })
}

fn check_trailing_semicolon(source: &str, end_pos: usize) -> bool {
    // Check if the first non-whitespace character after end_pos is a semicolon
    source[end_pos..].chars().find(|c| !c.is_whitespace()) == Some(';')
}

pub(crate) fn parse_parameters(
    pair: Pair<'_, Rule>,
    source: &str,
) -> Result<Vec<Parameter>, ParseError> {
    let mut params = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::param_list => params.extend(parse_param_list(inner, source)?),
            Rule::parameter | Rule::regular_param | Rule::star_param | Rule::kw_param => {
                params.push(parse_parameter(inner, source)?);
            }
            _ => {}
        }
    }
    Ok(params)
}

fn parse_param_list(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Parameter>, ParseError> {
    let mut params = Vec::new();
    for inner in pair.into_inner() {
        if matches!(
            inner.as_rule(),
            Rule::parameter | Rule::regular_param | Rule::star_param | Rule::kw_param
        ) {
            params.push(parse_parameter(inner, source)?);
        }
    }
    Ok(params)
}

fn parse_parameter(pair: Pair<'_, Rule>, source: &str) -> Result<Parameter, ParseError> {
    let span = span_from_pair(&pair, source);
    match pair.as_rule() {
        Rule::parameter => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing parameter", span.clone(), source))?;
            parse_parameter(inner, source)
        }
        Rule::regular_param => {
            let mut inner = pair.into_inner();
            let name = inner
                .next()
                .ok_or_else(|| error_with_span("missing parameter name", span.clone(), source))?
                .as_str()
                .to_string();
            let default = inner
                .next()
                .map(|value_pair| parse_expr_pair(value_pair, source))
                .transpose()?;
            Ok(Parameter::Regular {
                name,
                default,
                span,
            })
        }
        Rule::star_param => {
            let name = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing *args name", span.clone(), source))?
                .as_str()
                .to_string();
            Ok(Parameter::VarArgs { name, span })
        }
        Rule::kw_param => {
            let name = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing **kwargs name", span.clone(), source))?
                .as_str()
                .to_string();
            Ok(Parameter::KwArgs { name, span })
        }
        _ => Err(error_with_span(
            format!("unsupported parameter: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}
