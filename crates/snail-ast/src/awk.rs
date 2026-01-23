use crate::ast::{Expr, SourceSpan, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileMode {
    Snail,
    Awk,
    Map,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AwkProgram {
    pub begin_blocks: Vec<Vec<Stmt>>,
    pub rules: Vec<AwkRule>,
    pub end_blocks: Vec<Vec<Stmt>>,
    pub span: SourceSpan,
}

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
