use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::awk::lower_awk_file_loop_with_auto_print;
use super::desugar::LambdaHoister;
use super::helpers::{assign_name, name_expr, number_expr, string_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::lower_block_with_auto_print;
use super::validate::{validate_yield_usage_awk, validate_yield_usage_program};

pub fn lower_program(py: Python<'_>, program: &Program) -> Result<PyObject, LowerError> {
    lower_program_with_auto_print(py, program, false)
}

pub fn lower_program_with_auto_print(
    py: Python<'_>,
    program: &Program,
    auto_print_last: bool,
) -> Result<PyObject, LowerError> {
    let mut hoister = LambdaHoister::new();
    let program = hoister.desugar_program(program);
    validate_yield_usage_program(&program)?;
    let builder = AstBuilder::new(py).map_err(py_err_to_lower)?;
    let body =
        lower_block_with_auto_print(&builder, &program.stmts, auto_print_last, &program.span)?;
    builder.module(body, &program.span).map_err(py_err_to_lower)
}

pub fn lower_awk_program(py: Python<'_>, program: &AwkProgram) -> Result<PyObject, LowerError> {
    lower_awk_program_with_auto_print(py, program, false)
}

pub fn lower_awk_program_with_auto_print(
    py: Python<'_>,
    program: &AwkProgram,
    auto_print: bool,
) -> Result<PyObject, LowerError> {
    let mut hoister = LambdaHoister::new();
    let program = hoister.desugar_awk_program(program);
    validate_yield_usage_awk(&program)?;
    let builder = AstBuilder::new(py).map_err(py_err_to_lower)?;
    let span = program.span.clone();
    let mut body = Vec::new();

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

    let mut main_body = Vec::new();
    for block in &program.begin_blocks {
        let lowered = lower_block_with_auto_print(&builder, block, auto_print, &span)?;
        main_body.extend(lowered);
    }

    main_body.push(assign_name(
        &builder,
        "__snail_nr",
        number_expr(&builder, "0", &span)?,
        &span,
    )?);

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
                                                    &builder,
                                                    "sys",
                                                    &span,
                                                    builder.load_ctx().map_err(py_err_to_lower)?,
                                                )?,
                                                "argv".to_string().into_py(builder.py()),
                                                builder.load_ctx().map_err(py_err_to_lower)?,
                                            ],
                                            &span,
                                        )
                                        .map_err(py_err_to_lower)?,
                                    builder
                                        .call_node(
                                            "Slice",
                                            vec![
                                                number_expr(&builder, "1", &span)?,
                                                builder.py().None().into_py(builder.py()),
                                                builder.py().None().into_py(builder.py()),
                                            ],
                                            &span,
                                        )
                                        .map_err(py_err_to_lower)?,
                                    builder.load_ctx().map_err(py_err_to_lower)?,
                                ],
                                &span,
                            )
                            .map_err(py_err_to_lower)?,
                        builder
                            .call_node(
                                "List",
                                vec![
                                    PyList::new_bound(
                                        builder.py(),
                                        vec![string_expr(
                                            &builder,
                                            "-",
                                            false,
                                            StringDelimiter::Double,
                                            &span,
                                        )?],
                                    )
                                    .into_py(builder.py()),
                                    builder.load_ctx().map_err(py_err_to_lower)?,
                                ],
                                &span,
                            )
                            .map_err(py_err_to_lower)?,
                    ],
                )
                .into_py(builder.py()),
            ],
            &span,
        )
        .map_err(py_err_to_lower)?;

    let file_loop = lower_awk_file_loop_with_auto_print(&builder, &program, &span, auto_print)?;
    let for_loop = builder
        .call_node(
            "For",
            vec![
                name_expr(
                    &builder,
                    "__snail_path",
                    &span,
                    builder.store_ctx().map_err(py_err_to_lower)?,
                )?,
                files_expr,
                PyList::new_bound(builder.py(), file_loop).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            &span,
        )
        .map_err(py_err_to_lower)?;
    main_body.push(for_loop);

    for block in &program.end_blocks {
        let lowered = lower_block_with_auto_print(&builder, block, auto_print, &span)?;
        main_body.extend(lowered);
    }

    body.extend(main_body);

    builder.module(body, &span).map_err(py_err_to_lower)
}
