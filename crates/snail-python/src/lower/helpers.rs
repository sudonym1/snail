use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::{AssignTarget, BinaryOp, Expr, SourceSpan, StringDelimiter};
use snail_error::LowerError;

use super::constants::{SNAIL_LET_OK, SNAIL_LET_VALUE};
use super::expr::{lower_assign_target, lower_expr};
use super::py_ast::{AstBuilder, py_err_to_lower};

fn eval_literal(
    builder: &AstBuilder<'_>,
    source: &str,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let builtins = builder
        .py()
        .import_bound("builtins")
        .map_err(py_err_to_lower)?;
    let code = builtins
        .getattr("compile")
        .and_then(|compile| compile.call1((source, "", "eval")))
        .map_err(py_err_to_lower)?;
    let value = builtins
        .getattr("eval")
        .and_then(|eval| eval.call1((code,)))
        .map_err(py_err_to_lower)?;
    builder
        .call_node("Constant", vec![value.into_py(builder.py())], span)
        .map_err(py_err_to_lower)
}

pub(crate) fn assign_name(
    builder: &AstBuilder<'_>,
    name: &str,
    value: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let target = name_expr(
        builder,
        name,
        span,
        builder.store_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Assign",
            vec![
                PyList::new_bound(builder.py(), vec![target]).into_py(builder.py()),
                value,
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

pub(crate) fn name_expr(
    builder: &AstBuilder<'_>,
    name: &str,
    span: &SourceSpan,
    ctx: PyObject,
) -> Result<PyObject, LowerError> {
    builder
        .call_node(
            "Name",
            vec![name.to_string().into_py(builder.py()), ctx],
            span,
        )
        .map_err(py_err_to_lower)
}

pub(crate) fn string_expr(
    builder: &AstBuilder<'_>,
    value: &str,
    raw: bool,
    delimiter: StringDelimiter,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let rendered = match (raw, delimiter) {
        (true, StringDelimiter::Single) => format!("r'{}'", value),
        (true, StringDelimiter::Double) => format!("r\"{}\"", value),
        (true, StringDelimiter::TripleSingle) => format!("r'''{}'''", value),
        (true, StringDelimiter::TripleDouble) => format!("r\"\"\"{}\"\"\"", value),
        (false, StringDelimiter::Single) => format!("'{}'", value),
        (false, StringDelimiter::Double) => format!("\"{}\"", value),
        (false, StringDelimiter::TripleSingle) => format!("'''{}'''", value),
        (false, StringDelimiter::TripleDouble) => format!("\"\"\"{}\"\"\"", value),
    };
    eval_literal(builder, &rendered, span)
}

pub(crate) fn byte_string_expr(
    builder: &AstBuilder<'_>,
    value: &str,
    raw: bool,
    delimiter: StringDelimiter,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let rendered = match (raw, delimiter) {
        (true, StringDelimiter::Single) => format!("rb'{}'", value),
        (true, StringDelimiter::Double) => format!("rb\"{}\"", value),
        (true, StringDelimiter::TripleSingle) => format!("rb'''{}'''", value),
        (true, StringDelimiter::TripleDouble) => format!("rb\"\"\"{}\"\"\"", value),
        (false, StringDelimiter::Single) => format!("b'{}'", value),
        (false, StringDelimiter::Double) => format!("b\"{}\"", value),
        (false, StringDelimiter::TripleSingle) => format!("b'''{}'''", value),
        (false, StringDelimiter::TripleDouble) => format!("b\"\"\"{}\"\"\"", value),
    };
    eval_literal(builder, &rendered, span)
}

pub(crate) fn number_expr(
    builder: &AstBuilder<'_>,
    value: &str,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    eval_literal(builder, value, span)
}

pub(crate) fn regex_pattern_expr(
    builder: &AstBuilder<'_>,
    pattern: &str,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    string_expr(builder, pattern, true, StringDelimiter::Double, span)
}

pub(crate) fn bool_constant(
    builder: &AstBuilder<'_>,
    value: bool,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    builder
        .call_node("Constant", vec![value.into_py(builder.py())], span)
        .map_err(py_err_to_lower)
}

pub(crate) fn build_let_guard_test(
    builder: &AstBuilder<'_>,
    guard: Option<&Expr>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let ok_expr = Expr::Name {
        name: SNAIL_LET_OK.to_string(),
        span: span.clone(),
    };
    let test_expr = if let Some(guard) = guard {
        Expr::Binary {
            left: Box::new(ok_expr),
            op: BinaryOp::And,
            right: Box::new(guard.clone()),
            span: span.clone(),
        }
    } else {
        ok_expr
    };
    lower_expr(builder, &test_expr)
}

pub(crate) fn build_destructure_try(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let target = lower_assign_target(builder, target)?;
    let value_expr = name_expr(
        builder,
        SNAIL_LET_VALUE,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let assign = builder
        .call_node(
            "Assign",
            vec![
                PyList::new_bound(builder.py(), vec![target]).into_py(builder.py()),
                value_expr,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let ok_true = assign_name(
        builder,
        SNAIL_LET_OK,
        bool_constant(builder, true, span)?,
        span,
    )?;
    let ok_false = assign_name(
        builder,
        SNAIL_LET_OK,
        bool_constant(builder, false, span)?,
        span,
    )?;
    let handler = build_destructure_handler(builder, ok_false, span)?;
    builder
        .call_node(
            "Try",
            vec![
                PyList::new_bound(builder.py(), vec![assign]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![handler]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![ok_true]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn build_destructure_handler(
    builder: &AstBuilder<'_>,
    ok_false: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let exc_type = build_destructure_exception_tuple(builder, span)?;
    builder
        .call_node(
            "ExceptHandler",
            vec![
                exc_type,
                builder.py().None().into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![ok_false]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn build_destructure_exception_tuple(
    builder: &AstBuilder<'_>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let type_error = name_expr(
        builder,
        "TypeError",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let value_error = name_expr(
        builder,
        "ValueError",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Tuple",
            vec![
                PyList::new_bound(builder.py(), vec![type_error, value_error])
                    .into_py(builder.py()),
                builder.load_ctx().map_err(py_err_to_lower)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)
}
