mod expr;
mod feature_detection;
mod helpers;
mod writer;

use snail_python_ast::*;

use crate::expr::expr_source;
use crate::feature_detection::{
    module_uses_snail_regex, module_uses_snail_subprocess, module_uses_snail_try,
    module_uses_structured_accessor,
};
use crate::writer::PythonWriter;

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
