use super::utils::Value;
use crate::error::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedNode {
    /// The name of the native node (e.g., "Service")
    pub type_name: String,

    /// An optional ID for referencing this node in code
    pub id: Option<String>,

    /// Final, evaluated properties (no logic left, just data)
    #[cfg(feature = "metadata")]
    pub props: HashMap<String, (Value, Span)>,
    #[cfg(not(feature = "metadata"))]
    pub props: HashMap<String, Value>,

    /// Nested child nodes
    pub children: Vec<ResolvedNode>,

    /// Span metadata of the Node
    #[cfg(feature = "metadata")]
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedTree {
    pub root_nodes: Vec<ResolvedNode>,
}
