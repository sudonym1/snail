#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use pyo3::prelude::*;
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
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
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn pipeline_expressions_lower(expr in pipeline_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Pipeline expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn try_expressions_lower(expr in try_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Try expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn structured_json_pipeline_lower(expr in structured_json_pipeline_expr()) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Structured JSON pipeline expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn nested_compounds_lower(expr in compound_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Compound expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn lambda_exprs_lower(expr in lambda_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Lambda expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn attribute_expressions_lower(expr in attribute_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Attribute expressions should lower
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}
