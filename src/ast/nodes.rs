use std::boxed::Box;

use super::super::parser::{Location, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Assignment(Assignment),
    Block(Block),
    Function(Function),
    Identifier(Identifier),
    Infix(Infix),
    Integer(Integer),
    Let(Let),
    PostfixCall(PostfixCall),
    PostfixProperty(PostfixProperty),
    Return(Return),
    Var(Var),
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
    pub name: String,
    pub block: Block,
    pub location: Option<Location>,
}

impl Function {
    pub fn new(name: String, block: Block) -> Self {
        Self {
            name,
            block,
            location: None,
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        self.name == other.name && self.block == other.block
    }
}

#[derive(Clone, Debug)]
pub struct Identifier {
    pub value: String,
    pub location: Option<Location>,
}

impl Identifier {
    pub fn new<V: Into<String>>(value: V) -> Self {
        Self {
            value: value.into(),
            location: None,
        }
    }
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Identifier) -> bool {
        self.value == other.value
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
pub enum InfixOp {
    Add,
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
            Token::Minus => InfixOp::Subtract,
            Token::Plus => InfixOp::Add,
            Token::Star => InfixOp::Multiply,
            _ => unreachable!(),
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
    target: Box<Node>,
    value: String,
}

impl PostfixProperty {
    pub fn new(target: Node, value: String) -> Self {
        Self {
            target: Box::new(target),
            value: value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub nodes: Vec<Node>,
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
