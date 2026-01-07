use snail_ast::StringDelimiter;
use snail_python_ast::*;

/// Convert a Python expression to source code
pub fn expr_source(expr: &PyExpr) -> String {
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

pub fn import_name(name: &PyImportName) -> String {
    let mut item = name.name.join(".");
    if let Some(alias) = &name.asname {
        item.push_str(&format!(" as {}", alias));
    }
    item
}

pub fn param_source(param: &PyParameter) -> String {
    match param {
        PyParameter::Regular { name, default, .. } => match default {
            Some(expr) => format!("{}={}", name, expr_source(expr)),
            None => name.clone(),
        },
        PyParameter::VarArgs { name, .. } => format!("*{}", name),
        PyParameter::KwArgs { name, .. } => format!("**{}", name),
    }
}

pub fn argument_source(arg: &PyArgument) -> String {
    match arg {
        PyArgument::Positional { value, .. } => expr_source(value),
        PyArgument::Keyword { name, value, .. } => format!("{}={}", name, expr_source(value)),
        PyArgument::Star { value, .. } => format!("*{}", expr_source(value)),
        PyArgument::KwStar { value, .. } => format!("**{}", expr_source(value)),
    }
}

pub fn with_item_source(item: &PyWithItem) -> String {
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
