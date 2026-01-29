use pyo3::prelude::*;
use snail_ast::{AugAssignOp, BinaryOp, CompareOp, UnaryOp};
use snail_error::LowerError;

use super::py_ast::{AstBuilder, py_err_to_lower};

pub(crate) fn lower_unary_op(
    builder: &AstBuilder<'_>,
    op: UnaryOp,
) -> Result<PyObject, LowerError> {
    let name = match op {
        UnaryOp::Plus => "UAdd",
        UnaryOp::Minus => "USub",
        UnaryOp::Not => "Not",
    };
    builder.op(name).map_err(py_err_to_lower)
}

pub(crate) fn lower_binary_op(
    builder: &AstBuilder<'_>,
    op: BinaryOp,
) -> Result<PyObject, LowerError> {
    let name = match op {
        BinaryOp::Add => "Add",
        BinaryOp::Sub => "Sub",
        BinaryOp::Mul => "Mult",
        BinaryOp::Div => "Div",
        BinaryOp::FloorDiv => "FloorDiv",
        BinaryOp::Mod => "Mod",
        BinaryOp::Pow => "Pow",
        BinaryOp::Or | BinaryOp::And | BinaryOp::Pipeline => {
            return Err(LowerError::new(
                "boolean/pipeline ops should be handled specially",
            ));
        }
    };
    builder.op(name).map_err(py_err_to_lower)
}

pub(crate) fn lower_aug_assign_op(
    builder: &AstBuilder<'_>,
    op: AugAssignOp,
) -> Result<PyObject, LowerError> {
    let name = match op {
        AugAssignOp::Add => "Add",
        AugAssignOp::Sub => "Sub",
        AugAssignOp::Mul => "Mult",
        AugAssignOp::Div => "Div",
        AugAssignOp::FloorDiv => "FloorDiv",
        AugAssignOp::Mod => "Mod",
        AugAssignOp::Pow => "Pow",
    };
    builder.op(name).map_err(py_err_to_lower)
}

pub(crate) fn aug_op_to_string(op: AugAssignOp) -> &'static str {
    match op {
        AugAssignOp::Add => "+",
        AugAssignOp::Sub => "-",
        AugAssignOp::Mul => "*",
        AugAssignOp::Div => "/",
        AugAssignOp::FloorDiv => "//",
        AugAssignOp::Mod => "%",
        AugAssignOp::Pow => "**",
    }
}

pub(crate) fn lower_bool_op(
    builder: &AstBuilder<'_>,
    op: BinaryOp,
) -> Result<PyObject, LowerError> {
    let name = match op {
        BinaryOp::Or => "Or",
        BinaryOp::And => "And",
        _ => return Err(LowerError::new("expected boolean operator")),
    };
    builder.op(name).map_err(py_err_to_lower)
}

pub(crate) fn lower_compare_op(
    builder: &AstBuilder<'_>,
    op: CompareOp,
) -> Result<PyObject, LowerError> {
    let name = match op {
        CompareOp::Eq => "Eq",
        CompareOp::NotEq => "NotEq",
        CompareOp::Lt => "Lt",
        CompareOp::LtEq => "LtE",
        CompareOp::Gt => "Gt",
        CompareOp::GtEq => "GtE",
        CompareOp::In => "In",
        CompareOp::NotIn => "NotIn",
        CompareOp::Is => "Is",
        CompareOp::IsNot => "IsNot",
    };
    builder.op(name).map_err(py_err_to_lower)
}
