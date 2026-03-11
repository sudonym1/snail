use pest::iterators::Pair;
use snail_ast::{
    AssignTarget, Condition, ExceptHandler, Expr, ImportFromItems, ImportItem, Parameter,
    SourceSpan, Stmt, WithItem,
};
use snail_error::ParseError;

use crate::Rule;
use crate::expr::{
    apply_attr_index_suffix, apply_postfix_ops, assign_target_from_expr, parse_call,
    parse_expr_pair,
};
use crate::util::{
    LineIndex, error_with_span, expr_span, is_keyword_rule, merge_span, span_from_pair,
};

pub fn parse_stmt_list(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Vec<Stmt>, ParseError> {
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::segment_break {
            stmts.push(Stmt::SegmentBreak {
                span: span_from_pair(&inner, lx),
            });
        } else {
            stmts.push(parse_stmt(inner, lx)?);
        }
    }
    Ok(stmts)
}

pub fn parse_stmt(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    match pair.as_rule() {
        Rule::return_stmt => parse_return(pair, lx),
        Rule::raise_stmt => parse_raise(pair, lx),
        Rule::assert_stmt => parse_assert(pair, lx),
        Rule::del_stmt => parse_del(pair, lx),
        Rule::break_stmt => {
            let span = span_from_pair(&pair, lx);
            let value = pair
                .into_inner()
                .find(|p| !is_keyword_rule(p.as_rule()))
                .map(|p| parse_expr_pair(p, lx))
                .transpose()?;
            Ok(Stmt::Break { value, span })
        }
        Rule::continue_stmt => Ok(Stmt::Continue {
            span: span_from_pair(&pair, lx),
        }),
        Rule::pass_stmt => Ok(Stmt::Pass {
            span: span_from_pair(&pair, lx),
        }),
        Rule::import_from => parse_import_from(pair, lx),
        Rule::import_names => parse_import_names(pair, lx),
        Rule::assign_stmt => parse_assign(pair, lx),
        Rule::compound_expr_stmt => parse_compound_expr_stmt(pair, lx),
        Rule::expr_stmt => parse_expr_stmt(pair, lx),
        _ => Err(error_with_span(
            format!("unsupported statement: {:?}", pair.as_rule()),
            span_from_pair(&pair, lx),
            lx,
        )),
    }
}

pub fn parse_block(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Vec<Stmt>, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            return parse_stmt_list(inner, lx);
        }
    }
    Ok(Vec::new())
}

pub(crate) fn parse_condition(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Condition, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::if_cond | Rule::while_cond => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing condition", span.clone(), lx))?;
            parse_condition(inner, lx)
        }
        Rule::let_cond => parse_let_condition(pair, lx),
        Rule::expr => Ok(Condition::Expr(Box::new(parse_expr_pair(pair, lx)?))),
        _ => Err(error_with_span(
            format!("unsupported condition: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}

fn parse_let_condition(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Condition, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing let target", span.clone(), lx))?;
    let target = parse_assign_target_list(target_pair, lx)?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing let value", span.clone(), lx))?;
    let value = parse_expr_pair(value_pair, lx)?;
    let guard = inner
        .next()
        .map(|guard_pair| {
            let mut guard_inner = guard_pair.into_inner();
            let expr_pair = guard_inner
                .next()
                .ok_or_else(|| error_with_span("missing let guard", span.clone(), lx))?;
            parse_expr_pair(expr_pair, lx)
        })
        .transpose()?;
    Ok(Condition::Let {
        target: Box::new(target),
        value: Box::new(value),
        guard: guard.map(Box::new),
        span,
    })
}

fn parse_return(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let value = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, lx))
        .transpose()?;
    Ok(Stmt::Return { value, span })
}

fn parse_raise(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let value = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, lx))
        .transpose()?;
    let from = inner
        .next()
        .map(|value_pair| parse_expr_pair(value_pair, lx))
        .transpose()?;
    if value.is_none() && from.is_some() {
        return Err(error_with_span(
            "raise from requires an exception value",
            span,
            lx,
        ));
    }
    Ok(Stmt::Raise { value, from, span })
}

fn parse_assert(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let test_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assert condition", span.clone(), lx))?;
    let test = parse_expr_pair(test_pair, lx)?;
    let message = inner
        .next()
        .map(|message_pair| parse_expr_pair(message_pair, lx))
        .transpose()?;
    Ok(Stmt::Assert {
        test,
        message,
        span,
    })
}

fn parse_del(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut targets = Vec::new();
    for inner in pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule())) {
        targets.push(parse_assign_target(inner, lx)?);
    }
    if targets.is_empty() {
        return Err(error_with_span("missing del target", span, lx));
    }
    Ok(Stmt::Delete { targets, span })
}

