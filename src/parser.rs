//! Pest based Nbcl parser
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct NbclParser;
