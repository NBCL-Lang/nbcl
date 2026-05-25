//! Source AST builder

pub mod component;
pub mod expr;
pub mod node;

use crate::ast::source::*;
use crate::error::{Result, Span};
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
                    #[cfg_attr(not(feature = "module-imports"), allow(unreachable_code))]
                    Rule::import_stmt => {
                        let mut inner = child.clone().into_inner();
                        let path_pair = inner.next().unwrap();

                        #[cfg(not(feature = "module-imports"))]
                        {
                            use crate::error::NbclError;

                            return Err(NbclError::Ast {
                                message: "module imports are disabled".into(),
                                hint: Some(
                                    "Module import feature is disabled by the developer."
                                        .to_string(),
                                ),
                                span: Some(Span::from_pair(&path_pair)),
                            });
                        }

                        let path = unquote(path_pair.as_str());

                        // skip 'as' keyword
                        let _ = inner.next().unwrap();

                        let alias_pair = inner.next().unwrap();
                        let alias = alias_pair.as_str().to_string();

                        let components = if let Some(block_pair) = inner.next() {
                            let inner_block = block_pair.into_inner().next().unwrap();
                            match inner_block.as_rule() {
                                Rule::import_all_wildcard => Some(ComponentSelection::Wildcard),
                                Rule::layout_list => {
                                    let list = inner_block
                                        .into_inner()
                                        .map(|p| p.as_str().to_string())
                                        .collect();
                                    Some(ComponentSelection::List(list))
                                }
                                _ => unreachable!(),
                            }
                        } else {
                            None
                        };

                        TopLevelItem::Import(ImportDef {
                            def: ImportDefType::Module(path, alias, components),
                            span: Span::from_pair(&child),
                        })
                    }
                    #[cfg_attr(not(feature = "lib-imports"), allow(unreachable_code))]
                    Rule::import_lib_stmt => {
                        let mut inner = child.clone().into_inner();
                        let library_pair = inner.next().unwrap();

                        #[cfg(not(feature = "lib-imports"))]
                        {
                            use crate::error::NbclError;

                            return Err(NbclError::Ast {
                                message: "library imports are disabled".into(),
                                hint: Some(
                                    "Library import feature is disabled by the developer."
                                        .to_string(),
                                ),
                                span: Some(Span::from_pair(&library_pair)),
                            });
                        }

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
    let mut body = Vec::new();

    for part in inner {
        match part.as_rule() {
            Rule::fn_param => {
                let mut p_inner = part.into_inner();
                let p_name = p_inner.next().unwrap().as_str().to_string();
                params.push(p_name);
            }
            Rule::fn_body => {
                for item in part.into_inner() {
                    match item.as_rule() {
                        Rule::fn_item => {
                            let child = item.into_inner().next().unwrap();
                            match child.as_rule() {
                                Rule::node_invocation => {
                                    body.push(BodyItem::Node(node::build_node_invocation(child)?));
                                }
                                Rule::stmt => {
                                    body.push(BodyItem::Stmt(expr::build_stmt(child)?));
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

    Ok(FnDef { name, params, body, span })
}

pub fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}
