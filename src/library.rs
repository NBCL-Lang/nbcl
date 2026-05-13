//! API's for registering new Library that can be imported and used
use crate::ast::utils::{NativeFnSchema, Type, Value};
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Library {
    /// Name of the library (e.g. stdlib)
    pub(crate) name: String,
    /// Items within this library
    pub(crate) items: Vec<LibraryItem>,
}

impl Library {
    /// Create a new Library
    pub fn new(name: String, items: Vec<LibraryItem>) -> Self {
        Self { name, items }
    }

    /// Get the name of the library
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Set the name of the library
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Get all the library item
    pub fn items(&self) -> Vec<LibraryItem> {
        self.items.clone()
    }

    /// Get a library item from name
    pub fn get_item(&self, name: String) -> Option<LibraryItem> {
        self.items.iter().find(|&i| i.name == name).cloned()
    }

    /// Add a new library item
    pub fn add_item(&mut self, item: LibraryItem) {
        let position = self.items.iter().position(|existing| existing.name == item.name);

        match position {
            Some(index) => {
                self.items[index] = item;
            }
            None => {
                self.items.push(item);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LibraryItem {
    /// Name of the Library item
    pub(crate) name: String,
    /// All registered native functions
    pub(crate) native_functions: HashMap<String, NativeFnSchema>,
    /// All registered globals
    pub(crate) globals: HashMap<String, Value>,
}

impl LibraryItem {
    /// Internal create new library method
    fn new(name: String) -> Self {
        Self { name, native_functions: HashMap::new(), globals: HashMap::new() }
    }

    /// Start a new item definition
    pub fn define(name: impl Into<String>) -> Self {
        Self::new(name.into())
    }

    /// Chainable function registration
    pub fn with_fn<F>(mut self, name: &str, params: Vec<Type>, ret: Type, f: F) -> Self
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        self.add_native_fn(name, params, ret, f);
        self
    }

    /// Chainable global registration
    pub fn with_global(mut self, name: &str, value: Value) -> Self {
        self.set_global(name, value);
        self
    }

    /// Get the name of the library item
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Set the name of the library item
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Register a native function into the library item
    pub fn add_native_fn<F>(&mut self, name: &str, params: Vec<Type>, return_type: Type, f: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        let schema =
            NativeFnSchema { name: name.to_string(), params, return_type, body: Arc::new(f) };
        self.native_functions.insert(name.to_string(), schema);
    }

    /// Add and set a value of a global variable
    pub fn set_global(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_string(), value);
    }
}
