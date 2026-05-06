use super::expr;
use crate::ast::source::*;
use crate::error::{Result, Span};
use crate::parser::Rule;
use pest::iterators::Pair;

pub fn build_node_invocation(pair: Pair<Rule>) -> Result<NodeInvocation> {
    let span = Span::from_pair(&pair);
    let mut inner = pair.into_inner();

    let type_name = inner.next().unwrap().as_str().to_string();
    
    let mut id = None;
    let mut body_pair = None;

    if let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::node_block => {
                body_pair = Some(next);
            }
            _ => {
                id = Some(expr::build_expr(next)?);
                // The next one MUST be the block
                body_pair = inner.next();
            }
        }
    }

    let mut items = Vec::new();
    if let Some(block) = body_pair {
        for item_pair in block.into_inner() {
            items.push(build_node_item(item_pair)?);
        }
    }

    Ok(NodeInvocation { 
        type_name, 
        id, 
        body: items, 
        span 
    })
}

pub fn build_node_item(pair: Pair<Rule>) -> Result<NodeItem> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::node_prop => {
            let mut ii = inner.into_inner();
            let key = ii.next().unwrap().as_str().to_string();
            let val = expr::build_expr(ii.next().unwrap().into_inner().next().unwrap())?;
            Ok(NodeItem::Prop(key, val))
        }
        Rule::node_invocation => Ok(NodeItem::Child(build_node_invocation(inner)?)),
        Rule::node_stmt => {
            let stmt = expr::build_stmt(inner.into_inner().next().unwrap())?;
            Ok(NodeItem::Stmt(stmt))
        }
        _ => Ok(NodeItem::Prop("error".into(), expr::build_expr(inner)?)),
    }
}
