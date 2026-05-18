use super::{Evaluator, FlowControl, Scope, ScopeKind, VariableBinding};
use crate::{
    ast::source::*,
    ast::utils::Value,
    error::{NbclError, Result, Span},
};

impl Evaluator {
    pub(crate) fn execute_stmt(&mut self, stmt: &Stmt) -> Result<Value> {
        if let FlowControl::Return(val) = &self.flow {
            return Ok(val.clone());
        }

        let result = match stmt {
            Stmt::Expr(expr) => {
                // Standalone expressions are evaluated and discarded
                self.eval_expr(&expr)?
            }
            Stmt::Return(maybe_rt, span) => {
                // Ensure that we cant return at:
                // TopLevel or a Block child of TopLevel
                let is_at_root = match self.scopes.as_slice() {
                    [root] if root.kind == ScopeKind::TopLevel => true,
                    [root, current]
                        if root.kind == ScopeKind::TopLevel && current.kind == ScopeKind::Block =>
                    {
                        true
                    }
                    _ => false,
                };

                if is_at_root {
                    return Err(NbclError::Runtime {
                        message: "cannot return from the top level".to_string(),
                        hint: Some("Move this logic into a function or component if you need early returns.".to_string()),
                        span: Some(span.clone()),
                    });
                }

                let val = match maybe_rt {
                    Some(ReturnType::Node(n)) => {
                        let resolved_nodes = self.resolve_node(n.clone())?;
                        Value::Node(resolved_nodes)
                    }
                    Some(ReturnType::Expr(e)) => self.eval_expr(&e)?,
                    None => Value::Null,
                };
                self.flow = FlowControl::Return(val.clone());
                val
            }
            Stmt::Const(name, expr) => {
                let val = self.eval_expr(&expr)?;
                if let Some(current_scope) = self.scopes.last_mut() {
                    current_scope
                        .variables
                        .insert(name.to_string(), VariableBinding { value: val, is_const: true });
                }

                Value::Null
            }
            Stmt::Let(name, expr) => {
                let val = self.eval_expr(&expr)?;
                if let Some(current_scope) = self.scopes.last_mut() {
                    current_scope
                        .variables
                        .insert(name.to_string(), VariableBinding { value: val, is_const: false });
                }

                Value::Null
            }
            Stmt::Assign(lhs, assign_op, rhs_expr, span) => {
                let new_val = self.eval_expr(&rhs_expr)?;

                match &lhs.kind {
                    ExprKind::Variable(name) => {
                        let mut found = false;
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(binding_ref) = scope.variables.get_mut(name) {
                                if binding_ref.is_const {
                                    return Err(NbclError::Runtime {
                                        message: format!("cannot reassign to constant variable '{}'", name),
                                        hint: Some("This variable was declared with 'const', or is an immutable loop/property binding.".into()),
                                        span: Some(span.clone()),
                                    });
                                }

                                let updated = Self::apply_assign_op(
                                    binding_ref.value.clone(),
                                    &assign_op,
                                    new_val,
                                    Some(span),
                                )?;
                                binding_ref.value = updated;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            let candidates = self.scopes.iter().flat_map(|s| s.variables.keys());
                            let suggestion = crate::utils::find_best_match(&name, candidates);

                            let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                            return Err(NbclError::Runtime {
                                message: format!("variable '{}' doesn't exist", name),
                                hint,
                                span: Some(span.clone()),
                            });
                        }
                    }

                    ExprKind::Field(base, field_name, _is_safe) => {
                        let mut target_map = self.eval_expr(&base)?;
                        if let Value::Map(ref mut entries) = target_map {
                            if let Some(pos) = entries.iter().position(|(k, _)| k == field_name) {
                                let old_val = entries[pos].1.clone();
                                entries[pos].1 = Self::apply_assign_op(
                                    old_val,
                                    &assign_op,
                                    new_val,
                                    Some(&span),
                                )?;
                            } else {
                                if assign_op == &AssignOp::Equal {
                                    entries.push((field_name.clone(), new_val));
                                } else {
                                    return Err(NbclError::Runtime {
                                        message: format!(
                                            "cannot update field '{}' that doesn't exist",
                                            field_name
                                        ),
                                        span: Some(span.clone()),
                                        hint: None,
                                    });
                                }
                            }
                            self.reassign_to_lhs(&base, target_map)?;
                        } else {
                            return Err(NbclError::Runtime {
                                message: "cannot set field on non-map".into(),
                                span: Some(span.clone()),
                                hint: None,
                            });
                        }
                    }

                    ExprKind::Index(base, index_expr) => {
                        let mut target_coll = self.eval_expr(&base)?;
                        let index_val = self.eval_expr(&index_expr)?;

                        match (&mut target_coll, index_val) {
                            (Value::List(items), Value::Int(i)) => {
                                let idx = i as usize;
                                if idx < items.len() {
                                    let old_val = items[idx].clone();
                                    items[idx] = Self::apply_assign_op(
                                        old_val,
                                        &assign_op,
                                        new_val,
                                        Some(&span),
                                    )?;
                                    self.reassign_to_lhs(&base, target_coll)?;
                                } else {
                                    return Err(NbclError::Runtime {
                                        message: format!("index {} out of bounds", i),
                                        span: Some(span.clone()),
                                        hint: None,
                                    });
                                }
                            }
                            _ => {
                                return Err(NbclError::Runtime {
                                    message: "invalid index operation".into(),
                                    span: Some(span.clone()),
                                    hint: None,
                                });
                            }
                        }
                    }
                    _ => {
                        return Err(NbclError::Runtime {
                            message: "invalid assignment target".into(),
                            span: Some(span.clone()),
                            hint: None,
                        });
                    }
                }
                Value::Null
            }
            Stmt::For(patterns, iter_expr, body) => {
                let iter_val = self.eval_expr(&iter_expr)?;
                match iter_val {
                    Value::Range(start, end) => {
                        let mut loop_scope = Scope::new(ScopeKind::Block);

                        // Optimization:
                        // Create dummy patterns and then only modify
                        // the value in the loop to avoid allocations.
                        for pattern in patterns {
                            loop_scope.variables.insert(
                                pattern.clone(),
                                VariableBinding { value: Value::Null, is_const: true },
                            );
                        }

                        self.scopes.push(loop_scope);
                        let scope_idx = self.scopes.len() - 1;

                        for i in start..end {
                            if let FlowControl::Return(_) = self.flow {
                                break;
                            }

                            if patterns.len() == 1 {
                                if let Some(val) =
                                    self.scopes[scope_idx].variables.get_mut(&patterns[0])
                                {
                                    val.value = Value::Int(i);
                                }
                            } else if patterns.len() == 2 {
                                if let Some(val1) =
                                    self.scopes[scope_idx].variables.get_mut(&patterns[0])
                                {
                                    val1.value = Value::Int(i);
                                }
                                if let Some(val2) =
                                    self.scopes[scope_idx].variables.get_mut(&patterns[1])
                                {
                                    val2.value = Value::Int(i);
                                }
                            }

                            self.execute_block_internal(&body)?;

                            if let FlowControl::Return(_) = self.flow {
                                break;
                            }
                        }

                        self.scopes.pop();
                    }
                    Value::List(items) => {
                        let loop_scope = Scope::new(ScopeKind::Block);
                        self.scopes.push(loop_scope);
                        let scope_idx = self.scopes.len() - 1;

                        for (i, item) in items.into_iter().enumerate() {
                            if let FlowControl::Return(_) = self.flow {
                                break;
                            }

                            // Handle pattern matching (len 1 or len 2)
                            if patterns.len() == 1 {
                                self.scopes[scope_idx].variables.insert(
                                    patterns[0].clone(),
                                    VariableBinding { value: item, is_const: true },
                                );
                            } else if patterns.len() == 2 {
                                self.scopes[scope_idx].variables.insert(
                                    patterns[0].clone(),
                                    VariableBinding { value: Value::Int(i as i64), is_const: true },
                                );
                                self.scopes[scope_idx].variables.insert(
                                    patterns[1].clone(),
                                    VariableBinding { value: item, is_const: true },
                                );
                            }

                            // Execute the block logic
                            self.execute_block_internal(&body)?;

                            if let FlowControl::Return(_) = self.flow {
                                break;
                            }
                        }

                        self.scopes.pop();
                    }

                    // unreachable
                    _ => {}
                }

                Value::Null
            }

            Stmt::While(condition_expr, body) => {
                self.scopes.push(Scope::new(ScopeKind::Block));

                // Keep looping as long as the condition evaluates to truthy
                // and we haven't hit a Return statement.
                while self.eval_expr(&condition_expr)?.is_truthy() {
                    if let FlowControl::Return(_) = self.flow {
                        break;
                    }

                    // Execute the block logic
                    self.execute_block_internal(&body)?;

                    if let FlowControl::Return(_) = self.flow {
                        break;
                    }
                }

                self.scopes.pop();

                Value::Null
            }
        };
        Ok(result)
    }

