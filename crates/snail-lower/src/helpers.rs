use snail_ast::{SourceSpan, StringDelimiter};
use snail_python_ast::*;

pub(crate) fn assign_name(name: &str, value: PyExpr, span: &SourceSpan) -> PyStmt {
    PyStmt::Assign {
        targets: vec![name_expr(name, span)],
        value,
        span: span.clone(),
    }
}

pub(crate) fn name_expr(name: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Name {
        id: name.to_string(),
        span: span.clone(),
    }
}

pub(crate) fn string_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: value.to_string(),
        raw: false,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

pub(crate) fn number_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Number {
        value: value.to_string(),
        span: span.clone(),
    }
}

pub(crate) fn regex_pattern_expr(pattern: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: pattern.to_string(),
        raw: true,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

pub(crate) fn pos_arg(value: PyExpr, span: &SourceSpan) -> PyArgument {
    PyArgument::Positional {
        value,
        span: span.clone(),
    }
}
