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
//!     match engine.evaluate(code) {
//!         Ok(cfg) => println!("Resolved config: {:#?}", cfg),
//!         Err(e) => println!("Error: {}", e)
//!     }
//! }
//! ```

pub mod ast;
pub mod context;
pub mod error;
pub mod library;
pub mod module_resolver;

#[cfg(feature = "wasm")]
pub mod wasm;

mod builder;
mod builtin;
mod engine;
mod evaluate;
mod parser;
mod registry;
mod utils;

pub use ast::utils::*;
pub use engine::*;

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
