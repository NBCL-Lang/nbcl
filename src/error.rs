use crate::parser::Rule;
use pest::iterators::Pair;
use std::path::PathBuf;

/// Start..end data that is useful for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
    pub slice: String,
}

impl Span {
    /// Create span from a [`pest::iterators::Pair`].
    pub fn from_pair(pair: &Pair<Rule>) -> Self {
        let pest_span = pair.as_span();
        let (line, col) = pest_span.start_pos().line_col();
        Span {
            start: pest_span.start(),
            end: pest_span.end(),
            line,
            col,
            slice: pest_span.as_str().to_string(),
        }
    }
}

/// Custom error format used throughout the crate
///
/// Error style to preserve: 
/// - message: must not end with fullstop or punctuation, and must be all lowercase
/// - hint: Must end with fullstop or punctuation, and must start with a capital letter.
#[derive(Debug)]
pub enum NbclError {
    Parse(Box<pest::error::Error<Rule>>),
    Ast { message: String, hint: Option<String>, span: Option<Span> },
    IO { message: String, hint: Option<String>, path: PathBuf },
    Runtime { message: String, hint: Option<String>, span: Option<Span> },
}

impl std::fmt::Display for NbclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NbclError::Parse(e) => write!(f, "Parsing failed:\n{}", e),

            NbclError::Ast { message, hint, span } => {
                format_diagnostic(f, "Syntax Error", message, hint, span)
            }

            NbclError::Runtime { message, hint, span } => {
                format_diagnostic(f, "Runtime Error", message, hint, span)
            }

            NbclError::IO { message, hint, path } => {
                writeln!(f, "[IO Error] {}", message)?;
                writeln!(f, "  Path: {}", path.display())?;
                if let Some(h) = hint {
                    write!(f, "  Hint: {}", h)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for NbclError {}
pub type Result<T> = std::result::Result<T, NbclError>;

// Helper to keep Ast and Runtime formatting identical and clean
fn format_diagnostic(
    f: &mut std::fmt::Formatter<'_>,
    label: &str,
    message: &str,
    hint: &Option<String>,
    span: &Option<Span>,
) -> std::fmt::Result {
    if let Some(s) = span {
        writeln!(f, "[{}] at {}:{}: {}", label, s.line, s.col, message)?;
    } else {
        writeln!(f, "[{}] {}", label, message)?;
    }

    if let Some(h) = hint {
        write!(f, "  Hint: {}", h)?;
    }
    Ok(())
}
