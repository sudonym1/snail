use snail_ast::*;
use snail_error::LowerError;
use snail_python_ast::*;

use crate::constants::*;
use crate::expr::{lower_expr, lower_regex_match};
use crate::helpers::*;
use crate::stmt::lower_block;

pub(crate) fn lower_awk_file_loop_with_auto_print(
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

pub(crate) fn lower_awk_line_loop_with_auto_print(
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

pub(crate) fn wrap_block_with_auto_print(mut block: Vec<PyStmt>, auto_print: bool) -> Vec<PyStmt> {
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
