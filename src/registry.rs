//! Registry that holds core information for evaluation

use crate::ast::source::{ComponentDef, FnDef};
use crate::ast::utils::{NativeFnSchema, NativeNodeSchema, Type, Value};
use crate::error::Result;
use crate::library::Library;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::rc::Rc;
use std::fmt;

/// Registry containing important data about source.
#[derive(Default, Clone)]
pub struct Registry {
    /// Built-in nodes defined in Rust
    pub(crate) native_nodes: FxHashMap<String, NativeNodeSchema>,

    /// User-defined components from the .nbl file
    pub(crate) components: FxHashMap<String, ComponentDef>,

    /// Built-in functions defined in Rust
    pub(crate) native_functions: FxHashMap<String, NativeFnSchema>,

    /// User-defined functions from the .nbl file
    pub(crate) functions: FxHashMap<String, Rc<FnDef>>,

    /// Pre-evaluated global variables (regular name)
    pub(crate) globals: FxHashMap<String, Value>,

    /// All registered libraries
    pub(crate) libraries: Vec<Library>,

    /// Current file
    pub(crate) current_file: Option<PathBuf>,
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
    pub fn add_native_fn<F>(&mut self, name: &str, params: Vec<Type>, return_type: Type, f: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        let schema =
            NativeFnSchema { name: name.to_string(), params, return_type, body: Arc::new(f) };
        self.native_functions.insert(name.to_string(), schema);
    }

    pub fn add_node(&mut self, schema: NativeNodeSchema) {
        self.native_nodes.insert(schema.type_name.to_string(), schema);
    }

    pub fn register_component(&mut self, def: ComponentDef) {
        self.components.insert(def.name.clone(), def);
    }

    pub fn register_function(&mut self, def: FnDef) {
        self.functions.insert(def.name.clone(), Rc::new(def));
    }

    pub fn add_library(&mut self, library: Library) {
        let position = self.libraries.iter().position(|existing| existing.name == library.name);

        match position {
            Some(index) => {
                self.libraries[index] = library;
            }
            None => {
                self.libraries.push(library);
            }
        }
    }

    pub fn extend(&mut self, other: Registry) {
        self.components.extend(other.components);
        self.native_nodes.extend(other.native_nodes);
        self.native_functions.extend(other.native_functions);
        self.functions.extend(other.functions);
        self.globals.extend(other.globals);
        self.libraries.extend(other.libraries);
        if other.current_file.is_some() {
            self.current_file = other.current_file;
        }
    }
}
