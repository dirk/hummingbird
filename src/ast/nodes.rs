use std::boxed::Box;
use std::collections::HashSet;
use std::fmt::{Display, Error, Formatter};

use super::super::parser::{Location, Span, Token};

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

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use Node::*;
        match self {
            Assignment(_) => f.write_str("Assignment"),
            Block(_) => f.write_str("Block"),
            Function(_) => f.write_str("Function"),
            Identifier(_) => f.write_str("Identifier"),
            Infix(_) => f.write_str("Infix"),
            Integer(_) => f.write_str("Integer"),
            Let(_) => f.write_str("Let"),
            PostfixCall(_) => f.write_str("PostfixCall"),
            PostfixProperty(_) => f.write_str("PostfixProperty"),
            Return(_) => f.write_str("Return"),
            Var(_) => f.write_str("Var"),
        }
    }
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
    /// The variables of this function which must be stored in the `Closure`
    /// on the heap.
    bindings: Option<HashSet<String>>,
    /// The variables that this function depends on its parent closure to
    /// provide. To ensure nested closure forwarding a `Closure` will be
    /// created if this is present, even if `bindings` is not present.
    parent_bindings: Option<HashSet<String>>,
}

impl Function {
    pub fn new_anonymous(body: Box<Node>) -> Self {
        let (bindings, parent_bindings) = detect_bindings(&body);
        Self {
            name: None,
            body,
            location: None,
            bindings,
            parent_bindings,
        }
    }

    pub fn new_named(name: String, body: Box<Node>) -> Self {
        let (bindings, parent_bindings) = detect_bindings(&body);
        Self {
            name: Some(name),
            body,
            location: None,
            bindings,
            parent_bindings,
        }
    }

    pub fn has_bindings(&self) -> bool {
        self.bindings.is_some()
    }

    pub fn get_bindings(&self) -> Option<HashSet<String>> {
        self.bindings.clone()
    }

    pub fn has_parent_bindings(&self) -> bool {
        self.parent_bindings.is_some()
    }

    pub fn get_parent_bindings(&self) -> Option<HashSet<String>> {
        self.parent_bindings.clone()
    }
}

fn none_if_empty(set: HashSet<String>) -> Option<HashSet<String>> {
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

// FIXME: Flip this visitor on its head and make it a top-down walk to
//   discover variables, their usages (closures), and catch undefined
//   variables early.

/// Detect which locals need to be bound into a closure for functions within
/// this function. Also detect which variables this functions depends on a
/// parent closure for.
///
/// Returns a 2-tuple with `bound` and `parent_bound`.
fn detect_bindings(body: &Node) -> (Option<HashSet<String>>, Option<HashSet<String>>) {
    // Keep track of `let` and `var` declarations as we go.
    let mut locals = HashSet::new();
    // Locals which are bound by functions within.
    let mut bindings = HashSet::new();
    // Variables we depend on our parent for.
    let mut parent_bindings = HashSet::new();
    detect_bindings_visitor(body, &mut locals, &mut bindings, &mut parent_bindings);
    (none_if_empty(bindings), none_if_empty(parent_bindings))
}

fn detect_bindings_visitor(
    node: &Node,
    locals: &mut HashSet<String>,
    bindings: &mut HashSet<String>,
    parent_bindings: &mut HashSet<String>,
) {
    // Using macros to make things more concise and avoid extra allocations.
    macro_rules! identify {
        ($i:expr) => {{
            if !locals.contains($i) {
                parent_bindings.insert($i.clone());
            }
        }};
    }
    macro_rules! visit {
        ($x:expr) => {
            detect_bindings_visitor($x, locals, bindings, parent_bindings)
        };
    }
    match node {
        Node::Assignment(assignment) => {
            visit!(&assignment.lhs);
            visit!(&assignment.rhs);
        }
        Node::Block(block) => {
            for node in block.nodes.iter() {
                visit!(node);
            }
        }
        Node::Function(function) => {
            if let Some(name) = &function.name {
                locals.insert(name.clone());
            }
            if let Some(nested_parent_bindings) = &function.parent_bindings {
                for nested_parent_binding in nested_parent_bindings.iter() {
                    if locals.contains(nested_parent_binding) {
                        bindings.insert(nested_parent_binding.clone());
                    } else {
                        parent_bindings.insert(nested_parent_binding.clone());
                    }
                }
            }
        }
        Node::Identifier(identifier) => {
            identify!(&identifier.value);
        }
        Node::Infix(infix) => {
            visit!(&infix.lhs);
            visit!(&infix.rhs);
        }
        Node::Integer(_) => (),
        Node::Let(let_) => {
            locals.insert(let_.lhs.value.clone());
            if let Some(rhs) = &let_.rhs {
                visit!(rhs);
            }
        }
        Node::PostfixCall(call) => {
            visit!(&call.target);
            for argument in call.arguments.iter() {
                visit!(argument);
            }
        }
        Node::PostfixProperty(property) => {
            visit!(&property.target);
        }
        Node::Return(ret) => {
            if let Some(rhs) = &ret.rhs {
                visit!(rhs);
            }
        }
        Node::Var(var) => {
            locals.insert(var.lhs.value.clone());
            if let Some(rhs) = &var.rhs {
                visit!(rhs);
            }
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        self.name == other.name && self.body == other.body
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
pub struct Root {
    pub nodes: Vec<Node>,
    /// The root is like a function in that it's evaluated within a frame,
    /// therefore it needs to have closure bindings like a frame.
    pub bindings: Option<HashSet<String>>,
    /// These should be the union of imports and builtins.
    pub parent_bindings: Option<HashSet<String>>,
}

impl Root {
    pub fn new(nodes: Vec<Node>) -> Self {
        // Make a fake block for us to discover bindings in.
        let block = Block {
            nodes: nodes.clone(),
        };
        let (bindings, parent_bindings) = detect_bindings(&Node::Block(block));
        Self {
            nodes,
            bindings,
            parent_bindings,
        }
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
