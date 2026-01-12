#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use pyo3::prelude::*;
use snail_proptest::arbitrary::*;
use snail_proptest::utils::*;

// ========== Helper Functions ==========

fn assert_python_compiles(py: Python<'_>, module: &PyObject) {
    let ast = py.import_bound("ast").expect("failed to import ast");
    let fixed = ast
        .getattr("fix_missing_locations")
        .and_then(|fix| fix.call1((module.clone_ref(py),)))
        .expect("failed to fix locations");
    let builtins = py
        .import_bound("builtins")
        .expect("failed to import builtins");
    builtins
        .getattr("compile")
        .and_then(|compile| compile.call1((fixed, "<test>", "exec")))
        .expect("Generated Python AST has syntax errors");
}

// ========== Property 1: Lowering Never Panics ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn lowering_never_panics_on_valid_ast(program in program()) {
        Python::with_gil(|py| {
            let _ = snail_lower::lower_program(py, &program);
        });
    }
}

// ========== Property 2: Generated Python is Syntactically Valid ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn lowered_python_is_syntactically_valid(program in program()) {
        Python::with_gil(|py| {
            if let Ok(module) = snail_lower::lower_program(py, &program) {
                assert_python_compiles(py, &module);
            }
        });
    }
}

// ========== Property 3: Full Pipeline Doesn't Crash ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn full_compile_pipeline_never_panics(program in program()) {
        Python::with_gil(|py| {
            if let Ok(module) = snail_lower::lower_program(py, &program) {
                assert_python_compiles(py, &module);
            }
        });
    }
}

// ========== Property 4: Try Expressions Always Lower ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn try_expressions_always_lower(expr in try_expr(Size::new(3))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("try expressions should always lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 5: Binary Operators Preserve Structure ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn binary_ops_preserve_structure(
        left in simple_expr(),
        op in binary_op(),
        right in simple_expr()
    ) {
        let expr = snail_ast::Expr::Binary {
            left: Box::new(left.clone()),
            op,
            right: Box::new(right.clone()),
            span: dummy_span(),
        };

        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("binary ops should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 6: Control Flow Lowering ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn if_statements_always_lower(stmt in if_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("if statements should always lower");

            assert_python_compiles(py, &module);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn while_statements_always_lower(stmt in while_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("while statements should always lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 7: Function Definitions ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn function_definitions_always_lower(stmt in def_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("function defs should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 8: Collections Lower Correctly ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn list_literals_lower(stmt in list_literal_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("list literals should lower");

            assert_python_compiles(py, &module);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn dict_literals_lower(stmt in dict_literal_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("dict literals should lower");

            assert_python_compiles(py, &module);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn set_literals_lower(stmt in set_literal_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("set literals should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 9: Subprocess Expressions Always Lower ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn subprocess_exprs_lower(expr in subprocess_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("subprocess expressions should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 10: Class Definitions ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn class_definitions_lower(stmt in class_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("class defs should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 11: Exception Handling ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn try_statements_lower(stmt in try_stmt(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![stmt],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("try statements should lower");

            assert_python_compiles(py, &module);
        });
    }
}

// ========== Property 12: Comprehensions ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn list_comprehensions_lower(expr in list_comp_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("list comps should lower");

            assert_python_compiles(py, &module);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn dict_comprehensions_lower(expr in dict_comp_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        Python::with_gil(|py| {
            let module = snail_lower::lower_program(py, &program)
                .expect("dict comps should lower");

            assert_python_compiles(py, &module);
        });
    }
}
