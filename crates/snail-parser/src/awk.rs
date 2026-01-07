use pest::iterators::Pair;
use snail_ast::AwkRule;
use snail_error::ParseError;

use crate::Rule;
use crate::expr::parse_expr_pair;
use crate::stmt::parse_block;
use crate::util::{error_with_span, span_from_pair};

pub fn parse_awk_rule(pair: Pair<'_, Rule>, source: &str) -> Result<AwkRule, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut pattern = None;
    let mut action = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::awk_pattern => {
                let expr_pair = inner
                    .into_inner()
                    .next()
                    .ok_or_else(|| error_with_span("missing awk pattern", span.clone(), source))?;
                pattern = Some(parse_expr_pair(expr_pair, source)?);
            }
            Rule::block => action = Some(parse_block(inner, source)?),
            _ => {}
        }
    }

    if pattern.is_none() && action.is_none() {
        return Err(error_with_span(
            "awk rule requires a pattern or a block",
            span,
            source,
        ));
    }

    Ok(AwkRule {
        pattern,
        action,
        span,
    })
}
