use snail_ast::*;
use snail_error::LowerError;
use snail_python_ast::*;

// Constants that need to be public for codegen
pub const SNAIL_TRY_HELPER: &str = "__snail_compact_try";
pub const SNAIL_EXCEPTION_VAR: &str = "__snail_compact_exc";
pub const SNAIL_SUBPROCESS_CAPTURE_CLASS: &str = "__SnailSubprocessCapture";
pub const SNAIL_SUBPROCESS_STATUS_CLASS: &str = "__SnailSubprocessStatus";
pub const SNAIL_REGEX_SEARCH: &str = "__snail_regex_search";
pub const SNAIL_REGEX_COMPILE: &str = "__snail_regex_compile";
pub const SNAIL_STRUCTURED_ACCESSOR_CLASS: &str = "__SnailStructuredAccessor";
pub const SNAIL_JSON_OBJECT_CLASS: &str = "__SnailJsonObject";
pub const SNAIL_JSON_PIPELINE_WRAPPER_CLASS: &str = "__SnailJsonPipelineWrapper";

// Awk-related constants (private to this crate)
const SNAIL_AWK_LINE: &str = "$l";
const SNAIL_AWK_FIELDS: &str = "$f";
const SNAIL_AWK_NR: &str = "$n";
const SNAIL_AWK_FNR: &str = "$fn";
const SNAIL_AWK_PATH: &str = "$p";
const SNAIL_AWK_MATCH: &str = "$m";
const SNAIL_AWK_LINE_PYVAR: &str = "__snail_line";
const SNAIL_AWK_FIELDS_PYVAR: &str = "__snail_fields";
const SNAIL_AWK_NR_PYVAR: &str = "__snail_nr_user";
const SNAIL_AWK_FNR_PYVAR: &str = "__snail_fnr_user";
const SNAIL_AWK_PATH_PYVAR: &str = "__snail_path_user";
const SNAIL_AWK_MATCH_PYVAR: &str = "__snail_match";

fn injected_py_name(name: &str) -> Option<&'static str> {
    match name {
        SNAIL_AWK_LINE => Some(SNAIL_AWK_LINE_PYVAR),
        SNAIL_AWK_FIELDS => Some(SNAIL_AWK_FIELDS_PYVAR),
        SNAIL_AWK_NR => Some(SNAIL_AWK_NR_PYVAR),
        SNAIL_AWK_FNR => Some(SNAIL_AWK_FNR_PYVAR),
        SNAIL_AWK_PATH => Some(SNAIL_AWK_PATH_PYVAR),
        SNAIL_AWK_MATCH => Some(SNAIL_AWK_MATCH_PYVAR),
        _ => None,
    }
}

