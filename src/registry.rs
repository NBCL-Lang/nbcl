use std::collections::HashMap;
use crate::ast::source::{ComponentDef, FnDef};
use crate::ast::{Value, Type, NativeNodeSchema, NativeFnSchema};
use crate::error::Result;
use std::fmt;
use std::sync::Arc;

/// Registry containing important data about source.
#[derive(Default, Clone)]
pub struct Registry {
    /// Built-in nodes defined in Rust
    pub(crate) native_nodes: HashMap<String, NativeNodeSchema>,
    
    /// User-defined components from the .nbl file
    pub(crate) components: HashMap<String, ComponentDef>,

    /// Built-in functions defined in Rust
    pub(crate) native_functions: HashMap<String, NativeFnSchema>,

    /// User-defined functions from the .nbl file
    pub(crate) functions: HashMap<String, FnDef>,
    
    /// Pre-evaluated global variables
    pub(crate) globals: HashMap<String, Value>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("native_nodes", &self.native_nodes)
            .field("components", &self.components)
            .field("functions", &self.functions)
            .field("globals", &self.globals)
            .field("native_functions", &self.native_functions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Registry {
    pub fn add_native_fn<F>(
        &mut self, 
        name: &str, 
        params: Vec<Type>, 
        return_type: Type, 
        f: F
    ) where 
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static 
    {
        let schema = NativeFnSchema {
            name: name.to_string(),
            params,
            return_type,
            body: Arc::new(f),
        };
        self.native_functions.insert(name.to_string(), schema);
    }

    pub fn add_node(&mut self, schema: NativeNodeSchema) {
        self.native_nodes.insert(schema.type_name.to_string(), schema);
    }

    pub fn register_component(&mut self, def: ComponentDef) {
        self.components.insert(def.name.clone(), def);
    }

    pub fn register_function(&mut self, def: FnDef) {
        self.functions.insert(def.name.clone(), def);
    }
}