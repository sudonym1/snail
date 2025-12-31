use std::error::Error;
use std::fmt;

use crate::ast::SourceSpan;
use crate::lower::LowerError;

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

#[derive(Debug)]
pub enum SnailError {
    Parse(ParseError),
    Lower(LowerError),
}

impl fmt::Display for SnailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnailError::Parse(err) => write!(f, "{err}"),
            SnailError::Lower(err) => write!(f, "{err}"),
        }
    }
}

impl Error for SnailError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SnailError::Parse(err) => Some(err),
            SnailError::Lower(err) => Some(err),
        }
    }
}

impl From<ParseError> for SnailError {
    fn from(value: ParseError) -> Self {
        SnailError::Parse(value)
    }
}

impl From<LowerError> for SnailError {
    fn from(value: LowerError) -> Self {
        SnailError::Lower(value)
    }
}
