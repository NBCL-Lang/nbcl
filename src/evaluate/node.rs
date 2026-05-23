use super::{Evaluator, Scope, ScopeKind, VariableBinding};
use crate::{
    ast::resolved::ResolvedNode,
    ast::source::*,
    ast::utils::{PropValidation, Value},
    error::{NbclError, Result, Span},
};
use std::collections::HashMap;

impl Evaluator {
    pub(crate) fn resolve_node(&mut self, inv: NodeInvocation) -> Result<Vec<ResolvedNode>> {
        // Try to find it as a Component
        let component_def = self.registry.components.get(&inv.type_name).cloned();

        if let Some(def) = component_def {
            return self.expand_component(&def, inv);
        }

        let resolved_id = if let Some(id_expr) = &inv.id {
            let val = self.eval_expr(id_expr)?;
            match val {
                Value::Str(s) => Some(s),
                Value::Null => None,
                _ => {
                    return Err(NbclError::Runtime {
                        message: format!("expected string for node ID, found {}", val.type_name()),
                        hint: Some(
                            "If you're passing a variable, ensure it contains a string.".into(),
                        ),
                        span: Some(id_expr.span.clone()),
                    });
                }
            }
        } else {
            None
        };

        // Try to find it as a Native Node
        if let Some(schema) = self.registry.native_nodes.get(&inv.type_name).cloned() {
            // Check: Is an ID required by the schema?
            if schema.enforce_id && resolved_id.is_none() {
                return Err(NbclError::Runtime {
                    message: format!("node '{}' requires an #id", inv.type_name),
                    hint: Some(
                        "Try providing an id like this: 'Object \"id\" { ... }'.".to_string(),
                    ),
                    span: Some(inv.span),
                });
            }

            let mut props = HashMap::new();
            let mut children = Vec::new();

            self.resolve_node_items(inv.body, &mut props, &mut children)?;

            // Check: Are these properties allowed and are their type correct?
            if let PropValidation::Strict(allowed_map) = &schema.validation {
                for (key, (value, span)) in props.iter() {
                    // Existence Check
                    if let Some(expected_type) = allowed_map.get(key) {
                        // Type Check
                        if !expected_type.matches_value(value) {
                            return Err(NbclError::Runtime {
                                message: format!(
                                    "type mismatch for '{}' on '{}': expected {:?}, found {:?}",
                                    key,
                                    inv.type_name,
                                    expected_type,
                                    value.type_name()
                                ),
                                hint: None,
                                span: Some(span.clone()),
                            });
                        }
                    } else {
                        // Key not found in the allowed map
                        let suggestion = crate::utils::find_best_match(key, allowed_map.keys());
                        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                        return Err(NbclError::Runtime {
                            message: format!(
                                "property '{}' is not allowed on node '{}'",
                                key, inv.type_name
                            ),
                            hint,
                            span: Some(span.clone()),
                        });
                    }
                }
            }
            if let Some((min_children, max_children)) = &schema.child_count {
                let actual_count = children.len() as u32;
                if actual_count < *min_children || actual_count > *max_children {
                    return Err(NbclError::Runtime {
                        message: format!(
                            "Native Node '{}' requires between {} and {} children, but got {}.",
                            schema.type_name, min_children, max_children, actual_count
                        ),
                        hint: None,
                        span: Some(inv.span.clone()),
                    });
                }
            }

            let resolved_props = {
                #[cfg(feature = "metadata")]
                {
                    props
                }
                #[cfg(not(feature = "metadata"))]
                {
                    props.into_iter().map(|(k, (v, _span))| (k, v)).collect()
                }
            };

            return Ok(vec![ResolvedNode {
                type_name: inv.type_name,
                id: resolved_id,
                props: resolved_props,
                children,
                #[cfg(feature = "metadata")]
                span: inv.span.clone(),
            }]);
        }

        let all_node_names =
            self.registry.native_nodes.keys().chain(self.registry.components.keys());

        let suggestion = crate::utils::find_best_match(&inv.type_name, all_node_names);
        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

        Err(NbclError::Runtime {
            message: format!("unknown node or component: {}", inv.type_name),
            hint,
            span: Some(inv.span),
        })
    }

