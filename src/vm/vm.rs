use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

use super::super::ast::{Function as AstFunction, Node, Root};
use super::super::parser;
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

#[derive(Clone)]
pub struct Function {
    /// The captured/closed-over frame for functions which use external variables.
    captured_frame: Option<CapturedFrame>,
    /// Whether this function's own frame should be captured for functions within it.
    captured: bool,
    /// The AST node defining this function.
    node: AstFunction,
}

#[derive(Clone)]
enum CapturedFrame {
    Static(StaticFrame),
    Heap(HeapFrame),
}

impl Frame for CapturedFrame {
    fn get(&self, name: String) -> Option<Value> {
        match self {
            CapturedFrame::Static(static_frame) => static_frame.get(name),
            CapturedFrame::Heap(heap_frame) => heap_frame.get(name),
        }
    }

    fn set(&mut self, name: String, value: Value) {
        match self {
            CapturedFrame::Static(static_frame) => static_frame.set(name, value),
            CapturedFrame::Heap(heap_frame) => heap_frame.set(name, value),
        }
    }

    fn capture(&self) -> CapturedFrame {
        unreachable!("CapturedFrame not implemented")
    }
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

trait Frame {
    fn get(&self, name: String) -> Option<Value>;
    fn set(&mut self, name: String, value: Value);
    /// Every function gets access to the static frame (module scope) in
    /// which it was defined.
    fn capture(&self) -> CapturedFrame;
}

#[derive(Clone)]
struct BuiltinFrame(Rc<HashMap<String, Value>>);

impl BuiltinFrame {
    fn bootstrap() -> Self {
        let mut locals = HashMap::<String, Value>::new();
        let functions = vec![("println".to_string(), builtin_println)];
        for (name, function) in functions.into_iter() {
            locals.insert(name.clone(), BuiltinFunction::new(name, function).into());
        }
        Self(Rc::new(locals))
    }
}

impl Frame for BuiltinFrame {
    fn get(&self, name: String) -> Option<Value> {
        self.0.get(&name).map(Value::to_owned)
    }

    fn set(&mut self, _name: String, _value: Value) {
        unreachable!("BuiltinFrame doesn't support set")
    }

    fn capture(&self) -> CapturedFrame {
        unreachable!("BuiltinFrame cannot be captured")
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

    fn capture(&self) -> CapturedFrame {
        CapturedFrame::Static(self.clone())
    }
}

struct StackFrame {
    locals: HashMap<String, Value>,
    captured_frame: Option<CapturedFrame>,
}

impl StackFrame {
    fn new(captured_frame: Option<CapturedFrame>) -> Self {
        Self {
            locals: HashMap::new(),
            captured_frame,
        }
    }
}

impl Frame for StackFrame {
    fn get(&self, name: String) -> Option<Value> {
        if let Some(value) = self.locals.get(&name) {
            return Some(value.to_owned());
        }
        if let Some(captured_frame) = &self.captured_frame {
            return captured_frame.get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Value) {
        self.locals.insert(name, value);
    }

    fn capture(&self) -> CapturedFrame {
        unreachable!("StackFrame cannot be captured")
    }
}

struct InnerHeapFrame {
    locals: HashMap<String, Value>,
    captured_frame: Option<CapturedFrame>,
}

#[derive(Clone)]
struct HeapFrame(Rc<RefCell<InnerHeapFrame>>);

impl HeapFrame {
    fn new(captured_frame: Option<CapturedFrame>) -> Self {
        Self(Rc::new(RefCell::new(InnerHeapFrame {
            locals: HashMap::new(),
            captured_frame,
        })))
    }
}

impl Frame for HeapFrame {
    fn get(&self, name: String) -> Option<Value> {
        let inner = self.0.borrow();
        if let Some(value) = inner.locals.get(&name) {
            return Some(value.to_owned());
        }
        if let Some(captured_frame) = &inner.captured_frame {
            return captured_frame.get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Value) {
        let mut inner = self.0.borrow_mut();
        inner.locals.insert(name, value);
    }

    fn capture(&self) -> CapturedFrame {
        CapturedFrame::Heap(self.clone())
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
