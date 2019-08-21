use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

use super::super::ast::{Function as AstFunction, Module as AstModule, Node};
use super::super::parser;
use super::builtins::build as build_builtins;
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
    closure: Option<Closure>,
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

    pub fn build_closure(&self) -> Option<Closure> {
        // If the function has its own bindings (used by child function(s)) or
        // if it uses bindings from its parent then we need to set up a closure
        // for it.
        let needs_closure = self.node.has_bindings() || self.node.has_parent_bindings();
        if needs_closure {
            let bindings = self.node.get_bindings();
            let parent = self.closure.clone();
            Some(Closure::new(bindings, parent))
        } else {
            None
        }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_str("function")
    }
}

pub struct Module {
    source: String,
    node: AstModule,
    closure: Closure,
}

impl Module {
    fn new_from_source(source: String, builtins: Option<Closure>) -> Self {
        let node = parser::parse(&source);

        // NOTE: Parent bindings should only be imports or builtins. Anything
        //   else is probably use of undefined variables.
        if let Some(parent_bindings) = node.get_parent_bindings() {
            for binding in parent_bindings.iter() {
                if !builtins
                    .as_ref()
                    .map(|closure| closure.has(binding.clone()))
                    .unwrap_or(false)
                {
                    panic!("Dangling root parent binding: {}", binding);
                }
            }
        }
        let closure = Closure::new(node.get_bindings(), builtins);

        Self {
            source,
            node,
            closure,
        }
    }

    pub fn get_closure(&self) -> Closure {
        self.closure.clone()
    }
}

#[derive(Debug)]
enum Action {
    // Regular implicit return of evaluating a statement or expression.
    Value(Value),
    // "Interrupt" to return a value from a function.
    Return(Option<Value>),
}

trait Eval {
    fn eval(&self, frame: &mut Frame, vm: &mut Vm) -> Action;
}

impl Eval for Module {
    fn eval(&self, frame: &mut Frame, vm: &mut Vm) -> Action {
        for node in self.node.nodes.iter() {
            node.eval(frame, vm);
        }
        Action::Value(Value::Null)
    }
}

impl Eval for Node {
    fn eval(&self, frame: &mut Frame, vm: &mut Vm) -> Action {
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
            Node::Assignment(assignment) => {
                let rhs = value!(assignment.rhs.eval(frame, vm));
                match assignment.lhs.as_ref() {
                    Node::Identifier(identifier) => {
                        frame.set(identifier.value.clone(), rhs.clone());
                    }
                    other @ _ => unreachable!("Cannot assign to: {}", other),
                }
                rhs
            }
            Node::Block(block) => {
                if let Some((last_node, nodes)) = block.nodes.split_last() {
                    for node in nodes {
                        value!(node.eval(frame, vm));
                    }
                    value!(last_node.eval(frame, vm))
                } else {
                    Value::Null
                }
            }
            Node::Function(ast_function) => {
                let function = Function::new(frame.get_closure(), ast_function.clone());
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
                    Some(rhs) => value!(rhs.eval(frame, vm)),
                    None => Value::Null,
                };
                let lhs: String = let_.lhs.value.clone();
                frame.set(lhs, rhs);
                Value::Null
            }
            Node::PostfixCall(call) => {
                let target = value!(call.target.eval(frame, vm));
                let mut arguments = Vec::<Value>::with_capacity(call.arguments.len());
                for argument in call.arguments.iter() {
                    let value = value!(argument.eval(frame, vm));
                    arguments.push(value)
                }
                assert_eq!(arguments.len(), call.arguments.len(),);
                eval_function(target, arguments, vm)
            }
            Node::Return(ret) => {
                if let Some(rhs) = &ret.rhs {
                    let value = value!(rhs.eval(frame, vm));
                    return Action::Return(Some(value));
                } else {
                    return Action::Return(None);
                }
            }
            Node::Var(var) => {
                let rhs = match &var.rhs {
                    Some(rhs) => value!(rhs.eval(frame, vm)),
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

fn eval_function(target: Value, arguments: Vec<Value>, vm: &mut Vm) -> Value {
    match target {
        Value::BuiltinFunction(builtin_function) => builtin_function.call(arguments),
        Value::Function(function) => {
            let mut frame = Frame::new_for_function(&function);
            match function.node.body.eval(&mut frame, vm) {
                // Implicit return.
                Action::Value(value) => value,
                // Explicit return.
                Action::Return(value) => value.unwrap_or(Value::Null),
            }
        }
        other @ _ => unreachable!("Cannot call: {:?}", other),
    }
}

pub struct Vm {
    builtins: Option<Closure>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            builtins: Some(build_builtins()),
        }
    }

    pub fn eval_source(&mut self, source: String) {
        let module = Module::new_from_source(source, self.builtins.clone());
        let mut frame = Frame::new_for_module(&module);
        module.eval(&mut frame, self);
    }
}
