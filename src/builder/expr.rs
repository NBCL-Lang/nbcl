use super::unquote;
use crate::ast::source::*;
use crate::error::{NbclError, Result, Span};
use crate::parser::Rule;
use pest::iterators::Pair;

pub fn build_stmt(pair: Pair<Rule>) -> Result<Stmt> {
    let inner = match pair.as_rule() {
        Rule::stmt | Rule::node_stmt => pair.into_inner().next().unwrap(),
        _ => pair,
    };
    let span = Span::from_pair(&inner);

    match inner.as_rule() {
        Rule::local_stmt | Rule::global_stmt => {
            let mut ii = inner.clone().into_inner();
            let name = ii.next().unwrap().as_str().to_string();
            let mut next = ii.next().unwrap();

            let type_hint = if next.as_rule() == Rule::type_hint {
                let t = Some(next.as_str().to_string());
                next = ii.next().unwrap();
                t
            } else {
                None
            };

            let value = build_expr(next)?;
            if inner.as_rule() == Rule::local_stmt {
                Ok(Stmt::Local(name, type_hint, value))
            } else {
                Ok(Stmt::Global(name, type_hint, value))
            }
        }
        Rule::assign_stmt => {
            let span = Span::from_pair(&inner);

            // name
            let mut ii = inner.clone().into_inner();
            let name = ii.next().unwrap().as_str().to_string();

            // expr
            let next = ii.next().unwrap();
            let value = build_expr(next)?;

            Ok(Stmt::Assign(name, value, span))
        }
        Rule::for_stmt => {
            let mut ii = inner.into_inner();
            let pattern_pair = ii.next().unwrap();
            let mut patterns = Vec::new();
            for ident in pattern_pair.into_inner() {
                patterns.push(ident.as_str().to_string());
            }

            let iter_expr = build_expr(ii.next().unwrap())?;

            let block_pair = ii.next().unwrap();
            let body = build_block(block_pair)?;

            Ok(Stmt::For(patterns, iter_expr, body))
        }
        Rule::while_stmt => {
            let mut ii = inner.into_inner();
            let condition = build_expr(ii.next().unwrap())?;

            let block_pair = ii.next().unwrap();
            let body = build_block(block_pair)?;

            Ok(Stmt::While(condition, body))
        }

        Rule::return_stmt => {
            let mut ii = inner.into_inner();
            let expr = if let Some(e_pair) = ii.next() { Some(build_expr(e_pair)?) } else { None };
            Ok(Stmt::Return(expr, span))
        }

        Rule::expr_stmt => Ok(Stmt::Expr(build_expr(inner.into_inner().next().unwrap())?)),
        _ => Err(NbclError::Ast {
            message: format!("unknown Statement: {:?}", inner.as_rule()),
            hint: None,
            span: Some(span),
        }),
    }
}