    fn expand_component(
        &mut self,
        def: &ComponentDef,
        inv: NodeInvocation,
    ) -> Result<Vec<ResolvedNode>> {
        let resolved_id_val = if let Some(id_expr) = &inv.id {
            let val = self.eval_expr(id_expr)?;
            match val {
                Value::Str(_) => val,
                Value::Null => Value::Null,
                _ => {
                    return Err(NbclError::Runtime {
                        message: "node ID must resolve to a string".into(),
                        hint: Some(format!("Got a {} instead.", val.type_name())),
                        span: Some(id_expr.span.clone()),
                    });
                }
            }
        } else {
            Value::Null
        };

        let mut component_scope = Scope::new(ScopeKind::Component);

        // Resolve caller props once to avoid re-evaluating in different branches
        let mut caller_props = HashMap::new();
        let mut caller_children = Vec::new();
        self.resolve_node_items(inv.body, &mut caller_props, &mut caller_children)?;

        // Enforce properties
        for item in &def.body {
            match item {
                NodeItem::Prop(key, expr, _) => {
                    let constraint_val = self.eval_expr(expr)?;

                    match key.as_str() {
                        "id_required" => {
                            if let Value::Bool(true) = constraint_val {
                                if resolved_id_val == Value::Null {
                                    return Err(NbclError::Runtime {
                                        message: format!(
                                            "Component '{}' requires an ID.",
                                            def.name
                                        ),
                                        hint: Some(format!(
                                            "Usage: {} \"my_id\" {{ ... }}",
                                            def.name
                                        )),
                                        span: Some(inv.span.clone()),
                                    });
                                }
                            }
                        }

                        "child_count" => {
                            let actual_count = caller_children.len() as i64;
                            match constraint_val {
                                // Case: child_count = 3 (Exact)
                                Value::Int(expected) => {
                                    if actual_count != expected {
                                        let maybe_pronoun = if expected == 1 {
                                            "child"
                                        } else {
                                            "children"
                                        };

                                        return Err(NbclError::Runtime {
                                            message: format!("Component '{}' requires exactly {} {}, but got {}.", def.name, expected, maybe_pronoun, actual_count),
                                            hint: None,
                                            span: Some(inv.span.clone()),
                                        });
                                    }
                                }
                                // Case: child_count = [1, 3] (Range)
                                Value::List(range) if range.len() == 2 => {
                                    if let (Value::Int(min), Value::Int(max)) = (&range[0], &range[1]) {
                                        if actual_count < *min || actual_count > *max {
                                            return Err(NbclError::Runtime {
                                                message: format!("Component '{}' requires between {} and {} children, but got {}.", def.name, min, max, actual_count),
                                                hint: None,
                                                span: Some(inv.span.clone()),
                                            });
                                        }
                                    }
                                }
                                _ => return Err(NbclError::Runtime {
                                    message: "Property 'child_count' must be an Integer or a List of 2 Integers.".into(),
                                    hint: None,
                                    span: Some(expr.span.clone()),
                                }),
                            }
                        }

                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Generate component.* namspace
        let mut meta_map = Vec::new();
        meta_map.push(("id".to_string(), resolved_id_val));
        meta_map.push(("children".to_string(), Value::Node(caller_children)));
        component_scope.variables.insert(
            "self".to_string(),
            VariableBinding { value: Value::Map(meta_map), is_const: true },
        );

        match &def.interface {
            ComponentInterface::Loose(name) => {
                // Pack all props into a single Map value
                let mut prop_list = Vec::new();
                for (k, (v, _span)) in caller_props {
                    prop_list.push((k, v));
                }
                component_scope.variables.insert(
                    name.clone(),
                    VariableBinding { value: Value::Map(prop_list), is_const: true },
                );
            }

            ComponentInterface::Strict(params) => {
                for param in params {
                    let value = caller_props.remove(&param.name);

                    match value {
                        Some((v, _)) => {
                            component_scope.variables.insert(
                                param.name.clone(),
                                VariableBinding { value: v, is_const: true },
                            );
                        }
                        None => {
                            if !param.is_optional {
                                let suggestion =
                                    crate::utils::find_best_match(&param.name, caller_props.keys());

                                let hint = suggestion.map(|s|
                                    format!("You provided \"{}\", which is not a parameter. Did you mean \"{}\"?", param.name, s)
                                );

                                return Err(NbclError::Runtime {
                                    message: format!(
                                        "missing required prop '{}' for component '{}'",
                                        param.name, def.name
                                    ),
                                    hint,
                                    span: Some(inv.span.clone()),
                                });
                            }
                            component_scope.variables.insert(
                                param.name.clone(),
                                VariableBinding { value: Value::Null, is_const: true },
                            );
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
                            "unexpected property '{}' for component '{}'.",
                            extra_key, def.name
                        ),
                        hint,
                        span: Some(inv.span.clone()),
                    });
                }
            }
            ComponentInterface::None => {
                if !caller_props.is_empty() {
                    return Err(NbclError::Runtime {
                        message: format!(
                            "component '{}' requires no properties but one or more properties were passed",
                            def.name
                        ),
                        hint: Some("Remove all the properties passed to this component.".into()),
                        span: Some(inv.span.clone()),
                    });
                }
            }
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
        props: &mut HashMap<String, (Value, Span)>,
        children: &mut Vec<ResolvedNode>,
    ) -> Result<()> {
        for item in items {
            match item {
                // Capture the span from the variant here ---v
                NodeItem::Prop(key, expr, span) => {
                    let val = self.eval_expr(&expr)?;
                    // Store both the resolved value and the source span
                    props.insert(key, (val, span));
                }
                NodeItem::Child(child_inv) => {
                    children.extend(self.resolve_node(child_inv)?);
                }
                NodeItem::Stmt(stmt) => {
                    let result = self.execute_stmt(&stmt)?;

                    if let Value::Node(returned_nodes) = result {
                        children.extend(returned_nodes);
                    }
                }
            }
        }
        Ok(())
    }
}
