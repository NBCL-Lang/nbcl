pub mod expr;
pub mod node;
pub mod component;

use pest::iterators::Pair;
use crate::parser::Rule;
use crate::ast::source::*;
use crate::error::{NbclError, Result, Span};

pub(crate) fn build_file(pair: Pair<Rule>) -> Result<File> {
    let span = Span::from_pair(&pair);
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::top_level_item => {
                let child = inner.into_inner().next().unwrap();
                let item = match child.as_rule() {
                    Rule::import_stmt => {
                        let mut inner = child.clone().into_inner();
                        
                        let path_pair = inner.next().unwrap();
                        let path = unquote(path_pair.as_str());
                        
                        let alias_pair = inner.next().ok_or_else(|| NbclError::Ast {
                            message: "Import statement missing 'as' alias".into(),
                            span: Some(Span::from_pair(&path_pair)),
                        })?;
                        
                        let alias = alias_pair.as_str().to_string();

                        TopLevelItem::Import(ImportDef {
                            def: ImportDefType::Module(path, alias),
                            span: Span::from_pair(&child),
                        })
                    }
                    Rule::import_lib_stmt => {
                        let mut inner = child.clone().into_inner();
                        
                        let library_pair = inner.next().unwrap();
                        let library = unquote(library_pair.as_str());

                        TopLevelItem::Import(ImportDef {
                            def: ImportDefType::Library(library),
                            span: Span::from_pair(&child),
                        })
                    }
                    Rule::component_def => TopLevelItem::ComponentDef(component::build_component_def(child)?),
                    Rule::fn_def => TopLevelItem::FnDef(build_fn_def(child)?),
                    Rule::node_invocation => TopLevelItem::Node(node::build_node_invocation(child)?),
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

    Ok(FnDef {
        name,
        params,
        return_type,
        body,
        span,
    })
}

pub fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}