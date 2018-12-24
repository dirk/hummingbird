use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use gc::{Finalize, Gc, GcCell, Trace};

use super::call_target::CallTarget;
use super::loader::LoadedFunction;

#[derive(Clone)]
pub struct DynamicFunction {
    pub call_target: CallTarget,
}

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
    DynamicFunction(DynamicFunction),
    DynamicObject(Gc<GcCell<DynamicObject>>),
    Integer(i64),
    NativeFunction(NativeFunction),
    Null,
}

impl Value {
    pub fn from_dynamic_function(loaded_function: LoadedFunction) -> Self {
        let dynamic_function = DynamicFunction {
            call_target: CallTarget {
                function: loaded_function,
            },
        };
        Value::DynamicFunction(dynamic_function)
    }

    pub fn make_native_function<V: Fn(Vec<Value>) -> Value + 'static>(call_target: V) -> Self {
        let native_function = NativeFunction {
            call_target: Rc::new(call_target),
        };
        Value::NativeFunction(native_function)
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
