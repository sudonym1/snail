use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use crate::ast::*;
use crate::awk::{AwkProgram, AwkRule};

const SNAIL_TRY_HELPER: &str = "__snail_compact_try";
const SNAIL_EXCEPTION_VAR: &str = "__snail_compact_exc";
const SNAIL_SUBPROCESS_CAPTURE: &str = "__snail_subprocess_capture";
const SNAIL_SUBPROCESS_STATUS: &str = "__snail_subprocess_status";
const SNAIL_REGEX_SEARCH: &str = "__snail_regex_search";
const SNAIL_REGEX_COMPILE: &str = "__snail_regex_compile";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    pub message: String,
}

impl LowerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for LowerError {}

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

pub fn lower_program(program: &Program) -> Result<PyModule, LowerError> {
    let mut body = Vec::new();
    for stmt in &program.stmts {
        body.push(lower_stmt(stmt)?);
    }
    Ok(PyModule {
        body,
        span: program.span.clone(),
    })
}

pub fn lower_awk_program(program: &AwkProgram) -> Result<PyModule, LowerError> {
    let span = program.span.clone();
    let mut body = Vec::new();

    body.push(PyStmt::Import {
        names: vec![PyImportName {
            name: vec!["sys".to_string()],
            asname: None,
            span: span.clone(),
        }],
        span: span.clone(),
    });

    let mut main_body = Vec::new();
    for block in &program.begin_blocks {
        main_body.extend(lower_block(block)?);
    }

    main_body.push(assign_name("__snail_nr", number_expr("0", &span), &span));

    let files_expr = PyExpr::Binary {
        left: Box::new(PyExpr::Index {
            value: Box::new(PyExpr::Attribute {
                value: Box::new(name_expr("sys", &span)),
                attr: "argv".to_string(),
                span: span.clone(),
            }),
            index: Box::new(PyExpr::Slice {
                start: Some(Box::new(number_expr("1", &span))),
                end: None,
                span: span.clone(),
            }),
            span: span.clone(),
        }),
        op: PyBinaryOp::Or,
        right: Box::new(PyExpr::List {
            elements: vec![string_expr("-", &span)],
            span: span.clone(),
        }),
        span: span.clone(),
    };

    let file_loop = lower_awk_file_loop(program, &span)?;
    main_body.push(PyStmt::For {
        target: name_expr("__snail_path", &span),
        iter: files_expr,
        body: file_loop,
        orelse: Vec::new(),
        span: span.clone(),
    });

    for block in &program.end_blocks {
        main_body.extend(lower_block(block)?);
    }

    body.extend(main_body);

    Ok(PyModule { body, span })
}

