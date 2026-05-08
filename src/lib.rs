//! # NBCL - Node Based Configuration Language
//!
//! `nbcl` is a lightweight, declarative configuration DSL mainly designed for 
//! defining UI components and cloud infrastructure. The syntax is designed to be simple,
//! and thus follows an HCL-inspired Blocky syntax but with the added benifits of
//! modularity, scripting capabilities, and simplicity.

pub mod ast;
pub mod builder;
pub mod builtin;
pub mod error;
pub mod evaluate;
pub mod module_resolver;
pub mod parser;
pub mod registry;
pub mod utils;
pub mod wasm;

mod engine;
pub use engine::*;