use crate::ast::utils::{Type, Value};
use crate::library::{Library, LibraryItem};
use crate::registry::Registry;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Register a set of core libraries like stdllib.
pub(crate) fn register_builtin_functions(registry: &mut Registry) {
    // === Math Library ===
    let math = LibraryItem::define("math")
        .with_global("PI", Value::Float(std::f64::consts::PI))
        .with_global("E", Value::Float(std::f64::consts::E))
        .with_global("SQRT2", Value::Float(std::f64::consts::SQRT_2))
        .with_global("SQRT1_2", Value::Float(std::f64::consts::FRAC_1_SQRT_2))
        .with_global("LN2", Value::Float(std::f64::consts::LN_2))
        .with_global("LN_10", Value::Float(std::f64::consts::LN_10))
        // arithmetic
        .with_fn("abs", vec![Type::Int], Type::Int, |args| {
            Ok(Value::Int(args[0].as_int().unwrap().abs()))
        })
        .with_fn("sqrt", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().sqrt()))
        })
        .with_fn("cbrt", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().cbrt()))
        })
        .with_fn("pow", vec![Type::Float, Type::Float], Type::Float, |args| {
            let base = args[0].as_float().unwrap();
            let exp = args[1].as_float().unwrap();
            Ok(Value::Float(base.powf(exp)))
        })
        // rounding
        .with_fn("floor", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().floor()))
        })
        .with_fn("ceil", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().ceil()))
        })
        .with_fn("round", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().round()))
        })
        .with_fn("trunc", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().trunc()))
        })
        // trigonometry
        .with_fn("sin", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().sin()))
        })
        .with_fn("cos", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().cos()))
        })
        .with_fn("tan", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().tan()))
        })
        .with_fn("asin", vec![Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().asin()))
        })
        .with_fn("atan2", vec![Type::Float, Type::Float], Type::Float, |args| {
            Ok(Value::Float(args[0].as_float().unwrap().atan2(args[1].as_float().unwrap())))
        });

    // === Time Library ===
    let time = LibraryItem::define("time")
        .with_fn("now", vec![], Type::Float, |_| {
            let now =
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
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
