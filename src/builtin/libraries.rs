use crate::ast::{Type, Value};
use crate::registry::Registry;
use crate::library::{Library, LibraryItem};

/// Register a set of core libraries like stdllib.
pub(crate) fn register_builtin_functions(registry: &mut Registry) {
    // stdlib
    let math = LibraryItem::define("math")
        .with_global("PI", Value::Float(3.14))
        .with_fn("abs", vec![Type::Int], Type::Int, |args| {
            Ok(Value::Int(args[0].as_int().unwrap().abs()))
        });

    let std = Library::new("std".into(), vec![math]);

    registry.add_library(std);
}