fn escape_for_python_string(value: &str) -> String {
    // Escape special characters for a Python string literal
    // This is used for raw source text that needs to be embedded in a Python string
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn lower_program(program: &Program) -> Result<PyModule, LowerError> {
    let mut body = Vec::new();
    for stmt in &program.stmts {
        body.push(lower_stmt(stmt)?);
    }
    Ok(PyModule {
        body,
        span: program.span.clone(),
    })
}

pub fn lower_awk_program(program: &AwkProgram) -> Result<PyModule, LowerError> {
    lower_awk_program_with_auto_print(program, false)
}

pub fn lower_awk_program_with_auto_print(
    program: &AwkProgram,
    auto_print: bool,
) -> Result<PyModule, LowerError> {
    let span = program.span.clone();
    let mut body = Vec::new();

    body.push(PyStmt::Import {
        names: vec![PyImportName {
            name: vec!["sys".to_string()],
            asname: None,
            span: span.clone(),
        }],
        span: span.clone(),
    });

    let mut main_body = Vec::new();
    for block in &program.begin_blocks {
        let lowered = lower_block(block)?;
        main_body.extend(wrap_block_with_auto_print(lowered, auto_print));
    }

    main_body.push(assign_name("__snail_nr", number_expr("0", &span), &span));

    let files_expr = PyExpr::Binary {
        left: Box::new(PyExpr::Index {
            value: Box::new(PyExpr::Attribute {
                value: Box::new(name_expr("sys", &span)),
                attr: "argv".to_string(),
                span: span.clone(),
            }),
            index: Box::new(PyExpr::Slice {
                start: Some(Box::new(number_expr("1", &span))),
                end: None,
                span: span.clone(),
            }),
            span: span.clone(),
        }),
        op: PyBinaryOp::Or,
        right: Box::new(PyExpr::List {
            elements: vec![string_expr("-", &span)],
            span: span.clone(),
        }),
        span: span.clone(),
    };

    let file_loop = lower_awk_file_loop_with_auto_print(program, &span, auto_print)?;
    main_body.push(PyStmt::For {
        target: name_expr("__snail_path", &span),
        iter: files_expr,
        body: file_loop,
        orelse: Vec::new(),
        span: span.clone(),
    });

    for block in &program.end_blocks {
        let lowered = lower_block(block)?;
        main_body.extend(wrap_block_with_auto_print(lowered, auto_print));
    }

    body.extend(main_body);

    Ok(PyModule { body, span })
}

fn lower_stmt(stmt: &Stmt) -> Result<PyStmt, LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => lower_if(cond, body, elifs, else_body, span),
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => Ok(PyStmt::While {
            test: lower_expr(cond)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => Ok(PyStmt::For {
            target: lower_assign_target(target)?,
            iter: lower_expr(iter)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::Def {
            name,
            params,
            body,
            span,
        } => Ok(PyStmt::FunctionDef {
            name: name.clone(),
            args: params
                .iter()
                .map(lower_parameter)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Class { name, body, span } => Ok(PyStmt::ClassDef {
            name: name.clone(),
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => Ok(PyStmt::Try {
            body: lower_block(body)?,
            handlers: handlers
                .iter()
                .map(lower_except_handler)
                .collect::<Result<Vec<_>, _>>()?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            finalbody: finally_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::With { items, body, span } => Ok(PyStmt::With {
            items: items
                .iter()
                .map(lower_with_item)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Return { value, span } => Ok(PyStmt::Return {
            value: value.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Raise { value, from, span } => Ok(PyStmt::Raise {
            value: value.as_ref().map(lower_expr).transpose()?,
            from: from.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Assert {
            test,
            message,
            span,
        } => Ok(PyStmt::Assert {
            test: lower_expr(test)?,
            message: message.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Delete { targets, span } => Ok(PyStmt::Delete {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Stmt::Break { span } => Ok(PyStmt::Break { span: span.clone() }),
        Stmt::Continue { span } => Ok(PyStmt::Continue { span: span.clone() }),
        Stmt::Pass { span } => Ok(PyStmt::Pass { span: span.clone() }),
        Stmt::Import { items, span } => Ok(PyStmt::Import {
            names: items.iter().map(lower_import_name).collect(),
            span: span.clone(),
        }),
        Stmt::ImportFrom {
            module,
            items,
            span,
        } => {
            if module.len() == 1 && module[0] == "__future__" {
                let filtered: Vec<&ImportItem> = items
                    .iter()
                    .filter(|item| !(item.name.len() == 1 && item.name[0] == "braces"))
                    .collect();
                if filtered.is_empty() {
                    return Ok(PyStmt::Pass { span: span.clone() });
                }
                return Ok(PyStmt::ImportFrom {
                    module: module.clone(),
                    names: filtered
                        .iter()
                        .map(|item| lower_import_name(item))
                        .collect(),
                    span: span.clone(),
                });
            }
            Ok(PyStmt::ImportFrom {
                module: module.clone(),
                names: items.iter().map(lower_import_name).collect(),
                span: span.clone(),
            })
        }
        Stmt::Assign {
            targets,
            value,
            span,
        } => Ok(PyStmt::Assign {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Stmt::Expr {
            value,
            semicolon_terminated,
            span,
        } => Ok(PyStmt::Expr {
            value: lower_expr(value)?,
            semicolon_terminated: *semicolon_terminated,
            span: span.clone(),
        }),
    }
}

fn lower_awk_file_loop_with_auto_print(
    program: &AwkProgram,
    span: &SourceSpan,
    auto_print: bool,
) -> Result<Vec<PyStmt>, LowerError> {
    let mut file_loop = Vec::new();
    file_loop.push(assign_name("__snail_fnr", number_expr("0", span), span));

    let stdin_body = vec![
        assign_name(
            "__snail_file",
            PyExpr::Attribute {
                value: Box::new(name_expr("sys", span)),
                attr: "stdin".to_string(),
                span: span.clone(),
            },
            span,
        ),
        lower_awk_line_loop_with_auto_print(
            program,
            span,
            name_expr("__snail_file", span),
            auto_print,
        )?,
    ];

    let open_call = PyExpr::Call {
        func: Box::new(name_expr("open", span)),
        args: vec![pos_arg(name_expr("__snail_path", span), span)],
        span: span.clone(),
    };
    let with_item = PyWithItem {
        context: open_call,
        target: Some(name_expr("__snail_file", span)),
        span: span.clone(),
    };
    let with_stmt = PyStmt::With {
        items: vec![with_item],
        body: vec![lower_awk_line_loop_with_auto_print(
            program,
            span,
            name_expr("__snail_file", span),
            auto_print,
        )?],
        span: span.clone(),
    };

    let test = PyExpr::Compare {
        left: Box::new(name_expr("__snail_path", span)),
        ops: vec![PyCompareOp::Eq],
        comparators: vec![string_expr("-", span)],
        span: span.clone(),
    };

    file_loop.push(PyStmt::If {
        test,
        body: stdin_body,
        orelse: vec![with_stmt],
        span: span.clone(),
    });

    Ok(file_loop)
}

fn lower_awk_line_loop_with_auto_print(
    program: &AwkProgram,
    span: &SourceSpan,
    iter: PyExpr,
    auto_print: bool,
) -> Result<PyStmt, LowerError> {
    let mut loop_body = Vec::new();
    loop_body.push(assign_name(
        "__snail_nr",
        PyExpr::Binary {
            left: Box::new(name_expr("__snail_nr", span)),
            op: PyBinaryOp::Add,
            right: Box::new(number_expr("1", span)),
            span: span.clone(),
        },
        span,
    ));
    loop_body.push(assign_name(
        "__snail_fnr",
        PyExpr::Binary {
            left: Box::new(name_expr("__snail_fnr", span)),
            op: PyBinaryOp::Add,
            right: Box::new(number_expr("1", span)),
            span: span.clone(),
        },
        span,
    ));

    let rstrip_call = PyExpr::Call {
        func: Box::new(PyExpr::Attribute {
            value: Box::new(name_expr("__snail_raw", span)),
            attr: "rstrip".to_string(),
            span: span.clone(),
        }),
        args: vec![pos_arg(string_expr("\\n", span), span)],
        span: span.clone(),
    };
    loop_body.push(assign_name(SNAIL_AWK_LINE_PYVAR, rstrip_call, span));

    let split_call = PyExpr::Call {
        func: Box::new(PyExpr::Attribute {
            value: Box::new(name_expr(SNAIL_AWK_LINE_PYVAR, span)),
            attr: "split".to_string(),
            span: span.clone(),
        }),
        args: Vec::new(),
        span: span.clone(),
    };
    loop_body.push(assign_name(SNAIL_AWK_FIELDS_PYVAR, split_call, span));
    loop_body.push(assign_name(
        SNAIL_AWK_NR_PYVAR,
        name_expr("__snail_nr", span),
        span,
    ));
    loop_body.push(assign_name(
        SNAIL_AWK_FNR_PYVAR,
        name_expr("__snail_fnr", span),
        span,
    ));
    loop_body.push(assign_name(
        SNAIL_AWK_PATH_PYVAR,
        name_expr("__snail_path", span),
        span,
    ));

    loop_body.extend(lower_awk_rules_with_auto_print(&program.rules, auto_print)?);

    Ok(PyStmt::For {
        target: name_expr("__snail_raw", span),
        iter,
        body: loop_body,
        orelse: Vec::new(),
        span: span.clone(),
    })
}

fn wrap_block_with_auto_print(mut block: Vec<PyStmt>, auto_print: bool) -> Vec<PyStmt> {
    if !auto_print || block.is_empty() {
        return block;
    }

    let last_idx = block.len() - 1;
    if let PyStmt::Expr {
        value,
        semicolon_terminated,
        span,
    } = &block[last_idx]
    {
        // Don't auto-print if the statement was explicitly terminated with a semicolon
        if *semicolon_terminated {
            return block;
        }

        // Clone the data before modifying the block
        let expr_code = value.clone();
        let span = span.clone();

        block.pop(); // Remove the original expression statement

        block.push(PyStmt::Assign {
            targets: vec![PyExpr::Name {
                id: "__snail_last_result".to_string(),
                span: span.clone(),
            }],
            value: expr_code,
            span: span.clone(),
        });

        // Build: isinstance(__snail_last_result, str)
        let is_string = PyExpr::Call {
            func: Box::new(PyExpr::Name {
                id: "isinstance".to_string(),
                span: span.clone(),
            }),
            args: vec![
                PyArgument::Positional {
                    value: PyExpr::Name {
                        id: "__snail_last_result".to_string(),
                        span: span.clone(),
                    },
                    span: span.clone(),
                },
                PyArgument::Positional {
                    value: PyExpr::Name {
                        id: "str".to_string(),
                        span: span.clone(),
                    },
                    span: span.clone(),
                },
            ],
            span: span.clone(),
        };

        // Build: __snail_last_result is not None
        let is_not_none = PyExpr::Compare {
            left: Box::new(PyExpr::Name {
                id: "__snail_last_result".to_string(),
                span: span.clone(),
            }),
            ops: vec![PyCompareOp::Is],
            comparators: vec![PyExpr::Unary {
                op: PyUnaryOp::Not,
                operand: Box::new(PyExpr::None { span: span.clone() }),
                span: span.clone(),
            }],
            span: span.clone(),
        };

        // if isinstance(__snail_last_result, str): print(__snail_last_result)
        // elif __snail_last_result is not None: pprint.pprint(__snail_last_result)
        block.push(PyStmt::If {
            test: is_string,
            body: vec![PyStmt::Expr {
                value: PyExpr::Call {
                    func: Box::new(PyExpr::Name {
                        id: "print".to_string(),
                        span: span.clone(),
                    }),
                    args: vec![PyArgument::Positional {
                        value: PyExpr::Name {
                            id: "__snail_last_result".to_string(),
                            span: span.clone(),
                        },
                        span: span.clone(),
                    }],
                    span: span.clone(),
                },
                semicolon_terminated: false,
                span: span.clone(),
            }],
            orelse: vec![PyStmt::If {
                test: is_not_none,
                body: vec![
                    PyStmt::Import {
                        names: vec![PyImportName {
                            name: vec!["pprint".to_string()],
                            asname: None,
                            span: span.clone(),
                        }],
                        span: span.clone(),
                    },
                    PyStmt::Expr {
                        value: PyExpr::Call {
                            func: Box::new(PyExpr::Attribute {
                                value: Box::new(PyExpr::Name {
                                    id: "pprint".to_string(),
                                    span: span.clone(),
                                }),
                                attr: "pprint".to_string(),
                                span: span.clone(),
                            }),
                            args: vec![PyArgument::Positional {
                                value: PyExpr::Name {
                                    id: "__snail_last_result".to_string(),
                                    span: span.clone(),
                                },
                                span: span.clone(),
                            }],
                            span: span.clone(),
                        },
                        semicolon_terminated: false,
                        span: span.clone(),
                    },
                ],
                orelse: Vec::new(),
                span: span.clone(),
            }],
            span: span.clone(),
        });
    }

    block
}

fn lower_awk_rules_with_auto_print(
    rules: &[AwkRule],
    auto_print: bool,
) -> Result<Vec<PyStmt>, LowerError> {
    let mut stmts = Vec::new();
    for rule in rules {
        let mut action = if rule.has_explicit_action() {
            let lowered = lower_block(rule.action.as_ref().unwrap())?;
            wrap_block_with_auto_print(lowered, auto_print)
        } else {
            // Bare pattern with no action - default to printing the line
            vec![awk_default_print(&rule.span)]
        };

        if let Some(pattern) = &rule.pattern {
            if let Some((value_expr, regex, span)) = regex_pattern_components(pattern) {
                let match_call = lower_regex_match(&value_expr, &regex, &span, None)?;
                stmts.push(PyStmt::Assign {
                    targets: vec![name_expr(SNAIL_AWK_MATCH_PYVAR, &span)],
                    value: match_call,
                    span: span.clone(),
                });
                stmts.push(PyStmt::If {
                    test: name_expr(SNAIL_AWK_MATCH_PYVAR, &span),
                    body: action,
                    orelse: Vec::new(),
                    span: rule.span.clone(),
                });
            } else {
                stmts.push(PyStmt::If {
                    test: lower_expr(pattern)?,
                    body: action,
                    orelse: Vec::new(),
                    span: rule.span.clone(),
                });
            }
        } else {
            stmts.append(&mut action);
        }
    }
    Ok(stmts)
}

fn regex_pattern_components(pattern: &Expr) -> Option<(Expr, RegexPattern, SourceSpan)> {
    match pattern {
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Some((*value.clone(), pattern.clone(), span.clone())),
        Expr::Regex { pattern, span } => Some((
            Expr::Name {
                name: SNAIL_AWK_LINE.to_string(),
                span: span.clone(),
            },
            pattern.clone(),
            span.clone(),
        )),
        _ => None,
    }
}

fn awk_default_print(span: &SourceSpan) -> PyStmt {
    PyStmt::Expr {
        value: PyExpr::Call {
            func: Box::new(name_expr("print", span)),
            args: vec![pos_arg(name_expr(SNAIL_AWK_LINE_PYVAR, span), span)],
            span: span.clone(),
        },
        semicolon_terminated: false,
        span: span.clone(),
    }
}

fn assign_name(name: &str, value: PyExpr, span: &SourceSpan) -> PyStmt {
    PyStmt::Assign {
        targets: vec![name_expr(name, span)],
        value,
        span: span.clone(),
    }
}

fn name_expr(name: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Name {
        id: name.to_string(),
        span: span.clone(),
    }
}

fn string_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: value.to_string(),
        raw: false,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

fn number_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Number {
        value: value.to_string(),
        span: span.clone(),
    }
}

fn regex_pattern_expr(pattern: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: pattern.to_string(),
        raw: true,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

fn pos_arg(value: PyExpr, span: &SourceSpan) -> PyArgument {
    PyArgument::Positional {
        value,
        span: span.clone(),
    }
}

fn lower_if(
    cond: &Expr,
    body: &[Stmt],
    elifs: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span_hint: &SourceSpan,
) -> Result<PyStmt, LowerError> {
    let test = lower_expr(cond)?;
    let body = lower_block(body)?;
    let mut span = span_from_block(&body).unwrap_or_else(|| span_hint.clone());
    span.start = expr_span(&test).start.clone();
    if let Some((elif_cond, elif_body)) = elifs.first() {
        let nested = lower_if(elif_cond, elif_body, &elifs[1..], else_body, span_hint)?;
        span.end = stmt_span(&nested).end.clone();
        Ok(PyStmt::If {
            test,
            body,
            orelse: vec![nested],
            span,
        })
    } else {
        let orelse = match else_body {
            Some(else_block) => lower_block(else_block)?,
            None => Vec::new(),
        };
        if let Some(last) = orelse.last() {
            span.end = stmt_span(last).end.clone();
        }
        Ok(PyStmt::If {
            test,
            body,
            orelse,
            span,
        })
    }
}

fn lower_block(block: &[Stmt]) -> Result<Vec<PyStmt>, LowerError> {
    block.iter().map(lower_stmt).collect()
}

fn lower_except_handler(handler: &ExceptHandler) -> Result<PyExceptHandler, LowerError> {
    Ok(PyExceptHandler {
        type_name: handler.type_name.as_ref().map(lower_expr).transpose()?,
        name: handler.name.clone(),
        body: lower_block(&handler.body)?,
        span: handler.span.clone(),
    })
}

fn lower_with_item(item: &WithItem) -> Result<PyWithItem, LowerError> {
    Ok(PyWithItem {
        context: lower_expr(&item.context)?,
        target: item.target.as_ref().map(lower_assign_target).transpose()?,
        span: item.span.clone(),
    })
}

fn lower_import_name(item: &ImportItem) -> PyImportName {
    PyImportName {
        name: item.name.clone(),
        asname: item.alias.clone(),
        span: item.span.clone(),
    }
}

fn lower_assign_target(target: &AssignTarget) -> Result<PyExpr, LowerError> {
    match target {
        AssignTarget::Name { name, span } => Ok(PyExpr::Name {
            id: name.clone(),
            span: span.clone(),
        }),
        AssignTarget::Attribute { value, attr, span } => Ok(PyExpr::Attribute {
            value: Box::new(lower_expr(value)?),
            attr: attr.clone(),
            span: span.clone(),
        }),
        AssignTarget::Index { value, index, span } => Ok(PyExpr::Index {
            value: Box::new(lower_expr(value)?),
            index: Box::new(lower_expr(index)?),
            span: span.clone(),
        }),
    }
}

fn lower_expr(expr: &Expr) -> Result<PyExpr, LowerError> {
    lower_expr_with_exception(expr, None)
}

fn lower_regex_match(
    value: &Expr,
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    Ok(PyExpr::Call {
        func: Box::new(PyExpr::Name {
            id: SNAIL_REGEX_SEARCH.to_string(),
            span: span.clone(),
        }),
        args: vec![
            pos_arg(lower_expr_with_exception(value, exception_name)?, span),
            pos_arg(
                lower_regex_pattern_expr(pattern, span, exception_name)?,
                span,
            ),
        ],
        span: span.clone(),
    })
}

fn lower_regex_pattern_expr(
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    match pattern {
        RegexPattern::Literal(text) => Ok(regex_pattern_expr(text, span)),
        RegexPattern::Interpolated(parts) => Ok(PyExpr::FString {
            parts: lower_fstring_parts(parts, exception_name)?,
            span: span.clone(),
        }),
    }
}

fn lower_fstring_parts(
    parts: &[FStringPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyFStringPart>, LowerError> {
    let mut lowered = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => lowered.push(PyFStringPart::Text(text.clone())),
            FStringPart::Expr(expr) => {
                lowered.push(PyFStringPart::Expr(lower_expr_with_exception(
                    expr,
                    exception_name,
                )?));
            }
        }
    }
    Ok(lowered)
}

fn lower_expr_with_exception(
    expr: &Expr,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    match expr {
        Expr::Name { name, span } => {
            if name == "$e" {
                if let Some(exception_name) = exception_name {
                    return Ok(PyExpr::Name {
                        id: exception_name.to_string(),
                        span: span.clone(),
                    });
                }
                return Err(LowerError::new(
                    "`$e` is only available in compact exception fallbacks",
                ));
            }
            if let Some(py_name) = injected_py_name(name) {
                return Ok(PyExpr::Name {
                    id: py_name.to_string(),
                    span: span.clone(),
                });
            }
            Ok(PyExpr::Name {
                id: name.clone(),
                span: span.clone(),
            })
        }
        Expr::FieldIndex { index, span } => {
            // AWK convention: $0 is the whole line, $1 is first field, etc.
            if index == "0" {
                Ok(PyExpr::Name {
                    id: SNAIL_AWK_LINE_PYVAR.to_string(),
                    span: span.clone(),
                })
            } else {
                // Parse index and convert from 1-based to 0-based
                let field_index = index.parse::<i32>().map_err(|_| LowerError {
                    message: format!("Invalid field index: ${}", index),
                })?;
                let python_index = field_index - 1;

                Ok(PyExpr::Index {
                    value: Box::new(PyExpr::Name {
                        id: SNAIL_AWK_FIELDS_PYVAR.to_string(),
                        span: span.clone(),
                    }),
                    index: Box::new(PyExpr::Number {
                        value: python_index.to_string(),
                        span: span.clone(),
                    }),
                    span: span.clone(),
                })
            }
        }
        Expr::Number { value, span } => Ok(PyExpr::Number {
            value: value.clone(),
            span: span.clone(),
        }),
        Expr::String {
            value,
            raw,
            delimiter,
            span,
        } => Ok(PyExpr::String {
            value: value.clone(),
            raw: *raw,
            delimiter: *delimiter,
            span: span.clone(),
        }),
        Expr::FString { parts, span } => Ok(PyExpr::FString {
            parts: lower_fstring_parts(parts, exception_name)?,
            span: span.clone(),
        }),
        Expr::Bool { value, span } => Ok(PyExpr::Bool {
            value: *value,
            span: span.clone(),
        }),
        Expr::None { span } => Ok(PyExpr::None { span: span.clone() }),
        Expr::Unary { op, expr, span } => Ok(PyExpr::Unary {
            op: lower_unary_op(*op),
            operand: Box::new(lower_expr_with_exception(expr, exception_name)?),
            span: span.clone(),
        }),
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => {
            if *op == BinaryOp::Pipeline {
                // Pipeline: x | y becomes y.__pipeline__(x)
                let left_expr = lower_expr_with_exception(left, exception_name)?;

                // Special handling for Subprocess on RHS: just create the object, don't call __pipeline__(None)
                let right_obj = match right.as_ref() {
                    Expr::Subprocess {
                        kind,
                        parts,
                        span: s_span,
                    } => {
                        // Create just the __SnailSubprocess{Capture|Status}(cmd) object
                        let mut lowered_parts = Vec::with_capacity(parts.len());
                        for part in parts {
                            match part {
                                SubprocessPart::Text(text) => {
                                    lowered_parts.push(PyFStringPart::Text(text.clone()));
                                }
                                SubprocessPart::Expr(expr) => {
                                    lowered_parts.push(PyFStringPart::Expr(
                                        lower_expr_with_exception(expr, exception_name)?,
                                    ));
                                }
                            }
                        }
                        let command = PyExpr::FString {
                            parts: lowered_parts,
                            span: s_span.clone(),
                        };
                        let class_name = match kind {
                            SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
                            SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
                        };
                        PyExpr::Call {
                            func: Box::new(PyExpr::Name {
                                id: class_name.to_string(),
                                span: s_span.clone(),
                            }),
                            args: vec![PyArgument::Positional {
                                value: command,
                                span: s_span.clone(),
                            }],
                            span: s_span.clone(),
                        }
                    }
                    _ => lower_expr_with_exception(right, exception_name)?,
                };

                Ok(PyExpr::Call {
                    func: Box::new(PyExpr::Attribute {
                        value: Box::new(right_obj),
                        attr: "__pipeline__".to_string(),
                        span: span.clone(),
                    }),
                    args: vec![PyArgument::Positional {
                        value: left_expr,
                        span: span.clone(),
                    }],
                    span: span.clone(),
                })
            } else {
                Ok(PyExpr::Binary {
                    left: Box::new(lower_expr_with_exception(left, exception_name)?),
                    op: lower_binary_op(*op),
                    right: Box::new(lower_expr_with_exception(right, exception_name)?),
                    span: span.clone(),
                })
            }
        }
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Ok(PyExpr::Compare {
            left: Box::new(lower_expr_with_exception(left, exception_name)?),
            ops: ops.iter().map(|op| lower_compare_op(*op)).collect(),
            comparators: comparators
                .iter()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => Ok(PyExpr::IfExpr {
            test: Box::new(lower_expr_with_exception(test, exception_name)?),
            body: Box::new(lower_expr_with_exception(body, exception_name)?),
            orelse: Box::new(lower_expr_with_exception(orelse, exception_name)?),
            span: span.clone(),
        }),
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => {
            let try_lambda = PyExpr::Lambda {
                params: Vec::new(),
                body: Box::new(lower_expr_with_exception(expr, exception_name)?),
                span: span.clone(),
            };
            let mut args = vec![PyArgument::Positional {
                value: try_lambda,
                span: span.clone(),
            }];
            if let Some(fallback_expr) = fallback {
                let fallback_lambda = PyExpr::Lambda {
                    params: vec![SNAIL_EXCEPTION_VAR.to_string()],
                    body: Box::new(lower_expr_with_exception(
                        fallback_expr,
                        Some(SNAIL_EXCEPTION_VAR),
                    )?),
                    span: span.clone(),
                };
                args.push(PyArgument::Positional {
                    value: fallback_lambda,
                    span: span.clone(),
                });
            }
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: SNAIL_TRY_HELPER.to_string(),
                    span: span.clone(),
                }),
                args,
                span: span.clone(),
            })
        }
        Expr::Compound { expressions, span } => {
            let mut lowered = Vec::new();
            for expr in expressions {
                lowered.push(lower_expr_with_exception(expr, exception_name)?);
            }

            let tuple_expr = PyExpr::Tuple {
                elements: lowered,
                span: span.clone(),
            };

            let index_expr = PyExpr::Unary {
                op: PyUnaryOp::Minus,
                operand: Box::new(PyExpr::Number {
                    value: "1".to_string(),
                    span: span.clone(),
                }),
                span: span.clone(),
            };

            Ok(PyExpr::Index {
                value: Box::new(tuple_expr),
                index: Box::new(index_expr),
                span: span.clone(),
            })
        }
        Expr::Regex { pattern, span } => Ok(PyExpr::Call {
            func: Box::new(PyExpr::Name {
                id: SNAIL_REGEX_COMPILE.to_string(),
                span: span.clone(),
            }),
            args: vec![pos_arg(
                lower_regex_pattern_expr(pattern, span, exception_name)?,
                span,
            )],
            span: span.clone(),
        }),
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => lower_regex_match(value, pattern, span, exception_name),
        Expr::Subprocess { kind, parts, span } => {
            let mut lowered_parts = Vec::with_capacity(parts.len());
            for part in parts {
                match part {
                    SubprocessPart::Text(text) => {
                        lowered_parts.push(PyFStringPart::Text(text.clone()));
                    }
                    SubprocessPart::Expr(expr) => {
                        lowered_parts.push(PyFStringPart::Expr(lower_expr_with_exception(
                            expr,
                            exception_name,
                        )?));
                    }
                }
            }
            let command = PyExpr::FString {
                parts: lowered_parts,
                span: span.clone(),
            };
            let class_name = match kind {
                SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
                SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
            };
            // $(cmd) becomes __SnailSubprocessCapture(cmd).__pipeline__(None)
            let subprocess_obj = PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: class_name.to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: command,
                    span: span.clone(),
                }],
                span: span.clone(),
            };
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Attribute {
                    value: Box::new(subprocess_obj),
                    attr: "__pipeline__".to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: PyExpr::None { span: span.clone() },
                    span: span.clone(),
                }],
                span: span.clone(),
            })
        }
        Expr::StructuredAccessor { query, span } => {
            // $[query] becomes __SnailStructuredAccessor(query)
            // The query is raw source text, so we need to escape it for Python
            let escaped_query = escape_for_python_string(query);
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: SNAIL_STRUCTURED_ACCESSOR_CLASS.to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: PyExpr::String {
                        value: escaped_query,
                        raw: false,
                        delimiter: StringDelimiter::Double,
                        span: span.clone(),
                    },
                    span: span.clone(),
                }],
                span: span.clone(),
            })
        }
        Expr::Call { func, args, span } => Ok(PyExpr::Call {
            func: Box::new(lower_expr_with_exception(func, exception_name)?),
            args: args
                .iter()
                .map(|arg| lower_argument(arg, exception_name))
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::Attribute { value, attr, span } => Ok(PyExpr::Attribute {
            value: Box::new(lower_expr_with_exception(value, exception_name)?),
            attr: attr.clone(),
            span: span.clone(),
        }),
        Expr::Index { value, index, span } => Ok(PyExpr::Index {
            value: Box::new(lower_expr_with_exception(value, exception_name)?),
            index: Box::new(lower_expr_with_exception(index, exception_name)?),
            span: span.clone(),
        }),
        Expr::Paren { expr, span } => Ok(PyExpr::Paren {
            expr: Box::new(lower_expr_with_exception(expr, exception_name)?),
            span: span.clone(),
        }),
        Expr::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::List {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::Tuple {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Dict { entries, span } => {
            let mut lowered = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                lowered.push((
                    lower_expr_with_exception(key, exception_name)?,
                    lower_expr_with_exception(value, exception_name)?,
                ));
            }
            Ok(PyExpr::Dict {
                entries: lowered,
                span: span.clone(),
            })
        }
        Expr::Set { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::Set {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => {
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(cond, exception_name)?);
            }
            Ok(PyExpr::ListComp {
                element: Box::new(lower_expr_with_exception(element, exception_name)?),
                target: target.clone(),
                iter: Box::new(lower_expr_with_exception(iter, exception_name)?),
                ifs: lowered_ifs,
                span: span.clone(),
            })
        }
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => {
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(cond, exception_name)?);
            }
            Ok(PyExpr::DictComp {
                key: Box::new(lower_expr_with_exception(key, exception_name)?),
                value: Box::new(lower_expr_with_exception(value, exception_name)?),
                target: target.clone(),
                iter: Box::new(lower_expr_with_exception(iter, exception_name)?),
                ifs: lowered_ifs,
                span: span.clone(),
            })
        }
        Expr::Slice { start, end, span } => Ok(PyExpr::Slice {
            start: start
                .as_deref()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .transpose()?
                .map(Box::new),
            end: end
                .as_deref()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .transpose()?
                .map(Box::new),
            span: span.clone(),
        }),
    }
}

fn lower_argument(arg: &Argument, exception_name: Option<&str>) -> Result<PyArgument, LowerError> {
    match arg {
        Argument::Positional { value, span } => Ok(PyArgument::Positional {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::Keyword { name, value, span } => Ok(PyArgument::Keyword {
            name: name.clone(),
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::Star { value, span } => Ok(PyArgument::Star {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::KwStar { value, span } => Ok(PyArgument::KwStar {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
    }
}

fn lower_parameter(param: &Parameter) -> Result<PyParameter, LowerError> {
    match param {
        Parameter::Regular {
            name,
            default,
            span,
        } => Ok(PyParameter::Regular {
            name: name.clone(),
            default: default.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Parameter::VarArgs { name, span } => Ok(PyParameter::VarArgs {
            name: name.clone(),
            span: span.clone(),
        }),
        Parameter::KwArgs { name, span } => Ok(PyParameter::KwArgs {
            name: name.clone(),
            span: span.clone(),
        }),
    }
}

fn span_from_block(block: &[PyStmt]) -> Option<SourceSpan> {
    let first = block.first()?;
    let last = block.last()?;
    Some(merge_span(stmt_span(first), stmt_span(last)))
}

fn stmt_span(stmt: &PyStmt) -> &SourceSpan {
    match stmt {
        PyStmt::If { span, .. }
        | PyStmt::While { span, .. }
        | PyStmt::For { span, .. }
        | PyStmt::FunctionDef { span, .. }
        | PyStmt::ClassDef { span, .. }
        | PyStmt::Try { span, .. }
        | PyStmt::With { span, .. }
        | PyStmt::Return { span, .. }
        | PyStmt::Raise { span, .. }
        | PyStmt::Assert { span, .. }
        | PyStmt::Delete { span, .. }
        | PyStmt::Break { span, .. }
        | PyStmt::Continue { span, .. }
        | PyStmt::Pass { span, .. }
        | PyStmt::Import { span, .. }
        | PyStmt::ImportFrom { span, .. }
        | PyStmt::Assign { span, .. }
        | PyStmt::Expr { span, .. } => span,
    }
}

fn expr_span(expr: &PyExpr) -> &SourceSpan {
    match expr {
        PyExpr::Name { span, .. }
        | PyExpr::Number { span, .. }
        | PyExpr::String { span, .. }
        | PyExpr::FString { span, .. }
        | PyExpr::Bool { span, .. }
        | PyExpr::None { span }
        | PyExpr::Unary { span, .. }
        | PyExpr::Binary { span, .. }
        | PyExpr::Compare { span, .. }
        | PyExpr::IfExpr { span, .. }
        | PyExpr::Lambda { span, .. }
        | PyExpr::Call { span, .. }
        | PyExpr::Attribute { span, .. }
        | PyExpr::Index { span, .. }
        | PyExpr::Paren { span, .. }
        | PyExpr::List { span, .. }
        | PyExpr::Tuple { span, .. }
        | PyExpr::Dict { span, .. }
        | PyExpr::Set { span, .. }
        | PyExpr::ListComp { span, .. }
        | PyExpr::DictComp { span, .. }
        | PyExpr::Slice { span, .. } => span,
    }
}

fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start: left.start.clone(),
        end: right.end.clone(),
    }
}

fn lower_unary_op(op: UnaryOp) -> PyUnaryOp {
    match op {
        UnaryOp::Plus => PyUnaryOp::Plus,
        UnaryOp::Minus => PyUnaryOp::Minus,
        UnaryOp::Not => PyUnaryOp::Not,
    }
}

fn lower_binary_op(op: BinaryOp) -> PyBinaryOp {
    match op {
        BinaryOp::Or => PyBinaryOp::Or,
        BinaryOp::And => PyBinaryOp::And,
        BinaryOp::Add => PyBinaryOp::Add,
        BinaryOp::Sub => PyBinaryOp::Sub,
        BinaryOp::Mul => PyBinaryOp::Mul,
        BinaryOp::Div => PyBinaryOp::Div,
        BinaryOp::FloorDiv => PyBinaryOp::FloorDiv,
        BinaryOp::Mod => PyBinaryOp::Mod,
        BinaryOp::Pow => PyBinaryOp::Pow,
        BinaryOp::Pipeline => {
            panic!("Pipeline operator should be handled specially in lower_expr_with_exception")
        }
    }
}

fn lower_compare_op(op: CompareOp) -> PyCompareOp {
    match op {
        CompareOp::Eq => PyCompareOp::Eq,
        CompareOp::NotEq => PyCompareOp::NotEq,
        CompareOp::Lt => PyCompareOp::Lt,
        CompareOp::LtEq => PyCompareOp::LtEq,
        CompareOp::Gt => PyCompareOp::Gt,
        CompareOp::GtEq => PyCompareOp::GtEq,
        CompareOp::In => PyCompareOp::In,
        CompareOp::Is => PyCompareOp::Is,
    }
}
