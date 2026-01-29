use snail_ast::*;

pub(super) struct LambdaHoister {
    counter: usize,
}

impl LambdaHoister {
    pub(super) fn new() -> Self {
        Self { counter: 0 }
    }

    pub(super) fn desugar_program(&mut self, program: &Program) -> Program {
        Program {
            stmts: self.desugar_block(&program.stmts),
            span: program.span.clone(),
        }
    }

    pub(super) fn desugar_awk_program(&mut self, program: &AwkProgram) -> AwkProgram {
        let begin_blocks = program
            .begin_blocks
            .iter()
            .map(|block| self.desugar_block(block))
            .collect();
        let end_blocks = program
            .end_blocks
            .iter()
            .map(|block| self.desugar_block(block))
            .collect();
        let rules = program
            .rules
            .iter()
            .map(|rule| self.desugar_awk_rule(rule))
            .collect();
        AwkProgram {
            begin_blocks,
            rules,
            end_blocks,
            span: program.span.clone(),
        }
    }

    pub(super) fn desugar_block(&mut self, block: &[Stmt]) -> Vec<Stmt> {
        let mut out = Vec::new();
        for stmt in block {
            let mut prelude = Vec::new();
            let stmt = self.desugar_stmt(stmt, &mut prelude);
            out.extend(prelude);
            out.push(stmt);
        }
        out
    }

    fn desugar_awk_rule(&mut self, rule: &AwkRule) -> AwkRule {
        let action = rule.action.as_ref().map(|block| self.desugar_block(block));
        AwkRule {
            pattern: rule.pattern.clone(),
            action,
            span: rule.span.clone(),
        }
    }

    fn next_lambda_name(&mut self) -> String {
        self.counter += 1;
        format!("__snail_lambda_{}", self.counter)
    }

