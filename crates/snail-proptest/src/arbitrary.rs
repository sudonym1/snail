use crate::utils::{dummy_span, Size};
use proptest::prelude::*;
use snail_ast::*;

// ========== Identifiers ==========

/// Generate valid Python identifiers (no $ prefix, no keywords)
pub fn identifier() -> impl Strategy<Value = String> {
    // Start with letter/underscore, continue with letter/digit/underscore
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,15}")
        .unwrap()
        .prop_filter("not a keyword", |s| {
            !matches!(
                s.as_str(),
                "if" | "elif"
                    | "else"
                    | "while"
                    | "for"
                    | "def"
                    | "class"
                    | "try"
                    | "except"
                    | "finally"
                    | "with"
                    | "as"
                    | "return"
                    | "raise"
                    | "assert"
                    | "del"
                    | "break"
                    | "continue"
                    | "pass"
                    | "import"
                    | "from"
                    | "and"
                    | "or"
                    | "not"
                    | "in"
                    | "is"
                    | "True"
                    | "False"
                    | "None"
                    | "BEGIN"
                    | "END"
                    | "lambda"
                    | "yield"
                    | "global"
                    | "nonlocal"
                    | "async"
                    | "await"
            )
        })
}

// ========== Simple Expressions ==========

pub fn number_expr() -> impl Strategy<Value = Expr> {
    prop_oneof![
        // Integers (reasonable range)
        (-1_000_000i32..1_000_000i32).prop_map(|n| n.to_string()),
        // Floats (reasonable range, avoid extreme exponents)
        (-1000.0f64..1000.0f64).prop_filter_map("valid float", |f| {
            if f.is_finite() && !f.is_nan() {
                Some(format!("{:.6}", f))
            } else {
                None
            }
        }),
        // Hex (reasonable range)
        (0u32..0xFFFFFF).prop_map(|n| format!("0x{:x}", n)),
    ]
    .prop_map(|value| Expr::Number {
        value,
        span: dummy_span(),
    })
}

pub fn string_expr() -> impl Strategy<Value = Expr> {
    (
        prop::string::string_regex("[a-zA-Z0-9 _,.-]{0,20}").unwrap(),
        prop::bool::ANY,
        prop_oneof![
            Just(StringDelimiter::Single),
            Just(StringDelimiter::Double),
            Just(StringDelimiter::TripleSingle),
            Just(StringDelimiter::TripleDouble),
        ],
    )
        .prop_map(|(value, raw, delimiter)| Expr::String {
            value,
            raw,
            delimiter,
            span: dummy_span(),
        })
}

pub fn bool_expr() -> impl Strategy<Value = Expr> {
    any::<bool>().prop_map(|value| Expr::Bool {
        value,
        span: dummy_span(),
    })
}

pub fn none_expr() -> impl Strategy<Value = Expr> {
    Just(Expr::None { span: dummy_span() })
}

pub fn name_expr() -> impl Strategy<Value = Expr> {
    identifier().prop_map(|name| Expr::Name {
        name,
        span: dummy_span(),
    })
}

/// Simple expressions (non-recursive)
pub fn simple_expr() -> impl Strategy<Value = Expr> {
    prop_oneof![
        2 => number_expr(),
        2 => string_expr(),
        1 => bool_expr(),
        1 => none_expr(),
        3 => name_expr(),
    ]
}

// ========== Operators ==========

pub fn unary_op() -> impl Strategy<Value = UnaryOp> {
    prop_oneof![
        Just(UnaryOp::Plus),
        Just(UnaryOp::Minus),
        Just(UnaryOp::Not),
    ]
}

pub fn binary_op() -> impl Strategy<Value = BinaryOp> {
    prop_oneof![
        Just(BinaryOp::Add),
        Just(BinaryOp::Sub),
        Just(BinaryOp::Mul),
        Just(BinaryOp::Div),
        Just(BinaryOp::FloorDiv),
        Just(BinaryOp::Mod),
        Just(BinaryOp::Pow),
        Just(BinaryOp::And),
        Just(BinaryOp::Or),
        Just(BinaryOp::Pipeline),
    ]
}

pub fn compare_op() -> impl Strategy<Value = CompareOp> {
    prop_oneof![
        Just(CompareOp::Eq),
        Just(CompareOp::NotEq),
        Just(CompareOp::Lt),
        Just(CompareOp::LtEq),
        Just(CompareOp::Gt),
        Just(CompareOp::GtEq),
        Just(CompareOp::In),
        Just(CompareOp::NotIn),
        Just(CompareOp::Is),
        Just(CompareOp::IsNot),
    ]
}

