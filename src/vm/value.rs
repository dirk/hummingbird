use std::fmt::{Debug, Error, Formatter};
use std::ops::Deref;
use std::rc::Rc;

// use gc::{Finalize, Gc, GcCell, Trace};

use super::call_target::CallTarget;
use super::frame::Closure;
use super::loader::{LoadedFunction, LoadedModule};

#[derive(Clone)]
pub struct DynamicFunction {
    pub call_target: CallTarget,
    pub closure: Option<Closure>,
}

#[derive(Clone)]
pub struct NativeFunction {
    call_target: Rc<dyn Fn(Vec<Value>) -> Value>,
}

impl NativeFunction {
    pub fn new(call_target: Rc<dyn Fn(Vec<Value>) -> Value>) -> Self {
        Self { call_target }
    }

    pub fn call(&self, arguments: Vec<Value>) -> Value {
        self.call_target.deref()(arguments)
    }
}

// pub struct DynamicObject {
//     properties: HashMap<String, Value>,
// }
//
// impl Finalize for DynamicObject {}
//
// unsafe impl Trace for DynamicObject {
//     custom_trace!(this, {
//         for (_key, value) in this.properties.iter() {
//             mark(value);
//         }
//     });
// }

#[derive(Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    BuiltinFunction(NativeFunction),
    // DynamicObject(Gc<GcCell<DynamicObject>>),
    Function(DynamicFunction),
    Integer(i64),
    Module(LoadedModule),
    String(String),
}

impl Value {
    pub fn from_dynamic_function(
        loaded_function: LoadedFunction,
        closure: Option<Closure>,
    ) -> Self {
        let dynamic_function = DynamicFunction {
            call_target: CallTarget {
                function: loaded_function,
            },
            closure,
        };
        Value::Function(dynamic_function)
    }

    pub fn make_native_function<V: Fn(Vec<Value>) -> Value + 'static>(call_target: V) -> Self {
        let native_function = NativeFunction::new(Rc::new(call_target));
        Value::BuiltinFunction(native_function)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use Value::*;
        match self {
            Null => write!(f, "null"),
            Boolean(value) => write!(f, "{:?}", value),
            BuiltinFunction(_) => write!(f, "BuiltinFunction"),
            Function(function) => {
                let name = function.call_target.function.qualified_name();
                write!(f, "Function({})", name)
            }
            Integer(value) => write!(f, "{}", value),
            Module(module) => write!(f, "Module({})", module.name()),
            String(value) => write!(f, "{:?}", value),
        }
    }
}

// impl Finalize for Value {}
//
// unsafe impl Trace for Value {
//     custom_trace!(this, {
//         match this {
//             Value::DynamicObject(dynamic_object) => mark(dynamic_object),
//             _ => (),
//         }
//     });
// }
