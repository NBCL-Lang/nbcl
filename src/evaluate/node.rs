use crate::{
    ast::Value,
    ast::source::*,
    ast::resolved::ResolvedNode,
    registry::Registry,
    error::{Result, NbclError},
};
use super::Evaluator;
use std::collections::HashMap;

impl Evaluator {
    pub(crate) fn resolve_node(&mut self, inv: NodeInvocation) -> Result<Vec<ResolvedNode>> {
        // Try to find it as a Component
        let component_def = self.registry.components.get(&inv.type_name).cloned();

        if let Some(def) = component_def {
            return self.expand_component(&def, inv);
        }

        // Try to find it as a Native Node
        if let Some(schema) = self.registry.native_nodes.get(&inv.type_name) {
            let mut props = HashMap::new();
            let mut children = Vec::new();

            for item in inv.body {
                match item {
                    NodeItem::Prop(key, expr) => {
                        props.insert(key, self.eval_expr(&expr)?);
                    }
                    NodeItem::Child(child_inv) => {
                        children.extend(self.resolve_node(child_inv)?);
                    }
                    NodeItem::Stmt(stmt) => self.execute_stmt(stmt)?,
                    _ => todo!("Handle For/If in nodes"),
                }
            }

            return Ok(vec![ResolvedNode {
                type_name: inv.type_name,
                id: inv.id,
                props,
                children,
            }]);
        }

        Err(NbclError::Runtime {
            message: format!("Unknown node or component: {}", inv.type_name),
            span: Some(inv.span),
        })
    }

    fn expand_component(&mut self, def: &ComponentDef, inv: NodeInvocation) -> Result<Vec<ResolvedNode>> {
        let mut component_scope = HashMap::new();

        match &def.interface {
            ComponentInterface::Loose(name) => {
                let mut prop_map = Vec::new();
                for item in inv.body {
                    if let NodeItem::Prop(k, e) = item {
                        prop_map.push((k, self.eval_expr(&e)?));
                    }
                }
                component_scope.insert(name.clone(), Value::Map(prop_map));
            }
            ComponentInterface::Strict(_params) => todo!("Strict params"),
            ComponentInterface::None => {}
        }

        self.scopes.push(component_scope);
        
        let mut resolved_children = Vec::new();
        for item in &def.body {
            match item {
                NodeItem::Child(child_inv) => {
                    resolved_children.extend(self.resolve_node(child_inv.clone())?);
                }
                NodeItem::Stmt(stmt) => self.execute_stmt(stmt.clone())?,
                _ => {} // TODO: Handle If/For later
            }
        }

        self.scopes.pop();
        Ok(resolved_children)
    }
}