use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::*;
use super::expr::{lower_expr, lower_regex_match};
use super::helpers::{assign_name, name_expr, number_expr, string_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::lower_block_auto;

pub(crate) fn lower_awk_file_loop(
    builder: &AstBuilder<'_>,
    program: &AwkProgram,
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut file_loop = Vec::new();
    file_loop.push(assign_name(
        builder,
        "__snail_fnr",
        number_expr(builder, "0", span)?,
        span,
    )?);

    let stdin_assign = assign_name(
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
    )?;

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

    let enter_context_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "__snail_stack",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "enter_context".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                PyList::new_bound(builder.py(), vec![open_call]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let file_assign = assign_name(builder, "__snail_file", enter_context_call, span)?;

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
                PyList::new_bound(builder.py(), vec![stdin_assign]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![file_assign]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let exit_stack_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "contextlib",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "ExitStack".to_string().into_py(builder.py()),
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

    let with_item = builder
        .call_node_no_loc(
            "withitem",
            vec![
                exit_stack_call,
                name_expr(
                    builder,
                    "__snail_stack",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
            ],
        )
        .map_err(py_err_to_lower)?;

    let line_loop = lower_awk_line_loop(
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
        capture_last,
    )?;

    let with_stmt = builder
        .call_node(
            "With",
            vec![
                PyList::new_bound(builder.py(), vec![with_item]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![if_stmt, line_loop]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    file_loop.push(with_stmt);
    Ok(file_loop)
}

pub(crate) fn lower_awk_line_loop(
    builder: &AstBuilder<'_>,
    program: &AwkProgram,
    span: &SourceSpan,
    iter: PyObject,
    auto_print: bool,
    capture_last: bool,
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
                name_expr(
                    builder,
                    SNAIL_AWK_SPLIT_HELPER,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![
                        name_expr(
                            builder,
                            SNAIL_AWK_LINE_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        name_expr(
                            builder,
                            SNAIL_AWK_FIELD_SEPARATORS_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        name_expr(
                            builder,
                            SNAIL_AWK_INCLUDE_WHITESPACE_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                    ],
                )
                .into_py(builder.py()),
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
    loop_body.push(assign_name(
        builder,
        SNAIL_MAP_SRC_PYVAR,
        name_expr(
            builder,
            "__snail_path",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?,
        span,
    )?);

    loop_body.extend(lower_awk_rules(
        builder,
        &program.rules,
        auto_print,
        capture_last,
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

/// Lower a `lines` statement: sets up line-processing variables and iterates lines.
///
/// For `lines { body }` (no source), generates the full argv/stdin file loop.
/// For `lines(expr) { body }`, calls `__snail_lines_iter(expr)` and iterates.
pub(crate) fn lower_lines_stmt(
    builder: &AstBuilder<'_>,
    source: &Option<Expr>,
    body: &[Stmt],
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    if source.is_none() {
        // No source: generate the full argv/stdin file loop (same as awk mode)
        lower_lines_no_source(builder, body, span, auto_print, capture_last)
    } else {
        // With source: iterate lines from the expression
        lower_lines_with_source(
            builder,
            source.as_ref().unwrap(),
            body,
            span,
            auto_print,
            capture_last,
        )
    }
}

fn lower_lines_with_source(
    builder: &AstBuilder<'_>,
    source_expr: &Expr,
    body: &[Stmt],
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();

    // Initialize counters and path
    stmts.push(assign_name(
        builder,
        "__snail_nr",
        number_expr(builder, "0", span)?,
        span,
    )?);
    stmts.push(assign_name(
        builder,
        "__snail_fnr",
        number_expr(builder, "0", span)?,
        span,
    )?);
    stmts.push(assign_name(
        builder,
        "__snail_path",
        string_expr(builder, "<lines>", false, StringDelimiter::Double, span)?,
        span,
    )?);

    // Build the line loop body
    let loop_body = build_line_loop_body(builder, body, span, auto_print, capture_last)?;

    // __snail_lines_iter(source_expr)
    let source_lowered = lower_expr(builder, source_expr)?;
    let iter_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    SNAIL_LINES_ITER_HELPER,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(builder.py(), vec![source_lowered]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    // for __snail_raw in __snail_lines_iter(source):
    let for_loop = builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    "__snail_raw",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                iter_call,
                PyList::new_bound(builder.py(), loop_body).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(for_loop);
    Ok(stmts)
}

fn lower_lines_no_source(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    // Same structure as lower_awk in program.rs: import sys, contextlib;
    // set up __snail_nr = 0; for __snail_path in (sys.argv[1:] or ["-"]): file loop
    let mut stmts = Vec::new();

    // import sys, contextlib
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
                        builder
                            .call_node_no_loc(
                                "alias",
                                vec![
                                    "contextlib".to_string().into_py(builder.py()),
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

    // __snail_nr = 0
    stmts.push(assign_name(
        builder,
        "__snail_nr",
        number_expr(builder, "0", span)?,
        span,
    )?);

    // Build files_expr: sys.argv[1:] or ["-"]
    let files_expr = builder
        .call_node(
            "BoolOp",
            vec![
                builder.op("Or").map_err(py_err_to_lower)?,
                PyList::new_bound(
                    builder.py(),
                    vec![
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
                                                number_expr(builder, "1", span)?,
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
                            .map_err(py_err_to_lower)?,
                        builder
                            .call_node(
                                "List",
                                vec![
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
                                    builder.load_ctx().map_err(py_err_to_lower)?,
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)?,
                    ],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    // Build the file loop body (reuse lower_awk_file_loop_body)
    let file_loop_body = lower_lines_file_loop_body(builder, body, span, auto_print, capture_last)?;

    // for __snail_path in (sys.argv[1:] or ["-"]):
    let for_loop = builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    "__snail_path",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                files_expr,
                PyList::new_bound(builder.py(), file_loop_body).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(for_loop);
    Ok(stmts)
}

/// Generates the file-level loop body for `lines { }` with no source.
/// Opens each file path (or stdin for "-"), then iterates lines.
fn lower_lines_file_loop_body(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut file_loop = Vec::new();
    file_loop.push(assign_name(
        builder,
        "__snail_fnr",
        number_expr(builder, "0", span)?,
        span,
    )?);

    // if __snail_path == "-": __snail_file = sys.stdin
    // else: __snail_file = __snail_stack.enter_context(open(__snail_path))
    let stdin_assign = assign_name(
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
    )?;

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

    let enter_context_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "__snail_stack",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "enter_context".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                PyList::new_bound(builder.py(), vec![open_call]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let file_assign = assign_name(builder, "__snail_file", enter_context_call, span)?;

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
                PyList::new_bound(builder.py(), vec![stdin_assign]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![file_assign]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    // Build ExitStack context manager
    let exit_stack_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "contextlib",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "ExitStack".to_string().into_py(builder.py()),
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

    let with_item = builder
        .call_node_no_loc(
            "withitem",
            vec![
                exit_stack_call,
                name_expr(
                    builder,
                    "__snail_stack",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
            ],
        )
        .map_err(py_err_to_lower)?;

    // Build the line loop
    let loop_body = build_line_loop_body(builder, body, span, auto_print, capture_last)?;

    let line_loop = builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    "__snail_raw",
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                name_expr(
                    builder,
                    "__snail_file",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(builder.py(), loop_body).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let with_stmt = builder
        .call_node(
            "With",
            vec![
                PyList::new_bound(builder.py(), vec![with_item]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![if_stmt, line_loop]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    file_loop.push(with_stmt);
    Ok(file_loop)
}

/// Build the inner body of a line processing loop — sets up $0, $f, $n, $fn, $m, $src
/// and then runs the user body (which may contain pattern/action entries).
fn build_line_loop_body(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut loop_body = Vec::new();

    // __snail_nr += 1
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

    // __snail_fnr += 1
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

    // __snail_line = __snail_raw.rstrip('\n')
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

    // __snail_fields = __snail_awk_split(__snail_line, ...)
    let split_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    SNAIL_AWK_SPLIT_HELPER,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![
                        name_expr(
                            builder,
                            SNAIL_AWK_LINE_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        name_expr(
                            builder,
                            SNAIL_AWK_FIELD_SEPARATORS_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                        name_expr(
                            builder,
                            SNAIL_AWK_INCLUDE_WHITESPACE_PYVAR,
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                    ],
                )
                .into_py(builder.py()),
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

    // Set user-visible variables
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
    loop_body.push(assign_name(
        builder,
        SNAIL_MAP_SRC_PYVAR,
        name_expr(
            builder,
            "__snail_path",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?,
        span,
    )?);

    // Lower user body (mix of pattern/action and regular statements)
    loop_body.extend(lower_lines_body(builder, body, auto_print, capture_last)?);

    Ok(loop_body)
}

/// Lower a mixed body of statements and pattern/action entries for `lines { }` blocks.
pub(crate) fn lower_lines_body(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::PatternAction {
                pattern,
                action,
                span,
            } => {
                let rule = AwkRule {
                    pattern: pattern.clone(),
                    action: action.clone(),
                    span: span.clone(),
                };
                stmts.extend(lower_awk_rules(builder, &[rule], auto_print, capture_last)?);
            }
            other => {
                stmts.push(super::stmt::lower_stmt(builder, other)?);
            }
        }
    }
    Ok(stmts)
}

pub(crate) fn lower_awk_rules(
    builder: &AstBuilder<'_>,
    rules: &[AwkRule],
    auto_print: bool,
    capture_last: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for rule in rules {
        let mut action = if rule.has_explicit_action() {
            lower_block_auto(
                builder,
                rule.action.as_ref().unwrap(),
                auto_print,
                capture_last,
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
