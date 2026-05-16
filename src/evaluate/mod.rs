//! Module that evaluates Nbcl source AST

mod expr;
mod import;
mod node;
mod stmt;

use crate::{
    ast::utils::Value, ast::resolved::ResolvedTree, ast::source::*, error::Result,
    module_resolver::ModuleResolver, registry::Registry,
};
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

// Todo: Maybe add break and continue
pub enum FlowControl {
    None,
    Return(Value),
}

/// Kinds of scopes
#[derive(PartialEq)]
pub enum ScopeKind {
    TopLevel,
    Block,     // if, for, while
    Function,  // fn, lambda
    Component, // Object
}

/// Internal structure used for scope handling
#[derive(PartialEq)]
pub(crate) struct Scope {
    pub variables: FxHashMap<String, Value>,
    pub kind: ScopeKind,
}

/// An internal structure that evaluates the source AST
pub(crate) struct Evaluator {
    registry: Registry,
    scopes: Vec<Scope>,
    loaded_files: HashSet<PathBuf>,
    module_resolver: Rc<dyn ModuleResolver>,
    flow: FlowControl,
    call_stack_depth: usize,
    max_depth: usize,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self { variables: FxHashMap::default(), kind }
    }
}

impl Evaluator {
    /// Create a new [`Evaluator`]
    pub fn new(
        registry: Registry,
        module_resolver: Rc<dyn ModuleResolver>,
        max_depth: usize,
    ) -> Self {
        Self {
            registry,
            scopes: vec![Scope::new(ScopeKind::TopLevel)],
            loaded_files: HashSet::new(),
            module_resolver,
            flow: FlowControl::None,
            call_stack_depth: 0,
            max_depth,
        }
    }

    /// Entry point for evaluation
    pub fn run(&mut self, file: File) -> Result<ResolvedTree> {
        let mut root_nodes = Vec::new();

        // In NBCL, developers should be able to use components/functions
        // that are defined after the lines where it is used.
        //
        // So for this feature, we only evaluate imports, globals,
        // components, and functions first. Then loop through the items
        // again to handle nodes and statements.
        //
        // This ensures that data is present first before using it anywhere.

        for item in &file.items {
            match item {
                TopLevelItem::Import(imp) => self.handle_import(imp.clone(), &mut root_nodes)?,
                TopLevelItem::ComponentDef(def) => self.registry.register_component(def.clone()),
                TopLevelItem::FnDef(def) => self.registry.register_function(def.clone()),
                TopLevelItem::Stmt(Stmt::Global(name, expr)) => {
                    // We evaluate globals now so they are available in Loop 2
                    let val = self.eval_expr(expr)?;
                    self.registry.globals.insert(name.clone(), val);
                }
                _ => {}
            }
        }

        for item in file.items {
            match item {
                TopLevelItem::Node(invocation) => {
                    let nodes = self.resolve_node(invocation)?;
                    root_nodes.extend(nodes);
                }
                TopLevelItem::Stmt(stmt) => {
                    self.execute_stmt(&stmt)?;
                }
                _ => {} // Rest are already handled
            }
        }

        Ok(ResolvedTree { root_nodes })
    }
}
