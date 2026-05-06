use crate::{
    ast::Value,
    ast::source::*,
    ast::resolved::ResolvedTree,
    registry::Registry,
    error::{Result, NbclError, Span},
};
use std::collections::HashMap;
use super::{Evaluator, FlowControl};

// Extend for expr support.
impl Evaluator {
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.eval_literal(lit),
            
            ExprKind::Variable(name) => {
                self.lookup_var(name)
                    .ok_or_else(|| NbclError::Runtime {
                        message: format!("Undefined variable: {}", name),
                        span: Some(expr.span.clone()),
                    })
            }

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
                self.apply_binary_op(left, op, right, &expr.span)
            }

            ExprKind::Unary(op, operand) => {
                let val = self.eval_expr(operand)?;
                match op.as_str() {
                    "!" => Ok(Value::Bool(!val.is_truthy())),
                    "-" => self.apply_negation(val, &expr.span),
                    _ => unreachable!(),
                }
            }

            ExprKind::Field(source, field) => {
                let val = self.eval_expr(source)?;
                if let Value::Map(pairs) = val {
                    pairs.iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| NbclError::Runtime {
                            message: format!("Map has no field: {}", field),
                            span: Some(expr.span.clone()),
                        })
                } else {
                    Err(NbclError::Runtime {
                        message: "Cannot access field on non-map type".into(),
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
                            message: format!("Index {} out of bounds", i),
                            span: Some(expr.span.clone()),
                        })
                    }
                    (Value::Map(map), Value::Str(s)) => {
                        map.iter().find(|(k, _)| k == &s).map(|(_, v)| v.clone())
                            .ok_or_else(|| NbclError::Runtime {
                                message: format!("Key '{}' not found", s),
                                span: Some(expr.span.clone()),
                            })
                    }
                    _ => Err(NbclError::Runtime {
                        message: "Invalid index operation".into(),
                        span: Some(expr.span.clone()),
                    }),
                }
            }

            ExprKind::Call(callee, args_exprs) => {
                let func_name = match &callee.kind {
                    ExprKind::Variable(name) => name,
                    ExprKind::Field(source, field) => {
                        if let ExprKind::Variable(ref alias) = source.kind {
                            &format!("{}.{}", alias, field) 
                        } else {
                            return Err(NbclError::Runtime {
                                message: "Complex paths in calls are not supported yet".into(),
                                span: Some(callee.span.clone()),
                            });
                        }
                    }
                    _ => return Err(NbclError::Runtime {
                        message: "Only variables can be called as functions currently".into(),
                        span: Some(callee.span.clone()),
                    }),
                };

                let mut args = Vec::new();
                for e in args_exprs {
                    args.push(self.eval_expr(e)?);
                }

                // Native functions built into it
                if let Some(native_schema) = self.registry.native_functions.get(func_name) {
                    
                    if args.len() != native_schema.params.len() {
                        return Err(NbclError::Runtime {
                            message: format!("Native function '{}' expected {} args, got {}", 
                                func_name, native_schema.params.len(), args.len()),
                            span: Some(expr.span.clone()),
                        });
                    }

                    for (i, (arg, expected)) in args.iter().zip(&native_schema.params).enumerate() {
                        if !expected.matches_value(arg) {
                            return Err(NbclError::Runtime {
                                message: format!("Native function '{}' arg {} expected {:?}, got {}", 
                                    func_name, i, expected, arg.type_name()),
                                span: Some(expr.span.clone()),
                            });
                        }
                    }

                    return (native_schema.body)(args);
                }

                let func_def = self.registry.functions.get(func_name)
                    .cloned()
                    .ok_or_else(|| NbclError::Runtime {
                        message: format!("Undefined function: {}", func_name),
                        span: Some(callee.span.clone()),
                    })?;

                // Validate argument count
                if args.len() != func_def.params.len() {
                    return Err(NbclError::Runtime {
                        message: format!("Expected {} arguments, got {}", func_def.params.len(), args.len()),
                        span: Some(expr.span.clone()),
                    });
                }

                let mut call_scope = HashMap::new();
                for (param, value) in func_def.params.iter().zip(args) {
                    if let Some(expected_type) = &param.type_hint {
                        let actual_type = value.type_name();
                        
                        if expected_type != actual_type && expected_type != "Any" {
                            return Err(NbclError::Runtime {
                                message: format!(
                                    "Type mismatch for parameter '{}' in function '{}'. Expected {}, got {}",
                                    param.name, func_name, expected_type, actual_type
                                ),
                                span: Some(expr.span.clone()),
                            });
                        }
                    }
                    
                    call_scope.insert(param.name.clone(), value);
                }

                self.scopes.push(call_scope);
                let mut nodes = Vec::new();

                for item in &func_def.body {
                    match item {
                        FnItem::Stmt(s) => self.execute_stmt(s.clone())?,
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
                let condition = self.eval_expr(&if_expr.condition)?;
                if condition.is_truthy() {
                    self.eval_block(&if_expr.then_branch.0, &if_expr.then_branch.1)
                } else if let Some(else_branch) = &if_expr.else_branch {
                    self.eval_block(&else_branch.0, &else_branch.1)
                } else {
                    Ok(Value::Null)
                }
            }

            ExprKind::Match(_, _) => todo!(),

            ExprKind::Range(start_expr, end_expr, inclusive) => {
                let start = self.eval_expr(start_expr)?;
                let end = self.eval_expr(end_expr)?;

                match (start, end) {
                    (Value::Int(s), Value::Int(e)) => {
                        let values = if *inclusive {
                            (s..=e).map(Value::Int).collect()
                        } else {
                            (s..e).map(Value::Int).collect()
                        };
                        Ok(Value::List(values))
                    }
                    _ => Err(NbclError::Runtime {
                        message: "Range boundaries must be integers".into(),
                        span: Some(expr.span.clone()),
                    }),
                }
            }
        }
    }

    fn lookup_var(&self, name: &str) -> Option<Value> {
        // Search local scope stack (reversed for shadowing)
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
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

    fn apply_binary_op(&self, left: Value, op: &str, right: Value, span: &Span) -> Result<Value> {
        match (left, op, right) {
            (Value::Int(a), "+", Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), "-", Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), "*", Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), "/", Value::Int(b)) => {
                if b == 0 {
                    return Err(NbclError::Runtime {
                        message: "Division by zero".to_string(),
                        span: Some(span.clone()),
                    });
                }
                Ok(Value::Int(a / b))
            }
            (Value::Int(a), "%", Value::Int(b)) => {
                if b == 0 {
                    return Err(NbclError::Runtime {
                        message: "Modulo by zero".to_string(),
                        span: Some(span.clone()),
                    });
                }
                Ok(Value::Int(a % b))
            }
            (Value::Str(a), "+", Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
            (Value::Int(a), "==", Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Str(a), "==", Value::Str(b)) => Ok(Value::Bool(a == b)),
            (l, o, r) => Err(NbclError::Runtime {
                message: format!("Operation '{}' not supported between {:?} and {:?}", o, l, r),
                span: Some(span.clone()),
            }),
        }
    }

    fn apply_negation(&self, val: Value, span: &Span) -> Result<Value> {
        match val {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(NbclError::Runtime {
                message: "Unary '-' can only be applied to numbers".into(),
                span: Some(span.clone()),
            }),
        }
    }

    fn eval_block(&mut self, stmts: &[Stmt], expr: &Option<Expr>) -> Result<Value> {
        for stmt in stmts {
            self.execute_stmt(stmt.clone())?;
            // If a statement was a 'return', stop immediately
            if let FlowControl::Return(val) = &self.flow {
                return Ok(val.clone()); 
            }
        }
        
        if let Some(final_expr) = expr {
            self.eval_expr(final_expr)
        } else {
            Ok(Value::Null)
        }
    }
}