    pub(crate) fn execute_fnitem_with_scope(&mut self, body: &Vec<FnItem>, scope: Scope) -> Result<Value> {
        self.scopes.push(scope);
        let mut nodes = Vec::new();
        let mut implicit_return: Option<Value> = None;
        let body_len = &body.len();

        for (i, item) in body.iter().enumerate() {
            match item {
                FnItem::Node(n) => {
                    if i == body_len - 1 {
                        let resolved = self.resolve_node(n.clone())?;
                        nodes.extend(resolved);
                    }
                }
                FnItem::Stmt(s) => {
                    let val = self.execute_stmt(s)?;
                    if i == body_len - 1 {
                        if let Value::Node(new_nodes) = val {
                            nodes.extend(new_nodes);
                        } else {
                            implicit_return = Some(val);
                        }
                    }
                }
            }

            // If the statement set a return value, stop executing the body immediately
            if let FlowControl::Return(_) = self.flow {
                break;
            }
        }

        let explicit_return = std::mem::replace(&mut self.flow, FlowControl::None);
        self.scopes.pop();

        if let Some(val) = implicit_return {
            return Ok(val);
        }

        match explicit_return {
            FlowControl::Return(val) => return Ok(val),
            FlowControl::None => {
                if !nodes.is_empty() {
                    return Ok(Value::Node(nodes));
                } else {
                    return Ok(Value::Null);
                }
            }
        }
    }

