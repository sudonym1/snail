use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use snail_ast::SourceSpan;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerError {
    Message(String),
    MultiplePlaceholders { span: SourceSpan },
}

impl LowerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn multiple_placeholders(span: SourceSpan) -> Self {
        Self::MultiplePlaceholders { span }
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::Message(message) => write!(f, "{message}"),
            LowerError::MultiplePlaceholders { .. } => {
                write!(f, "pipeline calls may include at most one placeholder")
            }
        }
    }
}

impl Error for LowerError {}

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

pub fn format_snail_error(err: &SnailError, filename: &str) -> String {
    match err {
        SnailError::Parse(parse) => format_parse_error(parse, filename),
        SnailError::Lower(lower) => format!("error: {lower}"),
    }
}

fn format_parse_error(err: &ParseError, filename: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(&mut out, "error: {}", err.message);
    if let Some(span) = &err.span {
        let _ = writeln!(
            &mut out,
            "--> {}:{}:{}",
            filename, span.start.line, span.start.column
        );
        if let Some(line) = &err.line_text {
            let _ = writeln!(&mut out, " |");
            let _ = writeln!(&mut out, "{:>4} | {}", span.start.line, line);
            let mut caret = String::new();
            let col = span.start.column.saturating_sub(1);
            caret.push_str(&" ".repeat(col));
            caret.push('^');
            let _ = writeln!(&mut out, " | {}", caret);
        }
    }
    out
}
