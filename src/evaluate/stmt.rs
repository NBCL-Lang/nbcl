use crate::{
    ast::Value,
    ast::source::*,
    error::{Result, NbclError},
};
use std::collections::HashMap;
use super::{Evaluator, FlowControl};

impl Evaluator {
    pub(crate) fn execute_stmt(&mut self, stmt: Stmt) -> Result<()> {
        if let FlowControl::Return(_) = self.flow {
            return Ok(());
        }

        match stmt {
            Stmt::Return(maybe_expr) => {
                let val = match maybe_expr {
                    Some(e) => self.eval_expr(&e)?,
                    None => Value::Null,
                };
                self.flow = FlowControl::Return(val);
            }
            // TODO: Use typehint in global and local
            Stmt::Global(name, _type_hint, expr) => {
                let val = self.eval_expr(&expr)?;
                
                // A 'global' always goes into the very first scope (index 0),
                // regardless of how many components or blocks deep we are.
                if let Some(global_scope) = self.scopes.first_mut() {
                    global_scope.insert(name, val);
                } else {
                    // Fallback: if somehow scopes is empty (shouldn't happen),
                    // create a new one.
                    let mut map = HashMap::new();
                    map.insert(name, val);
                    self.scopes.push(map);
                }
            }
            Stmt::Local(name, _type_hint, expr) => {
                let val = self.eval_expr(&expr)?;
                if let Some(current_scope) = self.scopes.last_mut() {
                    current_scope.insert(name, val);
                }
            }
            Stmt::Assign(name, expr, span) => {
                let new_val = self.eval_expr(&expr)?;
                let mut found = false;

                for scope in self.scopes.iter_mut().rev() {
                    if scope.contains_key(&name) {
                        scope.insert(name.clone(), new_val);
                        found = true;
                        break;
                    }
                }

                if !found {
                    let candidates = self.scopes.iter().flat_map(|s| s.keys());
                    let suggestion = crate::utils::find_best_match(&name, candidates);

                    let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                    return Err(NbclError::Runtime {
                        message: format!("Variable '{}' doesn't exist.", name),
                        hint,
                        span: Some(span),
                    });
                }
            }
            Stmt::Expr(expr) => {
                // Standalone expressions are evaluated and discarded
                self.eval_expr(&expr)?;
            }
            Stmt::For(patterns, iter_expr, body) => {
                let iter_val = self.eval_expr(&iter_expr)?;
                if let Value::List(items) = iter_val {
                    for (i, item) in items.into_iter().enumerate() {
                        if let FlowControl::Return(_) = self.flow { break; }

                        let mut loop_scope = HashMap::new();
                        
                        // Handle pattern matching (len 1 or len 2)
                        if patterns.len() == 1 {
                            loop_scope.insert(patterns[0].clone(), item);
                        } else if patterns.len() == 2 {
                            loop_scope.insert(patterns[0].clone(), Value::Int(i as i64));
                            loop_scope.insert(patterns[1].clone(), item);
                        }

                        self.scopes.push(loop_scope);
                        
                        // Execute the block logic
                        self.execute_block_internal(&body)?;
                        
                        self.scopes.pop();
                        
                        if let FlowControl::Return(_) = self.flow { break; }
                    }
                }
            }
            
            Stmt::While(condition_expr, body) => {
                // Keep looping as long as the condition evaluates to truthy
                // and we haven't hit a Return statement.
                while self.eval_expr(&condition_expr)?.is_truthy() {
                    if let FlowControl::Return(_) = self.flow { break; }

                    self.scopes.push(HashMap::new());
                    
                    // Execute the block logic
                    self.execute_block_internal(&body)?;
                    
                    self.scopes.pop();
                    
                    if let FlowControl::Return(_) = self.flow { break; }
                }
            }
        }
        Ok(())
    }

    /// Executes the statements in a block and evaluates the terminator if present.
    fn execute_block_internal(&mut self, block: &Block) -> Result<Value> {
        // Run all statements
        for s in &block.stmts {
            self.execute_stmt(s.clone())?;
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
}