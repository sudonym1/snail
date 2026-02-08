use std::error::Error as StdError;
use std::fmt;

use snail_ast::SourceSpan;
use thiserror::Error;

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
        write_parse_error(
            f,
            self,
            ParseRenderOptions {
                filename: None,
                include_error_prefix: false,
            },
        )
    }
}

impl StdError for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LowerError {
    #[error("{0}")]
    Message(String),
    #[error("pipeline calls may include at most one placeholder")]
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

#[derive(Debug, Error)]
pub enum SnailError {
    #[error("{0}")]
    Parse(
        #[from]
        #[source]
        ParseError,
    ),
    #[error("{0}")]
    Lower(
        #[from]
        #[source]
        LowerError,
    ),
}

pub fn format_snail_error(err: &SnailError, filename: &str) -> String {
    match err {
        SnailError::Parse(parse) => format_parse_error(parse, filename),
        SnailError::Lower(lower) => format!("error: {lower}"),
    }
}

fn format_parse_error(err: &ParseError, filename: &str) -> String {
    render_parse_error(
        err,
        ParseRenderOptions {
            filename: Some(filename),
            include_error_prefix: true,
        },
    )
}

#[derive(Clone, Copy)]
struct ParseRenderOptions<'a> {
    filename: Option<&'a str>,
    include_error_prefix: bool,
}

fn render_parse_error(err: &ParseError, options: ParseRenderOptions<'_>) -> String {
    let mut out = String::new();
    let _ = write_parse_error(&mut out, err, options);
    out
}

fn write_parse_error(
    out: &mut impl fmt::Write,
    err: &ParseError,
    options: ParseRenderOptions<'_>,
) -> fmt::Result {
    if options.include_error_prefix {
        writeln!(out, "error: {}", err.message)?;
    } else {
        writeln!(out, "{}", err.message)?;
    }

    if let Some(span) = &err.span {
        write_parse_location(out, span, options.filename)?;
        write_parse_snippet(out, span, err.line_text.as_deref())?;
    }

    Ok(())
}

fn write_parse_location(
    out: &mut impl fmt::Write,
    span: &SourceSpan,
    filename: Option<&str>,
) -> fmt::Result {
    match filename {
        Some(filename) => writeln!(
            out,
            "--> {}:{}:{}",
            filename, span.start.line, span.start.column
        ),
        None => writeln!(out, "--> {}:{}", span.start.line, span.start.column),
    }
}

fn write_parse_snippet(
    out: &mut impl fmt::Write,
    span: &SourceSpan,
    line_text: Option<&str>,
) -> fmt::Result {
    let Some(line) = line_text else {
        return Ok(());
    };

    writeln!(out, " |")?;
    writeln!(out, "{:>4} | {line}", span.start.line)?;
    writeln!(out, " | {}", render_caret(span.start.column))
}

fn render_caret(column: usize) -> String {
    let padding = " ".repeat(column.saturating_sub(1));
    format!("{padding}^")
}
