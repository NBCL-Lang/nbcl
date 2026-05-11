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
                            message: "module resolver is not registered".into(),
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
                            let msg = format!("module not found: '{}'", target_path.display());
                            let hint =
                                "Ensure that the module exists and try adjusting the path.".to_string();

                            (msg, Some(hint))
                        }
                        ErrorKind::PermissionDenied => {
                            let msg =
                                format!("permission denied reading module: '{}'", target_path.display());
                            let hint = "Set proper file permissions".to_string();

                            (msg, Some(hint))
                        }
                        _ => {
                            let msg = format!("failed to read module '{}': {}", target_path.display(), e);
                            (msg, None)
                        }
                    };

                    NbclError::IO { message: msg, hint, path: target_path.clone() }
                })?;

                let mut tokens = NbclParser::parse(Rule::file, &source)?;

                let file_pair = tokens.next().ok_or_else(|| NbclError::Ast {
                    message: "empty file".into(),
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
            ImportDefType::Library(lib_name, lib_item) => {
                let maybe_library = self.registry.libraries.iter()
                    .find(|&lib| lib.name == lib_name);

                let library = match maybe_library {
                    Some(lib) => lib,
                    None => {
                        let library_names = self.registry.libraries.iter()
                            .map(|lib| lib.name.clone())
                            .collect::<Vec<String>>();

                        let suggestion = 
                            crate::utils::find_best_match(&lib_name, library_names.iter());

                        let hint = suggestion.map(|s|
                            format!("Library \"{}\" doesn't exist. Did you mean \"{}\"?", &lib_name, s)
                        );

                        return Err(NbclError::Runtime {
                            message: "library not found".into(),
                            hint,
                            span: Some(imp.span),
                        })
                    }
                };

                let maybe_lib_item = library.items.iter()
                    .find(|&i| i.name == lib_item);

                let item = match maybe_lib_item {
                    Some(i) => i,
                    None => {
                        let item_names = library.items.iter()
                            .map(|item| item.name.clone())
                            .collect::<Vec<String>>();

                        let suggestion = 
                            crate::utils::find_best_match(&lib_item, item_names.iter());

                        let hint = suggestion.map(|s|
                            format!("Library item \"{}\" doesn't exist. Did you mean \"{}\"?", &lib_item, s)
                        );

                        return Err(NbclError::Runtime {
                            message: format!("library item '{}' not found", &lib_item),
                            hint,
                            span: Some(imp.span),
                        })
                    }
                };

                for (fn_name, schema) in item.native_functions.clone() {
                    let new_name = format!("{}.{}", &lib_item, fn_name);
                    self.registry.native_functions.insert(new_name, schema);
                }

                for (name, var) in item.globals.clone() {
                    let new_name = format!("{}.{}", &lib_item, name);
                    self.registry.globals.insert(new_name, var);
                }

                Ok(())
            }
        }
    }
}
