use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::super::ast::{Node, Root};
use super::super::parser;

struct InnerBuiltinFunction {
    name: String,
    call_target: fn(Vec<Value>) -> Value,
}

#[derive(Clone)]
struct BuiltinFunction(Rc<InnerBuiltinFunction>);

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

trait Frame {
    fn get(&self, name: String) -> Option<Value>;
    fn set(&mut self, name: String, value: Value);
}

#[derive(Clone)]
struct BuiltinFrame(Rc<HashMap<String, Value>>);

impl BuiltinFrame {
    fn bootstrap() -> Self {
        let mut locals = HashMap::<String, Value>::new();
        let functions = vec![("println".to_string(), builtin_println)];
        for (name, function) in functions.into_iter() {
            locals.insert(
                name.clone(),
                BuiltinFunction::new(name, function).into(),
            );
        }
        Self(Rc::new(locals))
    }
}

impl Frame for BuiltinFrame {
    fn get(&self, name: String) -> Option<Value> {
        self.0.get(&name).map(Value::to_owned)
    }

    fn set(&mut self, _name: String, _value: Value) {
        unreachable!("Cannot set in builtin scope")
    }
}

#[derive(Clone, Debug)]
enum Value {
    Null,
    BuiltinFunction(BuiltinFunction),
    Integer(i64),
}

impl From<BuiltinFunction> for Value {
    fn from(builtin_function: BuiltinFunction) -> Self {
        Self::BuiltinFunction(builtin_function)
    }
}

struct InnerStaticFrame {
    locals: HashMap<String, Value>,
    builtin_frame: BuiltinFrame,
}

#[derive(Clone)]
struct StaticFrame(Rc<RefCell<InnerStaticFrame>>);

impl StaticFrame {
    fn new(builtin_frame: BuiltinFrame) -> Self {
        Self(Rc::new(RefCell::new(InnerStaticFrame {
            locals: HashMap::new(),
            builtin_frame,
        })))
    }
}

impl Frame for StaticFrame {
    fn get(&self, name: String) -> Option<Value> {
        let inner = self.0.borrow();
        if let Some(value) = inner.locals.get(&name) {
            Some(value.to_owned())
        } else {
            inner.builtin_frame.get(name)
        }
    }

    fn set(&mut self, name: String, value: Value) {
        let mut inner = self.0.borrow_mut();
        inner.locals.insert(name, value);
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

trait Eval<F: Frame> {
    fn eval(&self, frame: &mut F) -> Value;
}

impl<F: Frame> Eval<F> for Module {
    fn eval(&self, frame: &mut F) -> Value {
        for node in self.root.nodes.iter() {
            node.eval(frame);
        }
        Value::Null
    }
}

impl<F: Frame> Eval<F> for Node {
    fn eval(&self, frame: &mut F) -> Value {
        match self {
            Node::Identifier(identifier) => {
                let name = identifier.value.clone();
                frame
                    .get(name.clone())
                    .expect(&format!("Not found: {}", name))
            }
            Node::Integer(integer) => Value::Integer(integer.value),
            Node::Let(let_) => {
                let rhs = match &let_.rhs {
                    Some(rhs) => rhs.eval(frame),
                    None => Value::Null,
                };
                let lhs: String = let_.lhs.value.clone();
                frame.set(lhs, rhs);
                Value::Null
            }
            Node::PostfixCall(call) => {
                let target = call.target.eval(frame);
                let arguments: Vec<Value> = call
                    .arguments
                    .iter()
                    .map(|argument| argument.eval(frame))
                    .collect();
                match target {
                    Value::BuiltinFunction(builtin_function) => builtin_function.call(arguments),
                    other @ _ => unreachable!("Cannot call: {:?}", other),
                }
            }
            Node::Var(var) => {
                let rhs = match &var.rhs {
                    Some(rhs) => rhs.eval(frame),
                    None => Value::Null,
                };
                let lhs: String = var.lhs.value.clone();
                frame.set(lhs, rhs);
                Value::Null
            }
            other @ _ => unreachable!("Cannot eval: {}", other),
        }
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
