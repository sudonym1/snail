use snail_ast::*;
use snail_error::LowerError;
use snail_python_ast::*;

use crate::constants::*;
use crate::helpers::*;
use crate::operators::*;

pub(crate) fn lower_expr(expr: &Expr) -> Result<PyExpr, LowerError> {
    lower_expr_with_exception(expr, None)
}

pub(crate) fn lower_assign_target(target: &AssignTarget) -> Result<PyExpr, LowerError> {
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

pub(crate) fn lower_regex_match(
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

pub(crate) fn lower_expr_with_exception(
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
