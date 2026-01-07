use snail_ast::{SourceSpan, StringDelimiter};

#[derive(Debug, Clone, PartialEq)]
pub struct PyModule {
    pub body: Vec<PyStmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyStmt {
    If {
        test: PyExpr,
        body: Vec<PyStmt>,
        orelse: Vec<PyStmt>,
        span: SourceSpan,
    },
    While {
        test: PyExpr,
        body: Vec<PyStmt>,
        orelse: Vec<PyStmt>,
        span: SourceSpan,
    },
    For {
        target: PyExpr,
        iter: PyExpr,
        body: Vec<PyStmt>,
        orelse: Vec<PyStmt>,
        span: SourceSpan,
    },
    FunctionDef {
        name: String,
        args: Vec<PyParameter>,
        body: Vec<PyStmt>,
        span: SourceSpan,
    },
    ClassDef {
        name: String,
        body: Vec<PyStmt>,
        span: SourceSpan,
    },
    Try {
        body: Vec<PyStmt>,
        handlers: Vec<PyExceptHandler>,
        orelse: Vec<PyStmt>,
        finalbody: Vec<PyStmt>,
        span: SourceSpan,
    },
    With {
        items: Vec<PyWithItem>,
        body: Vec<PyStmt>,
        span: SourceSpan,
    },
    Return {
        value: Option<PyExpr>,
        span: SourceSpan,
    },
    Raise {
        value: Option<PyExpr>,
        from: Option<PyExpr>,
        span: SourceSpan,
    },
    Assert {
        test: PyExpr,
        message: Option<PyExpr>,
        span: SourceSpan,
    },
    Delete {
        targets: Vec<PyExpr>,
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
        names: Vec<PyImportName>,
        span: SourceSpan,
    },
    ImportFrom {
        module: Vec<String>,
        names: Vec<PyImportName>,
        span: SourceSpan,
    },
    Assign {
        targets: Vec<PyExpr>,
        value: PyExpr,
        span: SourceSpan,
    },
    Expr {
        value: PyExpr,
        semicolon_terminated: bool,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyExceptHandler {
    pub type_name: Option<PyExpr>,
    pub name: Option<String>,
    pub body: Vec<PyStmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyWithItem {
    pub context: PyExpr,
    pub target: Option<PyExpr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyImportName {
    pub name: Vec<String>,
    pub asname: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyExpr {
    Name {
        id: String,
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
    FString {
        parts: Vec<PyFStringPart>,
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
        op: PyUnaryOp,
        operand: Box<PyExpr>,
        span: SourceSpan,
    },
    Binary {
        left: Box<PyExpr>,
        op: PyBinaryOp,
        right: Box<PyExpr>,
        span: SourceSpan,
    },
    Compare {
        left: Box<PyExpr>,
        ops: Vec<PyCompareOp>,
        comparators: Vec<PyExpr>,
        span: SourceSpan,
    },
    IfExpr {
        test: Box<PyExpr>,
        body: Box<PyExpr>,
        orelse: Box<PyExpr>,
        span: SourceSpan,
    },
    Lambda {
        params: Vec<String>,
        body: Box<PyExpr>,
        span: SourceSpan,
    },
    Call {
        func: Box<PyExpr>,
        args: Vec<PyArgument>,
        span: SourceSpan,
    },
    Attribute {
        value: Box<PyExpr>,
        attr: String,
        span: SourceSpan,
    },
    Index {
        value: Box<PyExpr>,
        index: Box<PyExpr>,
        span: SourceSpan,
    },
    Paren {
        expr: Box<PyExpr>,
        span: SourceSpan,
    },
    List {
        elements: Vec<PyExpr>,
        span: SourceSpan,
    },
    Tuple {
        elements: Vec<PyExpr>,
        span: SourceSpan,
    },
    Dict {
        entries: Vec<(PyExpr, PyExpr)>,
        span: SourceSpan,
    },
    Set {
        elements: Vec<PyExpr>,
        span: SourceSpan,
    },
    ListComp {
        element: Box<PyExpr>,
        target: String,
        iter: Box<PyExpr>,
        ifs: Vec<PyExpr>,
        span: SourceSpan,
    },
    DictComp {
        key: Box<PyExpr>,
        value: Box<PyExpr>,
        target: String,
        iter: Box<PyExpr>,
        ifs: Vec<PyExpr>,
        span: SourceSpan,
    },
    Slice {
        start: Option<Box<PyExpr>>,
        end: Option<Box<PyExpr>>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyFStringPart {
    Text(String),
    Expr(PyExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyParameter {
    Regular {
        name: String,
        default: Option<PyExpr>,
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
pub enum PyArgument {
    Positional {
        value: PyExpr,
        span: SourceSpan,
    },
    Keyword {
        name: String,
        value: PyExpr,
        span: SourceSpan,
    },
    Star {
        value: PyExpr,
        span: SourceSpan,
    },
    KwStar {
        value: PyExpr,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyUnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyBinaryOp {
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
pub enum PyCompareOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
    Is,
}
