use snail_ast::*;

/// Rewrite `break expr` into `[capture_var = expr; break]` and
/// `break` (no value) into `[capture_var = None; break]` within a block.
///
/// Recurses into compound blocks (if, try, with, block) but NOT into
/// nested for/while (they have their own break scope).
pub(crate) fn rewrite_breaks_in_block(stmts: &mut Vec<Stmt>, capture_var: &str, span: &SourceSpan) {
    let mut i = 0;
    while i < stmts.len() {
        match &mut stmts[i] {
            Stmt::Break { value, span: bs } => {
                let val = value.take().unwrap_or(Expr::None { span: bs.clone() });
                let bs = bs.clone();
                stmts[i] = Stmt::Break {
                    value: None,
                    span: bs.clone(),
                };
                stmts.insert(
                    i,
                    Stmt::Assign {
                        targets: vec![AssignTarget::Name {
                            name: capture_var.to_string(),
                            span: span.clone(),
                        }],
                        value: val,
                        span: bs,
                    },
                );
                i += 2; // skip the assign and the break
            }
            Stmt::Expr { value: expr, .. } => {
                rewrite_breaks_in_expr(expr, capture_var, span);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
}

fn rewrite_breaks_in_expr(expr: &mut Expr, capture_var: &str, span: &SourceSpan) {
    match expr {
        Expr::Block { stmts, .. } => {
            rewrite_breaks_in_block(stmts, capture_var, span);
        }
        Expr::If {
            body,
            elifs,
            else_body,
            ..
        } => {
            rewrite_breaks_in_block(body, capture_var, span);
            for (_, eb) in elifs.iter_mut() {
                rewrite_breaks_in_block(eb, capture_var, span);
            }
            if let Some(eb) = else_body {
                rewrite_breaks_in_block(eb, capture_var, span);
            }
        }
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            rewrite_breaks_in_block(body, capture_var, span);
            for h in handlers.iter_mut() {
                rewrite_breaks_in_block(&mut h.body, capture_var, span);
            }
            if let Some(eb) = else_body {
                rewrite_breaks_in_block(eb, capture_var, span);
            }
            if let Some(fb) = finally_body {
                rewrite_breaks_in_block(fb, capture_var, span);
            }
        }
        Expr::With { body, .. } => {
            rewrite_breaks_in_block(body, capture_var, span);
        }
        // Do NOT recurse into for/while — they have their own break scope
        Expr::For { .. } | Expr::While { .. } => {}
        _ => {}
    }
}
