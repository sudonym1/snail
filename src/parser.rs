use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::*;
use crate::awk::{AwkProgram, AwkRule};
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

pub fn parse_awk_program(source: &str) -> Result<AwkProgram, ParseError> {
    let mut pairs = SnailParser::parse(Rule::awk_program, source)
        .map_err(|err| parse_error_from_pest(err, source))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing awk program root"))?;
    let span = full_span(source);

    let mut begin_blocks = Vec::new();
    let mut rules = Vec::new();
    let mut end_blocks = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::awk_entry_list {
            for entry in inner.into_inner() {
                match entry.as_rule() {
                    Rule::awk_begin => {
                        let block = entry
                            .into_inner()
                            .find(|pair| pair.as_rule() == Rule::block)
                            .ok_or_else(|| {
                                error_with_span("missing BEGIN block", span.clone(), source)
                            })?;
                        begin_blocks.push(parse_block(block, source)?);
                    }
                    Rule::awk_end => {
                        let block = entry
                            .into_inner()
                            .find(|pair| pair.as_rule() == Rule::block)
                            .ok_or_else(|| {
                                error_with_span("missing END block", span.clone(), source)
                            })?;
                        end_blocks.push(parse_block(block, source)?);
                    }
                    Rule::awk_rule => rules.push(parse_awk_rule(entry, source)?),
                    _ => {}
                }
            }
        }
    }

    Ok(AwkProgram {
        begin_blocks,
        rules,
        end_blocks,
        span,
    })
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

fn parse_awk_rule(pair: Pair<'_, Rule>, source: &str) -> Result<AwkRule, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut pattern = None;
    let mut action = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::awk_pattern => {
                let expr_pair = inner
                    .into_inner()
                    .next()
                    .ok_or_else(|| error_with_span("missing awk pattern", span.clone(), source))?;
                pattern = Some(parse_expr_pair(expr_pair, source)?);
            }
            Rule::block => action = Some(parse_block(inner, source)?),
            _ => {}
        }
    }

    if pattern.is_none() && action.is_none() {
        return Err(error_with_span(
            "awk rule requires a pattern or a block",
            span,
            source,
        ));
    }

    Ok(AwkRule {
        pattern,
        action: action.unwrap_or_default(),
        span,
    })
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
    let params_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing parameter list", span.clone(), source))?;
    let params = parse_parameters(params_pair, source)?;
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
        if inner.as_rule() == Rule::assign_target {
            targets.push(parse_assign_target(inner, source)?);
        }
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
            let mut expr = Expr::Name {
                name: name_pair.as_str().to_string(),
                span: span_from_pair(&name_pair, source),
            };
            for suffix in inner {
                let suffix_span = span_from_pair(&suffix, source);
                match suffix.as_rule() {
                    Rule::attribute => {
                        let attr = suffix
                            .into_inner()
                            .next()
                            .ok_or_else(|| {
                                error_with_span(
                                    "missing attribute name",
                                    suffix_span.clone(),
                                    source,
                                )
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
                    _ => {}
                }
            }
            assign_target_from_expr(expr, source)
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

fn assign_target_from_expr(expr: Expr, source: &str) -> Result<AssignTarget, ParseError> {
    match expr {
        Expr::Name { name, span } => Ok(AssignTarget::Name { name, span }),
        Expr::Attribute { value, attr, span } => Ok(AssignTarget::Attribute { value, attr, span }),
        Expr::Index { value, index, span } => Ok(AssignTarget::Index { value, index, span }),
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

fn parse_expr_stmt(pair: Pair<'_, Rule>, source: &str) -> Result<Stmt, ParseError> {
    let span = span_from_pair(&pair, source);
    let expr_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing expression", span.clone(), source))?;
    let value = parse_expr_pair(expr_pair, source)?;
    Ok(Stmt::Expr { value, span })
}

fn parse_parameters(pair: Pair<'_, Rule>, source: &str) -> Result<Vec<Parameter>, ParseError> {
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

fn parse_expr_pair(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr
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
        | Rule::compound_expr => parse_expr_rule(pair, source),
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
        Rule::identifier => Ok(Expr::Name {
            name: pair.as_str().to_string(),
            span: span_from_pair(&pair, source),
        }),
        Rule::list_literal => parse_list_literal(pair, source),
        Rule::dict_literal => parse_dict_literal(pair, source),
        Rule::tuple_literal => parse_tuple_literal(pair, source),
        Rule::set_literal => parse_set_literal(pair, source),
        Rule::list_comp => parse_list_comp(pair, source),
        Rule::dict_comp => parse_dict_comp(pair, source),
        Rule::regex => parse_regex_literal(pair, source),
        Rule::subprocess => parse_subprocess(pair, source),
        Rule::json_query => parse_json_query(pair, source),
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
        Rule::compound_expr => parse_compound_expr(pair, source),
        Rule::regex => parse_regex_literal(pair, source),
        _ => Err(error_with_span(
            format!("unsupported expression: {:?}", pair.as_rule()),
            span_from_pair(&pair, source),
            source,
        )),
    }
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
        && matches!(ops[0], CompareOp::In)
        && let [
            Expr::Regex {
                pattern,
                span: regex_span,
            },
        ] = comparators.as_slice()
    {
        let span = merge_span(expr_span(&left), regex_span);
        return Ok(Expr::RegexMatch {
            value: Box::new(left),
            pattern: pattern.clone(),
            span,
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
        Rule::identifier => Ok(Expr::Name {
            name: inner_pair.as_str().to_string(),
            span: span_from_pair(&inner_pair, source),
        }),
        Rule::list_literal => parse_list_literal(inner_pair, source),
        Rule::dict_literal => parse_dict_literal(inner_pair, source),
        Rule::tuple_literal => parse_tuple_literal(inner_pair, source),
        Rule::set_literal => parse_set_literal(inner_pair, source),
        Rule::list_comp => parse_list_comp(inner_pair, source),
        Rule::dict_comp => parse_dict_comp(inner_pair, source),
        Rule::regex => parse_regex_literal(inner_pair, source),
        Rule::subprocess => parse_subprocess(inner_pair, source),
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

fn parse_tuple_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::Tuple { elements, span })
}

fn parse_set_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::Set { elements, span })
}

fn parse_list_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::expr {
            elements.push(parse_expr_pair(inner, source)?);
        }
    }
    Ok(Expr::List { elements, span })
}

fn parse_slice(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
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
                        start = Some(parse_expr_pair(expr_pair, source)?);
                    }
                    Rule::slice_end => {
                        let expr_pair = part.into_inner().next().ok_or_else(|| {
                            error_with_span("missing slice end", span.clone(), source)
                        })?;
                        end = Some(parse_expr_pair(expr_pair, source)?);
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
        Rule::expr => parse_expr_pair(pair, source),
        _ => Err(error_with_span(
            format!("unsupported slice: {:?}", pair.as_rule()),
            span,
            source,
        )),
    }
}

fn parse_dict_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut entries = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::dict_entry {
            entries.push(parse_dict_entry(inner, source)?);
        }
    }
    Ok(Expr::Dict { entries, span })
}

