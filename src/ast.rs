#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePos {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourcePos,
    pub end: SourcePos,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    If {
        cond: Expr,
        body: Vec<Stmt>,
        elifs: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: SourceSpan,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        span: SourceSpan,
    },
    For {
        target: AssignTarget,
        iter: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        span: SourceSpan,
    },
    Def {
        name: String,
        params: Vec<Parameter>,
        body: Vec<Stmt>,
        span: SourceSpan,
    },
    Class {
        name: String,
        body: Vec<Stmt>,
        span: SourceSpan,
    },
    Try {
        body: Vec<Stmt>,
        handlers: Vec<ExceptHandler>,
        else_body: Option<Vec<Stmt>>,
        finally_body: Option<Vec<Stmt>>,
        span: SourceSpan,
    },
    With {
        items: Vec<WithItem>,
        body: Vec<Stmt>,
        span: SourceSpan,
    },
    Return {
        value: Option<Expr>,
        span: SourceSpan,
    },
    Raise {
        value: Option<Expr>,
        from: Option<Expr>,
        span: SourceSpan,
    },
    Assert {
        test: Expr,
        message: Option<Expr>,
        span: SourceSpan,
    },
    Delete {
        targets: Vec<AssignTarget>,
        span: SourceSpan,
    },
    Break {
        span: SourceSpan,
    },
    Continue {
        span: SourceSpan,
    },
    Pass {
        span: SourceSpan,
    },
    Import {
        items: Vec<ImportItem>,
        span: SourceSpan,
    },
    ImportFrom {
        module: Vec<String>,
        items: Vec<ImportItem>,
        span: SourceSpan,
    },
    Assign {
        targets: Vec<AssignTarget>,
        value: Expr,
        span: SourceSpan,
    },
    Expr {
        value: Expr,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithItem {
    pub context: Expr,
    pub target: Option<AssignTarget>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExceptHandler {
    pub type_name: Option<Expr>,
    pub name: Option<String>,
    pub body: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    pub name: Vec<String>,
    pub alias: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Name {
        name: String,
        span: SourceSpan,
    },
    Attribute {
        value: Box<Expr>,
        attr: String,
        span: SourceSpan,
    },
    Index {
        value: Box<Expr>,
        index: Box<Expr>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Name {
        name: String,
        span: SourceSpan,
    },
    Number {
        value: String,
        span: SourceSpan,
    },
    String {
        value: String,
        raw: bool,
        delimiter: StringDelimiter,
        span: SourceSpan,
    },
    Bool {
        value: bool,
        span: SourceSpan,
    },
    None {
        span: SourceSpan,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: SourceSpan,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: SourceSpan,
    },
    Compare {
        left: Box<Expr>,
        ops: Vec<CompareOp>,
        comparators: Vec<Expr>,
        span: SourceSpan,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Argument>,
        span: SourceSpan,
    },
    Attribute {
        value: Box<Expr>,
        attr: String,
        span: SourceSpan,
    },
    Index {
        value: Box<Expr>,
        index: Box<Expr>,
        span: SourceSpan,
    },
    Paren {
        expr: Box<Expr>,
        span: SourceSpan,
    },
    List {
        elements: Vec<Expr>,
        span: SourceSpan,
    },
    Tuple {
        elements: Vec<Expr>,
        span: SourceSpan,
    },
    Dict {
        entries: Vec<(Expr, Expr)>,
        span: SourceSpan,
    },
    Set {
        elements: Vec<Expr>,
        span: SourceSpan,
    },
    Slice {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        span: SourceSpan,
    },
    ListComp {
        element: Box<Expr>,
        target: String,
        iter: Box<Expr>,
        ifs: Vec<Expr>,
        span: SourceSpan,
    },
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        target: String,
        iter: Box<Expr>,
        ifs: Vec<Expr>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringDelimiter {
    Single,
    Double,
    TripleSingle,
    TripleDouble,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Regular {
        name: String,
        default: Option<Expr>,
        span: SourceSpan,
    },
    VarArgs {
        name: String,
        span: SourceSpan,
    },
    KwArgs {
        name: String,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    Positional {
        value: Expr,
        span: SourceSpan,
    },
    Keyword {
        name: String,
        value: Expr,
        span: SourceSpan,
    },
    Star {
        value: Expr,
        span: SourceSpan,
    },
    KwStar {
        value: Expr,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
    Is,
}
