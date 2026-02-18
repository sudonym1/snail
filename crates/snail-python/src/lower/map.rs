use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::*;
use super::expr::lower_expr;
use super::helpers::{assign_name, name_expr, string_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};

use super::stmt::lower_block;

/// Lower a `files` statement: iterates files from sources or argv.
pub(crate) fn lower_files_stmt(
    builder: &AstBuilder<'_>,
    sources: &[Expr],
    body: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    if sources.is_empty() {
        lower_files_no_source(builder, body, span)
    } else {
        lower_files_with_sources(builder, sources, body, span)
    }
}

/// Shared loop structure for all `files` variants.
///
/// Generates:
/// ```python
/// for __snail_src in <iter_expr>:
///     with __SnailLazyFile(__snail_src, 'r') as __snail_fd:
///         __snail_text = __SnailLazyText(__snail_fd)
///         <body>
/// ```
fn lower_files_loop(
    builder: &AstBuilder<'_>,
    iter_expr: PyObject,
    body: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();

    let with_body = build_files_with_body(builder, body, span)?;
    let lazy_file_call = build_lazy_file_call(builder, span)?;

    let with_item = builder
        .call_node_no_loc(
            "withitem",
            vec![
                lazy_file_call,
                name_expr(
                    builder,
                    SNAIL_MAP_FD_PYVAR,
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
                PyList::new_bound(builder.py(), with_body).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let for_loop = builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    SNAIL_MAP_SRC_PYVAR,
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                iter_expr,
                PyList::new_bound(builder.py(), vec![with_stmt]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(for_loop);
    Ok(stmts)
}

fn lower_files_with_sources(
    builder: &AstBuilder<'_>,
    sources: &[Expr],
    body: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let iter_expr = if sources.len() == 1 {
        // Single source: wrap in __snail_normalize_sources(expr) so a list of paths
        // is iterated as individual sources, while a single string becomes [path]
        let source_lowered = lower_expr(builder, &sources[0])?;
        builder
            .call_node(
                "Call",
                vec![
                    name_expr(
                        builder,
                        SNAIL_NORMALIZE_SOURCES_HELPER,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?,
                    PyList::new_bound(builder.py(), vec![source_lowered]).into_py(builder.py()),
                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                ],
                span,
            )
            .map_err(py_err_to_lower)?
    } else {
        // Multiple sources: build a list literal [e1, e2, ...]
        let mut lowered_sources = Vec::new();
        for src in sources {
            lowered_sources.push(lower_expr(builder, src)?);
        }
        builder
            .call_node(
                "List",
                vec![
                    PyList::new_bound(builder.py(), lowered_sources).into_py(builder.py()),
                    builder.load_ctx().map_err(py_err_to_lower)?,
                ],
                span,
            )
            .map_err(py_err_to_lower)?
    };

    lower_files_loop(builder, iter_expr, body, span)
}

fn lower_files_no_source(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    // import sys
    // for __snail_src in sys.argv[1:]:
    //     with __SnailLazyFile(__snail_src, 'r') as __snail_fd:
    //         __snail_text = __SnailLazyText(__snail_fd)
    //         <body>
    let mut stmts = Vec::new();

    // import sys
    let sys_import = builder
        .call_node(
            "Import",
            vec![
                PyList::new_bound(
                    builder.py(),
                    vec![
                        builder
                            .call_node_no_loc(
                                "alias",
                                vec![
                                    "sys".to_string().into_py(builder.py()),
                                    builder.py().None().into_py(builder.py()),
                                ],
                            )
                            .map_err(py_err_to_lower)?,
                    ],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(sys_import);

    // sys.argv[1:]
    let iter_expr = lower_paths_source(builder, span)?;

    stmts.extend(lower_files_loop(builder, iter_expr, body, span)?);
    Ok(stmts)
}

fn build_files_with_body(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut with_body = Vec::new();

    // __snail_text = __SnailLazyText(__snail_fd)
    let lazy_text_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    SNAIL_LAZY_TEXT_CLASS,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![name_expr(
                        builder,
                        SNAIL_MAP_FD_PYVAR,
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
    with_body.push(assign_name(
        builder,
        SNAIL_MAP_TEXT_PYVAR,
        lazy_text_call,
        span,
    )?);

    // Lower user code
    let user_code = lower_block(builder, body, span)?;
    with_body.extend(user_code);

    Ok(with_body)
}

fn lower_paths_source(builder: &AstBuilder<'_>, span: &SourceSpan) -> Result<PyObject, LowerError> {
    // sys.argv[1:]
    builder
        .call_node(
            "Subscript",
            vec![
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
                            "argv".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                builder
                    .call_node(
                        "Slice",
                        vec![
                            builder
                                .call_node("Constant", vec![1i64.into_py(builder.py())], span)
                                .map_err(py_err_to_lower)?,
                            builder.py().None().into_py(builder.py()),
                            builder.py().None().into_py(builder.py()),
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                builder.load_ctx().map_err(py_err_to_lower)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn build_lazy_file_call(
    builder: &AstBuilder<'_>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    SNAIL_LAZY_FILE_CLASS,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![
                        name_expr(
                            builder,
                            SNAIL_MAP_SRC_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        string_expr(builder, "r", false, StringDelimiter::Double, span)?,
                    ],
                )
                .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}
