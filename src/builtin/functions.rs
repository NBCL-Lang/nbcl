use crate::ast::{Type, Value};
use crate::error::NbclError;
use crate::registry::Registry;

/// Register a set of core functions like print.
pub(crate) fn register_builtin_functions(registry: &mut Registry) {
    // print(Any) -> Null
    registry.add_native_fn("print", vec![Type::Any], Type::Null, |args| {
        println!("{}", args[0]);
        Ok(Value::Null)
    });

    // to_string(Any) -> Str
    registry.add_native_fn("to_string", vec![Type::Any], Type::Str, |args| {
        Ok(Value::Str(args[0].to_string()))
    });

    // type_of(Any) -> Str
    registry.add_native_fn("type_of", vec![Type::Any], Type::Str, |args| {
        Ok(Value::Str(args[0].type_name().to_string()))
    });

    // len(Any) -> Int
    registry.add_native_fn("len", vec![Type::Any], Type::Int, |args| {
        let length = match &args[0] {
            Value::Str(s) => s.len() as i64,
            Value::List(l) => l.len() as i64,
            Value::Map(m) => m.len() as i64,
            _ => {
                return Err(NbclError::Runtime {
                    message: format!("len() not supported for type {}", args[0].type_name()),
                    hint: None,
                    span: None,
                });
            }
        };
        Ok(Value::Int(length))
    });

    // assert(Bool, Str) -> Null
    registry.add_native_fn("assert", vec![Type::Bool, Type::Str], Type::Null, |args| {
        let condition = match args[0] {
            Value::Bool(b) => b,
            _ => unreachable!(),
        };
        let msg = match &args[1] {
            Value::Str(s) => s,
            _ => unreachable!(),
        };
        if !condition {
            return Err(NbclError::Runtime {
                message: format!("Assertion Failed: {}", msg),
                hint: None,
                span: None,
            });
        }
        Ok(Value::Null)
    });
}
