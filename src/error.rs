use std::error::Error;
use std::fmt;

use crate::ast::SourceSpan;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Option<SourceSpan>,
    pub line_text: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            line_text: None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.message)?;
        if let Some(span) = &self.span {
            writeln!(f, "--> {}:{}", span.start.line, span.start.column)?;
            if let Some(line) = &self.line_text {
                writeln!(f, "{line}")?;
                let mut caret = String::new();
                let col = span.start.column.saturating_sub(1);
                caret.push_str(&" ".repeat(col));
                caret.push('^');
                writeln!(f, "{caret}")?;
            }
        }
        Ok(())
    }
}

impl Error for ParseError {}
