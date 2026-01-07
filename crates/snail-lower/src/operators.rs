use snail_ast::{BinaryOp, CompareOp, UnaryOp};
use snail_python_ast::{PyBinaryOp, PyCompareOp, PyUnaryOp};

pub(crate) fn lower_unary_op(op: UnaryOp) -> PyUnaryOp {
    match op {
        UnaryOp::Plus => PyUnaryOp::Plus,
        UnaryOp::Minus => PyUnaryOp::Minus,
        UnaryOp::Not => PyUnaryOp::Not,
    }
}

pub(crate) fn lower_binary_op(op: BinaryOp) -> PyBinaryOp {
    match op {
        BinaryOp::Or => PyBinaryOp::Or,
        BinaryOp::And => PyBinaryOp::And,
        BinaryOp::Add => PyBinaryOp::Add,
        BinaryOp::Sub => PyBinaryOp::Sub,
        BinaryOp::Mul => PyBinaryOp::Mul,
        BinaryOp::Div => PyBinaryOp::Div,
        BinaryOp::FloorDiv => PyBinaryOp::FloorDiv,
        BinaryOp::Mod => PyBinaryOp::Mod,
        BinaryOp::Pow => PyBinaryOp::Pow,
        BinaryOp::Pipeline => {
            panic!("Pipeline operator should be handled specially in lower_expr_with_exception")
        }
    }
}

pub(crate) fn lower_compare_op(op: CompareOp) -> PyCompareOp {
    match op {
        CompareOp::Eq => PyCompareOp::Eq,
        CompareOp::NotEq => PyCompareOp::NotEq,
        CompareOp::Lt => PyCompareOp::Lt,
        CompareOp::LtEq => PyCompareOp::LtEq,
        CompareOp::Gt => PyCompareOp::Gt,
        CompareOp::GtEq => PyCompareOp::GtEq,
        CompareOp::In => PyCompareOp::In,
        CompareOp::Is => PyCompareOp::Is,
    }
}
