use std::collections::HashMap;
use std::rc::Rc;

use gc::{Gc, GcCell, Finalize, Trace};

#[derive(Clone)]
struct NativeFunction {
    call_target: Rc<Fn(Vec<Value>) -> Value>,
}

struct DynamicObject {
    properties: HashMap<String, Value>,
}

impl Finalize for DynamicObject {}

unsafe impl Trace for DynamicObject {
    custom_trace!(this, {
        for (key, value) in this.properties.iter() {
            mark(key);
            mark(value);
        }
    });
}

#[derive(Clone)]
enum Value {
    DynamicObject(Gc<GcCell<DynamicObject>>),
    Integer(i64),
    NativeFunction(NativeFunction),
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
