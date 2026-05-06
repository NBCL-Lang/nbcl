use crate::ast::{Value, Type};
use crate::error::Span;
use std::sync::Arc;
use crate::error;
use std::fmt;

/// This essentially is the root of the AST.
#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<TopLevelItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TopLevelItem {
    Import(ImportDef),
    ComponentDef(ComponentDef),
    FnDef(FnDef),
    Node(NodeInvocation),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct ImportDef {
    pub(crate) def: ImportDefType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportDefType {
    Module(String, String),
    Library(String)
}

#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub interface: ComponentInterface,
    pub body: Vec<NodeItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ComponentInterface {
    Loose(String),            // (any: props)
    Strict(Vec<Parameter>),   // (a, b?, c: Int)
    None,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_hint: Option<String>,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<String>,
    pub body: Vec<FnItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FnItem {
    Node(NodeInvocation),
    Stmt(Stmt),
}

// Nodes
#[derive(Debug, Clone)]
pub struct NodeInvocation {
    pub type_name: String,
    pub id: Option<String>,
    pub body: Vec<NodeItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum NodeItem {
    Prop(String, Expr),
    Child(NodeInvocation),
    For(NodeFor),
    If(NodeIf),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct NodeFor {
    pub pattern: Vec<String>,
    pub iter: Expr,
    pub body: Vec<NodeItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NodeIf {
    pub condition: Expr,
    pub then_body: Vec<NodeItem>,
    pub else_ifs: Vec<(Expr, Vec<NodeItem>)>,
    pub else_body: Option<Vec<NodeItem>>,
    pub span: Span,
}

// Expressions
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub terminator: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(String, Expr, Span),
    Local(String, Option<String>, Expr),
    Global(String, Option<String>, Expr),
    For(Vec<String>, Expr, Block),
    While(Expr, Block),
    Return(Option<Expr>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Binary(Box<Expr>, String, Box<Expr>),
    Unary(String, Box<Expr>),
    Literal(Literal),
    Variable(String),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Lambda(Vec<(String, Option<String>)>, Box<LambdaBody>),
    If(Box<IfExpr>),
    Match(Box<Expr>, Vec<MatchArm>),
    Range(Box<Expr>, Box<Expr>, bool), // bool is inclusive
}

#[derive(Debug, Clone)]
pub enum LambdaBody {
    Expr(Expr),
    Block(Vec<Stmt>, Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Expr,
    pub then_branch: (Vec<Stmt>, Option<Expr>),
    pub else_ifs: Vec<(Expr, (Vec<Stmt>, Option<Expr>))>,
    pub else_branch: Option<(Vec<Stmt>, Option<Expr>)>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: String,
    pub body: LambdaBody,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<Expr>),
    Map(Vec<(String, Expr)>),
    Null,
}

/// Defines a host-provided node
#[derive(Debug, Clone)]
pub enum PropValidation {
    /// Allow any properties
    Loose,
    /// Only allow specific keys
    Strict(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct NativeNodeSchema {
    pub(crate) type_name: String,
    pub(crate) enforce_id: bool,
    pub(crate) validation: PropValidation,
}

pub struct NativeNodeSchemaBuilder {
    type_name: String,
    enforce_id: bool,
    validation: PropValidation,
}

impl NativeNodeSchemaBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            type_name: name.to_string(),
            enforce_id: false,
            validation: PropValidation::Loose,
        }
    }

    pub fn strict(mut self, props: Vec<&str>) -> Self {
        self.validation = PropValidation::Strict(
            props.into_iter().map(|s| s.to_string()).collect()
        );
        self
    }

    pub fn enforce_id(mut self) -> Self {
        self.enforce_id = true;
        self
    }

    pub fn build(self) -> NativeNodeSchema {
        NativeNodeSchema {
            type_name: self.type_name,
            enforce_id: self.enforce_id,
            validation: self.validation,
        }
    }
}

#[derive(Clone)]
pub struct NativeFnSchema {
    pub(crate) name: String,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) body: Arc<dyn Fn(Vec<Value>) -> error::Result<Value> + Send + Sync>,
}

impl fmt::Debug for NativeFnSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFnSchema")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("return_type", &self.return_type)
            .field("body", &"<native function>")
            .finish()
    }
}