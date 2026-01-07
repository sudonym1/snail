use snail_lower::*;
use snail_python_ast::*;

/// Check if a module uses the snail try helper
pub fn module_uses_snail_try(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_try)
}

/// Check if a module uses snail regex helpers
pub fn module_uses_snail_regex(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_regex)
}

/// Check if a module uses snail subprocess helpers
pub fn module_uses_snail_subprocess(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_snail_subprocess)
}

/// Check if a module uses structured accessors
pub fn module_uses_structured_accessor(module: &PyModule) -> bool {
    module.body.iter().any(stmt_uses_structured_accessor)
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
