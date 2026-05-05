pub mod source;
pub mod resolved;
use std::fmt;

// TODO: Maybe add `Any` value
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<Value>),
    Map(Vec<(String, Value)>),
    Nodes(Vec<resolved::ResolvedNode>),
    Null,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Null => write!(f, "null"),
            Value::List(items) => {
                let parts: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Value::Map(entries) => {
                let parts: Vec<String> = entries.iter()
                    .map(|(k, v)| format!("{} = {}", k, v))
                    .collect();
                write!(f, "{{{}}}", parts.join(", "))
            }
            Value::Nodes(_) => write!(f, "<nodes>"),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Str(_) => "String",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Nodes(_) => "Nodes",
            Value::Null => "Null",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    List,
    Map,
    Nodes,
    Any,
    Null,
}

impl Type {
    /// Convert a string hint into a Type enum
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Int" => Some(Type::Int),
            "Float" => Some(Type::Float),
            "Bool" => Some(Type::Bool),
            "String" => Some(Type::Str),
            "List" => Some(Type::List),
            "Map" => Some(Type::Map),
            "Nodes" => Some(Type::Nodes),
            "Null" => Some(Type::Null),
            _ => None,
        }
    }

    /// The core validation method
    pub fn matches_value(&self, value: &Value) -> bool {
        if matches!(self, Type::Any) {
            return true;
        }

        match (self, value) {
            (Type::Int, Value::Int(_)) => true,
            (Type::Float, Value::Float(_)) => true,
            (Type::Bool, Value::Bool(_)) => true,
            (Type::Str, Value::Str(_)) => true,
            (Type::List, Value::List(_)) => true,
            (Type::Map, Value::Map(_)) => true,
            (Type::Nodes, Value::Nodes(_)) => true,
            (Type::Null, Value::Null) => true,
            _ => false,
        }
    }
}