// ========== Recursive Expressions ==========

pub fn expr_with_size(size: Size) -> impl Strategy<Value = Expr> {
    if size.is_zero() {
        simple_expr().boxed()
    } else {
        prop_oneof![
            // 40% simple expressions
            8 => simple_expr(),
            // 60% recursive/complex expressions
            // Basic operations
            2 => binary_expr(size.half()),
            2 => unary_expr(size.half()),
            2 => compare_expr(size.half()),
            // Collections
            2 => list_expr(size.half()),
            2 => tuple_expr(size.half()),
            2 => dict_expr(size.half()),
            // Comprehensions
            1 => list_comp_expr(size.half()),
            1 => dict_comp_expr(size.half()),
            // Control flow
            1 => if_expr(size.half()),
            1 => try_expr(size.half()),
            1 => compound_expr(size.half()),
            // Access patterns
            2 => call_expr(size.half()),
            2 => attribute_expr(size.half()),
            2 => index_expr(size.half()),
            // Note: slice_expr removed - slices are only valid as indices, not standalone
            // Snail-specific features
            1 => fstring_expr(size.half()),
            1 => regex_expr(),
            1 => regex_match_expr(size.half()),
            1 => subprocess_expr(),
            1 => structured_accessor_expr(),
            1 => field_index_expr(),
            1 => paren_expr(size.half()),
        ]
        .boxed()
    }
}

pub fn binary_expr(size: Size) -> impl Strategy<Value = Expr> {
    (expr_with_size(size), binary_op(), expr_with_size(size)).prop_map(|(left, op, right)| {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span: dummy_span(),
        }
    })
}

pub fn unary_expr(size: Size) -> impl Strategy<Value = Expr> {
    (unary_op(), expr_with_size(size)).prop_map(|(op, expr)| Expr::Unary {
        op,
        expr: Box::new(expr),
        span: dummy_span(),
    })
}

pub fn call_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        prop::collection::vec(expr_with_size(size), 0..=2), // positional args
        prop::collection::vec((identifier(), expr_with_size(size)), 0..=2), // keyword args
    )
        .prop_map(|(func, positional_values, keyword_pairs)| {
            // Build args list with positional first, then keyword
            let mut args = Vec::new();
            for value in positional_values {
                args.push(Argument::Positional {
                    value,
                    span: dummy_span(),
                });
            }
            for (name, value) in keyword_pairs {
                args.push(Argument::Keyword {
                    name,
                    value,
                    span: dummy_span(),
                });
            }
            Expr::Call {
                func: Box::new(func),
                args,
                span: dummy_span(),
            }
        })
}

pub fn argument(size: Size) -> impl Strategy<Value = Argument> {
    prop_oneof![
        3 => expr_with_size(size).prop_map(|value| Argument::Positional {
            value,
            span: dummy_span(),
        }),
        1 => (identifier(), expr_with_size(size)).prop_map(|(name, value)| {
            Argument::Keyword {
                name,
                value,
                span: dummy_span(),
            }
        }),
    ]
}

pub fn list_expr(size: Size) -> impl Strategy<Value = Expr> {
    prop::collection::vec(expr_with_size(size), 0..=5).prop_map(|elements| Expr::List {
        elements,
        span: dummy_span(),
    })
}

pub fn tuple_expr(size: Size) -> impl Strategy<Value = Expr> {
    prop::collection::vec(expr_with_size(size), 0..=5).prop_map(|elements| Expr::Tuple {
        elements,
        span: dummy_span(),
    })
}

pub fn dict_expr(size: Size) -> impl Strategy<Value = Expr> {
    prop::collection::vec((expr_with_size(size), expr_with_size(size)), 0..=3).prop_map(|entries| {
        Expr::Dict {
            entries,
            span: dummy_span(),
        }
    })
}

pub fn if_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        expr_with_size(size),
        expr_with_size(size),
    )
        .prop_map(|(test, body, orelse)| Expr::IfExpr {
            test: Box::new(test),
            body: Box::new(body),
            orelse: Box::new(orelse),
            span: dummy_span(),
        })
}

pub fn try_expr(size: Size) -> impl Strategy<Value = Expr> {
    (expr_with_size(size), prop::option::of(expr_with_size(size))).prop_map(|(expr, fallback)| {
        Expr::TryExpr {
            expr: Box::new(expr),
            fallback: fallback.map(Box::new),
            span: dummy_span(),
        }
    })
}

