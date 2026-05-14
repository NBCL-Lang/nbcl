//! Module responsible for all the error handling in Nbcl

use crate::parser::Rule;
use pest::iterators::Pair;
use std::path::PathBuf;

#[cfg(feature = "pretty-errors")]
pub mod pretty_error {
    pub use ariadne::{Color, Config, Label, Report, ReportKind, Source};
    use std::cell::RefCell;

    #[cfg(feature = "pretty-errors")]
    thread_local! {
        static TEMP_SOURCE: RefCell<Option<String>> = RefCell::new(None);
    }

    pub fn set_source(source: &str) {
        TEMP_SOURCE.with(|s| *s.borrow_mut() = Some(source.to_string()));
    }

    pub fn get_source() -> Option<String> {
        TEMP_SOURCE.with(|s| s.borrow().clone())
    }

    pub fn clear_source() {
        TEMP_SOURCE.with(|s| *s.borrow_mut() = None);
    }
}

/// Start..end data that is useful for error reporting
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "metadata", derive(serde::Serialize, serde::Deserialize))]
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
        Self {
            start: pest_span.start(),
            end: pest_span.end(),
            line,
            col,
            slice: pest_span.as_str().to_string(),
        }
    }
    
    /// Create a dummy span
    pub fn dummy() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
            slice: String::new(),
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
    Parse { message: String, hint: Option<String>, span: Option<Span> },
    Ast { message: String, hint: Option<String>, span: Option<Span> },
    IO { message: String, hint: Option<String>, path: PathBuf },
    Runtime { message: String, hint: Option<String>, span: Option<Span> },
}

impl NbclError {
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Parse { span, .. } => span.clone(),
            Self::Ast { span, .. } => span.clone(),
            Self::IO { .. } => None,
            Self::Runtime { span, .. } => span.clone(),
        }
    }
}

