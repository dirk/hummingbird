use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

use super::super::ast::{Function as AstFunction, Node, Root};
use super::super::parser;
use super::frame::{BuiltinFrame, CapturedFrame, Frame, HeapFrame, StackFrame, StaticFrame};
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
    /// The captured/closed-over frame for functions which use external variables.
    captured_frame: Option<CapturedFrame>,
    /// Whether this function's own frame should be captured for functions within it.
    captured: bool,
    /// The AST node defining this function.
    node: AstFunction,
}

impl Function {
    fn new(captured_frame: Option<CapturedFrame>, root: AstFunction) -> Self {
        Self {
            captured_frame,
            captured: root.captured,
            node: root,
        }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_str("function")
    }
}

struct Module {
    source: String,
    root: Root,
    static_frame: StaticFrame,
}

impl Module {
    fn new_from_source(source: String, builtin_frame: BuiltinFrame) -> Self {
        let root = parser::parse(&source);
        Self {
            source,
            root,
            static_frame: StaticFrame::new(builtin_frame),
        }
    }
}

#[derive(Debug)]
enum Action {
    // Regular implicit return of evaluating a statement or expression.
    Value(Value),
    // "Interrupt" to return a value from a function.
    Return(Option<Value>),
}

trait Eval<F: Frame> {
    fn eval(&self, frame: &mut F) -> Action;
}

impl<F: Frame> Eval<F> for Module {
    fn eval(&self, frame: &mut F) -> Action {
        for node in self.root.nodes.iter() {
            node.eval(frame);
        }
        Action::Value(Value::Null)
    }
}

impl<F: Frame> Eval<F> for Node {
    fn eval(&self, frame: &mut F) -> Action {
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
                // If the function captures its environment then we need to capture the current
                // frame since that is its declaring environment.
                let captured_frame = if ast_function.captures.is_some() {
                    Some(frame.capture())
                } else {
                    None
                };
                let function = Function::new(captured_frame, ast_function.clone());
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
            // If the function's own frame/environment is captured (for
            // functions within it) then its frame needs to be on the heap
            // rather than on the stack.
            let action = if function.captured {
                let mut frame = HeapFrame::new(function.captured_frame);
                function.node.body.eval(&mut frame)
            } else {
                let mut frame = StackFrame::new(function.captured_frame);
                function.node.body.eval(&mut frame)
            };
            match action {
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
    builtin_frame: BuiltinFrame,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            builtin_frame: BuiltinFrame::bootstrap(),
        }
    }

    pub fn eval_source(&self, source: String) {
        let module = Module::new_from_source(source, self.builtin_frame.clone());
        module.eval(&mut module.static_frame.clone());
    }
}
