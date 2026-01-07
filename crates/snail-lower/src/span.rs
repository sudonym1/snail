use snail_ast::SourceSpan;
use snail_python_ast::*;

pub(crate) fn span_from_block(block: &[PyStmt]) -> Option<SourceSpan> {
    let first = block.first()?;
    let last = block.last()?;
    Some(merge_span(stmt_span(first), stmt_span(last)))
}

pub(crate) fn stmt_span(stmt: &PyStmt) -> &SourceSpan {
    match stmt {
        PyStmt::If { span, .. }
        | PyStmt::While { span, .. }
        | PyStmt::For { span, .. }
        | PyStmt::FunctionDef { span, .. }
        | PyStmt::ClassDef { span, .. }
        | PyStmt::Try { span, .. }
        | PyStmt::With { span, .. }
        | PyStmt::Return { span, .. }
        | PyStmt::Raise { span, .. }
        | PyStmt::Assert { span, .. }
        | PyStmt::Delete { span, .. }
        | PyStmt::Break { span, .. }
        | PyStmt::Continue { span, .. }
        | PyStmt::Pass { span, .. }
        | PyStmt::Import { span, .. }
        | PyStmt::ImportFrom { span, .. }
        | PyStmt::Assign { span, .. }
        | PyStmt::Expr { span, .. } => span,
    }
}

pub(crate) fn expr_span(expr: &PyExpr) -> &SourceSpan {
    match expr {
        PyExpr::Name { span, .. }
        | PyExpr::Number { span, .. }
        | PyExpr::String { span, .. }
        | PyExpr::FString { span, .. }
        | PyExpr::Bool { span, .. }
        | PyExpr::None { span }
        | PyExpr::Unary { span, .. }
        | PyExpr::Binary { span, .. }
        | PyExpr::Compare { span, .. }
        | PyExpr::IfExpr { span, .. }
        | PyExpr::Lambda { span, .. }
        | PyExpr::Call { span, .. }
        | PyExpr::Attribute { span, .. }
        | PyExpr::Index { span, .. }
        | PyExpr::Paren { span, .. }
        | PyExpr::List { span, .. }
        | PyExpr::Tuple { span, .. }
        | PyExpr::Dict { span, .. }
        | PyExpr::Set { span, .. }
        | PyExpr::ListComp { span, .. }
        | PyExpr::DictComp { span, .. }
        | PyExpr::Slice { span, .. } => span,
    }
}

pub(crate) fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start: left.start.clone(),
        end: right.end.clone(),
    }
}
