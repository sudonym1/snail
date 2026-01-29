use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::*;
use super::desugar::LambdaHoister;
use super::helpers::{assign_name, name_expr, string_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::lower_block_with_auto_print;
use super::validate::{validate_yield_usage_blocks, validate_yield_usage_program};

pub fn lower_map_program(py: Python<'_>, program: &Program) -> Result<PyObject, LowerError> {
    lower_map_program_with_auto_print(py, program, false)
}

pub fn lower_map_program_with_auto_print(
    py: Python<'_>,
    program: &Program,
    auto_print_last: bool,
) -> Result<PyObject, LowerError> {
    lower_map_program_with_begin_end(py, program, &[], &[], auto_print_last)
}

pub fn lower_map_program_with_begin_end(
    py: Python<'_>,
    program: &Program,
    begin_blocks: &[Vec<Stmt>],
    end_blocks: &[Vec<Stmt>],
    auto_print_last: bool,
) -> Result<PyObject, LowerError> {
    let mut hoister = LambdaHoister::new();
    let begin_blocks: Vec<Vec<Stmt>> = begin_blocks
        .iter()
        .map(|block| hoister.desugar_block(block))
        .collect();
    let program = hoister.desugar_program(program);
    let end_blocks: Vec<Vec<Stmt>> = end_blocks
        .iter()
        .map(|block| hoister.desugar_block(block))
        .collect();
    validate_yield_usage_program(&program)?;
    validate_yield_usage_blocks(&begin_blocks)?;
    validate_yield_usage_blocks(&end_blocks)?;
    let builder = AstBuilder::new(py).map_err(py_err_to_lower)?;
    let span = program.span.clone();
    let mut body = Vec::new();

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
            &span,
        )
        .map_err(py_err_to_lower)?;
    body.push(sys_import);

    // __snail_paths = sys.argv[1:] or [line.rstrip('\n') for line in sys.stdin if line.rstrip('\n')]
    let paths_expr = lower_paths_source(&builder, &span)?;
    body.push(assign_name(&builder, "__snail_paths", paths_expr, &span)?);

    // Initialize map variables for begin blocks
    let none_expr = builder
        .call_node(
            "Constant",
            vec![builder.py().None().into_py(builder.py())],
            &span,
        )
        .map_err(py_err_to_lower)?;
    body.push(assign_name(
        &builder,
        SNAIL_MAP_SRC_PYVAR,
        none_expr.clone_ref(builder.py()),
        &span,
    )?);
    body.push(assign_name(
        &builder,
        SNAIL_MAP_FD_PYVAR,
        none_expr.clone_ref(builder.py()),
        &span,
    )?);
    body.push(assign_name(
        &builder,
        SNAIL_MAP_TEXT_PYVAR,
        none_expr,
        &span,
    )?);

    // Begin blocks
    for block in begin_blocks {
        let lowered =
            lower_block_with_auto_print(&builder, block.as_slice(), auto_print_last, &span)?;
        body.extend(lowered);
    }

    // Generate file loop
    let file_loop = lower_map_file_loop(&builder, &program, &span, auto_print_last)?;
    body.push(file_loop);

    // End blocks
    for block in end_blocks {
        let lowered =
            lower_block_with_auto_print(&builder, block.as_slice(), auto_print_last, &span)?;
        body.extend(lowered);
    }

    builder.module(body, &span).map_err(py_err_to_lower)
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

fn lower_map_file_loop(
    builder: &AstBuilder<'_>,
    program: &Program,
    span: &SourceSpan,
    auto_print_last: bool,
) -> Result<PyObject, LowerError> {
    // for __snail_src in __snail_paths:
    //     with __SnailLazyFile(__snail_src, 'r') as __snail_fd:
    //         __snail_text = __SnailLazyText(__snail_fd)
    //         # user code

    // Build the with statement body
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
    let user_code =
        lower_block_with_auto_print(builder, &program.stmts, auto_print_last, &program.span)?;
    with_body.extend(user_code);

    // __SnailLazyFile(__snail_src, 'r')
    let lazy_file_call = builder
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
        .map_err(py_err_to_lower)?;

    // with __SnailLazyFile(...) as __snail_fd:
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

    // for __snail_src in __snail_paths:
    builder
        .call_node(
            "For",
            vec![
                name_expr(
                    builder,
                    SNAIL_MAP_SRC_PYVAR,
                    span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                name_expr(
                    builder,
                    "__snail_paths",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(builder.py(), vec![with_stmt]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}
