mod import;
mod expr;
mod node;
mod stmt;

use crate::{
    ast::Value,
    ast::source::*,
    ast::resolved::ResolvedTree,
    registry::Registry,
    error::Result,
    module_resolver::FileModuleResolver,
};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

pub enum FlowControl {
    None,
    Return(Value),
}

pub(crate) struct Evaluator {
    registry: Registry,
    scopes: Vec<HashMap<String, Value>>,
    loaded_files: HashSet<PathBuf>,
    mod_resolver: Option<FileModuleResolver>,
    flow: FlowControl,
}

impl Evaluator {
    pub fn new(registry: Registry, mod_resolver: Option<FileModuleResolver>) -> Self {
        Self {
            registry,
            scopes: vec![HashMap::new()],
            loaded_files: HashSet::new(),
            mod_resolver,
            flow: FlowControl::None,
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
                TopLevelItem::Import(imp) => self.handle_import(imp.clone())?,
                TopLevelItem::ComponentDef(def) => self.register_component(def.clone()),
                TopLevelItem::FnDef(def) => self.register_function(def.clone()),
                TopLevelItem::Stmt(Stmt::Global(name, _, expr)) => {
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
                    self.execute_stmt(stmt)?;
                }
                _ => {} // Rest are already handled
            }
        }

        Ok(ResolvedTree { root_nodes })
    }

    fn register_component(&mut self, def: ComponentDef) {
        self.registry.components.insert(def.name.clone(), def);
    }

    fn register_function(&mut self, def: FnDef) {
        self.registry.functions.insert(def.name.clone(), def);
    }
}