pub mod resolved;
pub mod source;
use crate::error;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Possible data types in Nbcl (used interanally to hold value)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Value {
    /// Integers (1)
    Int(i64),
    /// Floating numbers (1.5)
    Float(f64),
    /// Boolean (true/false)
    Bool(bool),
    /// String ("Hello, World")
    Str(String),
    /// List [1, 2, 3]
    List(Vec<Value>),
    /// Map { key: value  }
    Map(Vec<(String, Value)>),
    /// Regular Nodes
    Nodes(Vec<resolved::ResolvedNode>),
    /// Null (no data)
    Null,
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Int(v) => s.serialize_i64(*v),
            Value::Float(v) => s.serialize_f64(*v),
            Value::Bool(v) => s.serialize_bool(*v),
            Value::Str(v) => s.serialize_str(v),
            Value::Null => s.serialize_none(),
            Value::List(v) => {
                let mut seq = s.serialize_seq(Some(v.len()))?;
                for item in v {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Value::Map(v) => {
                let mut map = s.serialize_map(Some(v.len()))?;
                for (k, val) in v {
                    map.serialize_entry(k, val)?;
                }
                map.end()
            }
            Value::Nodes(v) => {
                let mut seq = s.serialize_seq(Some(v.len()))?;
                for item in v {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
        }
    }
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
                let parts: Vec<String> =
                    entries.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
                write!(f, "{{{}}}", parts.join(", "))
            }
            Value::Nodes(_) => write!(f, "<nodes>"),
        }
    }
}

impl Value {
    /// Check whether the value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            _ => true,
        }
    }

    /// Convert the Value into its Type name.
    /// Example: Value::Int(_) -> "Int"
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

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }
}

/// Possible data types in Nbcl (used for type hints)
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    List,
    Map,
    Nodes,
    /// Additional constant that
    /// symbolizes all data types.
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

/// Defines a host-provided node
#[derive(Debug, Clone)]
pub enum PropValidation {
    /// Allow any properties
    Loose,
    /// Only allow specific keys
    Strict(HashMap<String, Type>),
}

// == schemas ==

/// Public structure used for registering custom nodes.
#[derive(Debug, Clone)]
pub struct NativeNodeSchema {
    /// Name of the Node
    pub type_name: String,
    /// Whether to enforce ID or not
    pub enforce_id: bool,
    /// Whether the property validation should be loose or strict
    pub validation: PropValidation,
    /// Children count in <(min, max)>.
    /// Use None for default functionlaity
    /// (allows any number of children).
    pub child_count: Option<(u32, u32)>,
}

/// Internal structure used for registering custom functions.
#[derive(Clone)]
pub struct NativeFnSchema {
    pub(crate) name: String,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) body: Arc<dyn Fn(Vec<Value>) -> error::Result<Value> + Send + Sync>,
}

impl fmt::Debug for NativeFnSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFnSchema")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("return_type", &self.return_type)
            .field("body", &"<native function>")
            .finish()
    }
}
