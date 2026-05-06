use super::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedNode {
    /// The name of the native node (e.g., "Service")
    pub type_name: String,

    /// An optional ID for referencing this node in code
    pub id: Option<String>,

    /// Final, evaluated properties (no logic left, just data)
    pub props: HashMap<String, Value>,

    /// Nested child nodes
    pub children: Vec<ResolvedNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedTree {
    pub root_nodes: Vec<ResolvedNode>,
}
