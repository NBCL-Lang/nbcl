use crate::{
    ast::Value,
    ast::source::*,
    ast::resolved::ResolvedTree,
    registry::Registry,
    error::{Result, NbclError},
};
use std::collections::HashMap;
use super::{Evaluator, FlowControl};

impl Evaluator {
    pub fn execute_stmt(&mut self, stmt: Stmt) -> Result<()> {
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
            Stmt::Local(name, _type_hint, expr) => {
                let val = self.eval_expr(&expr)?;
                if let Some(current_scope) = self.scopes.last_mut() {
                    current_scope.insert(name, val);
                }
            }
            Stmt::Expr(expr) => {
                // Standalone expressions are evaluated and discarded
                self.eval_expr(&expr)?;
            }
            Stmt::For(patterns, iter_expr, body) => {
                let iter_val = self.eval_expr(&iter_expr)?;
                if let Value::List(items) = iter_val {
                    for item in items {
                        // Create a sub-scope for the loop iteration
                        let mut loop_scope = HashMap::new();
                        if patterns.len() == 1 {
                            loop_scope.insert(patterns[0].clone(), item);
                        }
                        // push, execute, pop
                        self.scopes.push(loop_scope);
                        for s in &body {
                            self.execute_stmt(s.clone())?;
                        }
                        self.scopes.pop();
                    }
                }
            }
            _ => todo!("Implement While/Return/Global"),
        }
        Ok(())
    }
}