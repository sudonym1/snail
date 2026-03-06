use std::process::Command;

use proptest::prelude::*;
use snail_ast::{Expr, Stmt};

const COMPACT_TRY_EXCEPTION_VAR: &str = "__snail_compact_exc";

#[derive(Clone, Debug)]
enum PostfixOp {
    Attr(String),
    Index(String),
    Call(Vec<String>),
}

fn identifier_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "x", "y", "z", "value", "item", "result", "count", "data",
    ])
    .prop_map(|name| name.to_owned())
}

fn literal_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        (0_u8..=50).prop_map(|number| number.to_string()),
        prop::sample::select(vec!["True", "False", "None", "\"hi\"", "'bye'"])
            .prop_map(|literal| literal.to_owned()),
    ]
}

fn atom_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        identifier_strategy(),
        literal_strategy(),
        identifier_strategy().prop_map(|name| format!("{{ {name} }}")),
        identifier_strategy().prop_map(|name| format!("if True {{ {name} }} else {{ {name} }}")),
    ]
}

fn simple_expr_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        identifier_strategy(),
        literal_strategy(),
        identifier_strategy().prop_map(|name| format!("({name})")),
    ]
}

fn postfix_strategy() -> impl Strategy<Value = PostfixOp> {
    prop_oneof![
        identifier_strategy().prop_map(PostfixOp::Attr),
        simple_expr_strategy().prop_map(PostfixOp::Index),
        prop::collection::vec(simple_expr_strategy(), 0..3).prop_map(PostfixOp::Call),
    ]
}

fn expression_strategy() -> impl Strategy<Value = String> {
    (
        atom_strategy(),
        prop::collection::vec(postfix_strategy(), 0..4),
        0_u8..3,
    )
        .prop_map(|(base, postfixes, paren_depth)| {
            let mut expr = base;
            for op in postfixes {
                expr = match op {
                    PostfixOp::Attr(name) => format!("{expr}.{name}"),
                    PostfixOp::Index(index_expr) => format!("{expr}[{index_expr}]"),
                    PostfixOp::Call(args) => format!("{expr}({})", args.join(", ")),
                };
            }
            for _ in 0..paren_depth {
                expr = format!("({expr})");
            }
            expr
        })
}

fn is_compact_try(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Try {
            body,
            handlers,
            else_body: None,
            finally_body: None,
            ..
        } if body.len() == 1
            && handlers.len() == 1
            && matches!(&body[0], Stmt::Expr { .. })
            && matches!(
                handlers[0].type_name.as_ref(),
                Some(Expr::Name { name, .. }) if name == "Exception"
            )
            && handlers[0].name.as_deref() == Some(COMPACT_TRY_EXCEPTION_VAR)
            && handlers[0].body.len() == 1
            && matches!(&handlers[0].body[0], Stmt::Expr { .. })
    )
}

fn parse_pipeline_ok(source: &str) -> Result<(), String> {
    let program = snail_parser::parse(source).map_err(|error| format!("parse: {error}"))?;
    if program.stmts.len() != 1 {
        return Err(format!(
            "expected exactly one statement, got {}",
            program.stmts.len()
        ));
    }

    let Stmt::Expr { value, .. } = &program.stmts[0] else {
        return Err("expected expression statement".to_string());
    };

    if !is_compact_try(value) {
        return Err(format!("expected compact try root, got {value:?}"));
    }

    Ok(())
}

fn maybe_cli_compile_ok(source: &str) -> Result<(), String> {
    if std::env::var_os("SNAIL_PROPTEST_CLI_E2E").is_none() {
        return Ok(());
    }

    let output = Command::new("uv")
        .args(["run", "--", "snail", "--debug-python-ast", source])
        .output()
        .map_err(|error| format!("failed to run snail CLI: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "snail CLI compile failed (status: {:?})\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        ))
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn compact_try_forms_parse_end_to_end(
        expr in expression_strategy(),
        fallback in expression_strategy(),
    ) {
        let sources = [
            format!("{expr}?"),
            format!("({expr})?"),
            format!("{expr}:{fallback}?"),
            format!("{expr} : {fallback} ?"),
            format!("({expr}):({fallback})?"),
        ];

        for source in sources {
            if let Err(error) = parse_pipeline_ok(&source) {
                prop_assert!(false, "source failed parse pipeline:\n{source}\n\nerror:\n{error}");
            }
            if let Err(error) = maybe_cli_compile_ok(&source) {
                prop_assert!(false, "source failed CLI compile pipeline:\n{source}\n\nerror:\n{error}");
            }
        }
    }
}
