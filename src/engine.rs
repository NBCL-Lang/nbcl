use crate::{
    ast::Value,
    ast::source::{File, NativeNodeSchema},
    ast::resolved::ResolvedTree,
    error::{NbclError, Result},
    parser::{NbclParser, Rule},
    registry::Registry,
    evaluate::Evaluator,
    builder::build_file,
};
use pest::Parser;

pub struct NbclEngine {
    registry: Registry,
}

impl NbclEngine {
    pub fn new() -> Self {
        let mut registry = Registry::default();
        crate::builtin::functions::register_builtin_functions(&mut registry);

        Self {
            registry,
        }
    }

    /// Parse the source into a source AST
    pub fn parse(&self, source: &str) -> Result<File> {
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
        let mut evaluator = Evaluator::new(self.registry.clone());
        evaluator.run(file)
    }

    // === Registration API's === 

    /// Registers a custom node into the engine.
    pub fn register_node(&mut self, name: &str, schema: NativeNodeSchema) {
        self.registry.native_nodes.insert(name.to_string(), schema);
    }

    /// Add a global variable available to all scripts.
    pub fn set_global(&mut self, name: &str, value: Value) {
        self.registry.globals.insert(name.to_string(), value);
    }
}