fn parse_dict_entry(pair: Pair<'_, Rule>, source: &str) -> Result<(Expr, Expr), ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let key_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict key", span.clone(), source))?;
    let value_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing dict value", span.clone(), source))?;
    let key = parse_expr_pair(key_pair, source)?;
    let value = parse_expr_pair(value_pair, source)?;
    Ok((key, value))
}

fn parse_list_comp(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let element_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing list comp expr", span.clone(), source))?;
    let comp_pair = inner
        .next()
        .ok_or_else(|| error_with_span("missing list comp for", span.clone(), source))?;
    let element = parse_expr_pair(element_pair, source)?;
    let (target, iter, ifs) = parse_comp_for(comp_pair, source)?;
    Ok(Expr::ListComp {
        element: Box::new(element),
        target,
        iter: Box::new(iter),
        ifs,
        span,
    })
}

fn parse_subprocess(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
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

fn parse_subprocess_body(
    pair: Pair<'_, Rule>,
    source: &str,
    span: SourceSpan,
) -> Result<Vec<SubprocessPart>, ParseError> {
    let mut parts = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::subprocess_text => {
                let start_offset = inner.as_span().start();
                let text_parts = parse_subprocess_text_parts(inner.as_str(), start_offset, source)?;
                parts.extend(text_parts);
            }
            Rule::subprocess_expr => {
                let expr_pair = inner.into_inner().next().ok_or_else(|| {
                    error_with_span("missing subprocess expression", span.clone(), source)
                })?;
                let expr = parse_expr_pair(expr_pair, source)?;
                parts.push(SubprocessPart::Expr(Box::new(expr)));
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        return Err(error_with_span("missing subprocess command", span, source));
    }
    Ok(parts)
}

fn parse_subprocess_text_parts(
    text: &str,
    start_offset: usize,
    source: &str,
) -> Result<Vec<SubprocessPart>, ParseError> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut iter = text.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        match ch {
            '{' => {
                if matches!(iter.peek(), Some((_, '{'))) {
                    iter.next();
                }
                buffer.push('{');
            }
            '}' => {
                if matches!(iter.peek(), Some((_, '}'))) {
                    iter.next();
                }
                buffer.push('}');
            }
            '$' => {
                if matches!(iter.peek(), Some((_, '$'))) {
                    iter.next();
                    buffer.push('$');
                    continue;
                }

                if let Some((_, next_ch)) = iter.peek().copied()
                    && next_ch.is_ascii_digit()
                {
                    let mut digits = String::new();
                    let mut end = idx + 1;
                    while let Some((d_idx, d_ch)) = iter.peek().copied() {
                        if d_ch.is_ascii_digit() {
                            iter.next();
                            digits.push(d_ch);
                            end = d_idx + d_ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    if !buffer.is_empty() {
                        parts.push(SubprocessPart::Text(std::mem::take(&mut buffer)));
                    }
                    let span = span_from_offset(start_offset + idx, start_offset + end, source);
                    parts.push(SubprocessPart::Expr(Box::new(Expr::FieldIndex {
                        index: digits,
                        span,
                    })));
                    continue;
                }

                if let Some((name, len)) = match_injected_name(&text[idx + 1..]) {
                    for _ in 0..len {
                        iter.next();
                    }
                    if !buffer.is_empty() {
                        parts.push(SubprocessPart::Text(std::mem::take(&mut buffer)));
                    }
                    let span =
                        span_from_offset(start_offset + idx, start_offset + idx + 1 + len, source);
                    parts.push(SubprocessPart::Expr(Box::new(Expr::Name {
                        name: format!("${name}"),
                        span,
                    })));
                    continue;
                }

                buffer.push('$');
            }
            _ => buffer.push(ch),
        }
    }

    if !buffer.is_empty() {
        parts.push(SubprocessPart::Text(buffer));
    }

    Ok(parts)
}

fn match_injected_name(text: &str) -> Option<(&'static str, usize)> {
    if text.starts_with("fn") {
        return Some(("fn", 2));
    }
    for name in ["l", "f", "n", "p", "m", "e"] {
        if text.starts_with(name) {
            return Some((name, 1));
        }
    }
    None
}

fn parse_json_query(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let body_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| error_with_span("missing json query body", span.clone(), source))?;
    let query = body_pair.as_str().to_string();
    Ok(Expr::JsonQuery { query, span })
}

