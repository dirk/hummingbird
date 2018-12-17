use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use gc::{Finalize, Gc, GcCell, Trace};

use super::super::target::bytecode::layout::{Function, Unit};

#[derive(Clone)]
pub struct NativeFunction {
    call_target: Rc<Fn(Vec<Value>) -> Value>,
}

impl NativeFunction {
    pub fn new(call_target: Rc<Fn(Vec<Value>) -> Value>) -> Self {
        Self { call_target }
    }

    pub fn call(&self, arguments: Vec<Value>) -> Value {
        self.call_target.deref()(arguments)
    }
}

pub struct DynamicObject {
    properties: HashMap<String, Value>,
}

impl Finalize for DynamicObject {}

unsafe impl Trace for DynamicObject {
    custom_trace!(this, {
        for (_key, value) in this.properties.iter() {
            mark(value);
        }
    });
}

#[derive(Clone)]
pub enum Value {
    DynamicObject(Gc<GcCell<DynamicObject>>),
    DynamicFunction(Rc<Unit>, Rc<Function>),
    Integer(i64),
    NativeFunction(NativeFunction),
    Null,
}

impl Value {
    pub fn dynamic_function(self) -> Option<(Rc<Unit>, Rc<Function>)> {
        match self {
            Value::DynamicFunction(unit, function) => Some((unit, function)),
            _ => None,
        }
    }
}

impl Finalize for Value {}

unsafe impl Trace for Value {
    custom_trace!(this, {
        match this {
            Value::DynamicObject(dynamic_object) => mark(dynamic_object),
            _ => (),
        }
    });
}
