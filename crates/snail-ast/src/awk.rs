use crate::ast::{Expr, SourceSpan, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub struct AwkRule {
    pub pattern: Option<Expr>,
    pub action: Option<Vec<Stmt>>,
    pub span: SourceSpan,
}

impl AwkRule {
    pub fn has_explicit_action(&self) -> bool {
        self.action.is_some()
    }
}