    fn desugar_stmt(&mut self, stmt: &Stmt, prelude: &mut Vec<Stmt>) -> Stmt {
        match stmt {
            Stmt::If {
                cond,
                body,
                elifs,
                else_body,
                span,
            } => {
                let cond = self.desugar_condition(cond, prelude);
                let body = self.desugar_block(body);
                let elifs = elifs
                    .iter()
                    .map(|(elif_cond, elif_body)| {
                        (
                            self.desugar_condition(elif_cond, prelude),
                            self.desugar_block(elif_body),
                        )
                    })
                    .collect();
                let else_body = else_body.as_ref().map(|body| self.desugar_block(body));
                Stmt::If {
                    cond,
                    body,
                    elifs,
                    else_body,
                    span: span.clone(),
                }
            }
            Stmt::While {
                cond,
                body,
                else_body,
                span,
            } => {
                let cond = self.desugar_condition(cond, prelude);
                let body = self.desugar_block(body);
                let else_body = else_body.as_ref().map(|body| self.desugar_block(body));
                Stmt::While {
                    cond,
                    body,
                    else_body,
                    span: span.clone(),
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                else_body,
                span,
            } => {
                let target = self.desugar_assign_target(target, prelude);
                let iter = self.desugar_expr(iter, prelude);
                let body = self.desugar_block(body);
                let else_body = else_body.as_ref().map(|body| self.desugar_block(body));
                Stmt::For {
                    target,
                    iter,
                    body,
                    else_body,
                    span: span.clone(),
                }
            }
            Stmt::Def {
                name,
                params,
                body,
                span,
            } => {
                let params = self.desugar_params(params, prelude);
                let body = self.desugar_block(body);
                Stmt::Def {
                    name: name.clone(),
                    params,
                    body,
                    span: span.clone(),
                }
            }
            Stmt::Class { name, body, span } => {
                let body = self.desugar_block(body);
                Stmt::Class {
                    name: name.clone(),
                    body,
                    span: span.clone(),
                }
            }
            Stmt::Try {
                body,
                handlers,
                else_body,
                finally_body,
                span,
            } => {
                let body = self.desugar_block(body);
                let handlers = handlers
                    .iter()
                    .map(|handler| self.desugar_except_handler(handler, prelude))
                    .collect();
                let else_body = else_body.as_ref().map(|body| self.desugar_block(body));
                let finally_body = finally_body.as_ref().map(|body| self.desugar_block(body));
                Stmt::Try {
                    body,
                    handlers,
                    else_body,
                    finally_body,
                    span: span.clone(),
                }
            }
            Stmt::With { items, body, span } => {
                let items = items
                    .iter()
                    .map(|item| self.desugar_with_item(item, prelude))
                    .collect();
                let body = self.desugar_block(body);
                Stmt::With {
                    items,
                    body,
                    span: span.clone(),
                }
            }
            Stmt::Return { value, span } => Stmt::Return {
                value: value
                    .as_ref()
                    .map(|value| self.desugar_expr(value, prelude)),
                span: span.clone(),
            },
            Stmt::Raise { value, from, span } => Stmt::Raise {
                value: value
                    .as_ref()
                    .map(|value| self.desugar_expr(value, prelude)),
                from: from.as_ref().map(|from| self.desugar_expr(from, prelude)),
                span: span.clone(),
            },
            Stmt::Assert {
                test,
                message,
                span,
            } => Stmt::Assert {
                test: self.desugar_expr(test, prelude),
                message: message
                    .as_ref()
                    .map(|message| self.desugar_expr(message, prelude)),
                span: span.clone(),
            },
            Stmt::Delete { targets, span } => Stmt::Delete {
                targets: targets
                    .iter()
                    .map(|target| self.desugar_assign_target(target, prelude))
                    .collect(),
                span: span.clone(),
            },
            Stmt::Break { span } => Stmt::Break { span: span.clone() },
            Stmt::Continue { span } => Stmt::Continue { span: span.clone() },
            Stmt::Pass { span } => Stmt::Pass { span: span.clone() },
            Stmt::Import { items, span } => Stmt::Import {
                items: items.clone(),
                span: span.clone(),
            },
            Stmt::ImportFrom {
                level,
                module,
                items,
                span,
            } => Stmt::ImportFrom {
                level: *level,
                module: module.clone(),
                items: items.clone(),
                span: span.clone(),
            },
            Stmt::Assign {
                targets,
                value,
                span,
            } => Stmt::Assign {
                targets: targets
                    .iter()
                    .map(|target| self.desugar_assign_target(target, prelude))
                    .collect(),
                value: self.desugar_expr(value, prelude),
                span: span.clone(),
            },
            Stmt::Expr {
                value,
                semicolon_terminated,
                span,
            } => Stmt::Expr {
                value: self.desugar_expr(value, prelude),
                semicolon_terminated: *semicolon_terminated,
                span: span.clone(),
            },
        }
    }

    fn desugar_condition(&mut self, cond: &Condition, prelude: &mut Vec<Stmt>) -> Condition {
        match cond {
            Condition::Expr(expr) => Condition::Expr(Box::new(self.desugar_expr(expr, prelude))),
            Condition::Let {
                target,
                value,
                guard,
                span,
            } => Condition::Let {
                target: Box::new(self.desugar_assign_target(target, prelude)),
                value: Box::new(self.desugar_expr(value, prelude)),
                guard: guard
                    .as_ref()
                    .map(|guard| Box::new(self.desugar_expr(guard, prelude))),
                span: span.clone(),
            },
        }
    }

    fn desugar_with_item(&mut self, item: &WithItem, prelude: &mut Vec<Stmt>) -> WithItem {
        WithItem {
            context: self.desugar_expr(&item.context, prelude),
            target: item
                .target
                .as_ref()
                .map(|target| self.desugar_assign_target(target, prelude)),
            span: item.span.clone(),
        }
    }

    fn desugar_except_handler(
        &mut self,
        handler: &ExceptHandler,
        prelude: &mut Vec<Stmt>,
    ) -> ExceptHandler {
        ExceptHandler {
            type_name: handler
                .type_name
                .as_ref()
                .map(|expr| self.desugar_expr(expr, prelude)),
            name: handler.name.clone(),
            body: self.desugar_block(&handler.body),
            span: handler.span.clone(),
        }
    }

    fn desugar_params(&mut self, params: &[Parameter], prelude: &mut Vec<Stmt>) -> Vec<Parameter> {
        params
            .iter()
            .map(|param| match param {
                Parameter::Regular {
                    name,
                    default,
                    span,
                } => Parameter::Regular {
                    name: name.clone(),
                    default: default
                        .as_ref()
                        .map(|default| self.desugar_expr(default, prelude)),
                    span: span.clone(),
                },
                Parameter::VarArgs { name, span } => Parameter::VarArgs {
                    name: name.clone(),
                    span: span.clone(),
                },
                Parameter::KwArgs { name, span } => Parameter::KwArgs {
                    name: name.clone(),
                    span: span.clone(),
                },
            })
            .collect()
    }

    fn desugar_argument(&mut self, arg: &Argument, prelude: &mut Vec<Stmt>) -> Argument {
        match arg {
            Argument::Positional { value, span } => Argument::Positional {
                value: self.desugar_expr(value, prelude),
                span: span.clone(),
            },
            Argument::Keyword { name, value, span } => Argument::Keyword {
                name: name.clone(),
                value: self.desugar_expr(value, prelude),
                span: span.clone(),
            },
            Argument::Star { value, span } => Argument::Star {
                value: self.desugar_expr(value, prelude),
                span: span.clone(),
            },
            Argument::KwStar { value, span } => Argument::KwStar {
                value: self.desugar_expr(value, prelude),
                span: span.clone(),
            },
        }
    }

    fn desugar_assign_target(
        &mut self,
        target: &AssignTarget,
        prelude: &mut Vec<Stmt>,
    ) -> AssignTarget {
        match target {
            AssignTarget::Name { name, span } => AssignTarget::Name {
                name: name.clone(),
                span: span.clone(),
            },
            AssignTarget::Attribute { value, attr, span } => AssignTarget::Attribute {
                value: Box::new(self.desugar_expr(value, prelude)),
                attr: attr.clone(),
                span: span.clone(),
            },
            AssignTarget::Index { value, index, span } => AssignTarget::Index {
                value: Box::new(self.desugar_expr(value, prelude)),
                index: Box::new(self.desugar_expr(index, prelude)),
                span: span.clone(),
            },
            AssignTarget::Starred { target, span } => AssignTarget::Starred {
                target: Box::new(self.desugar_assign_target(target, prelude)),
                span: span.clone(),
            },
            AssignTarget::Tuple { elements, span } => AssignTarget::Tuple {
                elements: elements
                    .iter()
                    .map(|element| self.desugar_assign_target(element, prelude))
                    .collect(),
                span: span.clone(),
            },
            AssignTarget::List { elements, span } => AssignTarget::List {
                elements: elements
                    .iter()
                    .map(|element| self.desugar_assign_target(element, prelude))
                    .collect(),
                span: span.clone(),
            },
        }
    }

    fn desugar_regex_pattern(
        &mut self,
        pattern: &RegexPattern,
        prelude: &mut Vec<Stmt>,
    ) -> RegexPattern {
        match pattern {
            RegexPattern::Literal(text) => RegexPattern::Literal(text.clone()),
            RegexPattern::Interpolated(parts) => RegexPattern::Interpolated(
                parts
                    .iter()
                    .map(|part| self.desugar_fstring_part(part, prelude))
                    .collect(),
            ),
        }
    }

    fn desugar_fstring_part(&mut self, part: &FStringPart, prelude: &mut Vec<Stmt>) -> FStringPart {
        match part {
            FStringPart::Text(text) => FStringPart::Text(text.clone()),
            FStringPart::Expr(expr) => FStringPart::Expr(self.desugar_fstring_expr(expr, prelude)),
        }
    }

    fn desugar_fstring_expr(&mut self, expr: &FStringExpr, prelude: &mut Vec<Stmt>) -> FStringExpr {
        let format_spec = expr.format_spec.as_ref().map(|parts| {
            parts
                .iter()
                .map(|part| self.desugar_fstring_part(part, prelude))
                .collect()
        });
        FStringExpr {
            expr: Box::new(self.desugar_expr(&expr.expr, prelude)),
            conversion: expr.conversion,
            format_spec,
        }
    }

    fn desugar_expr(&mut self, expr: &Expr, prelude: &mut Vec<Stmt>) -> Expr {
        match expr {
            Expr::Name { name, span } => Expr::Name {
                name: name.clone(),
                span: span.clone(),
            },
            Expr::Placeholder { span } => Expr::Placeholder { span: span.clone() },
            Expr::Number { value, span } => Expr::Number {
                value: value.clone(),
                span: span.clone(),
            },
            Expr::String {
                value,
                raw,
                bytes,
                delimiter,
                span,
            } => Expr::String {
                value: value.clone(),
                raw: *raw,
                bytes: *bytes,
                delimiter: *delimiter,
                span: span.clone(),
            },
            Expr::FString { parts, bytes, span } => Expr::FString {
                parts: parts
                    .iter()
                    .map(|part| self.desugar_fstring_part(part, prelude))
                    .collect(),
                bytes: *bytes,
                span: span.clone(),
            },
            Expr::Bool { value, span } => Expr::Bool {
                value: *value,
                span: span.clone(),
            },
            Expr::None { span } => Expr::None { span: span.clone() },
            Expr::Unary { op, expr, span } => Expr::Unary {
                op: *op,
                expr: Box::new(self.desugar_expr(expr, prelude)),
                span: span.clone(),
            },
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => Expr::Binary {
                left: Box::new(self.desugar_expr(left, prelude)),
                op: *op,
                right: Box::new(self.desugar_expr(right, prelude)),
                span: span.clone(),
            },
            Expr::AugAssign {
                target,
                op,
                value,
                span,
            } => Expr::AugAssign {
                target: Box::new(self.desugar_assign_target(target, prelude)),
                op: *op,
                value: Box::new(self.desugar_expr(value, prelude)),
                span: span.clone(),
            },
            Expr::PrefixIncr { op, target, span } => Expr::PrefixIncr {
                op: *op,
                target: Box::new(self.desugar_assign_target(target, prelude)),
                span: span.clone(),
            },
            Expr::PostfixIncr { op, target, span } => Expr::PostfixIncr {
                op: *op,
                target: Box::new(self.desugar_assign_target(target, prelude)),
                span: span.clone(),
            },
            Expr::Compare {
                left,
                ops,
                comparators,
                span,
            } => Expr::Compare {
                left: Box::new(self.desugar_expr(left, prelude)),
                ops: ops.clone(),
                comparators: comparators
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::IfExpr {
                test,
                body,
                orelse,
                span,
            } => Expr::IfExpr {
                test: Box::new(self.desugar_expr(test, prelude)),
                body: Box::new(self.desugar_expr(body, prelude)),
                orelse: Box::new(self.desugar_expr(orelse, prelude)),
                span: span.clone(),
            },
            Expr::TryExpr {
                expr,
                fallback,
                span,
            } => Expr::TryExpr {
                expr: Box::new(self.desugar_expr(expr, prelude)),
                fallback: fallback
                    .as_ref()
                    .map(|expr| Box::new(self.desugar_expr(expr, prelude))),
                span: span.clone(),
            },
            Expr::Yield { value, span } => Expr::Yield {
                value: value
                    .as_ref()
                    .map(|expr| Box::new(self.desugar_expr(expr, prelude))),
                span: span.clone(),
            },
            Expr::YieldFrom { expr, span } => Expr::YieldFrom {
                expr: Box::new(self.desugar_expr(expr, prelude)),
                span: span.clone(),
            },
            Expr::Lambda { params, body, span } => {
                if lambda_requires_def(params, body) {
                    let params = self.desugar_params(params, prelude);
                    let body = ensure_lambda_return(self.desugar_block(body));
                    let name = self.next_lambda_name();
                    prelude.push(Stmt::Def {
                        name: name.clone(),
                        params,
                        body,
                        span: span.clone(),
                    });
                    Expr::Name {
                        name,
                        span: span.clone(),
                    }
                } else {
                    Expr::Lambda {
                        params: params.clone(),
                        body: body.clone(),
                        span: span.clone(),
                    }
                }
            }
            Expr::Compound { expressions, span } => Expr::Compound {
                expressions: expressions
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::Regex { pattern, span } => Expr::Regex {
                pattern: self.desugar_regex_pattern(pattern, prelude),
                span: span.clone(),
            },
            Expr::RegexMatch {
                value,
                pattern,
                span,
            } => Expr::RegexMatch {
                value: Box::new(self.desugar_expr(value, prelude)),
                pattern: self.desugar_regex_pattern(pattern, prelude),
                span: span.clone(),
            },
            Expr::Subprocess { kind, parts, span } => Expr::Subprocess {
                kind: *kind,
                parts: parts
                    .iter()
                    .map(|part| match part {
                        SubprocessPart::Text(text) => SubprocessPart::Text(text.clone()),
                        SubprocessPart::Expr(expr) => {
                            SubprocessPart::Expr(Box::new(self.desugar_expr(expr, prelude)))
                        }
                    })
                    .collect(),
                span: span.clone(),
            },
            Expr::StructuredAccessor { query, span } => Expr::StructuredAccessor {
                query: query.clone(),
                span: span.clone(),
            },
            Expr::Call { func, args, span } => Expr::Call {
                func: Box::new(self.desugar_expr(func, prelude)),
                args: args
                    .iter()
                    .map(|arg| self.desugar_argument(arg, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::Attribute { value, attr, span } => Expr::Attribute {
                value: Box::new(self.desugar_expr(value, prelude)),
                attr: attr.clone(),
                span: span.clone(),
            },
            Expr::Index { value, index, span } => Expr::Index {
                value: Box::new(self.desugar_expr(value, prelude)),
                index: Box::new(self.desugar_expr(index, prelude)),
                span: span.clone(),
            },
            Expr::Paren { expr, span } => Expr::Paren {
                expr: Box::new(self.desugar_expr(expr, prelude)),
                span: span.clone(),
            },
            Expr::FieldIndex { index, span } => Expr::FieldIndex {
                index: index.clone(),
                span: span.clone(),
            },
            Expr::List { elements, span } => Expr::List {
                elements: elements
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::Tuple { elements, span } => Expr::Tuple {
                elements: elements
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::Set { elements, span } => Expr::Set {
                elements: elements
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::Dict { entries, span } => Expr::Dict {
                entries: entries
                    .iter()
                    .map(|(key, value)| {
                        (
                            self.desugar_expr(key, prelude),
                            self.desugar_expr(value, prelude),
                        )
                    })
                    .collect(),
                span: span.clone(),
            },
            Expr::Slice { start, end, span } => Expr::Slice {
                start: start
                    .as_ref()
                    .map(|expr| Box::new(self.desugar_expr(expr, prelude))),
                end: end
                    .as_ref()
                    .map(|expr| Box::new(self.desugar_expr(expr, prelude))),
                span: span.clone(),
            },
            Expr::ListComp {
                element,
                target,
                iter,
                ifs,
                span,
            } => Expr::ListComp {
                element: Box::new(self.desugar_expr(element, prelude)),
                target: target.clone(),
                iter: Box::new(self.desugar_expr(iter, prelude)),
                ifs: ifs
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
            Expr::DictComp {
                key,
                value,
                target,
                iter,
                ifs,
                span,
            } => Expr::DictComp {
                key: Box::new(self.desugar_expr(key, prelude)),
                value: Box::new(self.desugar_expr(value, prelude)),
                target: target.clone(),
                iter: Box::new(self.desugar_expr(iter, prelude)),
                ifs: ifs
                    .iter()
                    .map(|expr| self.desugar_expr(expr, prelude))
                    .collect(),
                span: span.clone(),
            },
        }
    }
}

fn lambda_requires_def(params: &[Parameter], body: &[Stmt]) -> bool {
    if body.iter().any(|stmt| !matches!(stmt, Stmt::Expr { .. })) {
        return true;
    }

    for param in params {
        if let Parameter::Regular { default, .. } = param
            && let Some(default) = default
            && expr_contains_complex_lambda(default)
        {
            return true;
        }
    }

    for stmt in body {
        if let Stmt::Expr { value, .. } = stmt
            && expr_contains_complex_lambda(value)
        {
            return true;
        }
    }

    false
}

fn expr_contains_complex_lambda(expr: &Expr) -> bool {
    match expr {
        Expr::Yield { value, .. } => value
            .as_ref()
            .is_some_and(|expr| expr_contains_complex_lambda(expr)),
        Expr::YieldFrom { expr, .. } => expr_contains_complex_lambda(expr),
        Expr::Lambda { params, body, .. } => lambda_requires_def(params, body),
        Expr::Unary { expr, .. } => expr_contains_complex_lambda(expr),
        Expr::Binary { left, right, .. } => {
            expr_contains_complex_lambda(left) || expr_contains_complex_lambda(right)
        }
        Expr::AugAssign { target, value, .. } => {
            assign_target_contains_complex_lambda(target) || expr_contains_complex_lambda(value)
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            assign_target_contains_complex_lambda(target)
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            if expr_contains_complex_lambda(left) {
                return true;
            }
            comparators.iter().any(expr_contains_complex_lambda)
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            expr_contains_complex_lambda(test)
                || expr_contains_complex_lambda(body)
                || expr_contains_complex_lambda(orelse)
        }
        Expr::TryExpr { expr, fallback, .. } => {
            expr_contains_complex_lambda(expr)
                || fallback
                    .as_ref()
                    .is_some_and(|expr| expr_contains_complex_lambda(expr))
        }
        Expr::Compound { expressions, .. } => expressions.iter().any(expr_contains_complex_lambda),
        Expr::Regex { pattern, .. } => regex_pattern_contains_complex_lambda(pattern),
        Expr::RegexMatch { value, pattern, .. } => {
            expr_contains_complex_lambda(value) || regex_pattern_contains_complex_lambda(pattern)
        }
        Expr::Subprocess { parts, .. } => parts.iter().any(|part| match part {
            SubprocessPart::Expr(expr) => expr_contains_complex_lambda(expr),
            SubprocessPart::Text(_) => false,
        }),
        Expr::Call { func, args, .. } => {
            if expr_contains_complex_lambda(func) {
                return true;
            }
            args.iter().any(|arg| match arg {
                Argument::Positional { value, .. }
                | Argument::Keyword { value, .. }
                | Argument::Star { value, .. }
                | Argument::KwStar { value, .. } => expr_contains_complex_lambda(value),
            })
        }
        Expr::Attribute { value, .. } => expr_contains_complex_lambda(value),
        Expr::Index { value, index, .. } => {
            expr_contains_complex_lambda(value) || expr_contains_complex_lambda(index)
        }
        Expr::Paren { expr, .. } => expr_contains_complex_lambda(expr),
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } => {
            elements.iter().any(expr_contains_complex_lambda)
        }
        Expr::Set { elements, .. } => elements.iter().any(expr_contains_complex_lambda),
        Expr::Dict { entries, .. } => entries.iter().any(|(key, value)| {
            expr_contains_complex_lambda(key) || expr_contains_complex_lambda(value)
        }),
        Expr::Slice { start, end, .. } => {
            start
                .as_ref()
                .is_some_and(|expr| expr_contains_complex_lambda(expr))
                || end
                    .as_ref()
                    .is_some_and(|expr| expr_contains_complex_lambda(expr))
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            expr_contains_complex_lambda(element)
                || expr_contains_complex_lambda(iter)
                || ifs.iter().any(expr_contains_complex_lambda)
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            expr_contains_complex_lambda(key)
                || expr_contains_complex_lambda(value)
                || expr_contains_complex_lambda(iter)
                || ifs.iter().any(expr_contains_complex_lambda)
        }
        Expr::FString { parts, .. } => parts.iter().any(fstring_part_contains_complex_lambda),
        Expr::Name { .. }
        | Expr::Placeholder { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => false,
    }
}

fn assign_target_contains_complex_lambda(target: &AssignTarget) -> bool {
    match target {
        AssignTarget::Name { .. } => false,
        AssignTarget::Attribute { value, .. } => expr_contains_complex_lambda(value),
        AssignTarget::Index { value, index, .. } => {
            expr_contains_complex_lambda(value) || expr_contains_complex_lambda(index)
        }
        AssignTarget::Starred { target, .. } => assign_target_contains_complex_lambda(target),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            elements.iter().any(assign_target_contains_complex_lambda)
        }
    }
}

fn regex_pattern_contains_complex_lambda(pattern: &RegexPattern) -> bool {
    match pattern {
        RegexPattern::Literal(_) => false,
        RegexPattern::Interpolated(parts) => parts.iter().any(fstring_part_contains_complex_lambda),
    }
}

fn fstring_part_contains_complex_lambda(part: &FStringPart) -> bool {
    match part {
        FStringPart::Text(_) => false,
        FStringPart::Expr(expr) => fstring_expr_contains_complex_lambda(expr),
    }
}

fn fstring_expr_contains_complex_lambda(expr: &FStringExpr) -> bool {
    if expr_contains_complex_lambda(expr.expr.as_ref()) {
        return true;
    }
    expr.format_spec
        .as_ref()
        .is_some_and(|parts| parts.iter().any(fstring_part_contains_complex_lambda))
}

fn ensure_lambda_return(mut body: Vec<Stmt>) -> Vec<Stmt> {
    let Some(last) = body.pop() else {
        return body;
    };
    match last {
        Stmt::Expr { value, span, .. } => {
            body.push(Stmt::Return {
                value: Some(value),
                span,
            });
        }
        other => body.push(other),
    }
    body
}