pub fn attribute_expr(size: Size) -> impl Strategy<Value = Expr> {
    (expr_with_size(size), identifier()).prop_map(|(value, attr)| Expr::Attribute {
        value: Box::new(value),
        attr,
        span: dummy_span(),
    })
}

pub fn index_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        prop_oneof![
            3 => expr_with_size(size),
            1 => slice_expr(size),
        ],
    )
        .prop_map(|(value, index)| Expr::Index {
            value: Box::new(value),
            index: Box::new(index),
            span: dummy_span(),
        })
}

pub fn compare_expr(size: Size) -> impl Strategy<Value = Expr> {
    (expr_with_size(size), compare_op(), expr_with_size(size)).prop_map(|(left, op, right)| {
        Expr::Compare {
            left: Box::new(left),
            ops: vec![op],
            comparators: vec![right],
            span: dummy_span(),
        }
    })
}

pub fn paren_expr(size: Size) -> impl Strategy<Value = Expr> {
    expr_with_size(size).prop_map(|expr| Expr::Paren {
        expr: Box::new(expr),
        span: dummy_span(),
    })
}

pub fn slice_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        prop::option::of(expr_with_size(size)),
        prop::option::of(expr_with_size(size)),
    )
        .prop_map(|(start, end)| Expr::Slice {
            start: start.map(Box::new),
            end: end.map(Box::new),
            span: dummy_span(),
        })
}

pub fn compound_expr(size: Size) -> impl Strategy<Value = Expr> {
    prop::collection::vec(expr_with_size(size), 2..=3).prop_map(|expressions| Expr::Compound {
        expressions,
        span: dummy_span(),
    })
}

pub fn list_comp_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        identifier(),
        expr_with_size(size),
        prop::collection::vec(expr_with_size(size), 0..=2),
    )
        .prop_map(|(element, target, iter, ifs)| Expr::ListComp {
            element: Box::new(element),
            target,
            iter: Box::new(iter),
            ifs,
            span: dummy_span(),
        })
}

pub fn dict_comp_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        expr_with_size(size),
        identifier(),
        expr_with_size(size),
        prop::collection::vec(expr_with_size(size), 0..=2),
    )
        .prop_map(|(key, value, target, iter, ifs)| Expr::DictComp {
            key: Box::new(key),
            value: Box::new(value),
            target,
            iter: Box::new(iter),
            ifs,
            span: dummy_span(),
        })
}

pub fn fstring_expr(size: Size) -> impl Strategy<Value = Expr> {
    prop::collection::vec(fstring_part(size), 1..=5).prop_map(|parts| Expr::FString {
        parts,
        span: dummy_span(),
    })
}

pub fn fstring_part(size: Size) -> impl Strategy<Value = FStringPart> {
    prop_oneof![
        2 => prop::string::string_regex("[a-zA-Z0-9 ]{0,10}")
            .unwrap()
            .prop_map(FStringPart::Text),
        1 => expr_with_size(size).prop_map(|e| FStringPart::Expr(Box::new(e))),
    ]
}

pub fn regex_expr() -> impl Strategy<Value = Expr> {
    prop::string::string_regex("[a-z0-9]{1,10}")
        .unwrap()
        .prop_map(|pattern| Expr::Regex {
            pattern: RegexPattern::Literal(pattern),
            span: dummy_span(),
        })
}

pub fn regex_match_expr(size: Size) -> impl Strategy<Value = Expr> {
    (
        expr_with_size(size),
        prop::string::string_regex("[a-z0-9]{1,10}").unwrap(),
    )
        .prop_map(|(value, pattern)| Expr::RegexMatch {
            value: Box::new(value),
            pattern: RegexPattern::Literal(pattern),
            span: dummy_span(),
        })
}

pub fn subprocess_expr() -> impl Strategy<Value = Expr> {
    (
        prop_oneof![Just(SubprocessKind::Capture), Just(SubprocessKind::Status),],
        prop::collection::vec(
            prop::string::string_regex("[a-z]{1,5}")
                .unwrap()
                .prop_map(SubprocessPart::Text),
            1..=3,
        ),
    )
        .prop_map(|(kind, parts)| Expr::Subprocess {
            kind,
            parts,
            span: dummy_span(),
        })
}

pub fn structured_accessor_expr() -> impl Strategy<Value = Expr> {
    prop::string::string_regex("[a-z]{1,10}")
        .unwrap()
        .prop_map(|query| Expr::StructuredAccessor {
            query,
            span: dummy_span(),
        })
}