fn lower_stmt(stmt: &Stmt) -> Result<PyStmt, LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => lower_if(cond, body, elifs, else_body, span),
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => Ok(PyStmt::While {
            test: lower_expr(cond)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => Ok(PyStmt::For {
            target: lower_assign_target(target)?,
            iter: lower_expr(iter)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::Def {
            name,
            params,
            body,
            span,
        } => Ok(PyStmt::FunctionDef {
            name: name.clone(),
            args: params
                .iter()
                .map(lower_parameter)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Class { name, body, span } => Ok(PyStmt::ClassDef {
            name: name.clone(),
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => Ok(PyStmt::Try {
            body: lower_block(body)?,
            handlers: handlers
                .iter()
                .map(lower_except_handler)
                .collect::<Result<Vec<_>, _>>()?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            finalbody: finally_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::With { items, body, span } => Ok(PyStmt::With {
            items: items
                .iter()
                .map(lower_with_item)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Return { value, span } => Ok(PyStmt::Return {
            value: value.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Raise { value, from, span } => Ok(PyStmt::Raise {
            value: value.as_ref().map(lower_expr).transpose()?,
            from: from.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Assert {
            test,
            message,
            span,
        } => Ok(PyStmt::Assert {
            test: lower_expr(test)?,
            message: message.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Delete { targets, span } => Ok(PyStmt::Delete {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Stmt::Break { span } => Ok(PyStmt::Break { span: span.clone() }),
        Stmt::Continue { span } => Ok(PyStmt::Continue { span: span.clone() }),
        Stmt::Pass { span } => Ok(PyStmt::Pass { span: span.clone() }),
        Stmt::Import { items, span } => Ok(PyStmt::Import {
            names: items.iter().map(lower_import_name).collect(),
            span: span.clone(),
        }),
        Stmt::ImportFrom {
            module,
            items,
            span,
        } => Ok(PyStmt::ImportFrom {
            module: module.clone(),
            names: items.iter().map(lower_import_name).collect(),
            span: span.clone(),
        }),
        Stmt::Assign {
            targets,
            value,
            span,
        } => Ok(PyStmt::Assign {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Stmt::Expr { value, span } => Ok(PyStmt::Expr {
            value: lower_expr(value)?,
            span: span.clone(),
        }),
    }
}

fn lower_awk_file_loop(program: &AwkProgram, span: &SourceSpan) -> Result<Vec<PyStmt>, LowerError> {
    let mut file_loop = Vec::new();
    file_loop.push(assign_name("__snail_fnr", number_expr("0", span), span));

    let stdin_body = vec![
        assign_name(
            "__snail_file",
            PyExpr::Attribute {
                value: Box::new(name_expr("sys", span)),
                attr: "stdin".to_string(),
                span: span.clone(),
            },
            span,
        ),
        lower_awk_line_loop(program, span, name_expr("__snail_file", span))?,
    ];

    let open_call = PyExpr::Call {
        func: Box::new(name_expr("open", span)),
        args: vec![pos_arg(name_expr("__snail_path", span), span)],
        span: span.clone(),
    };
    let with_item = PyWithItem {
        context: open_call,
        target: Some(name_expr("__snail_file", span)),
        span: span.clone(),
    };
    let with_stmt = PyStmt::With {
        items: vec![with_item],
        body: vec![lower_awk_line_loop(
            program,
            span,
            name_expr("__snail_file", span),
        )?],
        span: span.clone(),
    };

    let test = PyExpr::Compare {
        left: Box::new(name_expr("__snail_path", span)),
        ops: vec![PyCompareOp::Eq],
        comparators: vec![string_expr("-", span)],
        span: span.clone(),
    };

    file_loop.push(PyStmt::If {
        test,
        body: stdin_body,
        orelse: vec![with_stmt],
        span: span.clone(),
    });

    Ok(file_loop)
}

fn lower_awk_line_loop(
    program: &AwkProgram,
    span: &SourceSpan,
    iter: PyExpr,
) -> Result<PyStmt, LowerError> {
    let mut loop_body = Vec::new();
    loop_body.push(assign_name(
        "__snail_nr",
        PyExpr::Binary {
            left: Box::new(name_expr("__snail_nr", span)),
            op: PyBinaryOp::Add,
            right: Box::new(number_expr("1", span)),
            span: span.clone(),
        },
        span,
    ));
    loop_body.push(assign_name(
        "__snail_fnr",
        PyExpr::Binary {
            left: Box::new(name_expr("__snail_fnr", span)),
            op: PyBinaryOp::Add,
            right: Box::new(number_expr("1", span)),
            span: span.clone(),
        },
        span,
    ));

    let rstrip_call = PyExpr::Call {
        func: Box::new(PyExpr::Attribute {
            value: Box::new(name_expr("__snail_raw", span)),
            attr: "rstrip".to_string(),
            span: span.clone(),
        }),
        args: vec![pos_arg(string_expr("\\n", span), span)],
        span: span.clone(),
    };
    loop_body.push(assign_name("line", rstrip_call, span));

    let split_call = PyExpr::Call {
        func: Box::new(PyExpr::Attribute {
            value: Box::new(name_expr("line", span)),
            attr: "split".to_string(),
            span: span.clone(),
        }),
        args: Vec::new(),
        span: span.clone(),
    };
    loop_body.push(assign_name("fields", split_call, span));
    loop_body.push(assign_name("nr", name_expr("__snail_nr", span), span));
    loop_body.push(assign_name("fnr", name_expr("__snail_fnr", span), span));
    loop_body.push(assign_name("path", name_expr("__snail_path", span), span));

    loop_body.extend(lower_awk_rules(&program.rules)?);

    Ok(PyStmt::For {
        target: name_expr("__snail_raw", span),
        iter,
        body: loop_body,
        orelse: Vec::new(),
        span: span.clone(),
    })
}

fn lower_awk_rules(rules: &[AwkRule]) -> Result<Vec<PyStmt>, LowerError> {
    let mut stmts = Vec::new();
    for rule in rules {
        let mut action = if rule.has_action() {
            lower_block(&rule.action)?
        } else {
            vec![awk_default_print(&rule.span)]
        };

        if let Some(pattern) = &rule.pattern {
            if let Some((value_expr, regex, span)) = regex_pattern_components(pattern) {
                let match_call = lower_regex_match(&value_expr, &regex, &span, None)?;
                stmts.push(PyStmt::Assign {
                    targets: vec![name_expr("match", &span)],
                    value: match_call,
                    span: span.clone(),
                });
                stmts.push(PyStmt::If {
                    test: name_expr("match", &span),
                    body: action,
                    orelse: Vec::new(),
                    span: rule.span.clone(),
                });
            } else {
                stmts.push(PyStmt::If {
                    test: lower_expr(pattern)?,
                    body: action,
                    orelse: Vec::new(),
                    span: rule.span.clone(),
                });
            }
        } else {
            stmts.append(&mut action);
        }
    }
    Ok(stmts)
}

fn regex_pattern_components(pattern: &Expr) -> Option<(Expr, String, SourceSpan)> {
    match pattern {
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Some((*value.clone(), pattern.clone(), span.clone())),
        Expr::Regex { pattern, span } => Some((
            Expr::Name {
                name: "line".to_string(),
                span: span.clone(),
            },
            pattern.clone(),
            span.clone(),
        )),
        _ => None,
    }
}

fn awk_default_print(span: &SourceSpan) -> PyStmt {
    PyStmt::Expr {
        value: PyExpr::Call {
            func: Box::new(name_expr("print", span)),
            args: vec![pos_arg(name_expr("line", span), span)],
            span: span.clone(),
        },
        span: span.clone(),
    }
}

fn assign_name(name: &str, value: PyExpr, span: &SourceSpan) -> PyStmt {
    PyStmt::Assign {
        targets: vec![name_expr(name, span)],
        value,
        span: span.clone(),
    }
}

fn name_expr(name: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Name {
        id: name.to_string(),
        span: span.clone(),
    }
}

fn string_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: value.to_string(),
        raw: false,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

fn number_expr(value: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::Number {
        value: value.to_string(),
        span: span.clone(),
    }
}

fn regex_pattern_expr(pattern: &str, span: &SourceSpan) -> PyExpr {
    PyExpr::String {
        value: pattern.to_string(),
        raw: true,
        delimiter: StringDelimiter::Double,
        span: span.clone(),
    }
}

fn pos_arg(value: PyExpr, span: &SourceSpan) -> PyArgument {
    PyArgument::Positional {
        value,
        span: span.clone(),
    }
}

fn lower_if(
    cond: &Expr,
    body: &[Stmt],
    elifs: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span_hint: &SourceSpan,
) -> Result<PyStmt, LowerError> {
    let test = lower_expr(cond)?;
    let body = lower_block(body)?;
    let mut span = span_from_block(&body).unwrap_or_else(|| span_hint.clone());
    span.start = expr_span(&test).start.clone();
    if let Some((elif_cond, elif_body)) = elifs.first() {
        let nested = lower_if(elif_cond, elif_body, &elifs[1..], else_body, span_hint)?;
        span.end = stmt_span(&nested).end.clone();
        Ok(PyStmt::If {
            test,
            body,
            orelse: vec![nested],
            span,
        })
    } else {
        let orelse = match else_body {
            Some(else_block) => lower_block(else_block)?,
            None => Vec::new(),
        };
        if let Some(last) = orelse.last() {
            span.end = stmt_span(last).end.clone();
        }
        Ok(PyStmt::If {
            test,
            body,
            orelse,
            span,
        })
    }
}

fn lower_block(block: &[Stmt]) -> Result<Vec<PyStmt>, LowerError> {
    block.iter().map(lower_stmt).collect()
}

fn lower_except_handler(handler: &ExceptHandler) -> Result<PyExceptHandler, LowerError> {
    Ok(PyExceptHandler {
        type_name: handler.type_name.as_ref().map(lower_expr).transpose()?,
        name: handler.name.clone(),
        body: lower_block(&handler.body)?,
        span: handler.span.clone(),
    })
}

fn lower_with_item(item: &WithItem) -> Result<PyWithItem, LowerError> {
    Ok(PyWithItem {
        context: lower_expr(&item.context)?,
        target: item.target.as_ref().map(lower_assign_target).transpose()?,
        span: item.span.clone(),
    })
}

fn lower_import_name(item: &ImportItem) -> PyImportName {
    PyImportName {
        name: item.name.clone(),
        asname: item.alias.clone(),
        span: item.span.clone(),
    }
}

fn lower_assign_target(target: &AssignTarget) -> Result<PyExpr, LowerError> {
    match target {
        AssignTarget::Name { name, span } => Ok(PyExpr::Name {
            id: name.clone(),
            span: span.clone(),
        }),
        AssignTarget::Attribute { value, attr, span } => Ok(PyExpr::Attribute {
            value: Box::new(lower_expr(value)?),
            attr: attr.clone(),
            span: span.clone(),
        }),
        AssignTarget::Index { value, index, span } => Ok(PyExpr::Index {
            value: Box::new(lower_expr(value)?),
            index: Box::new(lower_expr(index)?),
            span: span.clone(),
        }),
    }
}

fn lower_expr(expr: &Expr) -> Result<PyExpr, LowerError> {
    lower_expr_with_exception(expr, None)
}

fn lower_regex_match(
    value: &Expr,
    pattern: &str,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    Ok(PyExpr::Call {
        func: Box::new(PyExpr::Name {
            id: SNAIL_REGEX_SEARCH.to_string(),
            span: span.clone(),
        }),
        args: vec![
            pos_arg(lower_expr_with_exception(value, exception_name)?, span),
            pos_arg(regex_pattern_expr(pattern, span), span),
        ],
        span: span.clone(),
    })
}

fn lower_expr_with_exception(
    expr: &Expr,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    match expr {
        Expr::Name { name, span } => {
            if name == "$e" {
                if let Some(exception_name) = exception_name {
                    return Ok(PyExpr::Name {
                        id: exception_name.to_string(),
                        span: span.clone(),
                    });
                }
                return Err(LowerError::new(
                    "`$e` is only available in compact exception fallbacks",
                ));
            }
            Ok(PyExpr::Name {
                id: name.clone(),
                span: span.clone(),
            })
        }
        Expr::Number { value, span } => Ok(PyExpr::Number {
            value: value.clone(),
            span: span.clone(),
        }),
        Expr::String {
            value,
            raw,
            delimiter,
            span,
        } => Ok(PyExpr::String {
            value: value.clone(),
            raw: *raw,
            delimiter: *delimiter,
            span: span.clone(),
        }),
        Expr::Bool { value, span } => Ok(PyExpr::Bool {
            value: *value,
            span: span.clone(),
        }),
        Expr::None { span } => Ok(PyExpr::None { span: span.clone() }),
        Expr::Unary { op, expr, span } => Ok(PyExpr::Unary {
            op: lower_unary_op(*op),
            operand: Box::new(lower_expr_with_exception(expr, exception_name)?),
            span: span.clone(),
        }),
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => Ok(PyExpr::Binary {
            left: Box::new(lower_expr_with_exception(left, exception_name)?),
            op: lower_binary_op(*op),
            right: Box::new(lower_expr_with_exception(right, exception_name)?),
            span: span.clone(),
        }),
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Ok(PyExpr::Compare {
            left: Box::new(lower_expr_with_exception(left, exception_name)?),
            ops: ops.iter().map(|op| lower_compare_op(*op)).collect(),
            comparators: comparators
                .iter()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => Ok(PyExpr::IfExpr {
            test: Box::new(lower_expr_with_exception(test, exception_name)?),
            body: Box::new(lower_expr_with_exception(body, exception_name)?),
            orelse: Box::new(lower_expr_with_exception(orelse, exception_name)?),
            span: span.clone(),
        }),
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => {
            let try_lambda = PyExpr::Lambda {
                params: Vec::new(),
                body: Box::new(lower_expr_with_exception(expr, exception_name)?),
                span: span.clone(),
            };
            let mut args = vec![PyArgument::Positional {
                value: try_lambda,
                span: span.clone(),
            }];
            if let Some(fallback_expr) = fallback {
                let fallback_lambda = PyExpr::Lambda {
                    params: vec![SNAIL_EXCEPTION_VAR.to_string()],
                    body: Box::new(lower_expr_with_exception(
                        fallback_expr,
                        Some(SNAIL_EXCEPTION_VAR),
                    )?),
                    span: span.clone(),
                };
                args.push(PyArgument::Positional {
                    value: fallback_lambda,
                    span: span.clone(),
                });
            }
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: SNAIL_TRY_HELPER.to_string(),
                    span: span.clone(),
                }),
                args,
                span: span.clone(),
            })
        }
        Expr::Regex { pattern, span } => Ok(PyExpr::Call {
            func: Box::new(PyExpr::Name {
                id: SNAIL_REGEX_COMPILE.to_string(),
                span: span.clone(),
            }),
            args: vec![pos_arg(regex_pattern_expr(pattern, span), span)],
            span: span.clone(),
        }),
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => lower_regex_match(value, pattern, span, exception_name),
        Expr::Subprocess { kind, parts, span } => {
            let mut lowered_parts = Vec::with_capacity(parts.len());
            for part in parts {
                match part {
                    SubprocessPart::Text(text) => {
                        lowered_parts.push(PyFStringPart::Text(text.clone()));
                    }
                    SubprocessPart::Expr(expr) => {
                        lowered_parts.push(PyFStringPart::Expr(lower_expr_with_exception(
                            expr,
                            exception_name,
                        )?));
                    }
                }
            }
            let command = PyExpr::FString {
                parts: lowered_parts,
                span: span.clone(),
            };
            let helper = match kind {
                SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE,
                SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS,
            };
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: helper.to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: command,
                    span: span.clone(),
                }],
                span: span.clone(),
            })
        }
        Expr::Call { func, args, span } => Ok(PyExpr::Call {
            func: Box::new(lower_expr_with_exception(func, exception_name)?),
            args: args
                .iter()
                .map(|arg| lower_argument(arg, exception_name))
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::Attribute { value, attr, span } => Ok(PyExpr::Attribute {
            value: Box::new(lower_expr_with_exception(value, exception_name)?),
            attr: attr.clone(),
            span: span.clone(),
        }),
        Expr::Index { value, index, span } => Ok(PyExpr::Index {
            value: Box::new(lower_expr_with_exception(value, exception_name)?),
            index: Box::new(lower_expr_with_exception(index, exception_name)?),
            span: span.clone(),
        }),
        Expr::Paren { expr, span } => Ok(PyExpr::Paren {
            expr: Box::new(lower_expr_with_exception(expr, exception_name)?),
            span: span.clone(),
        }),
        Expr::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::List {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::Tuple {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Dict { entries, span } => {
            let mut lowered = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                lowered.push((
                    lower_expr_with_exception(key, exception_name)?,
                    lower_expr_with_exception(value, exception_name)?,
                ));
            }
            Ok(PyExpr::Dict {
                entries: lowered,
                span: span.clone(),
            })
        }
        Expr::Set { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(element, exception_name)?);
            }
            Ok(PyExpr::Set {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => {
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(cond, exception_name)?);
            }
            Ok(PyExpr::ListComp {
                element: Box::new(lower_expr_with_exception(element, exception_name)?),
                target: target.clone(),
                iter: Box::new(lower_expr_with_exception(iter, exception_name)?),
                ifs: lowered_ifs,
                span: span.clone(),
            })
        }
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => {
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(cond, exception_name)?);
            }
            Ok(PyExpr::DictComp {
                key: Box::new(lower_expr_with_exception(key, exception_name)?),
                value: Box::new(lower_expr_with_exception(value, exception_name)?),
                target: target.clone(),
                iter: Box::new(lower_expr_with_exception(iter, exception_name)?),
                ifs: lowered_ifs,
                span: span.clone(),
            })
        }
        Expr::Slice { start, end, span } => Ok(PyExpr::Slice {
            start: start
                .as_deref()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .transpose()?
                .map(Box::new),
            end: end
                .as_deref()
                .map(|expr| lower_expr_with_exception(expr, exception_name))
                .transpose()?
                .map(Box::new),
            span: span.clone(),
        }),
    }
}

fn lower_argument(arg: &Argument, exception_name: Option<&str>) -> Result<PyArgument, LowerError> {
    match arg {
        Argument::Positional { value, span } => Ok(PyArgument::Positional {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::Keyword { name, value, span } => Ok(PyArgument::Keyword {
            name: name.clone(),
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::Star { value, span } => Ok(PyArgument::Star {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
        Argument::KwStar { value, span } => Ok(PyArgument::KwStar {
            value: lower_expr_with_exception(value, exception_name)?,
            span: span.clone(),
        }),
    }
}

fn lower_parameter(param: &Parameter) -> Result<PyParameter, LowerError> {
    match param {
        Parameter::Regular {
            name,
            default,
            span,
        } => Ok(PyParameter::Regular {
            name: name.clone(),
            default: default.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Parameter::VarArgs { name, span } => Ok(PyParameter::VarArgs {
            name: name.clone(),
            span: span.clone(),
        }),
        Parameter::KwArgs { name, span } => Ok(PyParameter::KwArgs {
            name: name.clone(),
            span: span.clone(),
        }),
    }
}

fn span_from_block(block: &[PyStmt]) -> Option<SourceSpan> {
    let first = block.first()?;
    let last = block.last()?;
    Some(merge_span(stmt_span(first), stmt_span(last)))
}

fn stmt_span(stmt: &PyStmt) -> &SourceSpan {
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

fn expr_span(expr: &PyExpr) -> &SourceSpan {
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

fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start: left.start.clone(),
        end: right.end.clone(),
    }
}

fn lower_unary_op(op: UnaryOp) -> PyUnaryOp {
    match op {
        UnaryOp::Plus => PyUnaryOp::Plus,
        UnaryOp::Minus => PyUnaryOp::Minus,
        UnaryOp::Not => PyUnaryOp::Not,
    }
}

fn lower_binary_op(op: BinaryOp) -> PyBinaryOp {
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
    }
}

fn lower_compare_op(op: CompareOp) -> PyCompareOp {
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

pub fn python_source(module: &PyModule) -> String {
    let mut writer = PythonWriter::new();
    let uses_try = module_uses_snail_try(module);
    let uses_regex = module_uses_snail_regex(module);
    let uses_subprocess = module_uses_snail_subprocess(module);
    if uses_try {
        writer.write_snail_try_helper();
    }
    if uses_regex {
        if uses_try {
            writer.write_line("");
        }
        writer.write_snail_regex_helpers();
    }
    if uses_subprocess {
        if uses_try || uses_regex {
            writer.write_line("");
        }
        writer.write_snail_subprocess_helpers();
    }
    if (uses_try || uses_regex || uses_subprocess) && !module.body.is_empty() {
        writer.write_line("");
    }
    writer.write_module(module);
    writer.finish()
}

fn module_uses_snail_try(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_try)
}

fn module_uses_snail_regex(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_regex)
}

fn module_uses_snail_subprocess(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_subprocess)
}

fn stmt_uses_snail_try(stmt: &PyStmt) -> bool {
    match stmt {
        PyStmt::If {
            test, body, orelse, ..
        } => {
            expr_uses_snail_try(test) || block_uses_snail_try(body) || block_uses_snail_try(orelse)
        }
        PyStmt::While {
            test, body, orelse, ..
        } => {
            expr_uses_snail_try(test) || block_uses_snail_try(body) || block_uses_snail_try(orelse)
        }
        PyStmt::For {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            expr_uses_snail_try(target)
                || expr_uses_snail_try(iter)
                || block_uses_snail_try(body)
                || block_uses_snail_try(orelse)
        }
        PyStmt::FunctionDef { body, .. } | PyStmt::ClassDef { body, .. } => {
            block_uses_snail_try(body)
        }
        PyStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } => {
            block_uses_snail_try(body)
                || handlers.iter().any(handler_uses_snail_try)
                || block_uses_snail_try(orelse)
                || block_uses_snail_try(finalbody)
        }
        PyStmt::With { items, body, .. } => {
            items.iter().any(with_item_uses_snail_try) || block_uses_snail_try(body)
        }
        PyStmt::Return { value, .. } => value.as_ref().is_some_and(expr_uses_snail_try),
        PyStmt::Raise { value, from, .. } => {
            value.as_ref().is_some_and(expr_uses_snail_try)
                || from.as_ref().is_some_and(expr_uses_snail_try)
        }
        PyStmt::Assert { test, message, .. } => {
            expr_uses_snail_try(test) || message.as_ref().is_some_and(expr_uses_snail_try)
        }
        PyStmt::Delete { targets, .. } => targets.iter().any(expr_uses_snail_try),
        PyStmt::Import { .. }
        | PyStmt::ImportFrom { .. }
        | PyStmt::Break { .. }
        | PyStmt::Continue { .. }
        | PyStmt::Pass { .. } => false,
        PyStmt::Assign { targets, value, .. } => {
            targets.iter().any(expr_uses_snail_try) || expr_uses_snail_try(value)
        }
        PyStmt::Expr { value, .. } => expr_uses_snail_try(value),
    }
}

fn stmt_uses_snail_subprocess(stmt: &PyStmt) -> bool {
    match stmt {
        PyStmt::If {
            test, body, orelse, ..
        } => {
            expr_uses_snail_subprocess(test)
                || block_uses_snail_subprocess(body)
                || block_uses_snail_subprocess(orelse)
        }
        PyStmt::While {
            test, body, orelse, ..
        } => {
            expr_uses_snail_subprocess(test)
                || block_uses_snail_subprocess(body)
                || block_uses_snail_subprocess(orelse)
        }
        PyStmt::For {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            expr_uses_snail_subprocess(target)
                || expr_uses_snail_subprocess(iter)
                || block_uses_snail_subprocess(body)
                || block_uses_snail_subprocess(orelse)
        }
        PyStmt::FunctionDef { body, .. } | PyStmt::ClassDef { body, .. } => {
            block_uses_snail_subprocess(body)
        }
        PyStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } => {
            block_uses_snail_subprocess(body)
                || handlers.iter().any(handler_uses_snail_subprocess)
                || block_uses_snail_subprocess(orelse)
                || block_uses_snail_subprocess(finalbody)
        }
        PyStmt::With { items, body, .. } => {
            items.iter().any(with_item_uses_snail_subprocess) || block_uses_snail_subprocess(body)
        }
        PyStmt::Return { value, .. } => value.as_ref().is_some_and(expr_uses_snail_subprocess),
        PyStmt::Raise { value, from, .. } => {
            value.as_ref().is_some_and(expr_uses_snail_subprocess)
                || from.as_ref().is_some_and(expr_uses_snail_subprocess)
        }
        PyStmt::Assert { test, message, .. } => {
            expr_uses_snail_subprocess(test)
                || message.as_ref().is_some_and(expr_uses_snail_subprocess)
        }
        PyStmt::Delete { targets, .. } => targets.iter().any(expr_uses_snail_subprocess),
        PyStmt::Import { .. }
        | PyStmt::ImportFrom { .. }
        | PyStmt::Break { .. }
        | PyStmt::Continue { .. }
        | PyStmt::Pass { .. } => false,
        PyStmt::Assign { targets, value, .. } => {
            targets.iter().any(expr_uses_snail_subprocess) || expr_uses_snail_subprocess(value)
        }
        PyStmt::Expr { value, .. } => expr_uses_snail_subprocess(value),
    }
}

fn block_uses_snail_subprocess(block: &[PyStmt]) -> bool {
    block.iter().any(stmt_uses_snail_subprocess)
}

fn handler_uses_snail_subprocess(handler: &PyExceptHandler) -> bool {
    handler
        .type_name
        .as_ref()
        .is_some_and(expr_uses_snail_subprocess)
        || block_uses_snail_subprocess(&handler.body)
}

fn with_item_uses_snail_subprocess(item: &PyWithItem) -> bool {
    expr_uses_snail_subprocess(&item.context)
        || item.target.as_ref().is_some_and(expr_uses_snail_subprocess)
}

fn argument_uses_snail_subprocess(arg: &PyArgument) -> bool {
    match arg {
        PyArgument::Positional { value, .. }
        | PyArgument::Keyword { value, .. }
        | PyArgument::Star { value, .. }
        | PyArgument::KwStar { value, .. } => expr_uses_snail_subprocess(value),
    }
}

fn expr_uses_snail_subprocess(expr: &PyExpr) -> bool {
    match expr {
        PyExpr::Name { .. }
        | PyExpr::Number { .. }
        | PyExpr::String { .. }
        | PyExpr::Bool { .. }
        | PyExpr::None { .. } => false,
        PyExpr::FString { parts, .. } => parts.iter().any(|part| match part {
            PyFStringPart::Text(_) => false,
            PyFStringPart::Expr(expr) => expr_uses_snail_subprocess(expr),
        }),
        PyExpr::Unary { operand, .. } => expr_uses_snail_subprocess(operand),
        PyExpr::Binary { left, right, .. } => {
            expr_uses_snail_subprocess(left) || expr_uses_snail_subprocess(right)
        }
        PyExpr::Compare {
            left, comparators, ..
        } => expr_uses_snail_subprocess(left) || comparators.iter().any(expr_uses_snail_subprocess),
        PyExpr::IfExpr {
            test, body, orelse, ..
        } => {
            expr_uses_snail_subprocess(test)
                || expr_uses_snail_subprocess(body)
                || expr_uses_snail_subprocess(orelse)
        }
        PyExpr::Lambda { body, .. } => expr_uses_snail_subprocess(body),
        PyExpr::Call { func, args, .. } => {
            if matches!(func.as_ref(), PyExpr::Name { id, .. }
                if id == SNAIL_SUBPROCESS_CAPTURE || id == SNAIL_SUBPROCESS_STATUS)
            {
                return true;
            }
            expr_uses_snail_subprocess(func) || args.iter().any(argument_uses_snail_subprocess)
        }
        PyExpr::Attribute { value, .. } => expr_uses_snail_subprocess(value),
        PyExpr::Index { value, index, .. } => {
            expr_uses_snail_subprocess(value) || expr_uses_snail_subprocess(index)
        }
        PyExpr::Paren { expr, .. } => expr_uses_snail_subprocess(expr),
        PyExpr::List { elements, .. } | PyExpr::Tuple { elements, .. } => {
            elements.iter().any(expr_uses_snail_subprocess)
        }
        PyExpr::Dict { entries, .. } => entries.iter().any(|(key, value)| {
            expr_uses_snail_subprocess(key) || expr_uses_snail_subprocess(value)
        }),
        PyExpr::Set { elements, .. } => elements.iter().any(expr_uses_snail_subprocess),
        PyExpr::ListComp {
            element, iter, ifs, ..
        } => {
            expr_uses_snail_subprocess(element)
                || expr_uses_snail_subprocess(iter)
                || ifs.iter().any(expr_uses_snail_subprocess)
        }
        PyExpr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            expr_uses_snail_subprocess(key)
                || expr_uses_snail_subprocess(value)
                || expr_uses_snail_subprocess(iter)
                || ifs.iter().any(expr_uses_snail_subprocess)
        }
        PyExpr::Slice { start, end, .. } => {
            start.as_deref().is_some_and(expr_uses_snail_subprocess)
                || end.as_deref().is_some_and(expr_uses_snail_subprocess)
        }
    }
}

fn block_uses_snail_try(block: &[PyStmt]) -> bool {
    block.iter().any(stmt_uses_snail_try)
}

fn handler_uses_snail_try(handler: &PyExceptHandler) -> bool {
    handler.type_name.as_ref().is_some_and(expr_uses_snail_try)
        || block_uses_snail_try(&handler.body)
}

fn with_item_uses_snail_try(item: &PyWithItem) -> bool {
    expr_uses_snail_try(&item.context) || item.target.as_ref().is_some_and(expr_uses_snail_try)
}

fn argument_uses_snail_try(arg: &PyArgument) -> bool {
    match arg {
        PyArgument::Positional { value, .. }
        | PyArgument::Keyword { value, .. }
        | PyArgument::Star { value, .. }
        | PyArgument::KwStar { value, .. } => expr_uses_snail_try(value),
    }
}

fn expr_uses_snail_try(expr: &PyExpr) -> bool {
    match expr {
        PyExpr::Name { .. }
        | PyExpr::Number { .. }
        | PyExpr::String { .. }
        | PyExpr::Bool { .. }
        | PyExpr::None { .. } => false,
        PyExpr::FString { parts, .. } => parts.iter().any(|part| match part {
            PyFStringPart::Text(_) => false,
            PyFStringPart::Expr(expr) => expr_uses_snail_try(expr),
        }),
        PyExpr::Unary { operand, .. } => expr_uses_snail_try(operand),
        PyExpr::Binary { left, right, .. } => {
            expr_uses_snail_try(left) || expr_uses_snail_try(right)
        }
        PyExpr::Compare {
            left, comparators, ..
        } => expr_uses_snail_try(left) || comparators.iter().any(expr_uses_snail_try),
        PyExpr::IfExpr {
            test, body, orelse, ..
        } => expr_uses_snail_try(test) || expr_uses_snail_try(body) || expr_uses_snail_try(orelse),
        PyExpr::Lambda { body, .. } => expr_uses_snail_try(body),
        PyExpr::Call { func, args, .. } => {
            if matches!(func.as_ref(), PyExpr::Name { id, .. } if id == SNAIL_TRY_HELPER) {
                return true;
            }
            expr_uses_snail_try(func) || args.iter().any(argument_uses_snail_try)
        }
        PyExpr::Attribute { value, .. } => expr_uses_snail_try(value),
        PyExpr::Index { value, index, .. } => {
            expr_uses_snail_try(value) || expr_uses_snail_try(index)
        }
        PyExpr::Paren { expr, .. } => expr_uses_snail_try(expr),
        PyExpr::List { elements, .. } | PyExpr::Tuple { elements, .. } => {
            elements.iter().any(expr_uses_snail_try)
        }
        PyExpr::Dict { entries, .. } => entries
            .iter()
            .any(|(key, value)| expr_uses_snail_try(key) || expr_uses_snail_try(value)),
        PyExpr::Set { elements, .. } => elements.iter().any(expr_uses_snail_try),
        PyExpr::ListComp {
            element, iter, ifs, ..
        } => {
            expr_uses_snail_try(element)
                || expr_uses_snail_try(iter)
                || ifs.iter().any(expr_uses_snail_try)
        }
        PyExpr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            expr_uses_snail_try(key)
                || expr_uses_snail_try(value)
                || expr_uses_snail_try(iter)
                || ifs.iter().any(expr_uses_snail_try)
        }
        PyExpr::Slice { start, end, .. } => {
            start.as_deref().is_some_and(expr_uses_snail_try)
                || end.as_deref().is_some_and(expr_uses_snail_try)
        }
    }
}

fn stmt_uses_snail_regex(stmt: &PyStmt) -> bool {
    match stmt {
        PyStmt::If {
            test, body, orelse, ..
        } => {
            expr_uses_snail_regex(test)
                || block_uses_snail_regex(body)
                || block_uses_snail_regex(orelse)
        }
        PyStmt::While {
            test, body, orelse, ..
        } => {
            expr_uses_snail_regex(test)
                || block_uses_snail_regex(body)
                || block_uses_snail_regex(orelse)
        }
        PyStmt::For {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            expr_uses_snail_regex(target)
                || expr_uses_snail_regex(iter)
                || block_uses_snail_regex(body)
                || block_uses_snail_regex(orelse)
        }
        PyStmt::FunctionDef { body, .. } | PyStmt::ClassDef { body, .. } => {
            block_uses_snail_regex(body)
        }
        PyStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } => {
            block_uses_snail_regex(body)
                || handlers.iter().any(handler_uses_snail_regex)
                || block_uses_snail_regex(orelse)
                || block_uses_snail_regex(finalbody)
        }
        PyStmt::With { items, body, .. } => {
            items.iter().any(with_item_uses_snail_regex) || block_uses_snail_regex(body)
        }
        PyStmt::Return { value, .. } => value.as_ref().is_some_and(expr_uses_snail_regex),
        PyStmt::Raise { value, from, .. } => {
            value.as_ref().is_some_and(expr_uses_snail_regex)
                || from.as_ref().is_some_and(expr_uses_snail_regex)
        }
        PyStmt::Assert { test, message, .. } => {
            expr_uses_snail_regex(test) || message.as_ref().is_some_and(expr_uses_snail_regex)
        }
        PyStmt::Delete { targets, .. } => targets.iter().any(expr_uses_snail_regex),
        PyStmt::Import { .. }
        | PyStmt::ImportFrom { .. }
        | PyStmt::Break { .. }
        | PyStmt::Continue { .. }
        | PyStmt::Pass { .. } => false,
        PyStmt::Assign { targets, value, .. } => {
            targets.iter().any(expr_uses_snail_regex) || expr_uses_snail_regex(value)
        }
        PyStmt::Expr { value, .. } => expr_uses_snail_regex(value),
    }
}

fn block_uses_snail_regex(block: &[PyStmt]) -> bool {
    block.iter().any(stmt_uses_snail_regex)
}

fn handler_uses_snail_regex(handler: &PyExceptHandler) -> bool {
    handler
        .type_name
        .as_ref()
        .is_some_and(expr_uses_snail_regex)
        || block_uses_snail_regex(&handler.body)
}

fn with_item_uses_snail_regex(item: &PyWithItem) -> bool {
    expr_uses_snail_regex(&item.context) || item.target.as_ref().is_some_and(expr_uses_snail_regex)
}

fn argument_uses_snail_regex(arg: &PyArgument) -> bool {
    match arg {
        PyArgument::Positional { value, .. }
        | PyArgument::Keyword { value, .. }
        | PyArgument::Star { value, .. }
        | PyArgument::KwStar { value, .. } => expr_uses_snail_regex(value),
    }
}

fn expr_uses_snail_regex(expr: &PyExpr) -> bool {
    match expr {
        PyExpr::Name { .. }
        | PyExpr::Number { .. }
        | PyExpr::String { .. }
        | PyExpr::Bool { .. }
        | PyExpr::None { .. } => false,
        PyExpr::FString { parts, .. } => parts.iter().any(|part| match part {
            PyFStringPart::Text(_) => false,
            PyFStringPart::Expr(expr) => expr_uses_snail_regex(expr),
        }),
        PyExpr::Unary { operand, .. } => expr_uses_snail_regex(operand),
        PyExpr::Binary { left, right, .. } => {
            expr_uses_snail_regex(left) || expr_uses_snail_regex(right)
        }
        PyExpr::Compare {
            left, comparators, ..
        } => expr_uses_snail_regex(left) || comparators.iter().any(expr_uses_snail_regex),
        PyExpr::IfExpr {
            test, body, orelse, ..
        } => {
            expr_uses_snail_regex(test)
                || expr_uses_snail_regex(body)
                || expr_uses_snail_regex(orelse)
        }
        PyExpr::Lambda { body, .. } => expr_uses_snail_regex(body),
        PyExpr::Call { func, args, .. } => {
            if matches!(func.as_ref(), PyExpr::Name { id, .. }
                if id == SNAIL_REGEX_SEARCH || id == SNAIL_REGEX_COMPILE)
            {
                return true;
            }
            expr_uses_snail_regex(func) || args.iter().any(argument_uses_snail_regex)
        }
        PyExpr::Attribute { value, .. } => expr_uses_snail_regex(value),
        PyExpr::Index { value, index, .. } => {
            expr_uses_snail_regex(value) || expr_uses_snail_regex(index)
        }
        PyExpr::Paren { expr, .. } => expr_uses_snail_regex(expr),
        PyExpr::List { elements, .. } | PyExpr::Tuple { elements, .. } => {
            elements.iter().any(expr_uses_snail_regex)
        }
        PyExpr::Dict { entries, .. } => entries
            .iter()
            .any(|(key, value)| expr_uses_snail_regex(key) || expr_uses_snail_regex(value)),
        PyExpr::Set { elements, .. } => elements.iter().any(expr_uses_snail_regex),
        PyExpr::ListComp {
            element, iter, ifs, ..
        } => {
            expr_uses_snail_regex(element)
                || expr_uses_snail_regex(iter)
                || ifs.iter().any(expr_uses_snail_regex)
        }
        PyExpr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            expr_uses_snail_regex(key)
                || expr_uses_snail_regex(value)
                || expr_uses_snail_regex(iter)
                || ifs.iter().any(expr_uses_snail_regex)
        }
        PyExpr::Slice { start, end, .. } => {
            start.as_deref().is_some_and(expr_uses_snail_regex)
                || end.as_deref().is_some_and(expr_uses_snail_regex)
        }
    }
}

struct PythonWriter {
    output: String,
    indent: usize,
}

impl PythonWriter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn finish(self) -> String {
        self.output
    }

    fn write_module(&mut self, module: &PyModule) {
        for stmt in &module.body {
            self.write_stmt(stmt);
        }
    }

    fn write_snail_try_helper(&mut self) {
        self.write_line(&format!(
            "def {}(expr_fn, fallback_fn=None):",
            SNAIL_TRY_HELPER
        ));
        self.indent += 1;
        self.write_line("try:");
        self.indent += 1;
        self.write_line("return expr_fn()");
        self.indent -= 1;
        self.write_line(&format!("except Exception as {}:", SNAIL_EXCEPTION_VAR));
        self.indent += 1;
        self.write_line("if fallback_fn is None:");
        self.indent += 1;
        self.write_line(&format!(
            "fallback_member = getattr({}, \"__fallback__\", None)",
            SNAIL_EXCEPTION_VAR
        ));
        self.write_line("if callable(fallback_member):");
        self.indent += 1;
        self.write_line("return fallback_member()");
        self.indent -= 1;
        self.write_line(&format!("return {}", SNAIL_EXCEPTION_VAR));
        self.indent -= 1;
        self.write_line(&format!("return fallback_fn({})", SNAIL_EXCEPTION_VAR));
        self.indent -= 2;
    }

    fn write_snail_regex_helpers(&mut self) {
        self.write_line("import re");
        self.write_line("");
        self.write_line(&format!("def {}(value, pattern):", SNAIL_REGEX_SEARCH));
        self.indent += 1;
        self.write_line("return re.search(pattern, value)");
        self.indent -= 1;
        self.write_line("");
        self.write_line(&format!("def {}(pattern):", SNAIL_REGEX_COMPILE));
        self.indent += 1;
        self.write_line("return re.compile(pattern)");
        self.indent -= 1;
    }

    fn write_snail_subprocess_helpers(&mut self) {
        self.write_line("import subprocess");
        self.write_line("");
        self.write_line(&format!("def {}(cmd):", SNAIL_SUBPROCESS_CAPTURE));
        self.indent += 1;
        self.write_line("try:");
        self.indent += 1;
        self.write_line(
            "completed = subprocess.run(cmd, shell=True, check=True, text=True, stdout=subprocess.PIPE)",
        );
        self.write_line("return completed.stdout.strip()");
        self.indent -= 1;
        self.write_line("except subprocess.CalledProcessError as exc:");
        self.indent += 1;
        self.write_line("def __fallback(exc=exc):");
        self.indent += 1;
        self.write_line("raise exc");
        self.indent -= 1;
        self.write_line("exc.__fallback__ = __fallback");
        self.write_line("raise");
        self.indent -= 2;
        self.write_line("");
        self.write_line(&format!("def {}(cmd):", SNAIL_SUBPROCESS_STATUS));
        self.indent += 1;
        self.write_line("try:");
        self.indent += 1;
        self.write_line("subprocess.run(cmd, shell=True, check=True)");
        self.write_line("return 0");
        self.indent -= 1;
        self.write_line("except subprocess.CalledProcessError as exc:");
        self.indent += 1;
        self.write_line("def __fallback(exc=exc):");
        self.indent += 1;
        self.write_line("return exc.returncode");
        self.indent -= 1;
        self.write_line("exc.__fallback__ = __fallback");
        self.write_line("raise");
        self.indent -= 2;
    }

    fn write_stmt(&mut self, stmt: &PyStmt) {
        match stmt {
            PyStmt::If {
                test, body, orelse, ..
            } => self.write_if_chain(test, body, orelse),
            PyStmt::While {
                test, body, orelse, ..
            } => {
                self.write_line(&format!("while {}:", expr_source(test)));
                self.write_suite(body);
                self.write_else_block(orelse);
            }
            PyStmt::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.write_line(&format!(
                    "for {} in {}:",
                    expr_source(target),
                    expr_source(iter)
                ));
                self.write_suite(body);
                self.write_else_block(orelse);
            }
            PyStmt::FunctionDef {
                name, args, body, ..
            } => {
                let args = args.iter().map(param_source).collect::<Vec<_>>().join(", ");
                self.write_line(&format!("def {}({}):", name, args));
                self.write_suite(body);
            }
            PyStmt::ClassDef { name, body, .. } => {
                self.write_line(&format!("class {}:", name));
                self.write_suite(body);
            }
            PyStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                self.write_line("try:");
                self.write_suite(body);
                for handler in handlers {
                    self.write_except(handler);
                }
                if !orelse.is_empty() {
                    self.write_line("else:");
                    self.write_suite(orelse);
                }
                if !finalbody.is_empty() {
                    self.write_line("finally:");
                    self.write_suite(finalbody);
                }
            }
            PyStmt::With { items, body, .. } => {
                let items = items
                    .iter()
                    .map(with_item_source)
                    .collect::<Vec<_>>()
                    .join(", ");
                self.write_line(&format!("with {}:", items));
                self.write_suite(body);
            }
            PyStmt::Return { value, .. } => match value {
                Some(expr) => self.write_line(&format!("return {}", expr_source(expr))),
                None => self.write_line("return"),
            },
            PyStmt::Raise { value, from, .. } => match (value, from) {
                (Some(expr), Some(from_expr)) => self.write_line(&format!(
                    "raise {} from {}",
                    expr_source(expr),
                    expr_source(from_expr)
                )),
                (Some(expr), None) => self.write_line(&format!("raise {}", expr_source(expr))),
                (None, _) => self.write_line("raise"),
            },
            PyStmt::Assert { test, message, .. } => match message {
                Some(expr) => self.write_line(&format!(
                    "assert {}, {}",
                    expr_source(test),
                    expr_source(expr)
                )),
                None => self.write_line(&format!("assert {}", expr_source(test))),
            },
            PyStmt::Delete { targets, .. } => {
                let targets = targets
                    .iter()
                    .map(expr_source)
                    .collect::<Vec<_>>()
                    .join(", ");
                self.write_line(&format!("del {}", targets));
            }
            PyStmt::Break { .. } => self.write_line("break"),
            PyStmt::Continue { .. } => self.write_line("continue"),
            PyStmt::Pass { .. } => self.write_line("pass"),
            PyStmt::Import { names, .. } => {
                let items = names.iter().map(import_name).collect::<Vec<_>>().join(", ");
                self.write_line(&format!("import {}", items));
            }
            PyStmt::ImportFrom { module, names, .. } => {
                let module = module.join(".");
                let items = names.iter().map(import_name).collect::<Vec<_>>().join(", ");
                self.write_line(&format!("from {} import {}", module, items));
            }
            PyStmt::Assign { targets, value, .. } => {
                let mut line = targets
                    .iter()
                    .map(expr_source)
                    .collect::<Vec<_>>()
                    .join(" = ");
                line.push_str(" = ");
                line.push_str(&expr_source(value));
                self.write_line(&line);
            }
            PyStmt::Expr { value, .. } => self.write_line(&expr_source(value)),
        }
    }

    fn write_if_chain(&mut self, test: &PyExpr, body: &[PyStmt], orelse: &[PyStmt]) {
        self.write_line(&format!("if {}:", expr_source(test)));
        self.write_suite(body);
        self.write_elif_or_else(orelse);
    }

    fn write_elif_or_else(&mut self, orelse: &[PyStmt]) {
        if orelse.is_empty() {
            return;
        }
        if orelse.len() == 1
            && let PyStmt::If {
                test,
                body,
                orelse: nested_orelse,
                ..
            } = &orelse[0]
        {
            self.write_line(&format!("elif {}:", expr_source(test)));
            self.write_suite(body);
            self.write_elif_or_else(nested_orelse);
            return;
        }
        self.write_line("else:");
        self.write_suite(orelse);
    }

    fn write_else_block(&mut self, orelse: &[PyStmt]) {
        if !orelse.is_empty() {
            self.write_line("else:");
            self.write_suite(orelse);
        }
    }

    fn write_suite(&mut self, suite: &[PyStmt]) {
        self.indent += 1;
        if suite.is_empty() {
            self.write_line("pass");
        } else {
            for stmt in suite {
                self.write_stmt(stmt);
            }
        }
        self.indent -= 1;
    }

    fn write_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        let _ = writeln!(self.output, "{}", line);
    }

    fn write_except(&mut self, handler: &PyExceptHandler) {
        let header = match (&handler.type_name, &handler.name) {
            (Some(type_name), Some(name)) => {
                format!("except {} as {}:", expr_source(type_name), name)
            }
            (Some(type_name), None) => format!("except {}:", expr_source(type_name)),
            (None, _) => "except:".to_string(),
        };
        self.write_line(&header);
        self.write_suite(&handler.body);
    }
}

fn expr_source(expr: &PyExpr) -> String {
    match expr {
        PyExpr::Name { id, .. } => id.clone(),
        PyExpr::Number { value, .. } => value.clone(),
        PyExpr::String {
            value,
            raw,
            delimiter,
            ..
        } => format_string_literal(value, *raw, *delimiter),
        PyExpr::FString { parts, .. } => format_f_string(parts),
        PyExpr::Bool { value, .. } => {
            if *value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        PyExpr::None { .. } => "None".to_string(),
        PyExpr::Unary { op, operand, .. } => match op {
            PyUnaryOp::Plus => format!("+{}", expr_source(operand)),
            PyUnaryOp::Minus => format!("-{}", expr_source(operand)),
            PyUnaryOp::Not => format!("not {}", expr_source(operand)),
        },
        PyExpr::Binary {
            left, op, right, ..
        } => format!(
            "({} {} {})",
            expr_source(left),
            binary_op(*op),
            expr_source(right)
        ),
        PyExpr::Compare {
            left,
            ops,
            comparators,
            ..
        } => {
            let mut parts = Vec::new();
            parts.push(expr_source(left));
            for (op, comparator) in ops.iter().zip(comparators) {
                parts.push(compare_op(*op).to_string());
                parts.push(expr_source(comparator));
            }
            format!("({})", parts.join(" "))
        }
        PyExpr::IfExpr {
            test, body, orelse, ..
        } => format!(
            "({} if {} else {})",
            expr_source(body),
            expr_source(test),
            expr_source(orelse)
        ),
        PyExpr::Lambda { params, body, .. } => {
            if params.is_empty() {
                format!("lambda: {}", expr_source(body))
            } else {
                let params = params.join(", ");
                format!("lambda {params}: {}", expr_source(body))
            }
        }
        PyExpr::Call { func, args, .. } => {
            let args = args
                .iter()
                .map(argument_source)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", expr_source(func), args)
        }
        PyExpr::Attribute { value, attr, .. } => format!("{}.{}", expr_source(value), attr),
        PyExpr::Index { value, index, .. } => {
            format!("{}[{}]", expr_source(value), expr_source(index))
        }
        PyExpr::Paren { expr, .. } => format!("({})", expr_source(expr)),
        PyExpr::List { elements, .. } => {
            let items = elements
                .iter()
                .map(expr_source)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", items)
        }
        PyExpr::Tuple { elements, .. } => {
            if elements.is_empty() {
                return "()".to_string();
            }
            let items = elements
                .iter()
                .map(expr_source)
                .collect::<Vec<_>>()
                .join(", ");
            if elements.len() == 1 {
                format!("({},)", items)
            } else {
                format!("({})", items)
            }
        }
        PyExpr::Dict { entries, .. } => {
            let items = entries
                .iter()
                .map(|(key, value)| format!("{}: {}", expr_source(key), expr_source(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{}}}", items)
        }
        PyExpr::Set { elements, .. } => {
            let items = elements
                .iter()
                .map(expr_source)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{}}}", items)
        }
        PyExpr::ListComp {
            element,
            target,
            iter,
            ifs,
            ..
        } => {
            let tail = comp_for_source(target, iter, ifs);
            format!("[{}{}]", expr_source(element), tail)
        }
        PyExpr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            ..
        } => {
            let tail = comp_for_source(target, iter, ifs);
            format!("{{{}: {}{}}}", expr_source(key), expr_source(value), tail)
        }
        PyExpr::Slice { start, end, .. } => {
            let start = start
                .as_ref()
                .map(|expr| expr_source(expr))
                .unwrap_or_default();
            let end = end
                .as_ref()
                .map(|expr| expr_source(expr))
                .unwrap_or_default();
            format!("{start}:{end}")
        }
    }
}

fn comp_for_source(target: &str, iter: &PyExpr, ifs: &[PyExpr]) -> String {
    let mut out = format!(" for {} in {}", target, expr_source(iter));
    for cond in ifs {
        out.push_str(" if ");
        out.push_str(&expr_source(cond));
    }
    out
}

fn import_name(name: &PyImportName) -> String {
    let mut item = name.name.join(".");
    if let Some(alias) = &name.asname {
        item.push_str(&format!(" as {}", alias));
    }
    item
}

fn param_source(param: &PyParameter) -> String {
    match param {
        PyParameter::Regular { name, default, .. } => match default {
            Some(expr) => format!("{}={}", name, expr_source(expr)),
            None => name.clone(),
        },
        PyParameter::VarArgs { name, .. } => format!("*{}", name),
        PyParameter::KwArgs { name, .. } => format!("**{}", name),
    }
}

fn argument_source(arg: &PyArgument) -> String {
    match arg {
        PyArgument::Positional { value, .. } => expr_source(value),
        PyArgument::Keyword { name, value, .. } => format!("{}={}", name, expr_source(value)),
        PyArgument::Star { value, .. } => format!("*{}", expr_source(value)),
        PyArgument::KwStar { value, .. } => format!("**{}", expr_source(value)),
    }
}

fn with_item_source(item: &PyWithItem) -> String {
    let mut out = expr_source(&item.context);
    if let Some(target) = &item.target {
        out.push_str(" as ");
        out.push_str(&expr_source(target));
    }
    out
}

fn format_string_literal(value: &str, raw: bool, delimiter: StringDelimiter) -> String {
    let (open, close) = match delimiter {
        StringDelimiter::Single => ("'", "'"),
        StringDelimiter::Double => ("\"", "\""),
        StringDelimiter::TripleSingle => ("'''", "'''"),
        StringDelimiter::TripleDouble => ("\"\"\"", "\"\"\""),
    };
    let prefix = if raw { "r" } else { "f" };
    format!("{prefix}{open}{value}{close}")
}

fn format_f_string(parts: &[PyFStringPart]) -> String {
    let mut out = String::new();
    for part in parts {
        match part {
            PyFStringPart::Text(text) => out.push_str(&escape_f_string_text(text)),
            PyFStringPart::Expr(expr) => {
                out.push('{');
                out.push_str(&expr_source(expr));
                out.push('}');
            }
        }
    }
    format!("f\"{}\"", out)
}

fn escape_f_string_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '{' => escaped.push_str("{{"),
            '}' => escaped.push_str("}}"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn binary_op(op: PyBinaryOp) -> &'static str {
    match op {
        PyBinaryOp::Or => "or",
        PyBinaryOp::And => "and",
        PyBinaryOp::Add => "+",
        PyBinaryOp::Sub => "-",
        PyBinaryOp::Mul => "*",
        PyBinaryOp::Div => "/",
        PyBinaryOp::FloorDiv => "//",
        PyBinaryOp::Mod => "%",
        PyBinaryOp::Pow => "**",
    }
}

fn compare_op(op: PyCompareOp) -> &'static str {
    match op {
        PyCompareOp::Eq => "==",
        PyCompareOp::NotEq => "!=",
        PyCompareOp::Lt => "<",
        PyCompareOp::LtEq => "<=",
        PyCompareOp::Gt => ">",
        PyCompareOp::GtEq => ">=",
        PyCompareOp::In => "in",
        PyCompareOp::Is => "is",
    }
}
