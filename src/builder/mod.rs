//! Source AST builder

pub mod component;
pub mod expr;
pub mod node;

use crate::ast::source::*;
use crate::error::{NbclError, Result, Span};
use crate::parser::Rule;
use pest::iterators::Pair;

pub(crate) fn build_file(pair: Pair<Rule>) -> Result<File> {
    let span = Span::from_pair(&pair);
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::top_level_item => {
                let child = inner.into_inner().next().unwrap();
                let item = match child.as_rule() {
                    Rule::import_stmt => {
                        #[cfg(feature = "no-module-imports")]
                        {
                            return NbclError::Ast {
                                message: "module imports are disabled".into(),
                                hint: Some("Module import feature is disabled by the developer.".to_string()),
                                span: Some(Span::from_pair(&path_pair)),
                            }
                        }

                        let mut inner = child.clone().into_inner();

                        let path_pair = inner.next().unwrap();
                        let path = unquote(path_pair.as_str());

                        let alias_pair = inner.next().ok_or_else(|| NbclError::Ast {
                            message: "import statement missing 'as' alias".into(),
                            hint: Some("All imports must follow this structure: 'import \"..\" as example'.".to_string()),
                            span: Some(Span::from_pair(&path_pair)),
                        })?;

                        let alias = alias_pair.as_str().to_string();

                        TopLevelItem::Import(ImportDef {
                            def: ImportDefType::Module(path, alias),
                            span: Span::from_pair(&child),
                        })
                    }
                    Rule::import_lib_stmt => {
                        #[cfg(feature = "no-lib-imports")]
                        {
                            return NbclError::Ast {
                                message: "library imports are disabled".into(),
                                hint: Some("Library import feature is disabled by the developer.".to_string()),
                                span: Some(Span::from_pair(&path_pair)),
                            }
                        }

                        let mut inner = child.clone().into_inner();

                        let library_pair = inner.next().unwrap();
                        let library = library_pair.as_str().to_string();

                        let item_pair = inner.nth(1).unwrap();
                        let item = item_pair.as_str().to_string();

                        TopLevelItem::Import(ImportDef {
                            def: ImportDefType::Library(library, item),
                            span: Span::from_pair(&child),
                        })
                    }
                    Rule::component_def => {
                        TopLevelItem::ComponentDef(component::build_component_def(child)?)
                    }
                    Rule::fn_def => TopLevelItem::FnDef(build_fn_def(child)?),
                    Rule::node_invocation => {
                        TopLevelItem::Node(node::build_node_invocation(child)?)
                    }
                    Rule::stmt => TopLevelItem::Stmt(expr::build_stmt(child)?),
                    _ => continue,
                };
                items.push(item);
            }
            Rule::EOI => break,
            _ => {}
        }
    }
    Ok(File { items, span })
}

fn build_fn_def(pair: Pair<Rule>) -> Result<FnDef> {
    let span = Span::from_pair(&pair);
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str().to_string();

    let mut params = Vec::new();
    let mut return_type = None;
    let mut body = Vec::new();

    for part in inner {
        match part.as_rule() {
            Rule::fn_param => {
                let mut p_inner = part.into_inner();
                let p_name = p_inner.next().unwrap().as_str().to_string();
                let p_type = p_inner.next().map(|t| t.as_str().to_string());
                params.push(FnParam { name: p_name, type_hint: p_type });
            }
            Rule::fn_return_type => {
                return_type = Some(part.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::fn_body => {
                for item in part.into_inner() {
                    match item.as_rule() {
                        Rule::fn_item => {
                            let child = item.into_inner().next().unwrap();
                            match child.as_rule() {
                                Rule::node_invocation => {
                                    body.push(FnItem::Node(node::build_node_invocation(child)?));
                                }
                                Rule::stmt => {
                                    body.push(FnItem::Stmt(expr::build_stmt(child)?));
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(FnDef { name, params, return_type, body, span })
}

pub fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}
