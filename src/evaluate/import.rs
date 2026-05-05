use crate::ast::source::*;
use crate::error::Result;
use super::Evaluator;

impl Evaluator {
    pub(crate) fn handle_import(&self, imp: ImportDef) -> Result<()> {
        match imp {
            ImportDef::Module(path, alias) => {
                // 1. Resolve file path relative to current file
                // 2. Parse the target file
                // 3. Hoist its components/functions into the current registry
                // (Or a namespaced sub-registry)
                todo!("Implement recursive file loading")
            }
            ImportDef::Library(lib_name) => {
                // This is for built-in libraries (e.g., import std)
                todo!("Load internal library components")
            }
        }
    }
}
