use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::*;
use super::expr::{lower_expr, lower_regex_match};
use super::helpers::{assign_name, name_expr, number_expr, string_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::lower_block_with_auto_print;

pub(crate) fn lower_awk_file_loop_with_auto_print(
    builder: &AstBuilder<'_>,
    program: &AwkProgram,
    span: &SourceSpan,
    auto_print: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut file_loop = Vec::new();
    file_loop.push(assign_name(
        builder,
        "__snail_fnr",
        number_expr(builder, "0", span)?,
        span,
    )?);

    let stdin_body = vec![
        assign_name(
            builder,
            "__snail_file",
            builder
                .call_node(
                    "Attribute",
                    vec![
                        name_expr(
                            builder,
                            "sys",
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        "stdin".to_string().into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?,
            span,
        )?,
        lower_awk_line_loop_with_auto_print(
            builder,
            program,
            span,
            name_expr(
                builder,
                "__snail_file",
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?,
            auto_print,
        )?,
    ];

    let open_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    "open",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![name_expr(
                        builder,
                        "__snail_path",
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?],
                )
                .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let with_item = builder
        .call_node_no_loc(
            "withitem",
            vec![
                open_call,
                name_expr(
                    builder,
                    "__snail_file",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
            ],
        )
        .map_err(py_err_to_lower)?;
    let with_stmt = builder
        .call_node(
            "With",
            vec![
                PyList::new_bound(builder.py(), vec![with_item]).into_py(builder.py()),
                PyList::new_bound(
                    builder.py(),
                    vec![lower_awk_line_loop_with_auto_print(
                        builder,
                        program,
                        span,
                        name_expr(
                            builder,
                            "__snail_file",
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        auto_print,
                    )?],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let test = builder
        .call_node(
            "Compare",
            vec![
                name_expr(
                    builder,
                    "__snail_path",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![builder.op("Eq").map_err(py_err_to_lower)?],
                )
                .into_py(builder.py()),
                PyList::new_bound(
                    builder.py(),
                    vec![string_expr(
                        builder,
                        "-",
                        false,
                        StringDelimiter::Double,
                        span,
                    )?],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let if_stmt = builder
        .call_node(
            "If",
            vec![
                test,
                PyList::new_bound(builder.py(), stdin_body).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![with_stmt]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    file_loop.push(if_stmt);
    Ok(file_loop)
}

pub(crate) fn lower_awk_line_loop_with_auto_print(
    builder: &AstBuilder<'_>,
    program: &AwkProgram,
    span: &SourceSpan,
    iter: PyObject,
    auto_print: bool,
) -> Result<PyObject, LowerError> {
    let mut loop_body = Vec::new();
    loop_body.push(assign_name(
        builder,
        "__snail_nr",
        builder
            .call_node(
                "BinOp",
                vec![
                    name_expr(
                        builder,
                        "__snail_nr",
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?,
                    builder.op("Add").map_err(py_err_to_lower)?,
                    number_expr(builder, "1", span)?,
                ],
                span,
            )
            .map_err(py_err_to_lower)?,
        span,
    )?);
    loop_body.push(assign_name(
        builder,
        "__snail_fnr",
        builder
            .call_node(
                "BinOp",
                vec![
                    name_expr(
                        builder,
                        "__snail_fnr",
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?,
                    builder.op("Add").map_err(py_err_to_lower)?,
                    number_expr(builder, "1", span)?,
                ],
                span,
            )
            .map_err(py_err_to_lower)?,
        span,
    )?);

    let rstrip_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "__snail_raw",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "rstrip".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                PyList::new_bound(
                    builder.py(),
                    vec![string_expr(
                        builder,
                        "\\n",
                        false,
                        StringDelimiter::Double,
                        span,
                    )?],
                )
                .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    loop_body.push(assign_name(
        builder,
        SNAIL_AWK_LINE_PYVAR,
        rstrip_call,
        span,
    )?);

    let split_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                SNAIL_AWK_LINE_PYVAR,
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "split".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    loop_body.push(assign_name(
        builder,
        SNAIL_AWK_FIELDS_PYVAR,
        split_call,
        span,
    )?);
    loop_body.push(assign_name(
        builder,
        SNAIL_AWK_NR_PYVAR,
        name_expr(
            builder,
            "__snail_nr",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?,
        span,
    )?);
    loop_body.push(assign_name(
        builder,
        SNAIL_AWK_FNR_PYVAR,
        name_expr(
            builder,
            "__snail_fnr",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?,
        span,
    )?);
    loop_body.push(assign_name(
        builder,
        SNAIL_AWK_PATH_PYVAR,
        name_expr(
            builder,
            "__snail_path",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?,
        span,
    )?);

    loop_body.extend(lower_awk_rules_with_auto_print(
        builder,
        &program.rules,
        auto_print,
    )?);

    builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    "__snail_raw",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                iter,
                PyList::new_bound(builder.py(), loop_body).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

pub(crate) fn lower_awk_rules_with_auto_print(
    builder: &AstBuilder<'_>,
    rules: &[AwkRule],
    auto_print: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for rule in rules {
        let mut action = if rule.has_explicit_action() {
            lower_block_with_auto_print(
                builder,
                rule.action.as_ref().unwrap(),
                auto_print,
                &rule.span,
            )?
        } else {
            vec![awk_default_print(builder, &rule.span)?]
        };

        if let Some(pattern) = &rule.pattern {
            if let Some((value_expr, regex, span)) = regex_pattern_components(pattern) {
                let match_call = lower_regex_match(builder, &value_expr, &regex, &span, None)?;
                stmts.push(assign_name(
                    builder,
                    SNAIL_AWK_MATCH_PYVAR,
                    match_call,
                    &span,
                )?);
                stmts.push(
                    builder
                        .call_node(
                            "If",
                            vec![
                                name_expr(
                                    builder,
                                    SNAIL_AWK_MATCH_PYVAR,
                                    &span,
                                    builder.load_ctx().map_err(py_err_to_lower)?,
                                )?,
                                PyList::new_bound(builder.py(), action).into_py(builder.py()),
                                PyList::empty_bound(builder.py()).into_py(builder.py()),
                            ],
                            &rule.span,
                        )
                        .map_err(py_err_to_lower)?,
                );
            } else {
                stmts.push(
                    builder
                        .call_node(
                            "If",
                            vec![
                                lower_expr(builder, pattern)?,
                                PyList::new_bound(builder.py(), action).into_py(builder.py()),
                                PyList::empty_bound(builder.py()).into_py(builder.py()),
                            ],
                            &rule.span,
                        )
                        .map_err(py_err_to_lower)?,
                );
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
            Expr::FieldIndex {
                index: "0".to_string(),
                span: span.clone(),
            },
            pattern.clone(),
            span.clone(),
        )),
        _ => None,
    }
}

fn awk_default_print(builder: &AstBuilder<'_>, span: &SourceSpan) -> Result<PyObject, LowerError> {
    let print_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    "print",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![name_expr(
                        builder,
                        SNAIL_AWK_LINE_PYVAR,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?],
                )
                .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    builder
        .call_node("Expr", vec![print_call], span)
        .map_err(py_err_to_lower)
}
