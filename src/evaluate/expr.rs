use super::{Evaluator, FlowControl, Scope, ScopeKind, VariableBinding};
use crate::{
    ast::source::*,
    ast::utils::Value,
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

                let mut args = Vec::new();
                for e in args_exprs {
                    args.push(self.eval_expr(e)?);
                }

                let func_name = match &callee.kind {
                    ExprKind::Variable(name) => {
                        if let Some(Value::Lambda(internal_name)) = self.lookup_var(name) {
                            internal_name
                        } else {
                            name.clone()
                        }
                    }
                    ExprKind::Field(source, field, is_safe) => {
                        let receiver = self.eval_expr(source)?;

                        if let Value::Map(ref pairs) = receiver {
                            let found =
                                pairs.iter().find(|(k, _)| k == field).map(|(_, v)| v.clone());

                            match found {
                                Some(Value::Lambda(internal_name)) => internal_name,
                                Some(other_val) => {
                                    return Err(NbclError::Runtime {
                                        message: format!(
                                            "field '{}' is not a callable function, got: {:?}",
                                            field, other_val
                                        ),
                                        hint: None,
                                        span: Some(callee.span.clone()),
                                    });
                                }
                                None => {
                                    if *is_safe {
                                        self.call_stack_depth -= 1;
                                        return Ok(Value::Null);
                                    }
                                    return Err(NbclError::Runtime {
                                        message: format!("map has no field: {}", field),
                                        hint: None,
                                        span: Some(callee.span.clone()),
                                    });
                                }
                            }
                        } else {
                            // Its a normal method
                            args.insert(0, receiver);
                            field.clone()
                        }
                    }
                    _ => {
                        if let Value::Lambda(internal_name) = self.eval_expr(callee)? {
                            internal_name
                        } else {
                            return Err(NbclError::Runtime {
                                message: "callee expression is not callable".into(),
                                hint: None,
                                span: Some(callee.span.clone()),
                            });
                        }
                    }
                };

                // Native function checking
                if let Some(native_schema) = self.registry.native_functions.get(&func_name) {
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
                    self.registry.functions.get(&func_name).map(Rc::clone).ok_or_else(|| {
                        // Collect all possible function names for the suggestion
                        let all_funcs = self
                            .registry
                            .native_functions
                            .keys()
                            .chain(self.registry.functions.keys());

                        let suggestion = crate::utils::find_best_match(&func_name, all_funcs);
                        let hint = suggestion.map(|s| format!("Did you mean \"{}\"?", s));

                        NbclError::Runtime {
                            message: format!("undefined function: {}", func_name),
                            hint,
                            span: Some(callee.span.clone()),
                        }
                    })?;

                // Validate argument count
                if args.len() != func_def.params.len() {
                    return Err(NbclError::Runtime {
                        message: format!(
                            "expected {} arguments, got {}",
                            func_def.params.len(),
                            args.len()
                        ),
                        hint: Some(format!(
                            "Signature: {}({})",
                            func_name,
                            func_def.params.join(", ")
                        )),
                        span: Some(expr.span.clone()),
                    });
                }

                let mut call_scope = Scope::new(ScopeKind::Function);
                for (param, value) in func_def.params.iter().zip(args) {
                    call_scope
                        .variables
                        .insert(param.clone(), VariableBinding { value, is_const: false });
                }

                self.scopes.push(call_scope);
                let mut nodes = Vec::new();
                let mut implicit_return: Option<Value> = None;
                let body_len = &func_def.body.len();

                for (i, item) in func_def.body.iter().enumerate() {
                    match item {
                        BodyItem::Node(n) => {
                            if i == body_len - 1 {
                                let resolved = self.resolve_node(n.clone())?;
                                nodes.extend(resolved);
                            }
                        }
                        BodyItem::Stmt(s) => {
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
                self.call_stack_depth -= 1;
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

            ExprKind::Lambda(fn_def) => {
                self.registry.register_function(fn_def.clone());
                Ok(Value::Lambda(fn_def.name.clone()))
            }

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
                if let Some((body, final_expr)) = target_branch {
                    self.scopes.push(Scope::new(ScopeKind::Block));
                    let mut return_node = Vec::new();

                    for item in body {
                        match item {
                            BodyItem::Stmt(s) => {
                                let result = self.execute_stmt(s)?;

                                if let Value::Node(nodes) = result {
                                    return_node.extend(nodes);
                                }
                            }
                            BodyItem::Node(n) => {
                                let resolved_node = self.resolve_node(n.clone())?;
                                return_node.extend(resolved_node);
                            }
                        }
                    }

                    let result = if let Some(e) = final_expr {
                        self.eval_expr(e)?
                    } else {
                        if !return_node.is_empty() { Value::Node(return_node) } else { Value::Null }
                    };

                    self.scopes.pop();
                    Ok(result)
                } else {
                    Ok(Value::Null)
                }
            }

            ExprKind::Match(subject_expr, arms) => {
                let value = self.eval_expr(subject_expr)?;

                let mut matched_branch = None;
                let mut match_scope = Scope::new(ScopeKind::Block);

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

                    if is_match || arm.is_var {
                        matched_branch = Some(&arm.body);
                        if arm.is_var {
                            match_scope.variables.insert(
                                arm.pattern.clone(),
                                VariableBinding { value, is_const: false },
                            );
                        }
                        break;
                    }
                }

                self.scopes.push(match_scope);

                // Execute the branch body
                let result = if let Some(body) = matched_branch {
                    match body {
                        MatchBody::Block(stmts, final_expr) => {
                            for stmt in stmts {
                                self.execute_stmt(stmt)?;
                            }

                            let result = if let Some(e) = final_expr {
                                self.eval_expr(e)?
                            } else {
                                Value::Null
                            };

                            Ok(result)
                        }
                        MatchBody::Expr(e) => self.eval_expr(e),
                    }
                } else {
                    Ok(Value::Null)
                };

                self.scopes.pop();
                result
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
                return Some(val.value.clone());
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
            Literal::Str(s, st) => Ok(Value::Str(self.evaluate_string_interpolation(&s, &st))),
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

    fn evaluate_string_interpolation(&self, s: &str, st: &StringType) -> String {
        match st {
            StringType::Raw => return s.to_string(),
            _ => {}
        }

        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.peek() {
                    Some(&'$') => {
                        chars.next();
                        result.push('$');
                    }
                    Some(&'\\') => {
                        chars.next();
                        result.push('\\');
                    }
                    Some(&'n') => {
                        chars.next();
                        result.push('\n');
                    }
                    Some(&'t') => {
                        chars.next();
                        result.push('\t');
                    }
                    Some(&'r') => {
                        chars.next();
                        result.push('\r');
                    }
                    Some(&'"') => {
                        chars.next();
                        result.push('"');
                    }
                    Some(&'\'') => {
                        chars.next();
                        result.push('\'');
                    }
                    Some(&'0') => {
                        chars.next();
                        result.push('\0');
                    }
                    _ => result.push('\\'),
                }
            } else if ch == '$' && chars.peek() == Some(&'{') && let StringType::Format = st {
                chars.next();

                let mut var_name = String::new();
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                    var_name.push(inner);
                }

                let value = self
                    .lookup_var(var_name.trim())
                    .map(|v| crate::builder::unquote(&v.to_string()))
                    .unwrap_or_default();

                result.push_str(&value);
            } else {
                result.push(ch);
            }
        }

        result
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
