use super::Evaluator;
use crate::{
    ast::resolved::ResolvedNode,
    ast::source::*,
    ast::{PropValidation, Type, Value},
    error::{NbclError, Result},
};
use std::collections::HashMap;

impl Evaluator {
    pub(crate) fn resolve_node(&mut self, inv: NodeInvocation) -> Result<Vec<ResolvedNode>> {
        // Try to find it as a Component
        let component_def = self.registry.components.get(&inv.type_name).cloned();

        if let Some(def) = component_def {
            return self.expand_component(&def, inv);
        }

        // Try to find it as a Native Node
        if let Some(schema) = self.registry.native_nodes.get(&inv.type_name).cloned() {
            // Check: Is an ID required by the schema?
            if schema.enforce_id && inv.id.is_none() {
                return Err(NbclError::Runtime {
                    message: format!("Node '{}' requires an #id", inv.type_name),
                    hint: Some("Try providing an id like this: Object \"id\" { ... }".to_string()),
                    span: Some(inv.span),
                });
            }

            let mut props = HashMap::new();
            let mut children = Vec::new();

            self.resolve_node_items(inv.body, &mut props, &mut children)?;

            // Check: Are these properties allowed and are their type correct?
            if let PropValidation::Strict(allowed_map) = &schema.validation {
                for (key, value) in props.iter() {
                    // Existence Check
                    if let Some(expected_type) = allowed_map.get(key) {
                        // Type Check
                        if expected_type.matches_value(value) {
                            return Err(NbclError::Runtime {
                                message: format!(
                                    "Type mismatch for '{}' on '{}': expected {:?}, found {:?}",
                                    key,
                                    inv.type_name,
                                    expected_type,
                                    value.type_name()
                                ),
                                hint: None,
                                span: Some(inv.span.clone()),
                            });
                        }
                    } else {
                        // Key not found in the allowed map
                        let suggestion = crate::utils::find_best_match(key, allowed_map.keys());
                        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                        return Err(NbclError::Runtime {
                            message: format!(
                                "Property '{}' is not allowed on node '{}'",
                                key, inv.type_name
                            ),
                            hint,
                            span: Some(inv.span.clone()),
                        });
                    }
                }
            }

            return Ok(vec![ResolvedNode {
                type_name: inv.type_name,
                id: inv.id,
                props,
                children,
            }]);
        }

        let all_node_names =
            self.registry.native_nodes.keys().chain(self.registry.components.keys());

        let suggestion = crate::utils::find_best_match(&inv.type_name, all_node_names);
        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

        Err(NbclError::Runtime {
            message: format!("Unknown node or component: {}", inv.type_name),
            hint,
            span: Some(inv.span),
        })
    }

