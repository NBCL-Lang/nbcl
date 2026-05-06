use crate::parser::Rule;
use pest::iterators::Pair;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
    pub slice: String,
}

impl Span {
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

#[derive(Debug)]
pub enum NbclError {
    Parse(Box<pest::error::Error<Rule>>),
    Ast { message: String, span: Option<Span> },
    IO {
        message: String,
        path: PathBuf,
    },
    Runtime { message: String, span: Option<Span> },
}

impl std::fmt::Display for NbclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NbclError::Parse(e) => write!(f, "Parsing failed:\n{}", e),
            NbclError::Ast { message, span } => {
                if let Some(s) = span {
                    write!(f, "Error at {}:{}: {}", s.line, s.col, message)
                } else {
                    write!(f, "Error: {}", message)
                }
            }
            NbclError::IO { message, path } => 
                write!(f, "IO error: {} at {}", message, path.display()),
            NbclError::Runtime { message, span } => {
                if let Some(s) = span {
                    write!(f, "Error at {}:{}: {}", s.line, s.col, message)
                } else {
                    write!(f, "Error: {}", message)
                }
            }
        }
    }
}

impl std::error::Error for NbclError {}
pub type Result<T> = std::result::Result<T, NbclError>;