use super::Evaluator;
use crate::ast::source::*;
use crate::error::{NbclError, Result};
use crate::parser::NbclParser;
use crate::parser::Rule;
use std::io::ErrorKind;
use pest::Parser;
use std::fs;

impl Evaluator {
    pub(crate) fn handle_import(&mut self, imp: ImportDef) -> Result<()> {
        match imp.def {
            ImportDefType::Module(path_str, alias) => {
                let target_path = match &self.mod_resolver {
                    Some(r) => r.find_target(&path_str),
                    None => {
                        return Err(NbclError::Runtime {
                            message: "Module resolver is not registered.".into(),
                            hint: Some("Looks like the developer messed with the module resolver... You'd have to stick with a singular crate for now.".to_string()),
                            span: Some(imp.span),
                        })
                    }
                }?;

                // Avoiding circular imports
                if self.loaded_files.contains(&target_path) {
                    return Ok(());
                }

                // Read and Parse
                let source = fs::read_to_string(&target_path).map_err(|e| {
                    let (msg, hint) = match e.kind() {
                        ErrorKind::NotFound => {
                            let msg = format!("Module not found: '{}'", target_path.display());
                            let hint =
                                "Ensure that the module exists and try adjusting the path.".to_string();

                            (msg, Some(hint))
                        }
                        ErrorKind::PermissionDenied => {
                            let msg =
                                format!("Permission denied reading module: '{}'", target_path.display());
                            let hint = "Set proper file permissions".to_string();

                            (msg, Some(hint))
                        }
                        _ => {
                            let msg = format!("Failed to read module '{}': {}", target_path.display(), e);
                            (msg, None)
                        }
                    };

                    NbclError::IO { message: msg, hint, path: target_path.clone() }
                })?;

                let mut tokens = NbclParser::parse(Rule::file, &source)
                    .map_err(|e| NbclError::Parse(Box::new(e)))?;

                let file_pair = tokens.next().ok_or_else(|| NbclError::Ast {
                    message: "Empty file".into(),
                    hint: None,
                    span: None,
                })?;

                let ast = crate::builder::build_file(file_pair)?;

                for item in ast.items {
                    match item {
                        TopLevelItem::FnDef(mut f) => {
                            f.name = format!("{}.{}", alias, f.name);
                            self.registry.register_function(f);
                        }
                        TopLevelItem::ComponentDef(mut c) => {
                            c.name = format!("{}.{}", alias, c.name);
                            self.registry.register_component(c);
                        }
                        _ => {} // Skip top-level statements/imports in the target file for now
                    }
                }

                self.loaded_files.insert(target_path);
                Ok(())
            }
            ImportDefType::Library(lib_name) => {
                // This is for built-in libraries (e.g., import std)
                todo!("Load internal library components")
            }
        }
    }
}