pub fn build_expr(pair: Pair<Rule>) -> Result<Expr> {
    let span = Span::from_pair(&pair);
    match pair.as_rule() {
        Rule::expr | Rule::or_expr | Rule::and_expr | Rule::add_expr | Rule::mul_expr => {
            build_binop(pair)
        }
        Rule::cmp_expr => {
            let mut inner = pair.into_inner();
            let lhs = build_expr(inner.next().unwrap())?;
            if let Some(op_pair) = inner.next() {
                let op = op_pair.as_str().to_string();
                let rhs = build_expr(inner.next().unwrap())?;
                Ok(Expr { kind: ExprKind::Binary(Box::new(lhs), op, Box::new(rhs)), span })
            } else {
                Ok(lhs)
            }
        }
        Rule::unary_expr => {
            let mut inner = pair.into_inner();
            let first = inner.next().ok_or_else(|| NbclError::Ast {
                message: "Empty unary expression".to_string(),
                hint: None,
                span: Some(span.clone()),
            })?;

            if first.as_rule() == Rule::postfix_expr {
                build_expr(first)
            } else {
                let op = first.as_str().to_string();
                let operand_pair = inner.next().ok_or_else(|| NbclError::Ast {
                    message: format!("Expected operand after unary operator '{}'", op),
                    hint: None,
                    span: Some(span.clone()),
                })?;

                let operand = build_expr(operand_pair)?;
                Ok(Expr { kind: ExprKind::Unary(op, Box::new(operand)), span })
            }
        }
        Rule::postfix_expr => {
            let mut inner = pair.into_inner();
            let mut res = build_expr(inner.next().unwrap())?;

            let mut it = inner.peekable();

            while let Some(suffix) = it.next() {
                res = match suffix.as_rule() {
                    Rule::accessor => {
                        let is_safe = suffix.as_str() == "?.";
                        let ident = it.next().unwrap().as_str().to_string();

                        Expr {
                            kind: ExprKind::Field(Box::new(res), ident, is_safe),
                            span: span.clone(),
                        }
                    }
                    Rule::expr => Expr {
                        kind: ExprKind::Index(Box::new(res), Box::new(build_expr(suffix)?)),
                        span: span.clone(),
                    },
                    Rule::call_args => {
                        let args = suffix
                            .into_inner()
                            .map(|arg_pair| build_expr(arg_pair))
                            .collect::<Result<Vec<_>>>()?;

                        Expr { kind: ExprKind::Call(Box::new(res), args), span: span.clone() }
                    }
                    _ => res,
                };
            }
            Ok(res)
        }
        Rule::range_expr => {
            let mut inner = pair.into_inner();
            let start = build_expr(inner.next().unwrap())?;

            // The operator (.. or ..=)
            let op = inner.next().unwrap();
            let inclusive = op.as_str() == "..=";

            let end = build_expr(inner.next().unwrap())?;

            Ok(Expr { kind: ExprKind::Range(Box::new(start), Box::new(end), inclusive), span })
        }
        Rule::id_expression => build_expr(pair.into_inner().next().unwrap()),
        Rule::string_lit => {
            Ok(Expr { kind: ExprKind::Literal(Literal::Str(unquote(pair.as_str()))), span })
        }
        Rule::primary_expr => build_expr(pair.into_inner().next().unwrap()),
        Rule::literal => Ok(Expr { kind: ExprKind::Literal(build_literal(pair)?), span }),
        Rule::if_expr => Ok(Expr { kind: ExprKind::If(Box::new(build_if(pair)?)), span }),
        Rule::match_expr => Ok(Expr { kind: build_match(pair)?, span }),
        Rule::lambda_expr => Ok(Expr { kind: build_lambda(pair)?, span }),
        Rule::snake_ident => Ok(Expr { kind: ExprKind::Variable(pair.as_str().to_string()), span }),
        _ => Err(NbclError::Ast {
            message: format!("unknown expr: {:?}", pair.as_rule()),
            hint: None,
            span: Some(span),
        }),
    }
}

fn build_binop(pair: Pair<Rule>) -> Result<Expr> {
    let span = Span::from_pair(&pair);
    let mut inner = pair.into_inner();

    let mut lhs = build_expr(inner.next().unwrap())?;

    while let Some(op_pair) = inner.next() {
        let op_str = op_pair.as_str().to_string();

        let rhs_pair = inner.next().ok_or_else(|| NbclError::Ast {
            message: "expected operand after operator".to_string(),
            hint: Some(
                "An operator like '+' must be followed by a value, variable, or '('.".to_string(),
            ),
            span: Some(span.clone()),
        })?;

        let rhs = build_expr(rhs_pair)?;

        lhs = Expr {
            kind: ExprKind::Binary(Box::new(lhs), op_str, Box::new(rhs)),
            span: span.clone(),
        };
    }

    Ok(lhs)
}

fn build_literal(pair: Pair<Rule>) -> Result<Literal> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::int_lit => Ok(Literal::Int(inner.as_str().parse().unwrap())),
        Rule::float_lit => Ok(Literal::Float(inner.as_str().parse().unwrap())),
        Rule::bool_lit => Ok(Literal::Bool(inner.as_str() == "true")),
        Rule::string_lit => Ok(Literal::Str(unquote(inner.as_str()))),
        Rule::list_lit => {
            let mut exprs = Vec::new();
            for p in inner.into_inner() {
                exprs.push(build_expr(p)?);
            }
            Ok(Literal::List(exprs))
        }
        Rule::map_lit => {
            let mut pairs = Vec::new();
            for p in inner.into_inner() {
                let mut inner_pair = p.into_inner();

                let key = inner_pair.next().unwrap().as_str().to_string();
                let value = build_expr(inner_pair.next().unwrap())?;

                pairs.push((key, value));
            }
            Ok(Literal::Map(pairs))
        }
        _ => Ok(Literal::Null),
    }
}

