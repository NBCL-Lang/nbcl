use crate::ast::{Type, Value};
use crate::library::{Library, LibraryItem};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use crate::registry::Registry;

/// Register a set of core libraries like stdllib.
pub(crate) fn register_builtin_functions(registry: &mut Registry) {
    // === Math Library ===
    let math = LibraryItem::define("math")
        .with_global("pi", Value::Float(std::f64::consts::PI))
        .with_global("e", Value::Float(std::f64::consts::E))
        .with_fn("abs", vec![Type::Int], Type::Int, |args| {
            Ok(Value::Int(args[0].as_int().unwrap().abs()))
        })
        .with_fn("sqrt", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().sqrt()))
        })
        .with_fn("pow", vec![Type::Float, Type::Float], Type::Float, |args| {
            let base = args[0].as_float().unwrap();
            let exp = args[1].as_float().unwrap();
            Ok(Value::Float(base.powf(exp)))
        });

    // === Time Library ===
    let time = LibraryItem::define("time")
        .with_fn("now", vec![], Type::Float, |_| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            Ok(Value::Float(now))
        })
        .with_fn("mark", vec![], Type::Float, |_| {
            static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
            let start = START.get_or_init(Instant::now);
            Ok(Value::Float(start.elapsed().as_secs_f64()))
        })
        .with_fn("elapsed", vec![Type::Float], Type::Float, |args| {
            static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
            let start = START.get_or_init(Instant::now);
            let prev_mark = args[0].as_float().unwrap();
            let current = start.elapsed().as_secs_f64();
            Ok(Value::Float(current - prev_mark))
        })
        .with_fn("sleep", vec![Type::Int], Type::Null, |args| {
            let ms = args[0].as_int().unwrap() as u64;
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(Value::Null)
        });

    let std = Library::new("std".into(), vec![math, time]);
    registry.add_library(std);
}