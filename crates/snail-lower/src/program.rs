use snail_ast::*;
use snail_error::LowerError;
use snail_python_ast::*;

use crate::awk::{lower_awk_file_loop_with_auto_print, wrap_block_with_auto_print};
use crate::helpers::*;
use crate::stmt::{lower_block, lower_stmt};

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
