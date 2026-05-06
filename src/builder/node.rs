use super::{expr, unquote};
use crate::ast::source::*;
use crate::error::{Result, Span};
use crate::parser::Rule;
use pest::iterators::Pair;

pub fn build_node_invocation(pair: Pair<Rule>) -> Result<NodeInvocation> {
    let span = Span::from_pair(&pair);
    let mut inner = pair.into_inner();

    let type_name = inner.next().unwrap().as_str().to_string();
    let mut id = None;
    let mut next = inner.next().unwrap();

    if next.as_rule() == Rule::string_lit {
        id = Some(unquote(next.as_str()));
        next = inner.next().unwrap();
    }

    let mut items = Vec::new();
    for item_pair in next.into_inner() {
        items.push(build_node_item(item_pair)?);
    }

    Ok(NodeInvocation { type_name, id, body: items, span })
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
