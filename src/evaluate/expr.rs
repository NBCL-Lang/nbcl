use super::{Evaluator, FlowControl, Scope, ScopeKind};
use crate::{
    ast::utils::Value,
    ast::source::*,
    error::{NbclError, Result, Span},
};
use std::rc::Rc;

// Extend for expr support.
impl Evaluator {
    pub(crate) fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.eval_literal(lit),

            ExprKind::Variable(name) => self.lookup_var(name).ok_or_else(|| {
                let candidates = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.variables.keys())
                    .chain(self.registry.globals.keys());

                let suggestion = crate::utils::find_best_match(name, candidates);
                let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                NbclError::Runtime {
                    message: format!("undefined variable: {}", name),
                    hint,
                    span: Some(expr.span.clone()),
                }
            }),

            ExprKind::Binary(lhs, op, rhs) => {
                let left = self.eval_expr(lhs)?;

                // Short-circuit logic for logical OR (||) and AND (&&)
                if op == "||" {
                    return Ok(if left.is_truthy() { left } else { self.eval_expr(rhs)? });
                }
                if op == "&&" {
                    return Ok(if !left.is_truthy() { left } else { self.eval_expr(rhs)? });
                }

                let right = self.eval_expr(rhs)?;
                self.apply_binary_op(&left, &op, &right, &expr.span)
            }

            ExprKind::Unary(op, operand) => {
                let val = self.eval_expr(operand)?;
                match op.as_str() {
                    "!" => Ok(Value::Bool(!val.is_truthy())),
                    "-" => self.apply_negation(val, &expr.span),
                    _ => unreachable!(),
                }
            }

            ExprKind::Field(source, field, is_safe) => {
                let val = self.eval_expr(source)?;

                if let Value::Map(pairs) = val {
                    let found = pairs.iter().find(|(k, _)| k == field).map(|(_, v)| v.clone());

                    match found {
                        Some(v) => Ok(v),
                        None => {
                            if *is_safe {
                                // If it's a ?. access, missing keys are just null
                                Ok(Value::Null)
                            } else {
                                Err(NbclError::Runtime {
                                    message: format!("map has no field: {}", field),
                                    hint: {
                                        let candidates = pairs.iter().map(|(k, _)| k);
                                        if let Some(suggestion) =
                                            crate::utils::find_best_match(field, candidates)
                                        {
                                            Some(format!("Did you mean \"{}\"?", suggestion))
                                        } else {
                                            Some(format!(
                                                "If this field is optional, try using the safe access operator: \"?.\"{}{}",
                                                if field.is_empty() { "" } else { "" },
                                                field
                                            ))
                                        }
                                    },
                                    span: Some(expr.span.clone()),
                                })
                            }
                        }
                    }
                } else if *is_safe && matches!(val, Value::Null) {
                    // Handle chaining: if 'val' is null and we are using ?., keep returning null
                    Ok(Value::Null)
                } else {
                    Err(NbclError::Runtime {
                        message: format!(
                            "cannot access field '{}' on non-map type: {:?}",
                            field, val
                        ),
                        hint: None,
                        span: Some(expr.span.clone()),
                    })
                }
            }

            ExprKind::Index(target, index_expr) => {
                let target_val = self.eval_expr(target)?;
                let key_val = self.eval_expr(index_expr)?;

                match (target_val, key_val) {
                    (Value::List(list), Value::Int(i)) => {
                        list.get(i as usize).cloned().ok_or_else(|| NbclError::Runtime {
                            message: format!("index {} out of bounds", i),
                            hint: None,
                            span: Some(expr.span.clone()),
                        })
                    }
                    (Value::Map(map), Value::Str(s)) => {
                        map.iter().find(|(k, _)| k == &s).map(|(_, v)| v.clone()).ok_or_else(|| {
                            let candidates = map.iter().map(|(k, _)| k);
                            let suggestion = crate::utils::find_best_match(&s, candidates);

                            let hint = suggestion.map(|best| format!("Did you mean \"{}\"?", best));

                            NbclError::Runtime {
                                message: format!("key '{}' not found in map", s),
                                hint,
                                span: Some(expr.span.clone()),
                            }
                        })
                    }
                    _ => Err(NbclError::Runtime {
                        message: "invalid index operation".into(),
                        hint: None,
                        span: Some(expr.span.clone()),
                    }),
                }
            }

            ExprKind::Call(callee, args_exprs) => {
                self.call_stack_depth += 1;
                if self.call_stack_depth > self.max_depth {
                    return Err(NbclError::Runtime {
                        message: format!("maximum recursion depth of {} exceeded", self.max_depth),
                        hint: Some(
                            "Check for infinite recursion in your functions or increase the limit."
                                .into(),
                        ),
                        span: Some(callee.span.clone()),
                    });
                }

                let func_name = match &callee.kind {
                    ExprKind::Variable(name) => name,
                    ExprKind::Field(source, field, _) => {
                        if let ExprKind::Variable(ref alias) = source.kind {
                            &format!("{}.{}", alias, field)
                        } else {
                            return Err(NbclError::Runtime {
                                message: "complex paths in calls are not supported yet".into(),
                                hint: Some("Try assigning the object to a local variable first, e.g. 'local f = obj.func; f()'".to_string()),
                                span: Some(callee.span.clone()),
                            });
                        }
                    }
                    _ => {
                        return Err(NbclError::Runtime {
                            message: "only variables can be called as functions currently".into(),
                            hint: None,
                            span: Some(callee.span.clone()),
                        });
                    }
                };

                let mut args = Vec::new();
                for e in args_exprs {
                    args.push(self.eval_expr(e)?);
                }

                // Native functions built into it
                if let Some(native_schema) = self.registry.native_functions.get(func_name) {
                    if args.len() != native_schema.params.len() {
                        let expected_params: Vec<String> =
                            native_schema.params.iter().map(|p| format!("{:?}", p)).collect();

                        return Err(NbclError::Runtime {
                            message: format!(
                                "native function '{}' expected {} args, got {}",
                                func_name,
                                native_schema.params.len(),
                                args.len()
                            ),
                            hint: Some(format!(
                                "Usage: {}({})",
                                func_name,
                                expected_params.join(", ")
                            )),
                            span: Some(expr.span.clone()),
                        });
                    }

                    for (i, (arg, expected)) in args.iter().zip(&native_schema.params).enumerate() {
                        if !expected.matches_value(arg) {
                            let hint = match (arg, expected) {
                                (Value::Str(s), _) if s.parse::<i64>().is_ok() =>
                                    Some("This value is a string, but the function needs a number. Try removing the quotes.".to_string()),
                                _ => Some(format!("Check the {} argument. It must be a {:?}.", crate::utils::ordinal(i + 1), expected)),
                            };

                            return Err(NbclError::Runtime {
                                message: format!(
                                    "native function '{}' arg {} expected {:?}, got {}",
                                    func_name,
                                    i,
                                    expected,
                                    arg.type_name()
                                ),
                                hint,
                                span: Some(expr.span.clone()),
                            });
                        }
                    }

                    self.call_stack_depth -= 1;
                    return (native_schema.body)(args);
                }

                let func_def =
                    self.registry.functions.get(func_name).map(Rc::clone).ok_or_else(|| {
                        // Collect all possible function names for the suggestion
                        let all_funcs = self
                            .registry
                            .native_functions
                            .keys()
                            .chain(self.registry.functions.keys());

                        let suggestion = crate::utils::find_best_match(func_name, all_funcs);
                        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                        NbclError::Runtime {
                            message: format!("undefined function: {}", func_name),
                            hint,
                            span: Some(callee.span.clone()),
                        }
                    })?;

                // Validate argument count
                if args.len() != func_def.params.len() {
                    let param_names: Vec<String> =
                        func_def.params.iter().map(|p| p.name.clone()).collect();

                    return Err(NbclError::Runtime {
                        message: format!(
                            "expected {} arguments, got {}",
                            func_def.params.len(),
                            args.len()
                        ),
                        hint: Some(format!("Signature: {}({})", func_name, param_names.join(", "))),
                        span: Some(expr.span.clone()),
                    });
                }

                let mut call_scope = Scope::new(ScopeKind::Function);
                for (param, value) in func_def.params.iter().zip(args) {
                    if let Some(expected_type) = &param.type_hint {
                        let actual_type = value.type_name();

                        if expected_type != actual_type && expected_type != "Any" {
                            return Err(NbclError::Runtime {
                                message: format!(
                                    "type mismatch for parameter '{}' in function '{}'. Expected {}, got {}",
                                    param.name, func_name, expected_type, actual_type
                                ),
                                hint: Some(format!(
                                    "The parameter '{}' expects {}, but you passed {}. Double-check the order of your arguments.",
                                    param.name, expected_type, actual_type
                                )),
                                span: Some(expr.span.clone()),
                            });
                        }
                    }

                    call_scope.variables.insert(param.name.clone(), value);
                }

                self.scopes.push(call_scope);
                let mut nodes = Vec::new();

                for item in &func_def.body {
                    match item {
                        FnItem::Stmt(s) => {
                            let val = self.execute_stmt(s)?;
                            if let Value::Nodes(new_nodes) = val {
                                nodes.extend(new_nodes);
                            }
                        }
                        FnItem::Node(n) => {
                            let resolved = self.resolve_node(n.clone())?;
                            nodes.extend(resolved);
                        }
                    }

                    // If the statement set a return value, stop executing the body immediately
                    if let FlowControl::Return(_) = self.flow {
                        break;
                    }
                }

                let explicit_return = std::mem::replace(&mut self.flow, FlowControl::None);
                self.call_stack_depth -= 1;
                self.scopes.pop();

                match explicit_return {
                    FlowControl::Return(val) => Ok(val),
                    FlowControl::None => {
                        if !nodes.is_empty() {
                            Ok(Value::Nodes(nodes))
                        } else {
                            Ok(Value::Null)
                        }
                    }
                }
            }

            ExprKind::Lambda(_, _) => todo!(),

            ExprKind::If(if_expr) => {
                let mut target_branch = None;

                // Evaluate main condition
                if self.eval_expr(&if_expr.condition)?.is_truthy() {
                    target_branch = Some(&if_expr.then_branch);
                } else {
                    // Evaluate else-ifs
                    for (cond, branch) in &if_expr.else_ifs {
                        if self.eval_expr(cond)?.is_truthy() {
                            target_branch = Some(branch);
                            break;
                        }
                    }
                    // Evaluate else
                    if target_branch.is_none() {
                        target_branch = if_expr.else_branch.as_ref();
                    }
                }

                // Execute the chosen branch
                if let Some((stmts, final_expr)) = target_branch {
                    self.scopes.push(Scope::new(ScopeKind::Block));

                    for stmt in stmts {
                        self.execute_stmt(stmt)?;
                    }

                    let result =
                        if let Some(e) = final_expr { self.eval_expr(e)? } else { Value::Null };

                    self.scopes.pop();
                    Ok(result)
                } else {
                    Ok(Value::Null)
                }
            }

            ExprKind::Match(subject_expr, arms) => {
                let value = self.eval_expr(subject_expr)?;

                let mut matched_branch = None;

                for arm in arms {
                    // "matching" logic
                    let is_match = match arm.pattern.as_str() {
                        "_" => true, // Wildcard matches everything
                        p => match &value {
                            Value::Int(i) if p == i.to_string() => true,
                            Value::Bool(b) if p == b.to_string() => true,
                            Value::Str(s) if p == s => true,
                            Value::Null if p == "null" => true,
                            _ => false,
                        },
                    };

                    if is_match {
                        matched_branch = Some(&arm.body);
                        break;
                    }
                }

                // Execute the branch body
                if let Some(body) = matched_branch {
                    match body {
                        LambdaBody::Block(stmts, final_expr) => {
                            self.scopes.push(Scope::new(ScopeKind::Block));

                            for stmt in stmts {
                                self.execute_stmt(stmt)?;
                            }

                            let result = if let Some(e) = final_expr {
                                self.eval_expr(e)?
                            } else {
                                Value::Null
                            };

                            self.scopes.pop();
                            Ok(result)
                        }
                        LambdaBody::Expr(e) => self.eval_expr(e),
                    }
                } else {
                    Ok(Value::Null)
                }
            }

            ExprKind::Range(start_expr, end_expr, inclusive) => {
                let start = self.eval_expr(start_expr)?;
                let end = self.eval_expr(end_expr)?;

                match (start, end) {
                    (Value::Int(s), Value::Int(e)) => {
                        let range = if *inclusive {
                            // s..=e (+1 for =e)
                            Value::Range(s, e + 1)
                        } else {
                            // s..e
                            Value::Range(s, e)
                        };
                        Ok(range)
                    }
                    _ => Err(NbclError::Runtime {
                        message: "range boundaries must be integers".into(),
                        hint: None,
                        span: Some(expr.span.clone()),
                    }),
                }
            }
        }
    }

    fn lookup_var(&self, name: &str) -> Option<Value> {
        // Search local scope stack (reversed for shadowing)
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.variables.get(name) {
                return Some(val.clone());
            }
        }
        // Search globals in Registry
        self.registry.globals.get(name).cloned()
    }

    fn eval_literal(&mut self, lit: &Literal) -> Result<Value> {
        match lit {
            Literal::Int(i) => Ok(Value::Int(*i)),
            Literal::Float(f) => Ok(Value::Float(*f)),
            Literal::Bool(b) => Ok(Value::Bool(*b)),
            Literal::Str(s) => Ok(Value::Str(s.clone())),
            Literal::Null => Ok(Value::Null),
            Literal::List(exprs) => {
                let mut values = Vec::new();
                for e in exprs {
                    values.push(self.eval_expr(e)?);
                }
                Ok(Value::List(values))
            }
            Literal::Map(pairs) => {
                let mut values = Vec::new();
                for (k, e) in pairs {
                    values.push((k.clone(), self.eval_expr(e)?));
                }
                Ok(Value::Map(values))
            }
        }
    }

    fn apply_binary_op(&self, left: &Value, op: &str, right: &Value, span: &Span) -> Result<Value> {
        match (left, op, right) {
            (Value::Int(a), "+", Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), "-", Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), "*", Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), "/", Value::Int(b)) => {
                if *b == 0 {
                    return Err(NbclError::Runtime {
                        message: "division by zero".to_string(),
                        hint: Some(
                            "Try replacing the zero with another number, silly!".to_string(),
                        ),
                        span: Some(span.clone()),
                    });
                }
                Ok(Value::Int(a / b))
            }
            (Value::Int(a), "%", Value::Int(b)) => {
                if *b == 0 {
                    return Err(NbclError::Runtime {
                        message: "modulo by zero".to_string(),
                        hint: Some("Try replacing the zero with another number.".to_string()),
                        span: Some(span.clone()),
                    });
                }
                Ok(Value::Int(a % b))
            }
            (Value::Str(a), "+", Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),

            // Integer Comparisons
            (Value::Int(a), "==", Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), "!=", Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Int(a), "<", Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), "<=", Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), ">", Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), ">=", Value::Int(b)) => Ok(Value::Bool(a >= b)),

            // String Comparisons
            (Value::Str(a), "==", Value::Str(b)) => Ok(Value::Bool(a == b)),
            (Value::Str(a), "!=", Value::Str(b)) => Ok(Value::Bool(a != b)),
            (Value::Str(a), "<", Value::Str(b)) => Ok(Value::Bool(a < b)),
            (Value::Str(a), ">", Value::Str(b)) => Ok(Value::Bool(a > b)),
            (Value::Str(a), "<=", Value::Str(b)) => Ok(Value::Bool(a <= b)),
            (Value::Str(a), ">=", Value::Str(b)) => Ok(Value::Bool(a >= b)),
            (l, o, r) => Err(NbclError::Runtime {
                message: format!("operation '{}' not supported between {:?} and {:?}", o, l, r),
                hint: Some(
                    "Try converting both sides to the same type using to_string() or to_int()."
                        .to_string(),
                ),
                span: Some(span.clone()),
            }),
        }
    }

    fn apply_negation(&self, val: Value, span: &Span) -> Result<Value> {
        match val {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(NbclError::Runtime {
                message: "unary '-' can only be applied to numbers".into(),
                hint: None,
                span: Some(span.clone()),
            }),
        }
    }
}
