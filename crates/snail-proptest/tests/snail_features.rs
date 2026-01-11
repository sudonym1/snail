#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use snail_proptest::arbitrary::*;
use snail_proptest::utils::*;

// ========== Snail-Specific Feature Tests ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn regex_expressions_lower(expr in regex_expr()) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Regex expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn regex_match_expressions_lower(expr in regex_match_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Regex match expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn subprocess_expressions_lower(expr in subprocess_expr()) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Subprocess expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn structured_accessor_expressions_lower(expr in structured_accessor_expr()) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Structured accessor expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn fstring_expressions_lower(expr in fstring_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // F-string expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn comprehensions_lower(expr in prop_oneof![
        list_comp_expr(Size::new(2)),
        dict_comp_expr(Size::new(2)),
    ]) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Comprehensions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn compare_expressions_lower(expr in compare_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Compare expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn compound_expressions_lower(expr in compound_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Compound expressions should lower
        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn class_statements_lower(stmt in class_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn with_statements_lower(stmt in with_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        let _ = snail_lower::lower_program(&program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn import_statements_lower(stmt in prop_oneof![
        import_stmt(),
        import_from_stmt(),
    ]) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        let _ = snail_lower::lower_program(&program);
    }
}