pub(crate) fn parse_with_items(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Vec<WithItem>, ParseError> {
    let mut items = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::with_item {
            items.push(parse_with_item(inner, lx)?);
        }
    }
    Ok(items)
}

pub(crate) fn parse_with_item(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<WithItem, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let context_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing with context", span.clone(), lx))?;
    let context = parse_expr_pair(context_pair, lx)?;
    let target = inner
        .next()
        .map(|target_pair| parse_assign_target(target_pair, lx))
        .transpose()?;
    Ok(WithItem {
        context,
        target,
        span,
    })
}

fn parse_single_except_type(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Expr {
    let type_span = span_from_pair(&pair, lx);
    let mut idents = pair.into_inner();
    let first = idents.next().unwrap();
    let mut expr = Expr::Name {
        name: first.as_str().to_string(),
        span: span_from_pair(&first, lx),
    };
    for attr_ident in idents {
        let attr_span = span_from_pair(&attr_ident, lx);
        expr = Expr::Attribute {
            value: Box::new(expr),
            attr: attr_ident.as_str().to_string(),
            span: SourceSpan {
                start: type_span.start.clone(),
                end: attr_span.end.clone(),
            },
        };
    }
    expr
}

pub(crate) fn parse_except_clause(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<ExceptHandler, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair
        .into_inner()
        .filter(|p| !is_keyword_rule(p.as_rule()))
        .peekable();
    let mut type_name = None;
    let mut name = None;
    let mut body = None;

    #[allow(clippy::while_let_on_iterator)]
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::except_types => {
                let types_inner = next.into_inner().next().unwrap();
                match types_inner.as_rule() {
                    Rule::except_type => {
                        type_name = Some(parse_single_except_type(types_inner, lx));
                    }
                    Rule::except_type_tuple => {
                        let tuple_span = span_from_pair(&types_inner, lx);
                        let elements: Vec<Expr> = types_inner
                            .into_inner()
                            .filter(|p| p.as_rule() == Rule::except_type)
                            .map(|p| parse_single_except_type(p, lx))
                            .collect();
                        type_name = Some(Expr::Tuple {
                            elements,
                            span: tuple_span,
                        });
                    }
                    _ => {}
                }
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
                body = Some(parse_block(next, lx)?);
            }
            _ => {}
        }
    }

    let body = body.ok_or_else(|| error_with_span("missing except block", span.clone(), lx))?;
    if type_name.is_none() && name.is_some() {
        return Err(error_with_span(
            "except alias requires an exception type",
            span,
            lx,
        ));
    }
    Ok(ExceptHandler {
        type_name,
        name,
        body,
        span,
    })
}

fn parse_import_from(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let module_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing module name", span.clone(), lx))?;
    let (level, module) = parse_import_from_module(module_pair, lx)?;
    let items_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing import items", span.clone(), lx))?;
    let items = parse_import_from_items(items_pair, lx)?;
    Ok(Stmt::ImportFrom {
        level,
        module,
        items,
        span,
    })
}

fn parse_import_names(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let items_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing import items", span.clone(), lx))?;
    let items = parse_import_items(items_pair, lx)?;
    Ok(Stmt::Import { items, span })
}

