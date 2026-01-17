use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use crate::constants::*;
use crate::helpers::{name_expr, number_expr, regex_pattern_expr, string_expr};
use crate::operators::{lower_binary_op, lower_bool_op, lower_compare_op, lower_unary_op};
use crate::py_ast::{AstBuilder, py_err_to_lower};

pub(crate) fn lower_expr(builder: &AstBuilder<'_>, expr: &Expr) -> Result<PyObject, LowerError> {
    lower_expr_with_exception(builder, expr, None)
}

pub(crate) fn lower_assign_target(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
) -> Result<PyObject, LowerError> {
    let store_ctx = builder.store_ctx().map_err(py_err_to_lower)?;
    match target {
        AssignTarget::Name { name, span } => name_expr(builder, name, span, store_ctx),
        AssignTarget::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, None)?;
            builder
                .call_node(
                    "Attribute",
                    vec![value, attr.to_string().into_py(builder.py()), store_ctx],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::Index { value, index, span } => {
            let value_expr = lower_expr_with_exception(builder, value, None)?;
            let index_expr = lower_expr_with_exception(builder, index, None)?;
            builder
                .call_node("Subscript", vec![value_expr, index_expr, store_ctx], span)
                .map_err(py_err_to_lower)
        }
    }
}

pub(crate) fn lower_delete_target(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
) -> Result<PyObject, LowerError> {
    let del_ctx = builder.del_ctx().map_err(py_err_to_lower)?;
    match target {
        AssignTarget::Name { name, span } => name_expr(builder, name, span, del_ctx),
        AssignTarget::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, None)?;
            builder
                .call_node(
                    "Attribute",
                    vec![value, attr.to_string().into_py(builder.py()), del_ctx],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::Index { value, index, span } => {
            let value_expr = lower_expr_with_exception(builder, value, None)?;
            let index_expr = lower_expr_with_exception(builder, index, None)?;
            builder
                .call_node("Subscript", vec![value_expr, index_expr, del_ctx], span)
                .map_err(py_err_to_lower)
        }
    }
}

