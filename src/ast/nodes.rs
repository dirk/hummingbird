use std::boxed::Box;
use std::path::PathBuf;

use super::super::parser::{Location, Span, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Assignment(Assignment),
    Block(Block),
    Function(Function),
    Export(Export),
    Identifier(Identifier),
    Import(Import),
    Infix(Infix),
    Integer(Integer),
    Let(Let),
    Module(Module),
    PostfixCall(PostfixCall),
    PostfixProperty(PostfixProperty),
    Return(Return),
    String(StringLiteral),
    Var(Var),
    While(While),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub lhs: Box<Node>,
    pub rhs: Box<Node>,
}

impl Assignment {
    pub fn new(lhs: Node, rhs: Node) -> Self {
        Self {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: Option<String>,
    pub body: Box<Node>,
    pub location: Option<Location>,
}

impl Function {
    pub fn new_anonymous(body: Box<Node>) -> Self {
        Self::new(None, body)
    }

    pub fn new_named(name: String, body: Box<Node>) -> Self {
        Self::new(Some(name), body)
    }

    fn new(name: Option<String>, body: Box<Node>) -> Self {
        Self {
            name,
            body,
            location: None,
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        self.name == other.name && self.body == other.body
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub identifiers: Vec<Identifier>,
}

impl Export {
    pub fn new(identifiers: Vec<Identifier>) -> Self {
        Self { identifiers }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identifier {
    pub value: String,
    pub span: Span,
}

impl Identifier {
    pub fn new<V: Into<String>>(value: V, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportBindings {
    /// Leaving empty to import the module by its file name:
    ///
    ///   import "a"
    ///   println(a.b)
    Module,

    /// Using `A` to import the module as `A` (doing `A.b` to get `b`).
    ///
    ///   import A "a"
    ///   println(A.b)
    AliasedModule(String),

    /// Using `{ b }` to import the `b` export:
    ///
    ///   import { b } "a"
    ///   println(b)
    NamedExports(Vec<String>),

    /// Using `*` to import all bindings:
    ///
    ///   import * "a"
    ///   println(b)
    AllExports,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub name: String,
    pub bindings: ImportBindings,
}

impl Import {
    pub fn new(name: String, bindings: ImportBindings) -> Self {
        Self { name, bindings }
    }

    pub fn path(&self) -> PathBuf {
        self.name.clone().into()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Integer {
    pub value: i64,
}

#[derive(Clone, Debug)]
pub struct Let {
    pub lhs: Identifier,
    pub rhs: Option<Box<Node>>,
    pub location: Option<Location>,
}

impl Let {
    pub fn new(lhs: Identifier, rhs: Option<Node>) -> Self {
        Self {
            lhs,
            rhs: rhs.map(|node| Box::new(node)),
            location: None,
        }
    }
}

impl PartialEq for Let {
    fn eq(&self, other: &Let) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InfixOp {
    Add,
    LessThan,
    Multiply,
    Subtract,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Infix {
    pub lhs: Box<Node>,
    pub op: InfixOp,
    pub rhs: Box<Node>,
}

impl Infix {
    pub fn new(lhs: Node, token: Token, rhs: Node) -> Self {
        let op = match token {
            Token::LeftAngle => InfixOp::LessThan,
            Token::Minus => InfixOp::Subtract,
            Token::Plus => InfixOp::Add,
            Token::Star => InfixOp::Multiply,
            _ => unreachable!("No operator for {:?}", token),
        };
        Self {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostfixCall {
    pub target: Box<Node>,
    pub arguments: Vec<Node>,
}

impl PostfixCall {
    pub fn new(target: Node, arguments: Vec<Node>) -> Self {
        Self {
            target: Box::new(target),
            arguments,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostfixProperty {
    pub target: Box<Node>,
    pub value: String,
    pub span: Span,
}

impl PostfixProperty {
    pub fn new(target: Node, value: String, span: Span) -> Self {
        Self {
            target: Box::new(target),
            value,
            span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub nodes: Vec<Node>,
}

impl Module {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Return {
    pub rhs: Option<Box<Node>>,
}

#[derive(Clone, Debug)]
pub struct Var {
    pub lhs: Identifier,
    pub rhs: Option<Box<Node>>,
    pub location: Option<Location>,
}

impl Var {
    pub fn new(lhs: Identifier, rhs: Option<Node>) -> Self {
        Self {
            lhs,
            rhs: rhs.map(|node| Box::new(node)),
            location: None,
        }
    }
}

impl PartialEq for Var {
    fn eq(&self, other: &Var) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs
    }
}

impl Return {
    pub fn new(rhs: Option<Node>) -> Self {
        Self {
            rhs: rhs.map(|node| Box::new(node)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct While {
    pub condition: Box<Node>,
    pub block: Block,
}

impl While {
    pub fn new(condition: Node, block: Block) -> Self {
        Self {
            condition: Box::new(condition),
            block,
        }
    }
}
