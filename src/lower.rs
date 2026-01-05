use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use crate::ast::*;
use crate::awk::{AwkProgram, AwkRule};

const SNAIL_TRY_HELPER: &str = "__snail_compact_try";
const SNAIL_EXCEPTION_VAR: &str = "__snail_compact_exc";
const SNAIL_SUBPROCESS_CAPTURE_CLASS: &str = "__SnailSubprocessCapture";
const SNAIL_SUBPROCESS_STATUS_CLASS: &str = "__SnailSubprocessStatus";
const SNAIL_REGEX_SEARCH: &str = "__snail_regex_search";
const SNAIL_REGEX_COMPILE: &str = "__snail_regex_compile";
const SNAIL_STRUCTURED_ACCESSOR_CLASS: &str = "__SnailStructuredAccessor";
const SNAIL_JSON_OBJECT_CLASS: &str = "__SnailJsonObject";
const SNAIL_AWK_LINE: &str = "$l";
const SNAIL_AWK_FIELDS: &str = "$f";
const SNAIL_AWK_NR: &str = "$n";
const SNAIL_AWK_FNR: &str = "$fn";
const SNAIL_AWK_PATH: &str = "$p";
const SNAIL_AWK_MATCH: &str = "$m";
const SNAIL_AWK_LINE_PYVAR: &str = "__snail_line";
const SNAIL_AWK_FIELDS_PYVAR: &str = "__snail_fields";
const SNAIL_AWK_NR_PYVAR: &str = "__snail_nr_user";
const SNAIL_AWK_FNR_PYVAR: &str = "__snail_fnr_user";
const SNAIL_AWK_PATH_PYVAR: &str = "__snail_path_user";
const SNAIL_AWK_MATCH_PYVAR: &str = "__snail_match";

// Vendored jmespath library
const JMESPATH_EXCEPTIONS: &str = include_str!("../vendored/jmespath/exceptions.py");
const JMESPATH_COMPAT: &str = include_str!("../vendored/jmespath/compat.py");
const JMESPATH_AST: &str = include_str!("../vendored/jmespath/ast.py");
const JMESPATH_LEXER: &str = include_str!("../vendored/jmespath/lexer.py");
const JMESPATH_FUNCTIONS: &str = include_str!("../vendored/jmespath/functions.py");
const JMESPATH_VISITOR: &str = include_str!("../vendored/jmespath/visitor.py");
const JMESPATH_PARSER: &str = include_str!("../vendored/jmespath/parser.py");
const JMESPATH_INIT: &str = include_str!("../vendored/jmespath/__init__.py");

fn injected_py_name(name: &str) -> Option<&'static str> {
    match name {
        SNAIL_AWK_LINE => Some(SNAIL_AWK_LINE_PYVAR),
        SNAIL_AWK_FIELDS => Some(SNAIL_AWK_FIELDS_PYVAR),
        SNAIL_AWK_NR => Some(SNAIL_AWK_NR_PYVAR),
        SNAIL_AWK_FNR => Some(SNAIL_AWK_FNR_PYVAR),
        SNAIL_AWK_PATH => Some(SNAIL_AWK_PATH_PYVAR),
        SNAIL_AWK_MATCH => Some(SNAIL_AWK_MATCH_PYVAR),
        _ => None,
    }
}

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
    lower_awk_program_with_auto_print(program, false)
}

pub fn lower_awk_program_with_auto_print(
    program: &AwkProgram,
    auto_print: bool,
) -> Result<PyModule, LowerError> {
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
        let lowered = lower_block(block)?;
        main_body.extend(wrap_block_with_auto_print(lowered, auto_print));
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

    let file_loop = lower_awk_file_loop_with_auto_print(program, &span, auto_print)?;
    main_body.push(PyStmt::For {
        target: name_expr("__snail_path", &span),
        iter: files_expr,
        body: file_loop,
        orelse: Vec::new(),
        span: span.clone(),
    });

    for block in &program.end_blocks {
        let lowered = lower_block(block)?;
        main_body.extend(wrap_block_with_auto_print(lowered, auto_print));
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
        } => {
            if module.len() == 1 && module[0] == "__future__" {
                let filtered: Vec<&ImportItem> = items
                    .iter()
                    .filter(|item| !(item.name.len() == 1 && item.name[0] == "braces"))
                    .collect();
                if filtered.is_empty() {
                    return Ok(PyStmt::Pass { span: span.clone() });
                }
                return Ok(PyStmt::ImportFrom {
                    module: module.clone(),
                    names: filtered
                        .iter()
                        .map(|item| lower_import_name(item))
                        .collect(),
                    span: span.clone(),
                });
            }
            Ok(PyStmt::ImportFrom {
                module: module.clone(),
                names: items.iter().map(lower_import_name).collect(),
                span: span.clone(),
            })
        }
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
        Stmt::Expr {
            value,
            semicolon_terminated,
            span,
        } => Ok(PyStmt::Expr {
            value: lower_expr(value)?,
            semicolon_terminated: *semicolon_terminated,
            span: span.clone(),
        }),
    }
}