pub fn field_index_expr() -> impl Strategy<Value = Expr> {
    (0u32..10).prop_map(|n| Expr::FieldIndex {
        index: n.to_string(),
        span: dummy_span(),
    })
}

// ========== Statements ==========

pub fn stmt_with_size(size: Size) -> impl Strategy<Value = Stmt> {
    if size.is_zero() {
        simple_stmt().boxed()
    } else {
        prop_oneof![
            // Simple statements (50%)
            5 => simple_stmt(),
            // Control flow (30%)
            2 => if_stmt(size.half()),
            2 => while_stmt(size.half()),
            2 => for_stmt(size.half()),
            1 => try_stmt(size.half()),
            1 => with_stmt(size.half()),
            // Definitions (10%)
            1 => def_stmt(size.half()),
            1 => class_stmt(size.half()),
            // Exception handling (5%)
            1 => raise_stmt(size.half()),
            1 => assert_stmt(size.half()),
            // Other (5%)
            1 => delete_stmt(),
            1 => import_stmt(),
            1 => import_from_stmt(),
        ]
        .boxed()
    }
}

pub fn simple_stmt() -> impl Strategy<Value = Stmt> {
    prop_oneof![
        2 => Just(Stmt::Pass {
            span: dummy_span()
        }),
        // Note: break, continue, and return removed to avoid invalid contexts
        // They should only appear in loops/functions but we don't track context yet
        3 => expr_stmt(),
        2 => assign_stmt(),
    ]
}

pub fn expr_stmt() -> impl Strategy<Value = Stmt> {
    (expr_with_size(Size::new(2)), any::<bool>()).prop_map(|(value, semicolon_terminated)| {
        Stmt::Expr {
            value,
            semicolon_terminated,
            span: dummy_span(),
        }
    })
}

pub fn assign_stmt() -> impl Strategy<Value = Stmt> {
    (
        prop::collection::vec(assign_target(), 1..=2),
        expr_with_size(Size::new(2)),
    )
        .prop_map(|(targets, value)| Stmt::Assign {
            targets,
            value,
            span: dummy_span(),
        })
}

pub fn assign_target() -> impl Strategy<Value = AssignTarget> {
    identifier().prop_map(|name| AssignTarget::Name {
        name,
        span: dummy_span(),
    })
}

pub fn return_stmt() -> impl Strategy<Value = Stmt> {
    prop::option::of(expr_with_size(Size::new(2))).prop_map(|value| Stmt::Return {
        value,
        span: dummy_span(),
    })
}

pub fn if_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        expr_with_size(size),
        prop::collection::vec(stmt_with_size(size), 1..=3),
        prop::option::of(prop::collection::vec(stmt_with_size(size), 1..=3)),
    )
        .prop_map(|(cond, body, else_body)| Stmt::If {
            cond: Condition::Expr(Box::new(cond)),
            body,
            elifs: vec![],
            else_body,
            span: dummy_span(),
        })
}

pub fn while_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        expr_with_size(size),
        prop::collection::vec(stmt_with_size(size), 1..=3),
    )
        .prop_map(|(cond, body)| Stmt::While {
            cond: Condition::Expr(Box::new(cond)),
            body,
            else_body: None,
            span: dummy_span(),
        })
}

pub fn for_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        assign_target(),
        expr_with_size(size),
        prop::collection::vec(stmt_with_size(size), 1..=3),
    )
        .prop_map(|(target, iter, body)| Stmt::For {
            target,
            iter,
            body,
            else_body: None,
            span: dummy_span(),
        })
}

pub fn def_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        identifier(),
        prop::collection::vec(parameter(), 0..=3),
        prop::collection::vec(stmt_with_size(size), 1..=3),
    )
        .prop_map(|(name, params, body)| Stmt::Def {
            name,
            params,
            body,
            span: dummy_span(),
        })
}

pub fn parameter() -> impl Strategy<Value = Parameter> {
    identifier().prop_map(|name| Parameter::Regular {
        name,
        default: None,
        span: dummy_span(),
    })
}

pub fn try_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        prop::collection::vec(stmt_with_size(size), 1..=2),
        prop::collection::vec(except_handler(size), 1..=2),
    )
        .prop_map(|(body, handlers)| Stmt::Try {
            body,
            handlers,
            else_body: None,
            finally_body: None,
            span: dummy_span(),
        })
}

