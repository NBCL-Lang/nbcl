use crate::ast::utils::{Type, Value};
use crate::error::NbclError;
use crate::registry::Registry;

/// Register a set of core functions like print.
pub(crate) fn register_builtin_functions(registry: &mut Registry) {
    // == basic functions == //
    // print(Any) -> Null
    registry.add_native_fn("print", vec![Type::Any], Type::Null, |args| {
        crate::print(format!("{}", args[0]));
        Ok(Value::Null)
    });

    // to_string(Any) -> Str
    registry.add_native_fn("to_string", vec![Type::Any], Type::Str, |args| {
        Ok(Value::Str(args[0].to_string()))
    });

    // as_int(Any) -> Int
    registry.add_native_fn("as_int", vec![Type::Any], Type::Int, |args| match &args[0] {
        Value::Str(s) => {
            let parsed = match s.parse::<i64>() {
                Ok(p) => p,
                Err(_) => {
                    return Err(NbclError::Runtime {
                        message: "cannot parse string into int inside as_int()".to_string(),
                        hint: None,
                        span: None,
                    })
                }
            };
            Ok(Value::Int(parsed))
        },
        Value::Float(f) => Ok(Value::Int(*f as i64)),
        Value::Int(i) => Ok(Value::Int(*i)),
        _ => {
            return Err(NbclError::Runtime {
                message: format!("as_int() not supported for type {}", args[0].type_name()),
                hint: None,
                span: None,
            });
        }
    });

    // as_float(Any) -> Float
    registry.add_native_fn("as_float", vec![Type::Any], Type::Float, |args| {
        match &args[0] {
            Value::Str(s) => {
                let parsed = match s.parse::<f64>() {
                    Ok(p) => p,
                    Err(_) => {
                        return Err(NbclError::Runtime {
                            message: "cannot parse string into int inside as_int()".to_string(),
                            hint: None,
                            span: None,
                        })
                    }
                };
                Ok(Value::Float(parsed))
            },
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::Float(f) => Ok(Value::Float(*f)),
            _ => {
                return Err(NbclError::Runtime {
                    message: format!("as_float() not supported for type {}", args[0].type_name()),
                    hint: None,
                    span: None,
                });
            }
        }
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

    // contains(List/Str, Any) -> Bool
    registry.add_native_fn(
        "contains",
        vec![Type::Any, Type::Any],
        Type::Bool,
        |args| match &args[0] {
            Value::List(v) => Ok(Value::Bool(v.contains(&args[1]))),
            Value::Str(s) => match &args[1] {
                Value::Str(sub) => Ok(Value::Bool(s.contains(sub.as_str()))),
                _ => Ok(Value::Bool(false)),
            },
            _ => Err(NbclError::Runtime {
                message: format!("contains() not supported for type {}", args[0].type_name()),
                hint: None,
                span: None,
            }),
        },
    );

    // == list functions == //

    // push(List, Any) -> List
    registry.add_native_fn("push", vec![Type::List, Type::Any], Type::List, |mut args| {
        let val = args.remove(1);
        match args.remove(0) {
            Value::List(mut v) => {
                v.push(val);
                Ok(Value::List(v))
            }
            _ => unreachable!(),
        }
    });

    // pop(List) -> Any
    registry.add_native_fn("pop", vec![Type::List], Type::Any, |mut args| match args.remove(0) {
        Value::List(mut v) => Ok(v.pop().unwrap_or(Value::Null)),
        _ => unreachable!(),
    });

    // == map functions == //

    // keys(Map) -> List
    registry.add_native_fn("keys", vec![Type::Map], Type::List, |args| match &args[0] {
        Value::Map(m) => Ok(Value::List(m.iter().map(|(k, _)| Value::Str(k.clone())).collect())),
        _ => unreachable!(),
    });

    // values(Map) -> List
    registry.add_native_fn("values", vec![Type::Map], Type::List, |args| match &args[0] {
        Value::Map(m) => Ok(Value::List(m.iter().map(|(_, v)| v.clone()).collect())),
        _ => unreachable!(),
    });

    // == string function == //
    registry.add_native_fn("to_lowercase", vec![Type::Str], Type::Str, |args| match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
        _ => unreachable!(),
    });

    registry.add_native_fn("to_uppercase", vec![Type::Str], Type::Str, |args| match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
        _ => unreachable!(),
    });

    registry.add_native_fn("trim", vec![Type::Str], Type::Str, |args| match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
        _ => unreachable!(),
    });

    registry.add_native_fn("trim_start", vec![Type::Str], Type::Str, |args| match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.trim_start().to_string())),
        _ => unreachable!(),
    });

    registry.add_native_fn("trim_end", vec![Type::Str], Type::Str, |args| match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.trim_end().to_string())),
        _ => unreachable!(),
    });

    registry.add_native_fn("split", vec![Type::Str, Type::Any], Type::List, |args| {
        match &args[0] {
            Value::Str(string) => match &args[1] {
                Value::Str(pattern) => {
                    let values: Vec<Value> =
                        string.split(pattern).map(|s| Value::Str(s.to_string())).collect();

                    Ok(Value::List(values))
                }
                Value::Null => {
                    let values: Vec<Value> =
                        string.split_whitespace().map(|s| Value::Str(s.to_string())).collect();

                    Ok(Value::List(values))
                }
                _ => Err(NbclError::Runtime {
                    message: format!("split must receive a parameter"),
                    hint: None,
                    span: None,
                }),
            },
            _ => unreachable!(),
        }
    });
}
