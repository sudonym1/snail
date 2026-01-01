use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use crate::ast::*;

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
    match expr {
        Expr::Name { name, span } => Ok(PyExpr::Name {
            id: name.clone(),
            span: span.clone(),
        }),
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
            operand: Box::new(lower_expr(expr)?),
            span: span.clone(),
        }),
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => Ok(PyExpr::Binary {
            left: Box::new(lower_expr(left)?),
            op: lower_binary_op(*op),
            right: Box::new(lower_expr(right)?),
            span: span.clone(),
        }),
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Ok(PyExpr::Compare {
            left: Box::new(lower_expr(left)?),
            ops: ops.iter().map(|op| lower_compare_op(*op)).collect(),
            comparators: comparators
                .iter()
                .map(lower_expr)
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => Ok(PyExpr::IfExpr {
            test: Box::new(lower_expr(test)?),
            body: Box::new(lower_expr(body)?),
            orelse: Box::new(lower_expr(orelse)?),
            span: span.clone(),
        }),
        Expr::Call { func, args, span } => Ok(PyExpr::Call {
            func: Box::new(lower_expr(func)?),
            args: args
                .iter()
                .map(lower_argument)
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Expr::Attribute { value, attr, span } => Ok(PyExpr::Attribute {
            value: Box::new(lower_expr(value)?),
            attr: attr.clone(),
            span: span.clone(),
        }),
        Expr::Index { value, index, span } => Ok(PyExpr::Index {
            value: Box::new(lower_expr(value)?),
            index: Box::new(lower_expr(index)?),
            span: span.clone(),
        }),
        Expr::Paren { expr, span } => Ok(PyExpr::Paren {
            expr: Box::new(lower_expr(expr)?),
            span: span.clone(),
        }),
        Expr::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr(element)?);
            }
            Ok(PyExpr::List {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr(element)?);
            }
            Ok(PyExpr::Tuple {
                elements: lowered,
                span: span.clone(),
            })
        }
        Expr::Dict { entries, span } => {
            let mut lowered = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                lowered.push((lower_expr(key)?, lower_expr(value)?));
            }
            Ok(PyExpr::Dict {
                entries: lowered,
                span: span.clone(),
            })
        }
        Expr::Set { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr(element)?);
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
                lowered_ifs.push(lower_expr(cond)?);
            }
            Ok(PyExpr::ListComp {
                element: Box::new(lower_expr(element)?),
                target: target.clone(),
                iter: Box::new(lower_expr(iter)?),
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
                lowered_ifs.push(lower_expr(cond)?);
            }
            Ok(PyExpr::DictComp {
                key: Box::new(lower_expr(key)?),
                value: Box::new(lower_expr(value)?),
                target: target.clone(),
                iter: Box::new(lower_expr(iter)?),
                ifs: lowered_ifs,
                span: span.clone(),
            })
        }
        Expr::Slice { start, end, span } => Ok(PyExpr::Slice {
            start: start.as_deref().map(lower_expr).transpose()?.map(Box::new),
            end: end.as_deref().map(lower_expr).transpose()?.map(Box::new),
            span: span.clone(),
        }),
    }
}

fn lower_argument(arg: &Argument) -> Result<PyArgument, LowerError> {
    match arg {
        Argument::Positional { value, span } => Ok(PyArgument::Positional {
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Argument::Keyword { name, value, span } => Ok(PyArgument::Keyword {
            name: name.clone(),
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Argument::Star { value, span } => Ok(PyArgument::Star {
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Argument::KwStar { value, span } => Ok(PyArgument::KwStar {
            value: lower_expr(value)?,
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
        | PyExpr::Bool { span, .. }
        | PyExpr::None { span }
        | PyExpr::Unary { span, .. }
        | PyExpr::Binary { span, .. }
        | PyExpr::Compare { span, .. }
        | PyExpr::IfExpr { span, .. }
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
    writer.write_module(module);
    writer.finish()
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
