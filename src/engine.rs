use crate::{
    ast::{Value, Type, NativeNodeSchema},
    ast::source::File,
    ast::resolved::ResolvedTree,
    error::{NbclError, Result},
    parser::{NbclParser, Rule},
    registry::Registry,
    evaluate::Evaluator,
    builder::build_file,
    module_resolver::FileModuleResolver,
};
use std::io::ErrorKind;
use std::path::PathBuf;
use pest::Parser;
use std::fs;

/// Nbcl Engine used for parsing and evaluation
#[derive(Debug, Clone)]
pub struct NbclEngine {
    registry: Registry,
    mod_resolver: Option<FileModuleResolver>,
}

impl NbclEngine {
    /// Create a ew Nbcl Engine
    pub fn new() -> Self {
        let mut registry = Registry::default();
        crate::builtin::functions::register_builtin_functions(&mut registry);
        crate::builtin::nodes::register_builtin_nodes(&mut registry);

        // default module resolver follows relative path
        let mod_resolver = FileModuleResolver::new(PathBuf::from("."));

        Self {
            registry,
            mod_resolver: Some(mod_resolver),
        }
    }

    /// Parse the a file into a source AST
    pub fn parse(&self, file_path: PathBuf) -> Result<File> {
        let source = fs::read_to_string(&file_path).map_err(|e| {
            let msg = match e.kind() {
                ErrorKind::NotFound => format!("Module not found: '{}'", file_path.display()),
                ErrorKind::PermissionDenied => format!("Permission denied reading module: '{}'", file_path.display()),
                _ => format!("Failed to read module '{}': {}", file_path.display(), e),
            };

            NbclError::IO {
                message: msg,
                path: file_path.clone(),
            }
        })?;

        self.parse_str(&source)
    }

    /// Parse a source string into AST
    pub fn parse_str(&self, source: &str) -> Result<File> {
        let mut pairs = NbclParser::parse(Rule::file, source)
            .map_err(|e| NbclError::Parse(Box::new(e)))?;

        let file_pair = pairs.next().ok_or_else(|| NbclError::Ast {
            message: "Empty file".into(),
            span: None,
        })?;

        build_file(file_pair)
    }

    /// Evaluate a source AST
    pub fn evaluate(&self, file: File) -> Result<ResolvedTree> {
        let mut evaluator = Evaluator::new(self.registry.clone(), self.mod_resolver.clone());
        evaluator.run(file)
    }

    // === Registration API's === 

    /// Registers a custom node into the engine.
    pub fn register_node(&mut self, schema: NativeNodeSchema) {
        self.registry.add_node(schema);
    }

    /// Registers a native function into the engine.
    pub fn register_native_fn<F>(
        &mut self, 
        name: &str, 
        params: Vec<Type>, 
        return_type: Type, 
        f: F
    ) where 
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static 
    {
        self.registry.add_native_fn(name, params, return_type, f)
    }

    /// Add a global variable available to all scripts.
    pub fn set_global(&mut self, name: &str, value: Value) {
        self.registry.globals.insert(name.to_string(), value);
    }

    /// Register the module resolver for imports to work.
    pub fn register_module_resolver(&mut self, mres: FileModuleResolver) {
        self.mod_resolver = Some(mres);
    }
}