pub fn except_handler(size: Size) -> impl Strategy<Value = ExceptHandler> {
    prop::collection::vec(stmt_with_size(size), 1..=2).prop_map(|body| ExceptHandler {
        // Always specify "Exception" to avoid "default except must be last" errors
        type_name: Some(Expr::Name {
            name: "Exception".to_string(),
            span: dummy_span(),
        }),
        name: None,
        body,
        span: dummy_span(),
    })
}

pub fn class_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        identifier(),
        prop::collection::vec(stmt_with_size(size), 1..=3),
    )
        .prop_map(|(name, body)| Stmt::Class {
            name,
            body,
            span: dummy_span(),
        })
}

pub fn with_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        prop::collection::vec(with_item(size), 1..=2),
        prop::collection::vec(stmt_with_size(size), 1..=3),
    )
        .prop_map(|(items, body)| Stmt::With {
            items,
            body,
            span: dummy_span(),
        })
}

pub fn with_item(size: Size) -> impl Strategy<Value = WithItem> {
    (expr_with_size(size), prop::option::of(assign_target())).prop_map(|(context, target)| {
        WithItem {
            context,
            target,
            span: dummy_span(),
        }
    })
}

pub fn raise_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (
        prop::option::of(expr_with_size(size)),
        prop::option::of(expr_with_size(size)),
    )
        .prop_map(|(value, from)| Stmt::Raise {
            value,
            from,
            span: dummy_span(),
        })
}

pub fn assert_stmt(size: Size) -> impl Strategy<Value = Stmt> {
    (expr_with_size(size), prop::option::of(expr_with_size(size))).prop_map(|(test, message)| {
        Stmt::Assert {
            test,
            message,
            span: dummy_span(),
        }
    })
}

pub fn delete_stmt() -> impl Strategy<Value = Stmt> {
    prop::collection::vec(assign_target(), 1..=2).prop_map(|targets| Stmt::Delete {
        targets,
        span: dummy_span(),
    })
}

pub fn import_stmt() -> impl Strategy<Value = Stmt> {
    prop::collection::vec(import_item(), 1..=3).prop_map(|items| Stmt::Import {
        items,
        span: dummy_span(),
    })
}

pub fn import_from_stmt() -> impl Strategy<Value = Stmt> {
    (
        prop::collection::vec(identifier(), 1..=2),
        prop::collection::vec(import_from_item(), 1..=3),
    )
        .prop_map(|(module, items)| Stmt::ImportFrom {
            module,
            items,
            span: dummy_span(),
        })
}

pub fn import_from_item() -> impl Strategy<Value = ImportItem> {
    // In "from x import y", y must be a simple identifier, not a dotted path
    (identifier(), prop::option::of(identifier())).prop_map(|(name, alias)| ImportItem {
        name: vec![name],
        alias,
        span: dummy_span(),
    })
}

pub fn import_item() -> impl Strategy<Value = ImportItem> {
    (
        prop::collection::vec(identifier(), 1..=2),
        prop::option::of(identifier()),
    )
        .prop_map(|(name, alias)| ImportItem {
            name,
            alias,
            span: dummy_span(),
        })
}

// ========== Programs ==========

pub fn program() -> impl Strategy<Value = Program> {
    prop::collection::vec(stmt_with_size(Size::new(3)), 0..=5).prop_map(|stmts| Program {
        stmts,
        span: dummy_span(),
    })
}

// ========== AWK Mode ==========

pub fn awk_program() -> impl Strategy<Value = AwkProgram> {
    (
        prop::collection::vec(awk_block(), 0..=2), // BEGIN blocks
        prop::collection::vec(awk_rule(), 0..=3),  // rules
        prop::collection::vec(awk_block(), 0..=2), // END blocks
    )
        .prop_map(|(begin_blocks, rules, end_blocks)| AwkProgram {
            begin_blocks,
            rules,
            end_blocks,
            span: dummy_span(),
        })
}

pub fn awk_block() -> impl Strategy<Value = Vec<Stmt>> {
    prop::collection::vec(stmt_with_size(Size::new(2)), 1..=3)
}

pub fn awk_rule() -> impl Strategy<Value = AwkRule> {
    (
        prop::option::of(expr_with_size(Size::new(2))),
        prop::option::of(prop::collection::vec(stmt_with_size(Size::new(2)), 1..=2)),
    )
        .prop_map(|(pattern, action)| AwkRule {
            pattern,
            action,
            span: dummy_span(),
        })
}
