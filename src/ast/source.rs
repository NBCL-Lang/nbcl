use crate::error::Span;

/// This essentially is the root of the AST.
#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<TopLevelItem>,
    pub span: Span,
}

/// Everything that can be found at the top level of the source
#[derive(Debug, Clone)]
pub enum TopLevelItem {
    Import(ImportDef),
    ComponentDef(ComponentDef),
    FnDef(FnDef),
    Node(NodeInvocation),
    Stmt(Stmt),
}

/// Import definitions
/// Example: `import "colors.nbl" as colors`
#[derive(Debug, Clone)]
pub struct ImportDef {
    pub(crate) def: ImportDefType,
    pub(crate) span: Span,
}

/// Types of import definition. Includes,
/// Module (e.g. `import "file" ...`), and Library (e.g. stdlib).
#[derive(Debug, Clone)]
pub enum ImportDefType {
    Module(String, String, Option<ComponentSelection>),
    Library(String, String),
}

/// Selected components to include in import
#[derive(Debug, Clone)]
pub enum ComponentSelection {
    Wildcard,
    List(Vec<String>),
}

/// Component definitions
#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub interface: ComponentInterface,
    pub body: Vec<NodeItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ComponentInterface {
    Loose(String),          // (any: props)
    Strict(Vec<Parameter>), // (a, b?, c: Int)
    None,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub is_optional: bool,
}

/// Function definition
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<BodyItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BodyItem {
    Node(NodeInvocation),
    Stmt(Stmt),
}

// Nodes
#[derive(Debug, Clone)]
pub struct NodeInvocation {
    pub type_name: String,
    pub id: Option<Expr>,
    pub body: Vec<NodeItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum NodeItem {
    Prop(String, Expr, Span),
    Child(NodeInvocation),
    Stmt(Stmt),
}

// Expressions
#[derive(Debug, Clone)]
pub struct Block {
    pub body: Vec<BodyItem>,
    pub terminator: Option<Expr>,
}

/// Statements supported in the language
#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(Expr, AssignOp, Expr, Span),
    Let(String, Expr),
    Const(String, Expr),
    For(Vec<String>, Expr, Block),
    While(Expr, Block),
    Return(Option<ReturnType>, Span),
    Expr(Expr),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssignOp {
    Equal,
    PlusEqual,
    MinEqual,
    MultEqual,
    DivEqual,
}

#[derive(Debug, Clone)]
pub enum ReturnType {
    Node(NodeInvocation),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Kinds of expression supported in the language
#[derive(Debug, Clone)]
pub enum ExprKind {
    Binary(Box<Expr>, String, Box<Expr>),
    Unary(String, Box<Expr>),
    Literal(Literal),
    Variable(String),
    // bool true = safe (?.)
    Field(Box<Expr>, String, bool),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Lambda(FnDef),
    If(Box<IfExpr>),
    Match(Box<Expr>, Vec<MatchArm>),
    Range(Box<Expr>, Box<Expr>, bool), // bool is inclusive
}

/// If/else expressions
#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Expr,
    pub then_branch: (Vec<BodyItem>, Option<Expr>),
    pub else_ifs: Vec<(Expr, (Vec<BodyItem>, Option<Expr>))>,
    pub else_branch: Option<(Vec<BodyItem>, Option<Expr>)>,
}

/// Match
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: String,
    pub body: MatchBody,
    pub is_var: bool,
}

#[derive(Debug, Clone)]
pub enum MatchBody {
    Expr(Expr),
    Block(Vec<Stmt>, Option<Expr>),
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