fn rule_to_human(rule: Rule) -> &'static str {
    match rule {
        // === Operators ===
        Rule::or_op => "'||' (or)",
        Rule::and_op => "'&&' (and)",
        Rule::add_op => "'+' or '-'",
        Rule::mul_op => "'*', '/' or '%'",
        Rule::cmp_op => "a comparison operator ('==', '!=', '<=', '>=', '<', '>')",
        Rule::range_op => "a range operator ('..' or '..=')",
        Rule::dot => "'.'",
        Rule::safe_dot => "'?.'",
        Rule::accessor => "a field accessor ('.' or '?.')",

        // === Punctuation / Delimiters ===
        Rule::call_args => "function call arguments '(...)'",

        // === Identifiers ===
        Rule::snake_ident => "an identifier (snake_case)",
        Rule::pascal_ident => "a type or component name (PascalCase)",
        Rule::prop_key => "a property key",
        Rule::keyword => "a keyword",

        // === Literals ===
        Rule::literal => "a literal value",
        Rule::int_lit => "an integer literal",
        Rule::float_lit => "a float literal",
        Rule::bool_lit => "'true' or 'false'",
        Rule::null_lit => "'null'",
        Rule::string_lit => "a string literal",
        Rule::double_quoted_inner | Rule::single_quoted_inner => "string content",
        Rule::escape_seq => "an escape sequence",
        Rule::list_lit => "a list literal '[...]'",
        Rule::map_lit => "a map literal '{...}'",
        Rule::map_entry => "a map entry (key = value)",

        // === Expressions ===
        Rule::expr => "an expression",
        Rule::or_expr => "an expression",
        Rule::and_expr => "an expression",
        Rule::cmp_expr => "an expression",
        Rule::add_expr => "an expression",
        Rule::mul_expr => "an expression",
        Rule::unary_expr => "an expression",
        Rule::not_expr => "an expression",
        Rule::neg_expr => "an expression",
        Rule::postfix_expr => "an expression",
        Rule::primary_expr => "a value, identifier, or '('",
        Rule::range_expr => "a range expression (start..end)",
        Rule::if_expr => "an 'if' expression",
        Rule::else_if_branch => "'else if' branch",
        Rule::else_branch => "'else' branch",
        Rule::match_expr => "a 'match' expression",

        // === Lambda ===
        Rule::lambda_expr => "a lambda expression '|params| body'",
        Rule::lambda_param => "a lambda parameter",
        Rule::lambda_body => "a lambda body",
        Rule::block_body => "a block '{...}'",

        // === Type Hints ===
        Rule::type_hint => "a type hint (String, Int, Float, Bool, List, Map, Any)",

        // === Statements ===
        Rule::stmt => "a statement",
        Rule::node_stmt => "a statement",
        Rule::equal => "'=' statement",
        Rule::plus_equal => "'+=' statement",
        Rule::min_equal => "'-=' statement",
        Rule::mult_equal => "'*=' statement",
        Rule::div_equal => "'/=' statement",
        Rule::assignable_lhs => "a 'set' assignment variable",
        Rule::assignment_op => "a 'set' assignment operator",
        Rule::assign_stmt => "a 'set' assignment",
        Rule::local_stmt => "a 'local' declaration",
        Rule::global_stmt => "a 'global' declaration",
        Rule::for_stmt => "a 'for' loop",
        Rule::for_pattern => "a loop variable or destructure pattern '(k, v)'",
        Rule::while_stmt => "a 'while' loop",
        Rule::return_stmt => "a 'return' statement",
        Rule::expr_stmt => "an expression statement",

        // === Match ===
        Rule::match_arm => "a match arm (pattern => body)",
        Rule::match_pattern => "a match pattern (literal, identifier, or '_')",
        Rule::match_underscore => "a match arm underscore ('_') wildcard",

        // === Functions ===
        Rule::fn_def => "a function definition ('fn name(...) { ... }')",
        Rule::fn_param => "a function parameter",
        Rule::fn_body => "a function body '{...}'",
        Rule::fn_item => "a statement or node inside a function",

        // === Components ===
        Rule::component_def => "a component definition",
        Rule::component_params => "component parameters '(...)'",
        Rule::any_params => "an 'any' parameter capture '(any: props)'",
        Rule::named_params => "named parameters",
        Rule::param_item => "a parameter name",

        // === Nodes ===
        Rule::node_invocation => "a node invocation (e.g. 'Window { ... }')",
        Rule::id_expression => "a node ID (string or identifier)",
        Rule::node_block => "a node block '{...}'",
        Rule::node_item => "a node property, child node, or statement",
        Rule::node_prop => "a property assignment (key = value)",
        Rule::prop_value => "a property value",

        // === Imports ===
        Rule::import_stmt => "an import statement",
        Rule::import_lib_stmt => "a library import statement",

        // === Top Level ===
        Rule::file => "a source file",
        Rule::top_level_item => "a top-level definition or statement",

        // === Whitespace / Meta ===
        Rule::EOI => "end of file",
        Rule::WHITESPACE => "whitespace",
        Rule::COMMENT => "a comment",
        Rule::line_comment => "a line comment ('#')",
        Rule::block_comment => "a block comment ('#- ... -#')",

        // === Keywords ===
        Rule::in_kw => "'in' keyword",
        Rule::as_kw => "'as' keyword",
    }
}

/// Classify a set of expected rules into a human context.
fn classify_expectation(positives: &[Rule]) -> (&'static str, &'static str) {
    // Returns (what_was_expected_msg, hint)
    let has = |r: Rule| positives.contains(&r);

    let is_binary_op = has(Rule::or_op)
        || has(Rule::and_op)
        || has(Rule::add_op)
        || has(Rule::mul_op)
        || has(Rule::cmp_op);
    let is_postfix = has(Rule::accessor) || has(Rule::call_args);

    match (is_binary_op, is_postfix) {
        (true, true) => (
            "an operator or continuation of expression",
            "You likely have an incomplete expression. An operand is missing a right-hand side, or a statement is not terminated.",
        ),
        (true, false) => (
            "a binary operator",
            "Expected an operator to continue the expression ('and', 'or', '+', etc).",
        ),
        (false, true) => {
            ("a field access or call", "Expected '.' for field access or '(' for a function call.")
        }
        _ => ("an unexpected token", "Check your syntax, something here was not expected."),
    }
}