pub(crate) fn lower_regex_match(
    builder: &AstBuilder<'_>,
    value: &Expr,
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let func = name_expr(
        builder,
        SNAIL_REGEX_SEARCH,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let value_expr = lower_expr_with_exception(builder, value, exception_name)?;
    let pattern_expr = lower_regex_pattern_expr(builder, pattern, span, exception_name)?;
    let args = vec![value_expr, pattern_expr];
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), args).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_regex_pattern_expr(
    builder: &AstBuilder<'_>,
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match pattern {
        RegexPattern::Literal(text) => regex_pattern_expr(builder, text, span),
        RegexPattern::Interpolated(parts) => {
            let values = lower_fstring_parts(builder, parts, exception_name)?;
            builder
                .call_node(
                    "JoinedStr",
                    vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn lower_fstring_parts(
    builder: &AstBuilder<'_>,
    parts: &[FStringPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyObject>, LowerError> {
    let mut lowered = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => {
                let const_node = builder
                    .call_node(
                        "Constant",
                        vec![text.clone().into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered.push(const_node);
            }
            FStringPart::Expr(expr) => {
                let value = lower_expr_with_exception(builder, expr, exception_name)?;
                let conversion = (-1i32).into_py(builder.py());
                let format_spec = builder.py().None();
                let formatted = builder
                    .call_node(
                        "FormattedValue",
                        vec![value, conversion, format_spec.into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered.push(formatted);
            }
        }
    }
    Ok(lowered)
}

pub(crate) fn lower_expr_with_exception(
    builder: &AstBuilder<'_>,
    expr: &Expr,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match expr {
        Expr::Name { name, span } => {
            if name == "$e" {
                if let Some(exception_name) = exception_name {
                    return name_expr(
                        builder,
                        exception_name,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    );
                }
                return Err(LowerError::new(
                    "`$e` is only available in compact exception fallbacks",
                ));
            }
            if let Some(py_name) = injected_py_name(name) {
                return name_expr(
                    builder,
                    py_name,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                );
            }
            name_expr(
                builder,
                name,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )
        }
        Expr::Placeholder { span } => name_expr(
            builder,
            "_",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        ),
        Expr::FieldIndex { index, span } => {
            if index == "0" {
                return name_expr(
                    builder,
                    SNAIL_AWK_LINE_PYVAR,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                );
            }
            let field_index = index
                .parse::<i32>()
                .map_err(|_| LowerError::new(format!("Invalid field index: ${}", index)))?;
            let python_index = field_index - 1;
            let value = name_expr(
                builder,
                SNAIL_AWK_FIELDS_PYVAR,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let index_expr = number_expr(builder, &python_index.to_string(), span)?;
            builder
                .call_node(
                    "Subscript",
                    vec![
                        value,
                        index_expr,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Number { value, span } => number_expr(builder, value, span),
        Expr::String {
            value,
            raw,
            delimiter,
            span,
        } => string_expr(builder, value, *raw, *delimiter, span),
        Expr::FString { parts, span } => {
            let values = lower_fstring_parts(builder, parts, exception_name)?;
            builder
                .call_node(
                    "JoinedStr",
                    vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Bool { value, span } => builder
            .call_node("Constant", vec![value.into_py(builder.py())], span)
            .map_err(py_err_to_lower),
        Expr::None { span } => builder
            .call_node(
                "Constant",
                vec![builder.py().None().into_py(builder.py())],
                span,
            )
            .map_err(py_err_to_lower),
        Expr::Unary { op, expr, span } => {
            let operand = lower_expr_with_exception(builder, expr, exception_name)?;
            let op = lower_unary_op(builder, *op)?;
            builder
                .call_node("UnaryOp", vec![op, operand], span)
                .map_err(py_err_to_lower)
        }
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => {
            if *op == BinaryOp::Pipeline {
                // Pipeline: a | b lowers to b(a) with placeholder support for call args.
                match right.as_ref() {
                    Expr::Call {
                        func,
                        args,
                        span: call_span,
                    } => {
                        let placeholder_info = placeholder_info_in_arguments(args);
                        if placeholder_info.count > 1 {
                            let span = placeholder_info
                                .first_span
                                .unwrap_or_else(|| call_span.clone());
                            return Err(LowerError::multiple_placeholders(span));
                        }
                        if placeholder_info.count == 1 {
                            let replaced_args = substitute_placeholder_in_arguments(args, left);
                            let func_expr =
                                lower_expr_with_exception(builder, func, exception_name)?;
                            let (args, keywords) =
                                lower_call_arguments(builder, &replaced_args, exception_name)?;
                            let mut helper_args = Vec::with_capacity(args.len() + 1);
                            helper_args.push(func_expr);
                            helper_args.extend(args);
                            let helper = name_expr(
                                builder,
                                SNAIL_PARTIAL_HELPER,
                                call_span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?;
                            let partial_call = builder
                                .call_node(
                                    "Call",
                                    vec![
                                        helper,
                                        PyList::new_bound(builder.py(), helper_args)
                                            .into_py(builder.py()),
                                        PyList::new_bound(builder.py(), keywords)
                                            .into_py(builder.py()),
                                    ],
                                    call_span,
                                )
                                .map_err(py_err_to_lower)?;
                            return builder
                                .call_node(
                                    "Call",
                                    vec![
                                        partial_call,
                                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                                    ],
                                    span,
                                )
                                .map_err(py_err_to_lower);
                        }

                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj = lower_expr_with_exception(builder, right, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                    Expr::Subprocess {
                        kind,
                        parts,
                        span: s_span,
                    } => {
                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj =
                            lower_subprocess_object(builder, kind, parts, s_span, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                    _ => {
                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj = lower_expr_with_exception(builder, right, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                }
            } else if *op == BinaryOp::Or || *op == BinaryOp::And {
                let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                let right_expr = lower_expr_with_exception(builder, right, exception_name)?;
                let op = lower_bool_op(builder, *op)?;
                builder
                    .call_node(
                        "BoolOp",
                        vec![
                            op,
                            PyList::new_bound(builder.py(), vec![left_expr, right_expr])
                                .into_py(builder.py()),
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)
            } else {
                let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                let right_expr = lower_expr_with_exception(builder, right, exception_name)?;
                let op = lower_binary_op(builder, *op)?;
                builder
                    .call_node("BinOp", vec![left_expr, op, right_expr], span)
                    .map_err(py_err_to_lower)
            }
        }
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => {
            let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
            let ops = ops
                .iter()
                .map(|op| lower_compare_op(builder, *op))
                .collect::<Result<Vec<_>, _>>()?;
            let comparators = comparators
                .iter()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .collect::<Result<Vec<_>, _>>()?;
            builder
                .call_node(
                    "Compare",
                    vec![
                        left_expr,
                        PyList::new_bound(builder.py(), ops).into_py(builder.py()),
                        PyList::new_bound(builder.py(), comparators).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => {
            let test = lower_expr_with_exception(builder, test, exception_name)?;
            let body = lower_expr_with_exception(builder, body, exception_name)?;
            let orelse = lower_expr_with_exception(builder, orelse, exception_name)?;
            builder
                .call_node("IfExp", vec![test, body, orelse], span)
                .map_err(py_err_to_lower)
        }
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => {
            let try_lambda = builder
                .call_node(
                    "Lambda",
                    vec![
                        empty_lambda_args(builder)?,
                        lower_expr_with_exception(builder, expr, exception_name)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let mut args = vec![try_lambda];
            if let Some(fallback_expr) = fallback {
                let fallback_lambda = builder
                    .call_node(
                        "Lambda",
                        vec![
                            lambda_args_with_param(builder, SNAIL_EXCEPTION_VAR)?,
                            lower_expr_with_exception(
                                builder,
                                fallback_expr,
                                Some(SNAIL_EXCEPTION_VAR),
                            )?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                args.push(fallback_lambda);
            }
            let func = name_expr(
                builder,
                SNAIL_TRY_HELPER,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), args).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Compound { expressions, span } => {
            let mut lowered = Vec::new();
            for expr in expressions {
                lowered.push(lower_expr_with_exception(builder, expr, exception_name)?);
            }
            let tuple_expr = builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let index_expr = builder
                .call_node(
                    "UnaryOp",
                    vec![
                        lower_unary_op(builder, UnaryOp::Minus)?,
                        number_expr(builder, "1", span)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "Subscript",
                    vec![
                        tuple_expr,
                        index_expr,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Regex { pattern, span } => {
            let func = name_expr(
                builder,
                SNAIL_REGEX_COMPILE,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let arg = lower_regex_pattern_expr(builder, pattern, span, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => lower_regex_match(builder, value, pattern, span, exception_name),
        Expr::Subprocess { kind, parts, span } => {
            // Standalone subprocess: $(cmd) lowers to SubprocessCapture(cmd)()
            let subprocess_obj =
                lower_subprocess_object(builder, kind, parts, span, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        subprocess_obj,
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::StructuredAccessor { query, span } => {
            // $[query] lowers to __snail_jmespath_query(query) which returns a callable
            let escaped_query = escape_for_python_string(query);
            let func = name_expr(
                builder,
                SNAIL_JMESPATH_QUERY,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let arg = string_expr(
                builder,
                &escaped_query,
                false,
                StringDelimiter::Double,
                span,
            )?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Call { func, args, span } => {
            let func = lower_expr_with_exception(builder, func, exception_name)?;
            let (args, keywords) = lower_call_arguments(builder, args, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), args).into_py(builder.py()),
                        PyList::new_bound(builder.py(), keywords).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            if attr.chars().all(|ch| ch.is_ascii_digit()) {
                let group_index = attr
                    .parse::<i32>()
                    .map_err(|_| LowerError::new(format!("Invalid match group index: .{attr}")))?;
                let group_attr = builder
                    .call_node(
                        "Attribute",
                        vec![
                            value,
                            "group".into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                let index_expr = number_expr(builder, &group_index.to_string(), span)?;
                return builder
                    .call_node(
                        "Call",
                        vec![
                            group_attr,
                            PyList::new_bound(builder.py(), vec![index_expr]).into_py(builder.py()),
                            PyList::empty_bound(builder.py()).into_py(builder.py()),
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower);
            }
            builder
                .call_node(
                    "Attribute",
                    vec![
                        value,
                        attr.to_string().into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Index { value, index, span } => {
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            let index = lower_expr_with_exception(builder, index, exception_name)?;
            builder
                .call_node(
                    "Subscript",
                    vec![value, index, builder.load_ctx().map_err(py_err_to_lower)?],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Paren { expr, .. } => lower_expr_with_exception(builder, expr, exception_name),
        Expr::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "List",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Dict { entries, span } => {
            let mut keys = Vec::with_capacity(entries.len());
            let mut values = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                keys.push(lower_expr_with_exception(builder, key, exception_name)?);
                values.push(lower_expr_with_exception(builder, value, exception_name)?);
            }
            builder
                .call_node(
                    "Dict",
                    vec![
                        PyList::new_bound(builder.py(), keys).into_py(builder.py()),
                        PyList::new_bound(builder.py(), values).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Set { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "Set",
                    vec![PyList::new_bound(builder.py(), lowered).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => {
            let element = lower_expr_with_exception(builder, element, exception_name)?;
            let target = name_expr(
                builder,
                target,
                span,
                builder.store_ctx().map_err(py_err_to_lower)?,
            )?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node_no_loc(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "ListComp",
                    vec![
                        element,
                        PyList::new_bound(builder.py(), vec![comprehension]).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => {
            let key = lower_expr_with_exception(builder, key, exception_name)?;
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            let target = name_expr(
                builder,
                target,
                span,
                builder.store_ctx().map_err(py_err_to_lower)?,
            )?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node_no_loc(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "DictComp",
                    vec![
                        key,
                        value,
                        PyList::new_bound(builder.py(), vec![comprehension]).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Slice { start, end, span } => {
            let start = start
                .as_deref()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .transpose()?;
            let end = end
                .as_deref()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .transpose()?;
            builder
                .call_node(
                    "Slice",
                    vec![
                        start.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                        end.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                        builder.py().None().into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn lower_subprocess_object(
    builder: &AstBuilder<'_>,
    kind: &SubprocessKind,
    parts: &[SubprocessPart],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let values = lower_subprocess_parts(builder, parts, exception_name)?;
    let command = builder
        .call_node(
            "JoinedStr",
            vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
            span,
        )
        .map_err(py_err_to_lower)?;
    let class_name = match kind {
        SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
        SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
    };
    let func = name_expr(
        builder,
        class_name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), vec![command]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_subprocess_parts(
    builder: &AstBuilder<'_>,
    parts: &[SubprocessPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyObject>, LowerError> {
    let mut lowered_parts = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            SubprocessPart::Text(text) => {
                let const_node = builder
                    .call_node(
                        "Constant",
                        vec![text.clone().into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered_parts.push(const_node);
            }
            SubprocessPart::Expr(expr) => {
                let value = lower_expr_with_exception(builder, expr, exception_name)?;
                let conversion = (-1i32).into_py(builder.py());
                let format_spec = builder.py().None();
                let formatted = builder
                    .call_node(
                        "FormattedValue",
                        vec![value, conversion, format_spec.into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered_parts.push(formatted);
            }
        }
    }
    Ok(lowered_parts)
}

fn lower_call_arguments(
    builder: &AstBuilder<'_>,
    args: &[Argument],
    exception_name: Option<&str>,
) -> Result<(Vec<PyObject>, Vec<PyObject>), LowerError> {
    let mut positional = Vec::new();
    let mut keywords = Vec::new();
    for arg in args {
        match arg {
            Argument::Positional { value, .. } => {
                positional.push(lower_expr_with_exception(builder, value, exception_name)?);
            }
            Argument::Keyword { name, value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node_no_loc(
                        "keyword",
                        vec![name.to_string().into_py(builder.py()), value],
                    )
                    .map_err(py_err_to_lower)?;
                keywords.push(keyword);
            }
            Argument::Star { value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let starred = builder
                    .call_node(
                        "Starred",
                        vec![value, builder.load_ctx().map_err(py_err_to_lower)?],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                positional.push(starred);
            }
            Argument::KwStar { value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node_no_loc(
                        "keyword",
                        vec![builder.py().None().into_py(builder.py()), value],
                    )
                    .map_err(py_err_to_lower)?;
                keywords.push(keyword);
            }
        }
    }
    Ok((positional, keywords))
}

#[derive(Default)]
struct PlaceholderInfo {
    count: usize,
    first_span: Option<SourceSpan>,
}

fn placeholder_info_in_arguments(args: &[Argument]) -> PlaceholderInfo {
    let mut info = PlaceholderInfo::default();
    for arg in args {
        match arg {
            Argument::Positional { value, .. }
            | Argument::Keyword { value, .. }
            | Argument::Star { value, .. }
            | Argument::KwStar { value, .. } => count_placeholders(value, &mut info),
        }
    }
    info
}

fn count_placeholders(expr: &Expr, info: &mut PlaceholderInfo) {
    match expr {
        Expr::Placeholder { span } => {
            if info.first_span.is_none() {
                info.first_span = Some(span.clone());
            }
            info.count += 1;
        }
        Expr::Name { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => {}
        Expr::FString { parts, .. } => {
            for part in parts {
                if let FStringPart::Expr(expr) = part {
                    count_placeholders(expr, info);
                }
            }
        }
        Expr::Unary { expr, .. } => count_placeholders(expr, info),
        Expr::Binary { left, right, .. } => {
            count_placeholders(left, info);
            count_placeholders(right, info);
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            count_placeholders(left, info);
            for expr in comparators {
                count_placeholders(expr, info);
            }
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            count_placeholders(test, info);
            count_placeholders(body, info);
            count_placeholders(orelse, info);
        }
        Expr::TryExpr { expr, fallback, .. } => {
            count_placeholders(expr, info);
            if let Some(fallback) = fallback {
                count_placeholders(fallback, info);
            }
        }
        Expr::Compound { expressions, .. } => {
            for expr in expressions {
                count_placeholders(expr, info);
            }
        }
        Expr::Regex { pattern, .. } => count_placeholders_in_regex(pattern, info),
        Expr::RegexMatch { value, pattern, .. } => {
            count_placeholders(value, info);
            count_placeholders_in_regex(pattern, info);
        }
        Expr::Subprocess { parts, .. } => {
            for part in parts {
                if let SubprocessPart::Expr(expr) = part {
                    count_placeholders(expr, info);
                }
            }
        }
        Expr::Call { func, args, .. } => {
            count_placeholders(func, info);
            for arg in args {
                match arg {
                    Argument::Positional { value, .. }
                    | Argument::Keyword { value, .. }
                    | Argument::Star { value, .. }
                    | Argument::KwStar { value, .. } => count_placeholders(value, info),
                }
            }
        }
        Expr::Attribute { value, .. } => count_placeholders(value, info),
        Expr::Index { value, index, .. } => {
            count_placeholders(value, info);
            count_placeholders(index, info);
        }
        Expr::Paren { expr, .. } => count_placeholders(expr, info),
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                count_placeholders(expr, info);
            }
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                count_placeholders(key, info);
                count_placeholders(value, info);
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                count_placeholders(start, info);
            }
            if let Some(end) = end {
                count_placeholders(end, info);
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            count_placeholders(element, info);
            count_placeholders(iter, info);
            for expr in ifs {
                count_placeholders(expr, info);
            }
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            count_placeholders(key, info);
            count_placeholders(value, info);
            count_placeholders(iter, info);
            for expr in ifs {
                count_placeholders(expr, info);
            }
        }
    }
}

fn count_placeholders_in_regex(pattern: &RegexPattern, info: &mut PlaceholderInfo) {
    if let RegexPattern::Interpolated(parts) = pattern {
        for part in parts {
            if let FStringPart::Expr(expr) = part {
                count_placeholders(expr, info);
            }
        }
    }
}

fn substitute_placeholder_in_arguments(args: &[Argument], replacement: &Expr) -> Vec<Argument> {
    args.iter()
        .map(|arg| match arg {
            Argument::Positional { value, span } => Argument::Positional {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::Keyword { name, value, span } => Argument::Keyword {
                name: name.clone(),
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::Star { value, span } => Argument::Star {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::KwStar { value, span } => Argument::KwStar {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
        })
        .collect()
}

fn substitute_placeholder(expr: &Expr, replacement: &Expr) -> Expr {
    match expr {
        Expr::Placeholder { .. } => replacement.clone(),
        Expr::Name { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => expr.clone(),
        Expr::FString { parts, span } => Expr::FString {
            parts: parts
                .iter()
                .map(|part| match part {
                    FStringPart::Text(text) => FStringPart::Text(text.clone()),
                    FStringPart::Expr(expr) => {
                        FStringPart::Expr(Box::new(substitute_placeholder(expr, replacement)))
                    }
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_placeholder(expr, replacement)),
            span: span.clone(),
        },
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(substitute_placeholder(left, replacement)),
            op: *op,
            right: Box::new(substitute_placeholder(right, replacement)),
            span: span.clone(),
        },
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Expr::Compare {
            left: Box::new(substitute_placeholder(left, replacement)),
            ops: ops.clone(),
            comparators: comparators
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => Expr::IfExpr {
            test: Box::new(substitute_placeholder(test, replacement)),
            body: Box::new(substitute_placeholder(body, replacement)),
            orelse: Box::new(substitute_placeholder(orelse, replacement)),
            span: span.clone(),
        },
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => Expr::TryExpr {
            expr: Box::new(substitute_placeholder(expr, replacement)),
            fallback: fallback
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            span: span.clone(),
        },
        Expr::Compound { expressions, span } => Expr::Compound {
            expressions: expressions
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Regex { pattern, span } => Expr::Regex {
            pattern: substitute_placeholder_in_regex(pattern, replacement),
            span: span.clone(),
        },
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Expr::RegexMatch {
            value: Box::new(substitute_placeholder(value, replacement)),
            pattern: substitute_placeholder_in_regex(pattern, replacement),
            span: span.clone(),
        },
        Expr::Subprocess { kind, parts, span } => Expr::Subprocess {
            kind: *kind,
            parts: parts
                .iter()
                .map(|part| match part {
                    SubprocessPart::Text(text) => SubprocessPart::Text(text.clone()),
                    SubprocessPart::Expr(expr) => {
                        SubprocessPart::Expr(Box::new(substitute_placeholder(expr, replacement)))
                    }
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Call { func, args, span } => Expr::Call {
            func: Box::new(substitute_placeholder(func, replacement)),
            args: substitute_placeholder_in_arguments(args, replacement),
            span: span.clone(),
        },
        Expr::Attribute { value, attr, span } => Expr::Attribute {
            value: Box::new(substitute_placeholder(value, replacement)),
            attr: attr.clone(),
            span: span.clone(),
        },
        Expr::Index { value, index, span } => Expr::Index {
            value: Box::new(substitute_placeholder(value, replacement)),
            index: Box::new(substitute_placeholder(index, replacement)),
            span: span.clone(),
        },
        Expr::Paren { expr, span } => Expr::Paren {
            expr: Box::new(substitute_placeholder(expr, replacement)),
            span: span.clone(),
        },
        Expr::List { elements, span } => Expr::List {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Tuple { elements, span } => Expr::Tuple {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Dict { entries, span } => Expr::Dict {
            entries: entries
                .iter()
                .map(|(key, value)| {
                    (
                        substitute_placeholder(key, replacement),
                        substitute_placeholder(value, replacement),
                    )
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Set { elements, span } => Expr::Set {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Slice { start, end, span } => Expr::Slice {
            start: start
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            end: end
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            span: span.clone(),
        },
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => Expr::ListComp {
            element: Box::new(substitute_placeholder(element, replacement)),
            target: target.clone(),
            iter: Box::new(substitute_placeholder(iter, replacement)),
            ifs: ifs
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => Expr::DictComp {
            key: Box::new(substitute_placeholder(key, replacement)),
            value: Box::new(substitute_placeholder(value, replacement)),
            target: target.clone(),
            iter: Box::new(substitute_placeholder(iter, replacement)),
            ifs: ifs
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
    }
}

fn substitute_placeholder_in_regex(pattern: &RegexPattern, replacement: &Expr) -> RegexPattern {
    match pattern {
        RegexPattern::Literal(text) => RegexPattern::Literal(text.clone()),
        RegexPattern::Interpolated(parts) => RegexPattern::Interpolated(
            parts
                .iter()
                .map(|part| match part {
                    FStringPart::Text(text) => FStringPart::Text(text.clone()),
                    FStringPart::Expr(expr) => {
                        FStringPart::Expr(Box::new(substitute_placeholder(expr, replacement)))
                    }
                })
                .collect(),
        ),
    }
}

fn empty_lambda_args(builder: &AstBuilder<'_>) -> Result<PyObject, LowerError> {
    builder
        .call_node_no_loc(
            "arguments",
            vec![
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)
}

fn lambda_args_with_param(builder: &AstBuilder<'_>, name: &str) -> Result<PyObject, LowerError> {
    let arg = builder
        .call_node_no_loc(
            "arg",
            vec![
                name.to_string().into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)?;
    builder
        .call_node_no_loc(
            "arguments",
            vec![
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)
}

fn dummy_span() -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line: 0,
            column: 0,
        },
        end: SourcePos {
            offset: 0,
            line: 0,
            column: 0,
        },
    }
}
