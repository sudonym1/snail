use std::fmt::Write as _;

use snail_ast::StringDelimiter;
use snail_lower::*;
use snail_python_ast::*;

// Vendored jmespath library
const JMESPATH_EXCEPTIONS: &str = include_str!("../../../vendored/jmespath/exceptions.py");
const JMESPATH_COMPAT: &str = include_str!("../../../vendored/jmespath/compat.py");
const JMESPATH_AST: &str = include_str!("../../../vendored/jmespath/ast.py");
const JMESPATH_LEXER: &str = include_str!("../../../vendored/jmespath/lexer.py");
const JMESPATH_FUNCTIONS: &str = include_str!("../../../vendored/jmespath/functions.py");
const JMESPATH_VISITOR: &str = include_str!("../../../vendored/jmespath/visitor.py");
const JMESPATH_PARSER: &str = include_str!("../../../vendored/jmespath/parser.py");
const JMESPATH_INIT: &str = include_str!("../../../vendored/jmespath/__init__.py");

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
                if id == SNAIL_STRUCTURED_ACCESSOR_CLASS || id == "json")
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
        self.write_line("return _json.dumps(self.data, indent=2)");
        self.indent -= 2;
        self.write_line("");

        // Write __SnailJsonPipelineWrapper class
        self.write_line(&format!("class {}:", SNAIL_JSON_PIPELINE_WRAPPER_CLASS));
        self.indent += 1;
        self.write_line(
            "\"\"\"Wrapper for json() to support pipeline operator without blocking stdin.\"\"\"",
        );
        self.write_line("");
        self.write_line("def __pipeline__(self, input):");
        self.indent += 1;
        self.write_line("\"\"\"Called when used in a pipeline: x | json()\"\"\"");
        self.write_line("return json(input)");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __structured__(self, query):");
        self.indent += 1;
        self.write_line("\"\"\"Called when used with structured accessor: json() | $[query]\"\"\"");
        self.write_line("data = json(_sys.stdin)");
        self.write_line("return data.__structured__(query)");
        self.indent -= 1;
        self.write_line("");
        self.write_line("def __repr__(self):");
        self.indent += 1;
        self.write_line("\"\"\"When printed, consume stdin and display parsed JSON.\"\"\"");
        self.write_line("data = json(_sys.stdin)");
        self.write_line("return repr(data)");
        self.indent -= 2;
        self.write_line("");

        // Write json() function
        self.write_line("def json(input=None):");
        self.indent += 1;
        self.write_line("\"\"\"Parse JSON from various input sources.\"\"\"");
        self.write_line("# Return wrapper when called with no arguments for pipeline support");
        self.write_line("if input is None:");
        self.indent += 1;
        self.write_line(&format!("return {}()", SNAIL_JSON_PIPELINE_WRAPPER_CLASS));
        self.indent -= 1;
        self.write_line("");
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
    // For raw strings, we need to choose the delimiter carefully to avoid escaping issues
    // Python raw strings can't escape their delimiter, so we pick the best one
    if raw {
        let has_double = value.contains('"');
        let has_single = value.contains('\'');
        let has_triple_double = value.contains("\"\"\"");
        let has_triple_single = value.contains("'''");

        // Choose the best delimiter based on what's in the string
        let (open, close) = if has_triple_double && has_triple_single {
            // Both triple quotes present - this is rare, use concatenation
            // For now, fall back to double quotes and hope for the best
            // This is a limitation of Python raw strings
            ("\"", "\"")
        } else if has_triple_double {
            ("'''", "'''")
        } else if has_triple_single {
            ("\"\"\"", "\"\"\"")
        } else if has_double && !has_single {
            ("'", "'")
        } else if has_single && !has_double {
            ("\"", "\"")
        } else if has_double && has_single {
            // Both quotes present, use triple quotes
            ("\"\"\"", "\"\"\"")
        } else {
            // No quotes, use the original delimiter preference
            match delimiter {
                StringDelimiter::Single => ("'", "'"),
                StringDelimiter::Double => ("\"", "\""),
                StringDelimiter::TripleSingle => ("'''", "'''"),
                StringDelimiter::TripleDouble => ("\"\"\"", "\"\"\""),
            }
        };
        format!("r{open}{value}{close}")
    } else {
        // For non-raw strings, the value is already properly escaped by the parser
        // We just need to wrap it in the appropriate delimiter
        let (open, close) = match delimiter {
            StringDelimiter::Single => ("'", "'"),
            StringDelimiter::Double => ("\"", "\""),
            StringDelimiter::TripleSingle => ("'''", "'''"),
            StringDelimiter::TripleDouble => ("\"\"\"", "\"\"\""),
        };
        format!("{open}{value}{close}")
    }
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
