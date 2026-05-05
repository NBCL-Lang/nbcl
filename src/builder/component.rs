use pest::iterators::Pair;
use crate::parser::Rule;
use crate::ast::source::*;
use crate::error::{Result, Span};
use super::node;

pub fn build_component_def(pair: Pair<Rule>) -> Result<ComponentDef> {
    let span = Span::from_pair(&pair);
    let mut inner = pair.into_inner();
    
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut next = inner.next().unwrap();
    let interface = if next.as_rule() == Rule::component_params {
        let interf = build_interface(next)?;
        next = inner.next().unwrap();
        interf
    } else {
        ComponentInterface::None
    };

    let mut body = Vec::new();
    for item_pair in next.into_inner() {
        body.push(node::build_node_item(item_pair)?);
    }

    Ok(ComponentDef { name, interface, body, span })
}

fn build_interface(pair: Pair<Rule>) -> Result<ComponentInterface> {
    // component_params -> ( any_params | named_params )?
    let inner = match pair.into_inner().next() {
        Some(p) => p,
        None => return Ok(ComponentInterface::None),
    };

    match inner.as_rule() {
        Rule::any_params => {
            // any_params -> "any" ~ ":" ~ snake_ident
            let ident = inner.into_inner()
                .find(|p| p.as_rule() == Rule::snake_ident)
                .expect("Grammar guaranteed a snake_ident in any_params")
                .as_str()
                .to_string();
            Ok(ComponentInterface::Loose(ident))
        }
        Rule::named_params => {
            let mut params = Vec::new();
            for item in inner.into_inner() {
                // item is Rule::param_item
                params.push(build_param_item(item)?);
            }
            Ok(ComponentInterface::Strict(params))
        }
        _ => unreachable!(),
    }
}

fn build_param_item(pair: Pair<Rule>) -> Result<Parameter> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut type_hint = None;
    let mut is_optional = false;

    for part in inner {
        match part.as_rule() {
            Rule::type_hint => type_hint = Some(part.as_str().to_string()),
            // If the grammar has "?" as a token, it will show up here
            _ if part.as_str() == "?" => is_optional = true,
            _ => {}
        }
    }

    Ok(Parameter { name, type_hint, is_optional })
}