fn lower_awk_file_loop_with_auto_print(
    program: &AwkProgram,
    span: &SourceSpan,
    auto_print: bool,
) -> Result<Vec<PyStmt>, LowerError> {
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
        lower_awk_line_loop_with_auto_print(
            program,
            span,
            name_expr("__snail_file", span),
            auto_print,
        )?,
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
        body: vec![lower_awk_line_loop_with_auto_print(
            program,
            span,
            name_expr("__snail_file", span),
            auto_print,
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

fn lower_awk_line_loop_with_auto_print(
    program: &AwkProgram,
    span: &SourceSpan,
    iter: PyExpr,
    auto_print: bool,
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
    loop_body.push(assign_name(SNAIL_AWK_LINE_PYVAR, rstrip_call, span));

    let split_call = PyExpr::Call {
        func: Box::new(PyExpr::Attribute {
            value: Box::new(name_expr(SNAIL_AWK_LINE_PYVAR, span)),
            attr: "split".to_string(),
            span: span.clone(),
        }),
        args: Vec::new(),
        span: span.clone(),
    };
    loop_body.push(assign_name(SNAIL_AWK_FIELDS_PYVAR, split_call, span));
    loop_body.push(assign_name(
        SNAIL_AWK_NR_PYVAR,
        name_expr("__snail_nr", span),
        span,
    ));
    loop_body.push(assign_name(
        SNAIL_AWK_FNR_PYVAR,
        name_expr("__snail_fnr", span),
        span,
    ));
    loop_body.push(assign_name(
        SNAIL_AWK_PATH_PYVAR,
        name_expr("__snail_path", span),
        span,
    ));

    loop_body.extend(lower_awk_rules_with_auto_print(&program.rules, auto_print)?);

    Ok(PyStmt::For {
        target: name_expr("__snail_raw", span),
        iter,
        body: loop_body,
        orelse: Vec::new(),
        span: span.clone(),
    })
}

fn wrap_block_with_auto_print(mut block: Vec<PyStmt>, auto_print: bool) -> Vec<PyStmt> {
    if !auto_print || block.is_empty() {
        return block;
    }

    let last_idx = block.len() - 1;
    if let PyStmt::Expr {
        value,
        semicolon_terminated,
        span,
    } = &block[last_idx]
    {
        // Don't auto-print if the statement was explicitly terminated with a semicolon
        if *semicolon_terminated {
            return block;
        }

        // Clone the data before modifying the block
        let expr_code = value.clone();
        let span = span.clone();

        block.pop(); // Remove the original expression statement

        block.push(PyStmt::Assign {
            targets: vec![PyExpr::Name {
                id: "__snail_last_result".to_string(),
                span: span.clone(),
            }],
            value: expr_code,
            span: span.clone(),
        });

        // Build: isinstance(__snail_last_result, str)
        let is_string = PyExpr::Call {
            func: Box::new(PyExpr::Name {
                id: "isinstance".to_string(),
                span: span.clone(),
            }),
            args: vec![
                PyArgument::Positional {
                    value: PyExpr::Name {
                        id: "__snail_last_result".to_string(),
                        span: span.clone(),
                    },
                    span: span.clone(),
                },
                PyArgument::Positional {
                    value: PyExpr::Name {
                        id: "str".to_string(),
                        span: span.clone(),
                    },
                    span: span.clone(),
                },
            ],
            span: span.clone(),
        };

        // Build: __snail_last_result is not None
        let is_not_none = PyExpr::Compare {
            left: Box::new(PyExpr::Name {
                id: "__snail_last_result".to_string(),
                span: span.clone(),
            }),
            ops: vec![PyCompareOp::Is],
            comparators: vec![PyExpr::Unary {
                op: PyUnaryOp::Not,
                operand: Box::new(PyExpr::None { span: span.clone() }),
                span: span.clone(),
            }],
            span: span.clone(),
        };

        // if isinstance(__snail_last_result, str): print(__snail_last_result)
        // elif __snail_last_result is not None: pprint.pprint(__snail_last_result)
        block.push(PyStmt::If {
            test: is_string,
            body: vec![PyStmt::Expr {
                value: PyExpr::Call {
                    func: Box::new(PyExpr::Name {
                        id: "print".to_string(),
                        span: span.clone(),
                    }),
                    args: vec![PyArgument::Positional {
                        value: PyExpr::Name {
                            id: "__snail_last_result".to_string(),
                            span: span.clone(),
                        },
                        span: span.clone(),
                    }],
                    span: span.clone(),
                },
                semicolon_terminated: false,
                span: span.clone(),
            }],
            orelse: vec![PyStmt::If {
                test: is_not_none,
                body: vec![
                    PyStmt::Import {
                        names: vec![PyImportName {
                            name: vec!["pprint".to_string()],
                            asname: None,
                            span: span.clone(),
                        }],
                        span: span.clone(),
                    },
                    PyStmt::Expr {
                        value: PyExpr::Call {
                            func: Box::new(PyExpr::Attribute {
                                value: Box::new(PyExpr::Name {
                                    id: "pprint".to_string(),
                                    span: span.clone(),
                                }),
                                attr: "pprint".to_string(),
                                span: span.clone(),
                            }),
                            args: vec![PyArgument::Positional {
                                value: PyExpr::Name {
                                    id: "__snail_last_result".to_string(),
                                    span: span.clone(),
                                },
                                span: span.clone(),
                            }],
                            span: span.clone(),
                        },
                        semicolon_terminated: false,
                        span: span.clone(),
                    },
                ],
                orelse: Vec::new(),
                span: span.clone(),
            }],
            span: span.clone(),
        });
    }

    block
}

fn lower_awk_rules_with_auto_print(
    rules: &[AwkRule],
    auto_print: bool,
) -> Result<Vec<PyStmt>, LowerError> {
    let mut stmts = Vec::new();
    for rule in rules {
        let mut action = if rule.has_explicit_action() {
            let lowered = lower_block(rule.action.as_ref().unwrap())?;
            wrap_block_with_auto_print(lowered, auto_print)
        } else {
            // Bare pattern with no action - default to printing the line
            vec![awk_default_print(&rule.span)]
        };

        if let Some(pattern) = &rule.pattern {
            if let Some((value_expr, regex, span)) = regex_pattern_components(pattern) {
                let match_call = lower_regex_match(&value_expr, &regex, &span, None)?;
                stmts.push(PyStmt::Assign {
                    targets: vec![name_expr(SNAIL_AWK_MATCH_PYVAR, &span)],
                    value: match_call,
                    span: span.clone(),
                });
                stmts.push(PyStmt::If {
                    test: name_expr(SNAIL_AWK_MATCH_PYVAR, &span),
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

fn regex_pattern_components(pattern: &Expr) -> Option<(Expr, RegexPattern, SourceSpan)> {
    match pattern {
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Some((*value.clone(), pattern.clone(), span.clone())),
        Expr::Regex { pattern, span } => Some((
            Expr::Name {
                name: SNAIL_AWK_LINE.to_string(),
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
            args: vec![pos_arg(name_expr(SNAIL_AWK_LINE_PYVAR, span), span)],
            span: span.clone(),
        },
        semicolon_terminated: false,
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
    pattern: &RegexPattern,
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
            pos_arg(
                lower_regex_pattern_expr(pattern, span, exception_name)?,
                span,
            ),
        ],
        span: span.clone(),
    })
}

fn lower_regex_pattern_expr(
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyExpr, LowerError> {
    match pattern {
        RegexPattern::Literal(text) => Ok(regex_pattern_expr(text, span)),
        RegexPattern::Interpolated(parts) => Ok(PyExpr::FString {
            parts: lower_fstring_parts(parts, exception_name)?,
            span: span.clone(),
        }),
    }
}

fn lower_fstring_parts(
    parts: &[FStringPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyFStringPart>, LowerError> {
    let mut lowered = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => lowered.push(PyFStringPart::Text(text.clone())),
            FStringPart::Expr(expr) => {
                lowered.push(PyFStringPart::Expr(lower_expr_with_exception(
                    expr,
                    exception_name,
                )?));
            }
        }
    }
    Ok(lowered)
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
            if let Some(py_name) = injected_py_name(name) {
                return Ok(PyExpr::Name {
                    id: py_name.to_string(),
                    span: span.clone(),
                });
            }
            Ok(PyExpr::Name {
                id: name.clone(),
                span: span.clone(),
            })
        }
        Expr::FieldIndex { index, span } => {
            // AWK convention: $0 is the whole line, $1 is first field, etc.
            if index == "0" {
                Ok(PyExpr::Name {
                    id: SNAIL_AWK_LINE_PYVAR.to_string(),
                    span: span.clone(),
                })
            } else {
                // Parse index and convert from 1-based to 0-based
                let field_index = index.parse::<i32>().map_err(|_| LowerError {
                    message: format!("Invalid field index: ${}", index),
                })?;
                let python_index = field_index - 1;

                Ok(PyExpr::Index {
                    value: Box::new(PyExpr::Name {
                        id: SNAIL_AWK_FIELDS_PYVAR.to_string(),
                        span: span.clone(),
                    }),
                    index: Box::new(PyExpr::Number {
                        value: python_index.to_string(),
                        span: span.clone(),
                    }),
                    span: span.clone(),
                })
            }
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
        Expr::FString { parts, span } => Ok(PyExpr::FString {
            parts: lower_fstring_parts(parts, exception_name)?,
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
        } => {
            if *op == BinaryOp::Pipeline {
                // Pipeline: x | y becomes y.__pipeline__(x)
                let left_expr = lower_expr_with_exception(left, exception_name)?;

                // Special handling for Subprocess on RHS: just create the object, don't call __pipeline__(None)
                let right_obj = match right.as_ref() {
                    Expr::Subprocess {
                        kind,
                        parts,
                        span: s_span,
                    } => {
                        // Create just the __SnailSubprocess{Capture|Status}(cmd) object
                        let mut lowered_parts = Vec::with_capacity(parts.len());
                        for part in parts {
                            match part {
                                SubprocessPart::Text(text) => {
                                    lowered_parts.push(PyFStringPart::Text(text.clone()));
                                }
                                SubprocessPart::Expr(expr) => {
                                    lowered_parts.push(PyFStringPart::Expr(
                                        lower_expr_with_exception(expr, exception_name)?,
                                    ));
                                }
                            }
                        }
                        let command = PyExpr::FString {
                            parts: lowered_parts,
                            span: s_span.clone(),
                        };
                        let class_name = match kind {
                            SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
                            SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
                        };
                        PyExpr::Call {
                            func: Box::new(PyExpr::Name {
                                id: class_name.to_string(),
                                span: s_span.clone(),
                            }),
                            args: vec![PyArgument::Positional {
                                value: command,
                                span: s_span.clone(),
                            }],
                            span: s_span.clone(),
                        }
                    }
                    _ => lower_expr_with_exception(right, exception_name)?,
                };

                Ok(PyExpr::Call {
                    func: Box::new(PyExpr::Attribute {
                        value: Box::new(right_obj),
                        attr: "__pipeline__".to_string(),
                        span: span.clone(),
                    }),
                    args: vec![PyArgument::Positional {
                        value: left_expr,
                        span: span.clone(),
                    }],
                    span: span.clone(),
                })
            } else {
                Ok(PyExpr::Binary {
                    left: Box::new(lower_expr_with_exception(left, exception_name)?),
                    op: lower_binary_op(*op),
                    right: Box::new(lower_expr_with_exception(right, exception_name)?),
                    span: span.clone(),
                })
            }
        }
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
        Expr::Compound { expressions, span } => {
            let mut lowered = Vec::new();
            for expr in expressions {
                lowered.push(lower_expr_with_exception(expr, exception_name)?);
            }

            let tuple_expr = PyExpr::Tuple {
                elements: lowered,
                span: span.clone(),
            };

            let index_expr = PyExpr::Unary {
                op: PyUnaryOp::Minus,
                operand: Box::new(PyExpr::Number {
                    value: "1".to_string(),
                    span: span.clone(),
                }),
                span: span.clone(),
            };

            Ok(PyExpr::Index {
                value: Box::new(tuple_expr),
                index: Box::new(index_expr),
                span: span.clone(),
            })
        }
        Expr::Regex { pattern, span } => Ok(PyExpr::Call {
            func: Box::new(PyExpr::Name {
                id: SNAIL_REGEX_COMPILE.to_string(),
                span: span.clone(),
            }),
            args: vec![pos_arg(
                lower_regex_pattern_expr(pattern, span, exception_name)?,
                span,
            )],
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
            let class_name = match kind {
                SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
                SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
            };
            // $(cmd) becomes __SnailSubprocessCapture(cmd).__pipeline__(None)
            let subprocess_obj = PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: class_name.to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: command,
                    span: span.clone(),
                }],
                span: span.clone(),
            };
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Attribute {
                    value: Box::new(subprocess_obj),
                    attr: "__pipeline__".to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: PyExpr::None { span: span.clone() },
                    span: span.clone(),
                }],
                span: span.clone(),
            })
        }
        Expr::StructuredAccessor { query, span } => {
            // $[query] becomes __SnailStructuredAccessor(query)
            Ok(PyExpr::Call {
                func: Box::new(PyExpr::Name {
                    id: SNAIL_STRUCTURED_ACCESSOR_CLASS.to_string(),
                    span: span.clone(),
                }),
                args: vec![PyArgument::Positional {
                    value: PyExpr::String {
                        value: query.clone(),
                        raw: false,
                        delimiter: StringDelimiter::Double,
                        span: span.clone(),
                    },
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
        BinaryOp::Pipeline => {
            panic!("Pipeline operator should be handled specially in lower_expr_with_exception")
        }
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
    python_source_with_auto_print(module, false)
}

pub fn python_source_with_auto_print(module: &PyModule, auto_print_last: bool) -> String {
    let mut writer = PythonWriter::new();
    let uses_try = module_uses_snail_try(module);
    let uses_regex = module_uses_snail_regex(module);
    let uses_subprocess = module_uses_snail_subprocess(module);
    let uses_structured = module_uses_structured_accessor(module);
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
    if uses_structured {
        if uses_try || uses_regex || uses_subprocess {
            writer.write_line("");
        }
        writer.write_structured_accessor_helpers();
    }
    if (uses_try || uses_regex || uses_subprocess || uses_structured) && !module.body.is_empty() {
        writer.write_line("");
    }

    // Handle auto-print of last expression in CLI mode
    if auto_print_last && !module.body.is_empty() {
        let last_idx = module.body.len() - 1;

        // Write all statements except the last
        for stmt in &module.body[..last_idx] {
            writer.write_stmt(stmt);
        }

        // Check if last statement is an expression
        if let PyStmt::Expr {
            value,
            semicolon_terminated,
            ..
        } = &module.body[last_idx]
        {
            // Don't auto-print if the statement was explicitly terminated with a semicolon
            if *semicolon_terminated {
                writer.write_stmt(&module.body[last_idx]);
            } else {
                // Generate code to capture and pretty-print the last expression
                let expr_code = expr_source(value);
                writer.write_line(&format!("__snail_last_result = {}", expr_code));
                writer.write_line("if isinstance(__snail_last_result, str):");
                writer.indent += 1;
                writer.write_line("print(__snail_last_result)");
                writer.indent -= 1;
                writer.write_line("elif __snail_last_result is not None:");
                writer.indent += 1;
                writer.write_line("import pprint");
                writer.write_line("pprint.pprint(__snail_last_result)");
                writer.indent -= 1;
            }
        } else {
            // Last statement is not an expression, write it normally
            writer.write_stmt(&module.body[last_idx]);
        }
    } else {
        writer.write_module(module);
    }

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
                if id == SNAIL_SUBPROCESS_CAPTURE_CLASS || id == SNAIL_SUBPROCESS_STATUS_CLASS)
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

fn module_uses_structured_accessor(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_structured_accessor)
}

fn stmt_uses_structured_accessor(stmt: &PyStmt) -> bool {
    match stmt {
        PyStmt::If {
            test, body, orelse, ..
        } => {
            expr_uses_structured_accessor(test)
                || block_uses_structured_accessor(body)
                || block_uses_structured_accessor(orelse)
        }
        PyStmt::While {
            test, body, orelse, ..
        } => {
            expr_uses_structured_accessor(test)
                || block_uses_structured_accessor(body)
                || block_uses_structured_accessor(orelse)
        }
        PyStmt::For {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            expr_uses_structured_accessor(target)
                || expr_uses_structured_accessor(iter)
                || block_uses_structured_accessor(body)
                || block_uses_structured_accessor(orelse)
        }
        PyStmt::FunctionDef { body, .. } | PyStmt::ClassDef { body, .. } => {
            block_uses_structured_accessor(body)
        }
        PyStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } => {
            block_uses_structured_accessor(body)
                || handlers.iter().any(handler_uses_structured_accessor)
                || block_uses_structured_accessor(orelse)
                || block_uses_structured_accessor(finalbody)
        }
        PyStmt::With { items, body, .. } => {
            items.iter().any(with_item_uses_structured_accessor)
                || block_uses_structured_accessor(body)
        }
        PyStmt::Return { value, .. } => value.as_ref().is_some_and(expr_uses_structured_accessor),
        PyStmt::Raise { value, from, .. } => {
            value.as_ref().is_some_and(expr_uses_structured_accessor)
                || from.as_ref().is_some_and(expr_uses_structured_accessor)
        }
        PyStmt::Assert { test, message, .. } => {
            expr_uses_structured_accessor(test)
                || message.as_ref().is_some_and(expr_uses_structured_accessor)
        }
        PyStmt::Delete { targets, .. } => targets.iter().any(expr_uses_structured_accessor),
        PyStmt::Import { .. }
        | PyStmt::ImportFrom { .. }
        | PyStmt::Break { .. }
        | PyStmt::Continue { .. }
        | PyStmt::Pass { .. } => false,
        PyStmt::Assign { targets, value, .. } => {
            targets.iter().any(expr_uses_structured_accessor)
                || expr_uses_structured_accessor(value)
        }
        PyStmt::Expr { value, .. } => expr_uses_structured_accessor(value),
    }
}

fn block_uses_structured_accessor(block: &[PyStmt]) -> bool {
    block.iter().any(stmt_uses_structured_accessor)
}

fn handler_uses_structured_accessor(handler: &PyExceptHandler) -> bool {
    handler
        .type_name
        .as_ref()
        .is_some_and(expr_uses_structured_accessor)
        || block_uses_structured_accessor(&handler.body)
}

fn with_item_uses_structured_accessor(item: &PyWithItem) -> bool {
    expr_uses_structured_accessor(&item.context)
        || item
            .target
            .as_ref()
            .is_some_and(expr_uses_structured_accessor)
}

fn argument_uses_structured_accessor(arg: &PyArgument) -> bool {
    match arg {
        PyArgument::Positional { value, .. }
        | PyArgument::Keyword { value, .. }
        | PyArgument::Star { value, .. }
        | PyArgument::KwStar { value, .. } => expr_uses_structured_accessor(value),
    }
}

fn expr_uses_structured_accessor(expr: &PyExpr) -> bool {
    match expr {
        PyExpr::Name { .. }
        | PyExpr::Number { .. }
        | PyExpr::String { .. }
        | PyExpr::Bool { .. }
        | PyExpr::None { .. } => false,
        PyExpr::FString { parts, .. } => parts.iter().any(|part| match part {
            PyFStringPart::Text(_) => false,
            PyFStringPart::Expr(expr) => expr_uses_structured_accessor(expr),
        }),
        PyExpr::Unary { operand, .. } => expr_uses_structured_accessor(operand),
        PyExpr::Binary { left, right, .. } => {
            expr_uses_structured_accessor(left) || expr_uses_structured_accessor(right)
        }
        PyExpr::Compare {
            left, comparators, ..
        } => {
            expr_uses_structured_accessor(left)
                || comparators.iter().any(expr_uses_structured_accessor)
        }
        PyExpr::IfExpr {
            test, body, orelse, ..
        } => {
            expr_uses_structured_accessor(test)
                || expr_uses_structured_accessor(body)
                || expr_uses_structured_accessor(orelse)
        }
        PyExpr::Lambda { body, .. } => expr_uses_structured_accessor(body),
        PyExpr::Call { func, args, .. } => {
            if matches!(func.as_ref(), PyExpr::Name { id, .. }
                if id == SNAIL_STRUCTURED_ACCESSOR_CLASS)
            {
                return true;
            }
            expr_uses_structured_accessor(func)
                || args.iter().any(argument_uses_structured_accessor)
        }
        PyExpr::Attribute { value, .. } => expr_uses_structured_accessor(value),
        PyExpr::Index { value, index, .. } => {
            expr_uses_structured_accessor(value) || expr_uses_structured_accessor(index)
        }
        PyExpr::Paren { expr, .. } => expr_uses_structured_accessor(expr),
        PyExpr::List { elements, .. } | PyExpr::Tuple { elements, .. } => {
            elements.iter().any(expr_uses_structured_accessor)
        }
        PyExpr::Dict { entries, .. } => entries.iter().any(|(key, value)| {
            expr_uses_structured_accessor(key) || expr_uses_structured_accessor(value)
        }),
        PyExpr::Set { elements, .. } => elements.iter().any(expr_uses_structured_accessor),
        PyExpr::ListComp {
            element, iter, ifs, ..
        } => {
            expr_uses_structured_accessor(element)
                || expr_uses_structured_accessor(iter)
                || ifs.iter().any(expr_uses_structured_accessor)
        }
        PyExpr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            expr_uses_structured_accessor(key)
                || expr_uses_structured_accessor(value)
                || expr_uses_structured_accessor(iter)
                || ifs.iter().any(expr_uses_structured_accessor)
        }
        PyExpr::Slice { start, end, .. } => {
            start.as_deref().is_some_and(expr_uses_structured_accessor)
                || end.as_deref().is_some_and(expr_uses_structured_accessor)
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

        // Write __SnailSubprocessCapture class
        self.write_line(&format!("class {}:", SNAIL_SUBPROCESS_CAPTURE_CLASS));
        self.indent += 1;
        self.write_line("def __init__(self, cmd):");
        self.indent += 1;
        self.write_line("self.cmd = cmd");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __pipeline__(self, input_data):");
        self.indent += 1;
        self.write_line("try:");
        self.indent += 1;
        self.write_line("if input_data is None:");
        self.indent += 1;
        self.write_line("# No stdin - run normally");
        self.write_line("completed = subprocess.run(self.cmd, shell=True, check=True, text=True, stdout=subprocess.PIPE)");
        self.indent -= 1;
        self.write_line("else:");
        self.indent += 1;
        self.write_line("# Pipe input to stdin");
        self.write_line("if not isinstance(input_data, (str, bytes)):");
        self.indent += 1;
        self.write_line("input_data = str(input_data)");
        self.indent -= 1;
        self.write_line("completed = subprocess.run(self.cmd, shell=True, check=True, text=True, input=input_data, stdout=subprocess.PIPE)");
        self.indent -= 1;
        self.write_line("return completed.stdout.rstrip('\\n')");
        self.indent -= 1;
        self.write_line("except subprocess.CalledProcessError as exc:");
        self.indent += 1;
        self.write_line("def __fallback(exc=exc):");
        self.indent += 1;
        self.write_line("raise exc");
        self.indent -= 1;
        self.write_line("exc.__fallback__ = __fallback");
        self.write_line("raise");
        self.indent -= 3;
        self.write_line("");

        // Write __SnailSubprocessStatus class
        self.write_line(&format!("class {}:", SNAIL_SUBPROCESS_STATUS_CLASS));
        self.indent += 1;
        self.write_line("def __init__(self, cmd):");
        self.indent += 1;
        self.write_line("self.cmd = cmd");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __pipeline__(self, input_data):");
        self.indent += 1;
        self.write_line("try:");
        self.indent += 1;
        self.write_line("if input_data is None:");
        self.indent += 1;
        self.write_line("# No stdin - run normally");
        self.write_line("subprocess.run(self.cmd, shell=True, check=True)");
        self.indent -= 1;
        self.write_line("else:");
        self.indent += 1;
        self.write_line("# Pipe input to stdin");
        self.write_line("if not isinstance(input_data, (str, bytes)):");
        self.indent += 1;
        self.write_line("input_data = str(input_data)");
        self.indent -= 1;
        self.write_line(
            "subprocess.run(self.cmd, shell=True, check=True, text=True, input=input_data)",
        );
        self.indent -= 1;
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
        self.indent -= 3;
    }

    fn write_vendored_jmespath(&mut self) {
        // Helper to escape Python source for embedding in a string
        fn escape_py_source(source: &str) -> String {
            source
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        }

        self.write_line("# Vendored jmespath library (embedded to avoid external dependency)");
        self.write_line("import sys");
        self.write_line("if 'jmespath' not in sys.modules:");
        self.indent += 1;
        self.write_line("import types");
        self.write_line("");

        // Create jmespath package
        self.write_line("__jmespath = types.ModuleType('jmespath')");
        self.write_line("__jmespath.__package__ = 'jmespath'");
        self.write_line("__jmespath.__path__ = []");
        self.write_line("sys.modules['jmespath'] = __jmespath");
        self.write_line("");

        // Inject each submodule using compile+exec (in dependency order)
        self.write_line("# Inject jmespath.compat (base module)");
        self.write_line("__mod = types.ModuleType('jmespath.compat')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/compat.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_COMPAT)
        ));
        self.write_line("sys.modules['jmespath.compat'] = __mod");
        self.write_line("__jmespath.compat = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.exceptions");
        self.write_line("__mod = types.ModuleType('jmespath.exceptions')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/exceptions.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_EXCEPTIONS)
        ));
        self.write_line("sys.modules['jmespath.exceptions'] = __mod");
        self.write_line("__jmespath.exceptions = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.ast");
        self.write_line("__mod = types.ModuleType('jmespath.ast')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/ast.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_AST)
        ));
        self.write_line("sys.modules['jmespath.ast'] = __mod");
        self.write_line("__jmespath.ast = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.lexer");
        self.write_line("__mod = types.ModuleType('jmespath.lexer')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/lexer.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_LEXER)
        ));
        self.write_line("sys.modules['jmespath.lexer'] = __mod");
        self.write_line("__jmespath.lexer = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.functions");
        self.write_line("__mod = types.ModuleType('jmespath.functions')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/functions.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_FUNCTIONS)
        ));
        self.write_line("sys.modules['jmespath.functions'] = __mod");
        self.write_line("__jmespath.functions = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.visitor");
        self.write_line("__mod = types.ModuleType('jmespath.visitor')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/visitor.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_VISITOR)
        ));
        self.write_line("sys.modules['jmespath.visitor'] = __mod");
        self.write_line("__jmespath.visitor = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath.parser");
        self.write_line("__mod = types.ModuleType('jmespath.parser')");
        self.write_line("__mod.__package__ = 'jmespath'");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/parser.py', 'exec'), __mod.__dict__)",
            escape_py_source(JMESPATH_PARSER)
        ));
        self.write_line("sys.modules['jmespath.parser'] = __mod");
        self.write_line("__jmespath.parser = __mod");
        self.write_line("");

        self.write_line("# Inject jmespath main module");
        self.write_line(&format!(
            "exec(compile(\"{}\", 'jmespath/__init__.py', 'exec'), __jmespath.__dict__)",
            escape_py_source(JMESPATH_INIT)
        ));
        self.write_line("");

        self.indent -= 1;
        self.write_line("");
    }

    fn write_structured_accessor_helpers(&mut self) {
        self.write_vendored_jmespath();
        self.write_line("import jmespath");
        self.write_line("import json as _json");
        self.write_line("import sys as _sys");
        self.write_line("");

        // Write __SnailStructuredAccessor class
        self.write_line(&format!("class {}:", SNAIL_STRUCTURED_ACCESSOR_CLASS));
        self.indent += 1;
        self.write_line("def __init__(self, query):");
        self.indent += 1;
        self.write_line("self.query = query");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __pipeline__(self, obj):");
        self.indent += 1;
        self.write_line("if not hasattr(obj, '__structured__'):");
        self.indent += 1;
        self.write_line("raise TypeError(f\"Pipeline target must implement __structured__, got {type(obj).__name__}\")");
        self.indent -= 1;
        self.write_line("return obj.__structured__(self.query)");
        self.indent -= 2;
        self.write_line("");

        // Write __SnailJsonObject class
        self.write_line(&format!("class {}:", SNAIL_JSON_OBJECT_CLASS));
        self.indent += 1;
        self.write_line("def __init__(self, data):");
        self.indent += 1;
        self.write_line("self.data = data");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __structured__(self, query):");
        self.indent += 1;
        self.write_line("return jmespath.search(query, self.data)");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __repr__(self):");
        self.indent += 1;
        self.write_line("return f\"__SnailJsonObject({self.data!r})\"");
        self.indent -= 2;
        self.write_line("");

        // Write json() function
        self.write_line("def json(input=_sys.stdin):");
        self.indent += 1;
        self.write_line("\"\"\"Parse JSON from various input sources.\"\"\"");
        self.write_line("# Handle different input types");
        self.write_line("if isinstance(input, str):");
        self.indent += 1;
        self.write_line("# Try parsing as JSON string first");
        self.write_line("try:");
        self.indent += 1;
        self.write_line("data = _json.loads(input)");
        self.indent -= 1;
        self.write_line("except _json.JSONDecodeError:");
        self.indent += 1;
        self.write_line("# Fall back to file path");
        self.write_line("with open(input, 'r') as f:");
        self.indent += 1;
        self.write_line("data = _json.load(f)");
        self.indent -= 3;
        self.write_line("elif hasattr(input, 'read'):");
        self.indent += 1;
        self.write_line("# File-like object (including sys.stdin)");
        self.write_line("content = input.read()");
        self.write_line("if isinstance(content, bytes):");
        self.indent += 1;
        self.write_line("content = content.decode('utf-8')");
        self.indent -= 1;
        self.write_line("data = _json.loads(content)");
        self.indent -= 1;
        self.write_line("elif isinstance(input, (dict, list, int, float, bool, type(None))):");
        self.indent += 1;
        self.write_line("# Already JSON-native type");
        self.write_line("data = input");
        self.indent -= 1;
        self.write_line("else:");
        self.indent += 1;
        self.write_line("raise TypeError(f\"json() input must be JSON-compatible, got {type(input).__name__}\")");
        self.indent -= 1;
        self.write_line("");
        self.write_line(&format!("return {}(data)", SNAIL_JSON_OBJECT_CLASS));
        self.indent -= 1;
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
    let prefix = if raw { "r" } else { "" };
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