fn build_if(pair: Pair<Rule>) -> Result<IfExpr> {
    let mut inner = pair.into_inner();

    // Handle main 'if'
    let condition = build_expr(inner.next().unwrap())?;
    let then_branch = build_branch(inner.next().unwrap())?;

    let mut else_ifs = Vec::new();
    let mut else_branch = None;

    // Handle remaining 'else if' and 'else'
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::else_if_branch => {
                let mut ei_inner = next.into_inner();
                let cond = build_expr(ei_inner.next().unwrap())?;
                let body = build_branch(ei_inner.next().unwrap())?;
                else_ifs.push((cond, body));
            }
            Rule::else_branch => {
                else_branch = Some(build_branch(next.into_inner().next().unwrap())?);
            }
            _ => unreachable!("Unexpected rule in if_expr: {:?}", next.as_rule()),
        }
    }

    Ok(IfExpr { condition, then_branch, else_ifs, else_branch })
}

fn build_branch(pair: Pair<Rule>) -> Result<(Vec<Stmt>, Option<Expr>)> {
    let mut stmts = Vec::new();
    let mut final_expr = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::stmt => stmts.push(build_stmt(p)?),
            Rule::expr => final_expr = Some(build_expr(p)?),
            _ => {}
        }
    }

    if final_expr.is_none() {
        if let Some(last_stmt) = stmts.last() {
            if let Stmt::Expr(expr) = last_stmt {
                final_expr = Some(expr.clone());
                stmts.pop();
            }
        }
    }

    Ok((stmts, final_expr))
}

fn build_match(pair: Pair<Rule>) -> Result<ExprKind> {
    let mut inner = pair.into_inner();

    // expression
    let condition = build_expr(inner.next().unwrap())?;

    // arms
    let mut arms = Vec::new();
    for arm_pair in inner {
        arms.push(build_match_arm(arm_pair)?);
    }

    Ok(ExprKind::Match(Box::new(condition), arms))
}

fn build_match_arm(pair: Pair<Rule>) -> Result<MatchArm> {
    let mut inner = pair.into_inner();

    let pattern_pair = inner.next().unwrap();
    let pattern = pattern_pair.as_str().to_string();

    let body_pair = inner.next().unwrap();
    let body = match body_pair.as_rule() {
        Rule::block_body => {
            let block = build_block(body_pair)?;
            LambdaBody::Block(block.stmts, block.terminator)
        }
        _ => {
            let expr = build_expr(body_pair)?;
            LambdaBody::Expr(expr)
        }
    };

    Ok(MatchArm { pattern, body })
}

pub fn build_lambda(pair: Pair<Rule>) -> Result<ExprKind> {
    let mut inner = pair.clone().into_inner();
    let mut params = Vec::new();

    while let Some(next) = inner.peek() {
        if next.as_rule() == Rule::lambda_param {
            let mut param_inner = inner.next().unwrap().into_inner();
            let name = param_inner.next().unwrap().as_str().to_string();
            let type_hint = param_inner.next().map(|t| t.as_str().to_string());
            params.push((name, type_hint));
        } else {
            // Found the lambda_body
            break;
        }
    }

    // Parse lambda body
    let body_pair = inner.next().ok_or_else(|| NbclError::Ast {
        message: "Lambda must have a body".into(),
        hint: None,
        span: Some(Span::from_pair(&pair)),
    })?;

    let body = match body_pair.as_rule() {
        Rule::lambda_body => {
            let actual_body = body_pair.into_inner().next().unwrap();
            match actual_body.as_rule() {
                Rule::block_body => {
                    let block = build_block(actual_body)?;
                    LambdaBody::Block(block.stmts, block.terminator)
                }
                _ => LambdaBody::Expr(build_expr(actual_body)?),
            }
        }
        Rule::block_body => {
            let block = build_block(body_pair)?;
            LambdaBody::Block(block.stmts, block.terminator)
        }
        _ => LambdaBody::Expr(build_expr(body_pair)?),
    };

    Ok(ExprKind::Lambda(params, Box::new(body)))
}

pub fn build_block(pair: Pair<Rule>) -> Result<Block> {
    let mut stmts = Vec::new();
    let mut terminator = None;

    // inner will be the contents of the { ... }
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::stmt => {
                stmts.push(build_stmt(inner_pair)?);
            }
            Rule::expr => {
                // In the grammar: { stmt* ~ expr? }
                // If we hit an expr, it's the implicit return/terminator
                terminator = Some(build_expr(inner_pair)?);
            }
            _ => {}
        }
    }

    Ok(Block { stmts, terminator })
}
