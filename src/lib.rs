//! # NBCL - Node Based Configuration Language
//!
//! `nbcl` is a lightweight, declarative configuration DSL mainly designed for
//! defining UI components and cloud infrastructure. The syntax is designed to be simple,
//! and thus follows an HCL-inspired Blocky syntax but with the added benifits of
//! modularity, scripting capabilities, and simplicity.
//!
//! ## Example
//!
//! ```rust
//! use nbcl::NbclEngine;
//!
//! fn main() {
//!     let code = r#"
//!         print("Hello, World")
//!
//!         Object "language" {
//!             name = "nbcl"
//!             website = "nbcl-lang.github.io"
//!             playground = "nbcl-lang.github.io/playground"
//!             documnetation = "nbcl-lang.github.io/docs"
//!         }
//!     "#;
//!
//!     let engine = NbclEngine::new();
//!     match engine.parse_str(code) {
//!         Ok(ast) => {
//!             match engine.evaluate(ast) {
//!                 Ok(resolved) => {
//!                     println!("Resolved configuration: {:#?}", resolved);
//!                 }
//!                 Err(e) => println!("Evaluation error: {}", e)
//!             }
//!         }
//!         Err(e) => println!("Parse Error: {}", e)
//!     }
//! }
//! ```

pub mod ast;
pub mod error;
pub mod library;
pub mod module_resolver;

#[cfg(feature = "wasm")]
pub mod wasm;

mod engine;
mod utils;
mod evaluate;
mod builder;
mod builtin;
mod parser;
mod registry;

pub use engine::*;
pub use ast::utils::*;

/// Print a message that automatically goes to right buffer.
///
/// - If wasm: Goes to wasm buffer
/// - If not wasm: Goes to system buffer
pub fn print<S: AsRef<str> + std::fmt::Display>(name: S) {
    #[cfg(feature = "wasm")]
    crate::wasm::wasm_print(format!("{}", name));

    #[cfg(not(feature = "wasm"))]
    println!("{}", name);
}
