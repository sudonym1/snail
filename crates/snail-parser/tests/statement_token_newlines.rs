mod common;

use common::parse_ok;
use snail_ast::{Expr, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StmtKind {
    If,
    While,
    For,
    Def,
    Class,
    Try,
    With,
    Return,
    Break,
    Continue,
    Pass,
    Raise,
    Assert,
    Delete,
    Import,
    ImportFrom,
    Assign,
    Expr,
}

fn stmt_kind(stmt: &Stmt) -> StmtKind {
    match stmt {
        Stmt::If { .. } => StmtKind::If,
        Stmt::While { .. } => StmtKind::While,
        Stmt::For { .. } => StmtKind::For,
        Stmt::Def { .. } => StmtKind::Def,
        Stmt::Class { .. } => StmtKind::Class,
        Stmt::Try { .. } => StmtKind::Try,
        Stmt::With { .. } => StmtKind::With,
        Stmt::Return { .. } => StmtKind::Return,
        Stmt::Break { .. } => StmtKind::Break,
        Stmt::Continue { .. } => StmtKind::Continue,
        Stmt::Pass { .. } => StmtKind::Pass,
        Stmt::Raise { .. } => StmtKind::Raise,
        Stmt::Assert { .. } => StmtKind::Assert,
        Stmt::Delete { .. } => StmtKind::Delete,
        Stmt::Import { .. } => StmtKind::Import,
        Stmt::ImportFrom { .. } => StmtKind::ImportFrom,
        Stmt::Assign { .. } => StmtKind::Assign,
        Stmt::Expr { .. } => StmtKind::Expr,
        Stmt::Lines { .. } | Stmt::Files { .. } | Stmt::PatternAction { .. } => StmtKind::Expr,
    }
}

/// Under Go-style semicolon injection, compound-statement headers support
/// multi-line splitting via header mode. Simple statements (return, raise,
/// assert, yield, etc.) require arguments on the same line because their
/// keywords are StmtEnders.
#[test]
fn parses_each_statement_with_newline_split_tokens() {
    let cases = [
        ("if_stmt", "if\ncond\n{\npass\n}\n", StmtKind::If),
        (
            "if_let_stmt",
            "if\nlet\n[lhs\n,\nrhs]\n=\npair\n;\nlhs\n{\npass\n}\n",
            StmtKind::If,
        ),
        (
            "if_let_stmt_newline_before_guard_semicolon_minimal",
            "if let x = y\n;\nz { }\n",
            StmtKind::If,
        ),
        ("while_stmt", "while\ncond\n{\npass\n}\n", StmtKind::While),
        (
            "while_let_stmt",
            "while\nlet\nvalue\n=\nnext()\n;\nvalue\n{\npass\n}\n",
            StmtKind::While,
        ),
        (
            "for_stmt",
            "for\nitem\nin\nitems\n{\npass\n}\n",
            StmtKind::For,
        ),
        (
            "def_stmt",
            "def\nbuild\n(\na\n,\nb\n=\n1\n)\n{\nreturn a\n}\n",
            StmtKind::Def,
        ),
        ("class_stmt", "class\nBucket\n{\npass\n}\n", StmtKind::Class),
        (
            "try_stmt",
            "try\n{\npass\n}\nexcept\nException\nas\nerr\n{\npass\n}\nelse\n{\npass\n}\nfinally\n{\npass\n}\n",
            StmtKind::Try,
        ),
        (
            "with_stmt",
            "with\nopen(\n\"data\"\n)\nas\nhandle\n{\npass\n}\n",
            StmtKind::With,
        ),
        // Simple statements: arguments on same line
        ("return_stmt", "return 1\n", StmtKind::Return),
        ("break_stmt", "break\n", StmtKind::Break),
        ("continue_stmt", "continue\n", StmtKind::Continue),
        ("pass_stmt", "pass\n", StmtKind::Pass),
        (
            "raise_stmt",
            "raise ValueError(\"bad\") from err\n",
            StmtKind::Raise,
        ),
        ("assert_stmt", "assert cond, \"msg\"\n", StmtKind::Assert),
        ("del_stmt", "del\nitems[\n0\n],\nother\n", StmtKind::Delete),
        (
            "import_stmt_import_names",
            "import\nos as os_mod,\nsys\n",
            StmtKind::Import,
        ),
        (
            "import_stmt_import_from",
            "from pkg import name as alias, other\n",
            StmtKind::ImportFrom,
        ),
        ("assign_stmt", "value =\n1\n", StmtKind::Assign),
        ("expr_stmt", "print(\n1\n)\n", StmtKind::Expr),
        ("expr_stmt_yield", "yield 1\n", StmtKind::Expr),
        ("expr_stmt_yield_from", "yield from items\n", StmtKind::Expr),
    ];

    for (case_name, source, expected_kind) in cases {
        let program = parse_ok(source);
        assert_eq!(
            program.stmts.len(),
            1,
            "expected one top-level statement for {case_name}\nsource:\n{source}"
        );
        let actual_kind = stmt_kind(&program.stmts[0]);
        assert_eq!(
            actual_kind, expected_kind,
            "unexpected statement kind for {case_name}\nsource:\n{source}"
        );
    }
}

#[test]
fn parses_yield_expression_statements() {
    let program = parse_ok("yield 1\nyield from items\nyield\n");
    assert_eq!(program.stmts.len(), 3);

    match &program.stmts[0] {
        Stmt::Expr { value, .. } => match value {
            Expr::Yield { value, .. } => match value.as_deref() {
                Some(Expr::Number { value, .. }) => assert_eq!(value, "1"),
                other => panic!("expected numeric yield value, got {other:?}"),
            },
            other => panic!("expected yield expression, got {other:?}"),
        },
        other => panic!("expected expression statement, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::Expr { value, .. } => match value {
            Expr::YieldFrom { expr, .. } => {
                assert!(matches!(expr.as_ref(), Expr::Name { name, .. } if name == "items"));
            }
            other => panic!("expected yield from expression, got {other:?}"),
        },
        other => panic!("expected expression statement, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::Expr { value, .. } => match value {
            Expr::Yield { value, .. } => assert!(value.is_none()),
            other => panic!("expected bare yield expression, got {other:?}"),
        },
        other => panic!("expected expression statement, got {other:?}"),
    }
}
