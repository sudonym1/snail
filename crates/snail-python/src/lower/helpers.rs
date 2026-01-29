use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::{SourceSpan, StringDelimiter};
use snail_error::LowerError;

use super::py_ast::{AstBuilder, py_err_to_lower, set_location};

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
    let expr = builder
        .py()
        .import_bound("ast")
        .and_then(|ast| ast.getattr("parse"))
        .and_then(|parse| parse.call1((rendered,)))
        .and_then(|module| module.getattr("body"))
        .and_then(|body| body.get_item(0))
        .and_then(|expr_stmt| expr_stmt.getattr("value"));

    let expr = expr.map_err(py_err_to_lower)?;
    set_location(&expr, span).map_err(py_err_to_lower)?;
    Ok(expr.into_py(builder.py()))
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
    let expr = builder
        .py()
        .import_bound("ast")
        .and_then(|ast| ast.getattr("parse"))
        .and_then(|parse| parse.call1((rendered,)))
        .and_then(|module| module.getattr("body"))
        .and_then(|body| body.get_item(0))
        .and_then(|expr_stmt| expr_stmt.getattr("value"));

    let expr = expr.map_err(py_err_to_lower)?;
    set_location(&expr, span).map_err(py_err_to_lower)?;
    Ok(expr.into_py(builder.py()))
}

pub(crate) fn number_expr(
    builder: &AstBuilder<'_>,
    value: &str,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let expr = builder
        .py()
        .import_bound("ast")
        .and_then(|ast| ast.getattr("parse"))
        .and_then(|parse| parse.call1((value,)))
        .and_then(|module| module.getattr("body"))
        .and_then(|body| body.get_item(0))
        .and_then(|expr_stmt| expr_stmt.getattr("value"));

    let expr = expr.map_err(py_err_to_lower)?;
    set_location(&expr, span).map_err(py_err_to_lower)?;
    Ok(expr.into_py(builder.py()))
}

pub(crate) fn regex_pattern_expr(
    builder: &AstBuilder<'_>,
    pattern: &str,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    string_expr(builder, pattern, true, StringDelimiter::Double, span)
}
