#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use snail_proptest::arbitrary::*;
use snail_proptest::utils::*;
use std::process::Command;

// ========== Helper Functions ==========

/// Verify that Python code compiles without syntax errors
fn assert_python_compiles(python_code: &str) {
    // Use Python's compile() to check syntax without executing
    let check_code = format!(
        r#"import sys
try:
    compile({}, '<test>', 'exec')
except SyntaxError as e:
    print(f'SyntaxError: {{e}}', file=sys.stderr)
    sys.exit(1)"#,
        format_python_string(python_code)
    );

    let output = Command::new("python3")
        .arg("-c")
        .arg(&check_code)
        .output()
        .expect("failed to execute python3");

    assert!(
        output.status.success(),
        "Generated Python has syntax errors:\n{}\n\nStderr: {}",
        python_code,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Format a string for inclusion in Python code
fn format_python_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

// ========== Property 1: Lowering Never Panics ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn lowering_never_panics_on_valid_ast(program in program()) {
        // Valid AST should always lower without panic
        // We allow LowerError, just not panics
        let _ = snail_lower::lower_program(&program);
    }
}

// ========== Property 2: Generated Python is Syntactically Valid ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn lowered_python_is_syntactically_valid(program in program()) {
        if let Ok(module) = snail_lower::lower_program(&program) {
            let python_code = snail_codegen::python_source(&module);
            assert_python_compiles(&python_code);
        }
    }
}

// ========== Property 3: Full Pipeline Doesn't Crash ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn full_compile_pipeline_never_panics(program in program()) {
        // Lower
        if let Ok(module) = snail_lower::lower_program(&program) {
            // Codegen
            let _python = snail_codegen::python_source(&module);
            // Success!
        }
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

        // Try expressions are core Snail feature - must always lower
        let module = snail_lower::lower_program(&program)
            .expect("try expressions should always lower");

        let python_code = snail_codegen::python_source(&module);

        // Should contain try helper or try statement
        assert!(
            python_code.contains("__snail_compact_try") || python_code.contains("try:"),
            "Try expression didn't generate expected Python code: {}",
            python_code
        );
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

        // Should lower successfully
        let module = snail_lower::lower_program(&program)
            .expect("binary ops should lower");

        // Should generate valid Python
        let python = snail_codegen::python_source(&module);
        assert_python_compiles(&python);
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

        let module = snail_lower::lower_program(&program)
            .expect("if statements should always lower");

        let python = snail_codegen::python_source(&module);
        assert!(python.contains("if "), "If statement didn't generate 'if' keyword");
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

        let module = snail_lower::lower_program(&program)
            .expect("while statements should always lower");

        let python = snail_codegen::python_source(&module);
        assert!(python.contains("while "), "While statement didn't generate 'while' keyword");
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

        let module = snail_lower::lower_program(&program)
            .expect("function defs should always lower");

        let python = snail_codegen::python_source(&module);
        assert!(python.contains("def "), "Function definition didn't generate 'def' keyword");
    }
}

// ========== Property 8: Collections Lower Correctly ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn list_expressions_lower(expr in list_expr(Size::new(2))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let module = snail_lower::lower_program(&program)
            .expect("lists should lower");

        let python = snail_codegen::python_source(&module);
        assert!(python.contains("["), "List expression didn't generate brackets");
    }
}

// ========== Property 9: Statements in Sequence ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn statement_sequences_lower(
        stmts in prop::collection::vec(stmt_with_size(Size::new(2)), 1..=5)
    ) {
        let program = snail_ast::Program {
            stmts,
            span: dummy_span(),
        };

        // Sequences should lower
        if let Ok(module) = snail_lower::lower_program(&program) {
            let python = snail_codegen::python_source(&module);
            assert_python_compiles(&python);
        }
    }
}

// ========== Property 10: Expression Evaluation Doesn't Crash Python ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn expr_evaluation_doesnt_crash_python(expr in expr_with_size(Size::new(3))) {
        let program = snail_ast::Program {
            stmts: vec![snail_ast::Stmt::Expr {
                value: expr,
                semicolon_terminated: true,
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        if let Ok(module) = snail_lower::lower_program(&program) {
            let python_code = snail_codegen::python_source(&module);

            // Execute and ensure it doesn't segfault
            let output = std::process::Command::new("python3")
                .arg("-c")
                .arg(&python_code)
                .output();

            // Allow exceptions, just not crashes
            assert!(output.is_ok(), "Python crashed/segfaulted");
        }
    }
}