    fn expand_component(
        &mut self,
        def: &ComponentDef,
        inv: NodeInvocation,
    ) -> Result<Vec<ResolvedNode>> {
        let mut component_scope = HashMap::new();

        // Resolve caller props once to avoid re-evaluating in different branches
        let mut caller_props = HashMap::new();
        let mut caller_children = Vec::new();
        self.resolve_node_items(inv.body, &mut caller_props, &mut caller_children)?;

        match &def.interface {
            ComponentInterface::Loose(name) => {
                // Pack all props into a single Map value
                let mut prop_list = Vec::new();
                for (k, v) in caller_props {
                    prop_list.push((k, v));
                }
                component_scope.insert(name.clone(), Value::Map(prop_list));
            }

            ComponentInterface::Strict(params) => {
                for param in params {
                    let value = caller_props.remove(&param.name);

                    match value {
                        Some(v) => {
                            // Validate Type Hint if it exists
                            if let Some(hint) = &param.type_hint {
                                if let Some(expected_type) = Type::from_str(hint) {
                                    if !expected_type.matches_value(&v) {
                                        return Err(NbclError::Runtime {
                                            message: format!(
                                                "Component '{}' expected {} for prop '{}', got {}",
                                                def.name,
                                                hint,
                                                param.name,
                                                v.type_name()
                                            ),
                                            hint: None,
                                            span: Some(inv.span.clone()),
                                        });
                                    }
                                }
                            }
                            component_scope.insert(param.name.clone(), v);
                        }
                        None => {
                            if !param.is_optional {
                                let suggestion =
                                    crate::utils::find_best_match(&param.name, caller_props.keys());

                                let hint = suggestion.map(|s|
                                    format!("You provided \"{}\", which is not a parameter. Did you mean \"{}\"?", s, param.name)
                                );

                                return Err(NbclError::Runtime {
                                    message: format!(
                                        "Missing required prop '{}' for component '{}'",
                                        param.name, def.name
                                    ),
                                    hint,
                                    span: Some(inv.span.clone()),
                                });
                            }
                            component_scope.insert(param.name.clone(), Value::Null);
                        }
                    }
                }

                if !caller_props.is_empty() {
                    let (extra_key, _) = caller_props.into_iter().next().unwrap();

                    let param_names = params.iter().map(|p| &p.name);
                    let suggestion = crate::utils::find_best_match(&extra_key, param_names);

                    let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                    return Err(NbclError::Runtime {
                        message: format!(
                            "Unexpected property '{}' for component '{}'.",
                            extra_key, def.name
                        ),
                        hint,
                        span: Some(inv.span.clone()),
                    });
                }
            }
            ComponentInterface::None => {}
        }

        self.scopes.push(component_scope);

        let mut final_nodes = Vec::new();
        let mut ignored_props = HashMap::new(); // Components usually don't "output" props, only nodes

        // Use the recursive helper so components get 'if' and 'for' for free!
        self.resolve_node_items(def.body.clone(), &mut ignored_props, &mut final_nodes)?;

        self.scopes.pop();
        Ok(final_nodes)
    }

    fn resolve_node_items(
        &mut self,
        items: Vec<NodeItem>,
        props: &mut HashMap<String, Value>,
        children: &mut Vec<ResolvedNode>,
    ) -> Result<()> {
        for item in items {
            match item {
                NodeItem::Prop(key, expr) => {
                    props.insert(key, self.eval_expr(&expr)?);
                }
                NodeItem::Child(child_inv) => {
                    children.extend(self.resolve_node(child_inv)?);
                }
                NodeItem::Stmt(stmt) => self.execute_stmt(stmt)?,

                NodeItem::If(node_if) => {
                    let mut target_body = None;

                    // Check main 'if' condition
                    if self.eval_expr(&node_if.condition)?.is_truthy() {
                        target_body = Some(&node_if.then_body);
                    } else {
                        // Check 'else if' branches
                        for (cond_expr, body) in &node_if.else_ifs {
                            if self.eval_expr(cond_expr)?.is_truthy() {
                                target_body = Some(body);
                                break;
                            }
                        }
                        // Check 'else' branch
                        if target_body.is_none() {
                            if let Some(else_body) = &node_if.else_body {
                                target_body = Some(else_body);
                            }
                        }
                    }

                    if let Some(body) = target_body {
                        // Recursively resolve the items in the chosen branch
                        self.resolve_node_items(body.clone(), props, children)?;
                    }
                }

                NodeItem::For(node_for) => {
                    let iter_value = self.eval_expr(&node_for.iter)?;

                    if let Value::List(items) = iter_value {
                        for val in items {
                            let mut loop_scope = HashMap::new();

                            // Support for `for item in list` (pattern len 1)
                            // or `for i, item in list` (pattern len 2)
                            if node_for.pattern.len() == 1 {
                                loop_scope.insert(node_for.pattern[0].clone(), val);
                            } else if node_for.pattern.len() == 2 {
                                todo!("Handle index patterns in for loops");
                            }

                            self.scopes.push(loop_scope);
                            self.resolve_node_items(node_for.body.clone(), props, children)?;
                            self.scopes.pop();
                        }
                    } else {
                        return Err(NbclError::Runtime {
                            message: "Can only iterate over a List in a for-node".into(),
                            hint: None,
                            span: Some(node_for.span),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
