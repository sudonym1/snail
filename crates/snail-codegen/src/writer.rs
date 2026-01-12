use std::fmt::Write as _;

use snail_python_ast::*;

use crate::expr::{expr_source, import_name, param_source, with_item_source};

pub struct PythonWriter {
    output: String,
    pub indent: usize,
}

impl PythonWriter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn finish(self) -> String {
        self.output
    }

    pub fn write_module(&mut self, module: &PyModule) {
        for stmt in &module.body {
            self.write_stmt(stmt);
        }
    }

    pub fn write_stmt(&mut self, stmt: &PyStmt) {
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

    pub fn write_line(&mut self, line: &str) {
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