impl From<pest::error::Error<Rule>> for NbclError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        let (line, col) = match err.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c),
            pest::error::LineColLocation::Span((l, c), _) => (l, c),
        };
        let (start, end) = match err.location {
            pest::error::InputLocation::Pos(pos) => (pos, pos),
            pest::error::InputLocation::Span((s, e)) => (s, e),
        };

        let (message, hint) = match &err.variant {
            pest::error::ErrorVariant::ParsingError { positives, negatives } => {
                // Filter EOI and WHITESPACE noise from positives
                let clean_pos: Vec<Rule> = positives
                    .iter()
                    .copied()
                    .filter(|r| !matches!(r, Rule::EOI | Rule::WHITESPACE | Rule::COMMENT))
                    .collect();

                if clean_pos.is_empty() && negatives.is_empty() {
                    return NbclError::Parse {
                        message: "unexpected token".to_string(),
                        hint: Some("Nothing was expected here.".to_string()),
                        span: Some(Span { start, end, line, col, slice: String::new() }),
                    };
                }

                if !clean_pos.is_empty() {
                    let (ctx, hint) = classify_expectation(&clean_pos);

                    // If it's just one thing, be specific
                    let msg = if clean_pos.len() == 1 {
                        format!("expected {}", rule_to_human(clean_pos[0]))
                    } else {
                        // Deduplicate semantic groups: if all are operators, say "an operator"
                        format!("expected {}", ctx)
                    };

                    (msg, hint.to_string())
                } else {
                    let rejected: Vec<&str> = negatives.iter().map(|r| rule_to_human(*r)).collect();
                    (
                        format!("{} is not valid here", rejected.join(", ")),
                        "This construct is not allowed in this position.".to_string(),
                    )
                }
            }
            pest::error::ErrorVariant::CustomError { message } => {
                (message.clone(), "Check your syntax against the language grammar.".to_string())
            }
        };

        NbclError::Parse {
            message,
            hint: Some(hint),
            span: Some(Span { start, end, line, col, slice: String::new() }),
        }
    }
}

impl std::fmt::Display for NbclError {
    #[cfg(not(feature = "pretty-errors"))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NbclError::Parse { message, hint, span } => {
                format_diagnostic(f, "Parse Error", message, hint, span)
            }

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

    #[cfg(feature = "pretty-errors")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use pretty_error::*;

        let use_color = cfg!(not(feature = "wasm"));
        let config = Config::default().with_color(use_color);
        let source = pretty_error::get_source().unwrap_or(String::new());

        match self {
            NbclError::Parse { message, hint, span } => write_report(
                f,
                config,
                "Parse Error",
                "E001",
                message,
                hint,
                span,
                Color::Red,
                &source,
            ),
            NbclError::Ast { message, hint, span } => write_report(
                f,
                config,
                "Syntax Error",
                "E002",
                message,
                hint,
                span,
                Color::Yellow,
                &source,
            ),
            NbclError::Runtime { message, hint, span } => write_report(
                f,
                config,
                "Runtime Error",
                "E003",
                message,
                hint,
                span,
                Color::Magenta,
                &source,
            ),
            NbclError::IO { message, hint, path } => {
                use ariadne::Source;
                let mut report = Report::build(ReportKind::Error, 0..0)
                    .with_config(config)
                    .with_code("E004")
                    .with_message(format!("IO Error: {}", message))
                    .with_note(format!("Path: {}", path.display()));
                if let Some(h) = hint {
                    report = report.with_help(h);
                }
                let mut buf = Vec::new();
                report.finish().write(Source::from(""), &mut buf).map_err(|_| std::fmt::Error)?;
                write!(f, "{}", String::from_utf8_lossy(&buf))
            }
        }
    }
}

impl std::error::Error for NbclError {}
pub type Result<T> = std::result::Result<T, NbclError>;

// Helper to keep Ast and Runtime formatting identical and clean
#[cfg(not(feature = "pretty-errors"))]
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

#[cfg(feature = "pretty-errors")]
fn write_report(
    f: &mut std::fmt::Formatter<'_>,
    config: ariadne::Config,
    kind_label: &str,
    code: &str,
    message: &str,
    hint: &Option<String>,
    span: &Option<Span>,
    color: ariadne::Color,
    source: &str,
) -> std::fmt::Result {
    use pretty_error::*;
    let mut buf = Vec::new();

    if let Some(span) = span {
        let mut report = Report::build(ReportKind::Error, span.start..span.end)
            .with_config(config)
            .with_code(code)
            .with_message(format!("{}: {}", kind_label, message))
            .with_label(Label::new(span.start..span.end).with_message(message).with_color(color));
        if let Some(h) = hint {
            report = report.with_help(h);
        }
        report.finish().write(Source::from(source), &mut buf).map_err(|_| std::fmt::Error)?;
    } else {
        // No span available, plain report
        let mut report = Report::build(ReportKind::Error, 0..0)
            .with_config(config)
            .with_code(code)
            .with_message(format!("{}: {}", kind_label, message));
        if let Some(h) = hint {
            report = report.with_help(h);
        }
        report.finish().write(Source::from(""), &mut buf).map_err(|_| std::fmt::Error)?;
    }

    write!(f, "{}", String::from_utf8_lossy(&buf))
}
