//! NbclEngine API's

use crate::{
    ast::resolved::ResolvedTree,
    ast::source::File,
    ast::utils::{NativeNodeSchema, Type, Value},
    builder::build_file,
    context::Context,
    error::{NbclError, Result},
    evaluate::{Evaluator, Scope, ScopeKind, VariableBinding},
    library::Library,
    module_resolver::{FileModuleResolver, ModuleResolver},
    parser::{NbclParser, Rule},
    registry::Registry,
};
use pest::Parser;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::rc::Rc;

/// Nbcl Engine used for parsing and evaluation
#[derive(Debug, Clone)]
pub struct NbclEngine {
    registry: Registry,
    module_resolver: Rc<dyn ModuleResolver>,
    max_depth: usize,
}

impl NbclEngine {
    /// Create a new Nbcl Engine
    pub fn new() -> Self {
        let mut registry = Registry::default();
        crate::builtin::functions::register_builtin_functions(&mut registry);
        crate::builtin::libraries::register_builtin_functions(&mut registry);
        crate::builtin::nodes::register_builtin_nodes(&mut registry);

        // default module resolver follows relative path
        let module_resolver = Rc::new(FileModuleResolver::new(PathBuf::from(".")));

        Self { registry, module_resolver, max_depth: 5 }
    }

    /// Parse the a file into a source AST
    pub fn parse(&self, file_path: PathBuf) -> Result<File> {
        let source = fs::read_to_string(&file_path).map_err(|e| {
            let (msg, hint) = match e.kind() {
                ErrorKind::NotFound => {
                    let msg = format!("File not found: '{}'", file_path.display());
                    let hint = "Ensure that the file exists and try again".to_string();

                    (msg, Some(hint))
                }
                ErrorKind::PermissionDenied => {
                    let msg =
                        format!("Permission denied reading module: '{}'", file_path.display());
                    let hint = "Set proper file permissions".to_string();

                    (msg, Some(hint))
                }
                _ => {
                    let msg = format!("Failed to read module '{}': {}", file_path.display(), e);
                    (msg, None)
                }
            };

            NbclError::IO { message: msg, hint, path: file_path.clone() }
        })?;

        self.parse_str(&source)
    }

    /// Parse a source string into AST
    pub fn parse_str(&self, source: &str) -> Result<File> {
        #[cfg(feature = "pretty-errors")]
        crate::error::pretty_error::set_source(&source);

        let mut pairs = NbclParser::parse(Rule::file, source)?;

        let file_pair = pairs.next().ok_or_else(|| NbclError::Ast {
            message: "empty file".into(),
            hint: Some("Make sure your file contains at least one statement or expression.".into()),
            span: None,
        })?;

        build_file(file_pair)
    }

    /// Evaluate a source AST
    pub fn evaluate_ast(&self, file: File) -> Result<ResolvedTree> {
        let mut evaluator = Evaluator::new(
            self.registry.clone(),
            self.module_resolver.clone(),
            self.max_depth.clone(),
        );
        evaluator.run(file)
    }

    /// Evaluate a source AST and get the context
    pub fn evaluate_ast_for_ctx(&self, file: File) -> Result<(ResolvedTree, Context)> {
        let mut evaluator = Evaluator::new(
            self.registry.clone(),
            self.module_resolver.clone(),
            self.max_depth.clone(),
        );

        let tree = evaluator.run(file)?;
        let ctx = evaluator.return_context();

        Ok((tree, ctx))
    }

    /// Parse and evaluate a source string
    pub fn evaluate(&self, source: &str) -> Result<ResolvedTree> {
        let ast = self.parse_str(source)?;
        self.evaluate_ast(ast)
    }

    // === Registration API's ===

    /// Registers a custom node into the engine.
    pub fn register_node(&mut self, schema: NativeNodeSchema) {
        self.registry.add_node(schema);
    }

    /// Registers a native function into the engine.
    pub fn register_native_fn<F>(&mut self, name: &str, params: Vec<Type>, return_type: Type, f: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        self.registry.add_native_fn(name, params, return_type, f)
    }

    /// Register a library into the engine.
    pub fn register_library(&mut self, library: Library) {
        self.registry.add_library(library)
    }

    /// Add a global variable available to all scripts.
    pub fn set_global(&mut self, name: &str, value: Value) {
        self.registry.globals.insert(name.to_string(), value);
    }

    /// Register the module resolver for imports to work.
    pub fn register_module_resolver<M>(&mut self, mres: M)
    where
        M: ModuleResolver + 'static,
    {
        self.module_resolver = Rc::new(mres);
    }

    // === Other API ===

    /// Set maximum recursion depth
    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }

    /// Call an Nbcl function (including lambdas)
    pub fn call_function(&self, name: &str, args: Vec<Value>, ctx: &Context) -> Result<Value> {
        let mut evaluator =
            Evaluator::new(ctx.0.clone(), self.module_resolver.clone(), self.max_depth.clone());

        if let Some(user_fn) = ctx.functions.get(name) {
            let mut function_scope = Scope::new(ScopeKind::Function);

            if user_fn.params.len() != args.len() {
                return Err(NbclError::Ast {
                    message: format!(
                        "Argument count mismatch for function '{}'. Expected {}, got {}.",
                        name,
                        user_fn.params.len(),
                        args.len()
                    ),
                    hint: Some("Verify your parameter count match over the FFI boundary.".into()),
                    span: None,
                });
            }

            for (param, arg_value) in user_fn.params.iter().zip(args) {
                function_scope
                    .variables
                    .insert(param.clone(), VariableBinding { value: arg_value, is_const: false });
            }

            return evaluator.execute_fnitem_with_scope(&user_fn.body, function_scope);
        }

        Err(NbclError::Ast {
            message: format!("Function or Lambda identifier '{}' could not be resolved.", name),
            hint: Some("Ensure the function was registered or declared before invoking it.".into()),
            span: None,
        })
    }
}