    /// Executes the statements in a block and evaluates the terminator if present.
    fn execute_block_internal(&mut self, block: &Block) -> Result<Value> {
        // Run all statements
        for s in &block.stmts {
            self.execute_stmt(s)?;
            if let FlowControl::Return(_) = self.flow {
                return Ok(Value::Null);
            }
        }

        // Evaluate the implicit return (terminator) if it exists
        if let Some(expr) = &block.terminator {
            let val = self.eval_expr(expr)?;
            return Ok(val);
        }

        Ok(Value::Null)
    }

    fn reassign_to_lhs(&mut self, lhs: &Expr, value: Value) -> Result<()> {
        match &lhs.kind {
            ExprKind::Variable(name) => {
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(binding_ref) = scope.variables.get_mut(name) {
                        binding_ref.value = value;
                        return Ok(());
                    }
                }
                Err(NbclError::Runtime {
                    message: "Variable lost during assignment".into(),
                    hint: None,
                    span: None,
                })
            }
            ExprKind::Field(base, field, _) => {
                let mut parent = self.eval_expr(&base)?;
                if let Value::Map(ref mut entries) = parent {
                    if let Some(pos) = entries.iter().position(|(k, _)| k == field) {
                        entries[pos].1 = value;
                    } else {
                        entries.push((field.clone(), value));
                    }
                    self.reassign_to_lhs(&base, parent)
                } else {
                    Ok(())
                }
            }
            ExprKind::Index(base, index_expr) => {
                let mut parent = self.eval_expr(&base)?;
                let idx_val = self.eval_expr(&index_expr)?;
                if let (Value::List(items), Value::Int(i)) = (&mut parent, idx_val) {
                    let idx = i as usize;
                    if idx < items.len() {
                        items[idx] = value;
                        self.reassign_to_lhs(&base, parent)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn apply_assign_op(
        current: Value,
        op: &AssignOp,
        rhs: Value,
        span: Option<&Span>,
    ) -> Result<Value> {
        match op {
            AssignOp::Equal => Ok(rhs),
            AssignOp::PlusEqual => match (current, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Str(mut a), Value::Str(b)) => {
                    a.push_str(&b);
                    Ok(Value::Str(a))
                }
                _ => Err(NbclError::Runtime {
                    message: "type mismatch in '+=' operation".into(),
                    span: span.cloned(),
                    hint: Some("Both sides must be the same numeric or string type.".into()),
                }),
            },
            AssignOp::MinEqual => match (current, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(NbclError::Runtime {
                    message: "type mismatch in '-=' operation".into(),
                    span: span.cloned(),
                    hint: Some("Subtraction is only supported for numeric types.".into()),
                }),
            },
            AssignOp::MultEqual => match (current, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(NbclError::Runtime {
                    message: "type mismatch in '*=' operation".into(),
                    span: span.cloned(),
                    hint: Some("Multiplication is only supported for numeric types.".into()),
                }),
            },
            AssignOp::DivEqual => match (current, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                _ => Err(NbclError::Runtime {
                    message: "type mismatch in '/=' operation".into(),
                    span: span.cloned(),
                    hint: Some("Division is only supported for numeric types.".into()),
                }),
            },
        }
    }
}
