use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::value::Value;
use super::vm::BuiltinFunction;

pub trait Frame {
    fn get(&self, name: String) -> Option<Value>;
    fn set(&mut self, name: String, value: Value);
    /// Every function gets access to the static frame (module scope) in
    /// which it was defined.
    fn capture(&self) -> CapturedFrame;
}

#[derive(Clone)]
pub struct BuiltinFrame(Rc<HashMap<String, Value>>);

impl BuiltinFrame {
    pub fn bootstrap() -> Self {
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

struct InnerStaticFrame {
    locals: HashMap<String, Value>,
    builtin_frame: BuiltinFrame,
}

#[derive(Clone)]
pub struct StaticFrame(Rc<RefCell<InnerStaticFrame>>);

impl StaticFrame {
    pub fn new(builtin_frame: BuiltinFrame) -> Self {
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

pub struct StackFrame {
    locals: HashMap<String, Value>,
    captured_frame: Option<CapturedFrame>,
}

impl StackFrame {
    pub fn new(captured_frame: Option<CapturedFrame>) -> Self {
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
pub struct HeapFrame(Rc<RefCell<InnerHeapFrame>>);

impl HeapFrame {
    pub fn new(captured_frame: Option<CapturedFrame>) -> Self {
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

/// Sized object which holds a frame on the heap that has been captured by
/// a function.
#[derive(Clone)]
pub enum CapturedFrame {
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
