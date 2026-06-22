use super::Evaluator;
use crate::ast::resolved::ResolvedNode;
use crate::ast::source::*;
use crate::error::{NbclError, Result};
use crate::evaluate::Value;
use crate::parser::NbclParser;
use crate::parser::Rule;
use pest::Parser;
use std::fs;
use std::io::ErrorKind;

impl Evaluator {
    pub(crate) fn handle_import(
        &mut self,
        imp: ImportDef,
        root_nodes: &mut Vec<ResolvedNode>,
    ) -> Result<()> {
        match imp.def {
            ImportDefType::Module(path_str, alias, components) => {
                let target_path = self.module_resolver.find_target(&path_str)?;
                self.registry.current_file = Some(target_path.clone());

                // Avoiding circular imports
                if self.loaded_files.contains(&target_path) {
                    return Ok(());
                }

                // Read and Parse
                let source = fs::read_to_string(&target_path).map_err(|e| {
                    let (msg, hint) = match e.kind() {
                        ErrorKind::NotFound => {
                            let msg = format!("module not found: '{}'", target_path.display());
                            let hint = "Ensure that the module exists and try adjusting the path."
                                .to_string();

                            (msg, Some(hint))
                        }
                        ErrorKind::PermissionDenied => {
                            let msg = format!(
                                "permission denied reading module: '{}'",
                                target_path.display()
                            );
                            let hint = "Set proper file permissions".to_string();

                            (msg, Some(hint))
                        }
                        _ => {
                            let msg =
                                format!("failed to read module '{}': {}", target_path.display(), e);
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
                let mut local_components = Vec::new();
                let mut imported_components = Vec::new();
                let mut import_map = Vec::new();

                for item in ast.items.clone() {
                    match item {
                        TopLevelItem::Import(imp) => self.handle_import(imp.clone(), root_nodes)?,
                        TopLevelItem::FnDef(mut f) => {
                            let new_internal_name = crate::builder::expr::generate_anon_fn_name();
                            let old_name = f.name;
                            f.name = new_internal_name.clone();

                            self.registry.register_function(f);
                            import_map.push((old_name, Value::Lambda(new_internal_name)))
                        }
                        TopLevelItem::ComponentDef(c) => {
                            let name = c.name.clone();
                            local_components.push(name.clone());
                            self.registry.register_component(c);

                            if let Some(ref selection) = components {
                                match selection {
                                    ComponentSelection::Wildcard => {
                                        imported_components.push(name)
                                    }
                                    ComponentSelection::List(allowed_comps) => {
                                        if allowed_comps.contains(&name) {
                                            imported_components.push(name)
                                        }
                                    }
                                }
                            }
                        }
                        // We insert globals now so they are available in Loop 2
                        TopLevelItem::Stmt(Stmt::Const(name, expr, _)) => {
                            let val = self.eval_expr(&expr)?;
                            import_map.push((name.clone(), val));
                        }
                        TopLevelItem::Stmt(Stmt::Let(name, expr, _)) => {
                            let val = self.eval_expr(&expr)?;
                            import_map.push((name.clone(), val));
                        }
                        _ => {}
                    }
                }

                if !import_map.is_empty() {
                    self.registry.globals.insert(alias, Value::Map(import_map));
                }

                if let Some(ComponentSelection::List(ref allowed_comps)) = components {
                    for req in allowed_comps {
                        let found = ast.items.iter().any(|i| match i {
                            TopLevelItem::ComponentDef(c) => &c.name == req,
                            _ => false,
                        });

                        if !found {
                            return Err(NbclError::Ast {
                                message: format!("Component '{}' not found in '{}'", req, path_str),
                                hint: Some(
                                    "Check your spelling inside the import block.".to_string(),
                                ),
                                span: None,
                            });
                        }
                    }
                }

                for item in ast.items {
                    match item {
                        TopLevelItem::Node(invocation) => {
                            let nodes = self.resolve_node(invocation)?;
                            root_nodes.extend(nodes);
                        }
                        TopLevelItem::Stmt(stmt) => {
                            let result = self.execute_stmt(&stmt)?;

                            if let Value::Node(returned_nodes) = result {
                                root_nodes.extend(returned_nodes);
                            }
                        }
                        _ => {} // Rest are already handled
                    }
                }

                let names_to_retain: std::collections::HashSet<String> = 
                    imported_components.into_iter().collect();

                for comp_name in local_components {
                    if !names_to_retain.contains(&comp_name) {
                        self.registry.components.remove(&comp_name); 
                    }
                }

                self.registry.current_file = None;
                self.loaded_files.insert(target_path);
                Ok(())
            }
            ImportDefType::Library(lib_name, lib_item) => {
                let maybe_library =
                    self.registry.libraries.iter().find(|&lib| lib.name == lib_name);

                let library = match maybe_library {
                    Some(lib) => lib,
                    None => {
                        let library_names = self
                            .registry
                            .libraries
                            .iter()
                            .map(|lib| lib.name.clone())
                            .collect::<Vec<String>>();

                        let suggestion =
                            crate::utils::find_best_match(&lib_name, library_names.iter());

                        let hint = suggestion.map(|s| {
                            format!(
                                "Library \"{}\" doesn't exist. Did you mean \"{}\"?",
                                &lib_name, s
                            )
                        });

                        return Err(NbclError::Runtime {
                            message: "library not found".into(),
                            hint,
                            span: Some(imp.span),
                        });
                    }
                };

                let maybe_lib_item = library.items.iter().find(|&i| i.name == lib_item);

                let item = match maybe_lib_item {
                    Some(i) => i,
                    None => {
                        let item_names = library
                            .items
                            .iter()
                            .map(|item| item.name.clone())
                            .collect::<Vec<String>>();

                        let suggestion =
                            crate::utils::find_best_match(&lib_item, item_names.iter());

                        let hint = suggestion.map(|s| {
                            format!(
                                "Library item \"{}\" doesn't exist. Did you mean \"{}\"?",
                                &lib_item, s
                            )
                        });

                        return Err(NbclError::Runtime {
                            message: format!("library item '{}' not found", &lib_item),
                            hint,
                            span: Some(imp.span),
                        });
                    }
                };

                let mut import_map = Vec::new();

                for (name, var) in item.globals.clone() {
                    import_map.push((name, var))
                }

                for (fn_name, mut schema) in item.native_functions.clone() {
                    let new_internal_name = crate::builder::expr::generate_anon_fn_name();
                    schema.name = new_internal_name.clone();

                    self.registry.native_functions.insert(new_internal_name.clone(), schema);
                    import_map.push((fn_name, Value::Lambda(new_internal_name)));
                }

                if !import_map.is_empty() {
                    self.registry.globals.insert(lib_item, Value::Map(import_map));
                }

                Ok(())
            }
        }
    }
}
