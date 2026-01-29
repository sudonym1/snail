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
        cond: Condition,
        body: Vec<Stmt>,
        elifs: Vec<(Condition, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: SourceSpan,
    },
    While {
        cond: Condition,
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
        level: usize,
        module: Option<Vec<String>>,
        items: ImportFromItems,
        span: SourceSpan,
    },
    Assign {
        targets: Vec<AssignTarget>,
        value: Expr,
        span: SourceSpan,
    },
    Expr {
        value: Expr,
        semicolon_terminated: bool,
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
pub enum ImportFromItems {
    Names(Vec<ImportItem>),
    Star { span: SourceSpan },
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
    Starred {
        target: Box<AssignTarget>,
        span: SourceSpan,
    },
    Tuple {
        elements: Vec<AssignTarget>,
        span: SourceSpan,
    },
    List {
        elements: Vec<AssignTarget>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Expr(Box<Expr>),
    Let {
        target: Box<AssignTarget>,
        value: Box<Expr>,
        guard: Option<Box<Expr>>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Name {
        name: String,
        span: SourceSpan,
    },
    Placeholder {
        span: SourceSpan,
    },
    Number {
        value: String,
        span: SourceSpan,
    },
    String {
        value: String,
        raw: bool,
        bytes: bool,
        delimiter: StringDelimiter,
        span: SourceSpan,
    },
    FString {
        parts: Vec<FStringPart>,
        bytes: bool,
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
    AugAssign {
        target: Box<AssignTarget>,
        op: AugAssignOp,
        value: Box<Expr>,
        span: SourceSpan,
    },
    PrefixIncr {
        op: IncrOp,
        target: Box<AssignTarget>,
        span: SourceSpan,
    },
    PostfixIncr {
        op: IncrOp,
        target: Box<AssignTarget>,
        span: SourceSpan,
    },
    Compare {
        left: Box<Expr>,
        ops: Vec<CompareOp>,
        comparators: Vec<Expr>,
        span: SourceSpan,
    },
    IfExpr {
        test: Box<Expr>,
        body: Box<Expr>,
        orelse: Box<Expr>,
        span: SourceSpan,
    },
    TryExpr {
        expr: Box<Expr>,
        fallback: Option<Box<Expr>>,
        span: SourceSpan,
    },
    Yield {
        value: Option<Box<Expr>>,
        span: SourceSpan,
    },
    YieldFrom {
        expr: Box<Expr>,
        span: SourceSpan,
    },
    Lambda {
        params: Vec<Parameter>,
        body: Vec<Stmt>,
        span: SourceSpan,
    },
    Compound {
        expressions: Vec<Expr>,
        span: SourceSpan,
    },
    Regex {
        pattern: RegexPattern,
        span: SourceSpan,
    },
    RegexMatch {
        value: Box<Expr>,
        pattern: RegexPattern,
        span: SourceSpan,
    },
    Subprocess {
        kind: SubprocessKind,
        parts: Vec<SubprocessPart>,
        span: SourceSpan,
    },
    StructuredAccessor {
        query: String,
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
    FieldIndex {
        index: String,
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
    Set {
        elements: Vec<Expr>,
        span: SourceSpan,
    },
    Dict {
        entries: Vec<(Expr, Expr)>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FStringConversion {
    #[default]
    None,
    Str,
    Repr,
    Ascii,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FStringExpr {
    pub expr: Box<Expr>,
    pub conversion: FStringConversion,
    pub format_spec: Option<Vec<FStringPart>>,
}

impl FStringExpr {
    pub fn new(expr: Box<Expr>) -> Self {
        Self {
            expr,
            conversion: FStringConversion::None,
            format_spec: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Text(String),
    Expr(FStringExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexPattern {
    Literal(String),
    Interpolated(Vec<FStringPart>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubprocessKind {
    Capture,
    Status,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubprocessPart {
    Text(String),
    Expr(Box<Expr>),
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
    Pipeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AugAssignOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrOp {
    Increment,
    Decrement,
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
    NotIn,
    Is,
    IsNot,
}