fn parse_import_from_module(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<(usize, Option<Vec<String>>), ParseError> {
    match pair.as_rule() {
        Rule::import_from_module => {
            let span = span_from_pair(&pair, lx);
            let mut inner = pair.into_inner();
            let module_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing module name", span, lx))?;
            parse_import_from_module(module_pair, lx)
        }
        Rule::relative_module => {
            let span = span_from_pair(&pair, lx);
            let mut inner = pair.into_inner();
            let dots_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing relative import dots", span.clone(), lx))?;
            let level = dots_pair.as_str().chars().filter(|ch| *ch == '.').count();
            let module = inner.next().map(parse_dotted_name);
            Ok((level, module))
        }
        Rule::dotted_name => Ok((0, Some(parse_dotted_name(pair)))),
        _ => Err(error_with_span(
            format!("unsupported import module: {:?}", pair.as_rule()),
            span_from_pair(&pair, lx),
            lx,
        )),
    }
}

fn parse_import_from_items(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<ImportFromItems, ParseError> {
    match pair.as_rule() {
        Rule::import_from_items => {
            let span = span_from_pair(&pair, lx);
            let mut inner = pair.into_inner();
            let items_pair = inner
                .next()
                .ok_or_else(|| error_with_span("missing import items", span, lx))?;
            parse_import_from_items(items_pair, lx)
        }
        Rule::import_star => Ok(ImportFromItems::Star {
            span: span_from_pair(&pair, lx),
        }),
        Rule::import_items => Ok(ImportFromItems::Names(parse_import_items(pair, lx)?)),
        Rule::import_paren_items => {
            let span = span_from_pair(&pair, lx);
            let mut inner = pair.into_inner();
            let items_pair = inner
                .find(|inner| inner.as_rule() == Rule::import_items)
                .ok_or_else(|| error_with_span("missing import items", span, lx))?;
            Ok(ImportFromItems::Names(parse_import_items(items_pair, lx)?))
        }
        _ => Err(error_with_span(
            format!("unsupported import items: {:?}", pair.as_rule()),
            span_from_pair(&pair, lx),
            lx,
        )),
    }
}

fn parse_import_items(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Vec<ImportItem>, ParseError> {
    let mut items = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::import_item {
            items.push(parse_import_item(inner, lx)?);
        }
    }
    Ok(items)
}

fn parse_import_item(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<ImportItem, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner().filter(|p| !is_keyword_rule(p.as_rule()));
    let name = parse_dotted_name(
        inner
            .next()
            .ok_or_else(|| error_with_span("missing import name", span.clone(), lx))?,
    );
    let alias = inner.next().map(|pair| pair.as_str().to_string());
    Ok(ImportItem { name, alias, span })
}

fn parse_dotted_name(pair: Pair<'_, Rule>) -> Vec<String> {
    pair.into_inner()
        .map(|part| part.as_str().to_string())
        .collect()
}

fn parse_assign(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let target_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
    let targets = vec![parse_assign_target_list(target_pair, lx)?];
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment value", span.clone(), lx))?;
    let value = parse_expr_pair(value_pair, lx)?;
    Ok(Stmt::Assign {
        targets,
        value,
        span,
    })
}

pub fn parse_assign_target_list(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::assign_target_list => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
            parse_assign_target_list(inner, lx)
        }
        Rule::assign_target_tuple => parse_assign_target_tuple(pair, lx),
        Rule::assign_target => parse_assign_target(pair, lx),
        Rule::assign_target_ref
        | Rule::assign_list
        | Rule::assign_tuple
        | Rule::assign_target_atom
        | Rule::identifier => parse_assign_target(pair, lx),
        _ => Err(error_with_span(
            format!("unsupported assignment target list: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}

fn parse_assign_target_tuple(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        elements.push(parse_assign_target_item(inner, lx)?);
    }
    Ok(AssignTarget::Tuple { elements, span })
}

fn parse_assign_target_ref(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let expr = parse_assign_target_ref_expr(pair, lx)?;
    assign_target_from_expr(expr, lx)
}

pub(crate) fn parse_assign_target_ref_expr(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let atom_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
    let mut expr = parse_assign_target_atom_expr(atom_pair, lx)?;
    for suffix in inner {
        match suffix.as_rule() {
            Rule::call => {
                let suffix_span = span_from_pair(&suffix, lx);
                let args = parse_call(suffix, lx)?;
                let span = merge_span(expr_span(&expr), &suffix_span);
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span,
                };
            }
            _ => {
                expr = apply_attr_index_suffix(expr, suffix, lx)?;
            }
        }
    }
    Ok(expr)
}

fn parse_assign_target_atom_expr(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::assign_target_atom => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
            parse_assign_target_atom_expr(inner, lx)
        }
        Rule::identifier => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span,
        }),
        Rule::assign_target_ref => parse_assign_target_ref_expr(pair, lx),
        _ => Err(error_with_span(
            format!("unsupported assignment target: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}

fn parse_assign_list(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::assign_target_items {
            for item in inner.into_inner() {
                elements.push(parse_assign_target_item(item, lx)?);
            }
        }
    }
    Ok(AssignTarget::List { elements, span })
}

fn parse_assign_tuple(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
    let tuple = parse_assign_target_tuple(inner, lx)?;
    if let AssignTarget::Tuple { elements, .. } = tuple {
        Ok(AssignTarget::Tuple { elements, span })
    } else {
        Err(error_with_span("invalid tuple assignment target", span, lx))
    }
}

pub fn parse_assign_target(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::assign_target => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
            parse_assign_target(inner, lx)
        }
        Rule::assign_target_star => parse_assign_target_star(pair, lx),
        Rule::assign_target_ref => parse_assign_target_ref(pair, lx),
        Rule::assign_list => parse_assign_list(pair, lx),
        Rule::assign_tuple => parse_assign_tuple(pair, lx),
        Rule::assign_target_atom => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
            parse_assign_target(inner, lx)
        }
        Rule::identifier => Ok(AssignTarget::Name {
            name: pair.as_str().to_string(),
            span,
        }),
        _ => Err(error_with_span(
            format!("unsupported assignment target: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}

fn parse_assign_target_item(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::assign_target_item => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing assignment target", span.clone(), lx))?;
            parse_assign_target_item(inner, lx)
        }
        Rule::assign_target_star => parse_assign_target_star(pair, lx),
        _ => parse_assign_target(pair, lx),
    }
}

fn parse_assign_target_star(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<AssignTarget, ParseError> {
    let span = span_from_pair(&pair, lx);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing starred assignment target", span.clone(), lx))?;
    let target = parse_assign_target(inner, lx)?;
    Ok(AssignTarget::Starred {
        target: Box::new(target),
        span,
    })
}

pub fn parse_pattern_action(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut pattern = None;
    let mut action = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::pattern_action_pattern => {
                let expr_pair = inner.into_inner().next().ok_or_else(|| {
                    error_with_span("missing pattern action pattern", span.clone(), lx)
                })?;
                pattern = Some(parse_expr_pair(expr_pair, lx)?);
            }
            Rule::block => action = Some(parse_block(inner, lx)?),
            _ => {}
        }
    }

    if pattern.is_none() && action.is_none() {
        return Err(error_with_span(
            "pattern/action requires a pattern or a block",
            span,
            lx,
        ));
    }

    Ok(Stmt::PatternAction {
        pattern,
        action,
        span,
    })
}

fn parse_compound_expr_stmt(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let mut inner = pair.into_inner();
    let expr_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing expression", span.clone(), lx))?;
    let mut value = parse_expr_pair(expr_pair, lx)?;

    // Apply full postfix chain (call, attribute, index, try_suffix, postfix_incr)
    value = apply_postfix_ops(value, inner, lx)?;

    let semicolon_terminated = check_trailing_semicolon(lx, span.end.offset);

    Ok(Stmt::Expr {
        value,
        semicolon_terminated,
        span,
    })
}

fn parse_expr_stmt(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, lx);
    let expr_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing expression", span.clone(), lx))?;
    let value = parse_expr_pair(expr_pair, lx)?;

    // Check if there's a semicolon immediately after this expression statement
    let semicolon_terminated = check_trailing_semicolon(lx, span.end.offset);

    Ok(Stmt::Expr {
        value,
        semicolon_terminated,
        span,
    })
}

fn check_trailing_semicolon(lx: &LineIndex<'_>, end_pos: usize) -> bool {
    // Check if the first non-whitespace character after end_pos is a semicolon.
    // Skip \x1e (injected record separator) since it's not a real semicolon.
    lx.source()[end_pos..]
        .chars()
        .find(|c| !c.is_whitespace() && *c != '\x1e')
        == Some(';')
}

pub(crate) fn parse_parameters(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Vec<Parameter>, ParseError> {
    let mut params = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::param_list => params.extend(parse_param_list(inner, lx)?),
            Rule::parameter | Rule::regular_param | Rule::star_param | Rule::kw_param => {
                params.push(parse_parameter(inner, lx)?);
            }
            _ => {}
        }
    }
    Ok(params)
}