fn parse_dict_comp(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
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
    let key = parse_expr_pair(key_pair, source)?;
    let value = parse_expr_pair(value_pair, source)?;
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

fn parse_comp_for(
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
    let iter = parse_expr_pair(iter_pair, source)?;
    let mut ifs = Vec::new();
    for next in inner {
        if next.as_rule() == Rule::comp_if {
            let mut if_inner = next.into_inner();
            let cond = if_inner.next().ok_or_else(|| {
                error_with_span("missing comp if condition", pair_span.clone(), source)
            })?;
            ifs.push(parse_expr_pair(cond, source)?);
        }
    }
    Ok((target, iter, ifs))
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

fn parse_regex_literal(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
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

fn parse_string_or_fstring(pair: Pair<'_, Rule>, source: &str) -> Result<Expr, ParseError> {
    let span = span_from_pair(&pair, source);
    let parsed = parse_string_literal(pair)?;

    // Raw strings should not have f-string interpolation
    if parsed.raw {
        return Ok(Expr::String {
            value: parsed.content,
            raw: true,
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
        Ok(Expr::FString { parts, span })
    } else {
        let value = join_fstring_text(parts);
        Ok(Expr::String {
            value,
            raw: parsed.raw,
            delimiter: parsed.delimiter,
            span,
        })
    }
}

struct ParsedStringLiteral {
    content: String,
    raw: bool,
    delimiter: StringDelimiter,
    content_offset: usize,
}

fn parse_string_literal(pair: Pair<'_, Rule>) -> Result<ParsedStringLiteral, ParseError> {
    let value = pair.as_str();
    let span = pair.as_span();
    let (raw, rest, prefix_len) = if let Some(stripped) = value.strip_prefix('r') {
        (true, stripped, 1usize)
    } else {
        (false, value, 0usize)
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
        delimiter,
        content_offset,
    })
}

fn parse_fstring_parts(
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

fn parse_inline_expr(
    expr_text: &str,
    expr_offset: usize,
    source: &str,
) -> Result<Expr, ParseError> {
    let mut pairs = SnailParser::parse(Rule::expr, expr_text)
        .map_err(|err| parse_error_from_pest_with_offset(err, source, expr_offset))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing f-string expression"))?;
    let mut expr = parse_expr_pair(pair, expr_text)?;
    shift_expr_spans(&mut expr, expr_offset, source);
    Ok(expr)
}

fn find_fstring_expr_end(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'r' => {
                if let Some(next) = bytes.get(i + 1)
                    && (*next == b'\'' || *next == b'"')
                {
                    if let Some(end) = skip_string_literal(bytes, i) {
                        i = end;
                        continue;
                    } else {
                        return None;
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

fn skip_string_literal(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    let raw = if bytes.get(i) == Some(&b'r') {
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

fn normalize_string_parts(
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

fn normalize_regex_parts(parts: Vec<FStringPart>) -> Result<Vec<FStringPart>, ParseError> {
    let mut normalized = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => {
                normalized.push(FStringPart::Text(unescape_regex_text(&text)));
            }
            FStringPart::Expr(expr) => normalized.push(FStringPart::Expr(expr)),
        }
    }
    Ok(normalized)
}

fn normalize_regex_text(text: &str) -> String {
    text.replace("\\/", "/")
}

fn unescape_string_text(text: &str) -> String {
    unescape_text(text, false)
}

fn unescape_regex_text(text: &str) -> String {
    unescape_text(text, true)
}

fn unescape_text(text: &str, escape_slash: bool) -> String {
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

fn join_fstring_text(parts: Vec<FStringPart>) -> String {
    let mut text = String::new();
    for part in parts {
        if let FStringPart::Text(value) = part {
            text.push_str(&value);
        }
    }
    text
}

fn expr_span(expr: &Expr) -> &SourceSpan {
    match expr {
        Expr::Name { span, .. }
        | Expr::Number { span, .. }
        | Expr::String { span, .. }
        | Expr::FString { span, .. }
        | Expr::Bool { span, .. }
        | Expr::None { span }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Compare { span, .. }
        | Expr::IfExpr { span, .. }
        | Expr::TryExpr { span, .. }
        | Expr::Compound { span, .. }
        | Expr::Regex { span, .. }
        | Expr::RegexMatch { span, .. }
        | Expr::Subprocess { span, .. }
        | Expr::JsonQuery { span, .. }
        | Expr::Call { span, .. }
        | Expr::Attribute { span, .. }
        | Expr::Index { span, .. }
        | Expr::Paren { span, .. }
        | Expr::FieldIndex { span, .. }
        | Expr::List { span, .. }
        | Expr::Tuple { span, .. }
        | Expr::Dict { span, .. }
        | Expr::Set { span, .. }
        | Expr::ListComp { span, .. }
        | Expr::DictComp { span, .. }
        | Expr::Slice { span, .. } => span,
    }
}

fn shift_expr_spans(expr: &mut Expr, offset: usize, source: &str) {
    match expr {
        Expr::Name { span, .. }
        | Expr::Number { span, .. }
        | Expr::String { span, .. }
        | Expr::Bool { span, .. }
        | Expr::None { span }
        | Expr::Subprocess { span, .. }
        | Expr::JsonQuery { span, .. }
        | Expr::FieldIndex { span, .. }
        | Expr::List { span, .. }
        | Expr::Tuple { span, .. }
        | Expr::Dict { span, .. }
        | Expr::Set { span, .. }
        | Expr::Slice { span, .. } => {
            *span = shift_span(span, offset, source);
        }
        Expr::FString { parts, span } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part {
                    shift_expr_spans(expr, offset, source);
                }
            }
            *span = shift_span(span, offset, source);
        }
        Expr::Regex { pattern, span } => {
            if let RegexPattern::Interpolated(parts) = pattern {
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

fn parse_error_from_pest_with_offset(
    err: pest::error::Error<Rule>,
    source: &str,
    offset: usize,
) -> ParseError {
    use pest::error::InputLocation;
    let message = err.to_string();
    let span = match err.location {
        InputLocation::Pos(pos) => Some(span_from_offset(offset + pos, offset + pos, source)),
        InputLocation::Span((start, end)) => {
            Some(span_from_offset(offset + start, offset + end, source))
        }
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
