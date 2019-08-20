use std::collections::HashSet;
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

use super::super::ast::{Function as AstFunction, Node, Root};
use super::super::parser;
use super::frame::{Closure, Frame};
use super::value::Value;

struct InnerBuiltinFunction {
    name: String,
    call_target: fn(Vec<Value>) -> Value,
}

#[derive(Clone)]
pub struct BuiltinFunction(Rc<InnerBuiltinFunction>);

impl BuiltinFunction {
    pub fn new(name: String, call_target: fn(Vec<Value>) -> Value) -> Self {
        Self(Rc::new(InnerBuiltinFunction { name, call_target }))
    }

    pub fn call(&self, arguments: Vec<Value>) -> Value {
        let call_target = &self.0.call_target;
        call_target(arguments)
    }

    pub fn name(&self) -> String {
        self.0.name.clone()
    }
}

impl Debug for BuiltinFunction {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_str(&self.name())
    }
}

#[derive(Clone)]
pub struct Function {
    /// The closure in which this function was defined.
    pub closure: Option<Closure>,
    /// The AST node defining this function.
    node: AstFunction,
}

impl Function {
    fn new(closure: Option<Closure>, root: AstFunction) -> Self {
        Self {
            closure,
            node: root,
        }
    }

    pub fn bindings(&self) -> Option<HashSet<String>> {
        self.node.bindings.clone()
    }

    pub fn has_parent_bindings(&self) -> bool {
        self.node.parent_bindings.is_some()
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_str("function")
    }
}

fn merge_bindings(dest: &mut HashSet<String>, src: &Option<HashSet<String>>) {
    if let Some(bindings) = src {
        for binding in bindings.iter() {
            dest.insert(binding.clone());
        }
    }
}

struct Module {
    source: String,
    root: Root,
    closure: Closure,
}

impl Module {
    fn new_from_source(source: String) -> Self {
        let root = parser::parse(&source);

        let mut combined_bindings = HashSet::new();
        merge_bindings(&mut combined_bindings, &root.bindings);
        combined_bindings.insert("println".to_string());
        // NOTE: Parent bindings should only be imports. Anything else is
        //   probably use of undefined variables.
        // merge_bindings(&mut combined_bindings, &root.parent_bindings);
        // TODO: Make the builtins a "closure" that all other module closures
        //   can have as their parent for easier reuse and efficiency.
        let closure = Closure::new(Some(combined_bindings), None);
        closure.set(
            "println".to_string(),
            BuiltinFunction::new("println".to_string(), builtin_println).into(),
        );

        Self {
            source,
            root,
            closure,
        }
    }
}

fn builtin_println(arguments: Vec<Value>) -> Value {
    for argument in arguments.into_iter() {
        use Value::*;
        match argument {
            BuiltinFunction(builtin_function) => println!("{:?}", builtin_function),
            Integer(value) => println!("{}", value),
            other @ _ => println!("{:?}", other),
        };
    }
    Value::Null
}

#[derive(Debug)]
enum Action {
    // Regular implicit return of evaluating a statement or expression.
    Value(Value),
    // "Interrupt" to return a value from a function.
    Return(Option<Value>),
}

trait Eval {
    fn eval(&self, frame: &mut Frame) -> Action;
}

impl Eval for Module {
    fn eval(&self, frame: &mut Frame) -> Action {
        for node in self.root.nodes.iter() {
            node.eval(frame);
        }
        Action::Value(Value::Null)
    }
}

impl Eval for Node {
    fn eval(&self, frame: &mut Frame) -> Action {
        // Utility macro to get the value or early-return if it's a
        // `Return` action.
        macro_rules! value {
            ($x:expr) => {{
                match $x {
                    Action::Value(val) => val,
                    ret @ Action::Return(_) => return ret,
                }
            }};
        }

        let value: Value = match self {
            Node::Block(block) => {
                if let Some((last_node, nodes)) = block.nodes.split_last() {
                    for node in nodes {
                        value!(node.eval(frame));
                    }
                    value!(last_node.eval(frame))
                } else {
                    Value::Null
                }
            }
            Node::Function(ast_function) => {
                let function = Function::new(frame.closure_cloned(), ast_function.clone());
                let value = Value::Function(function);
                if let Some(name) = &ast_function.name {
                    frame.set(name.clone(), value.clone())
                }
                value
            }
            Node::Identifier(identifier) => {
                let name = identifier.value.clone();
                frame
                    .get(name.clone())
                    .expect(&format!("Not found: {}", name))
            }
            Node::Integer(integer) => Value::Integer(integer.value),
            Node::Let(let_) => {
                let rhs = match &let_.rhs {
                    Some(rhs) => value!(rhs.eval(frame)),
                    None => Value::Null,
                };
                let lhs: String = let_.lhs.value.clone();
                frame.set(lhs, rhs);
                Value::Null
            }
            Node::PostfixCall(call) => {
                let target = value!(call.target.eval(frame));
                let mut arguments = Vec::<Value>::with_capacity(call.arguments.len());
                for argument in call.arguments.iter() {
                    let value = value!(argument.eval(frame));
                    arguments.push(value)
                }
                assert_eq!(arguments.len(), call.arguments.len(),);
                eval_function(target, arguments)
            }
            Node::Return(ret) => {
                if let Some(rhs) = &ret.rhs {
                    let value = value!(rhs.eval(frame));
                    return Action::Return(Some(value));
                } else {
                    return Action::Return(None);
                }
            }
            Node::Var(var) => {
                let rhs = match &var.rhs {
                    Some(rhs) => value!(rhs.eval(frame)),
                    None => Value::Null,
                };
                let lhs: String = var.lhs.value.clone();
                frame.set(lhs, rhs);
                Value::Null
            }
            other @ _ => unreachable!("Cannot eval: {}", other),
        };
        Action::Value(value)
    }
}

fn eval_function(target: Value, arguments: Vec<Value>) -> Value {
    match target {
        Value::BuiltinFunction(builtin_function) => builtin_function.call(arguments),
        Value::Function(function) => {
            let mut frame = Frame::new_for_function(&function);
            match function.node.body.eval(&mut frame) {
                // Implicit return.
                Action::Value(value) => value,
                // Explicit return.
                Action::Return(value) => value.unwrap_or(Value::Null),
            }
        }
        other @ _ => unreachable!("Cannot call: {:?}", other),
    }
}

pub struct Vm {}

impl Vm {
    pub fn new() -> Self {
        Self {}
    }

    pub fn eval_source(&self, source: String) {
        let module = Module::new_from_source(source);
        let mut frame = Frame::new_with_closure(module.closure.clone());
        module.eval(&mut frame);
    }
}