fn parse_param_list(
    pair: Pair<'_, Rule>,
    lx: &LineIndex<'_>,
) -> Result<Vec<Parameter>, ParseError> {
    let mut params = Vec::new();
    for inner in pair.into_inner() {
        if matches!(
            inner.as_rule(),
            Rule::parameter
                | Rule::regular_param
                | Rule::star_param
                | Rule::kw_param
                | Rule::posonly_sep
                | Rule::kwonly_sep
        ) {
            params.push(parse_parameter(inner, lx)?);
        }
    }
    Ok(params)
}

fn parse_parameter(pair: Pair<'_, Rule>, lx: &LineIndex<'_>) -> Result<Parameter, ParseError> {
    let span = span_from_pair(&pair, lx);
    match pair.as_rule() {
        Rule::parameter => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing parameter", span.clone(), lx))?;
            parse_parameter(inner, lx)
        }
        Rule::regular_param => {
            let mut inner = pair.into_inner();
            let name = inner
                .next()
                .ok_or_else(|| error_with_span("missing parameter name", span.clone(), lx))?
                .as_str()
                .to_string();
            let default = inner
                .next()
                .map(|value_pair| parse_expr_pair(value_pair, lx))
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
                .ok_or_else(|| error_with_span("missing *args name", span.clone(), lx))?
                .as_str()
                .to_string();
            Ok(Parameter::VarArgs { name, span })
        }
        Rule::kw_param => {
            let name = pair
                .into_inner()
                .next()
                .ok_or_else(|| error_with_span("missing **kwargs name", span.clone(), lx))?
                .as_str()
                .to_string();
            Ok(Parameter::KwArgs { name, span })
        }
        Rule::posonly_sep => Ok(Parameter::PosonlySep { span }),
        Rule::kwonly_sep => Ok(Parameter::KwonlySep { span }),
        _ => Err(error_with_span(
            format!("unsupported parameter: {:?}", pair.as_rule()),
            span,
            lx,
        )),
    }
}
