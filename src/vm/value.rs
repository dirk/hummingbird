use std::fmt::{Debug, Error, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::frame::Closure;
use super::gc::{GcManaged, GcPtr, GcTrace};
use super::loader::{LoadedFunction, LoadedModule};

#[derive(Clone)]
pub struct Function {
    pub loaded_function: LoadedFunction,
    /// The closure in which the function was originally defined.
    pub parent: Option<Closure>,
}

#[derive(Clone)]
pub struct BuiltinFunction {
    call_target: Rc<dyn Fn(Vec<Value>) -> Value>,
}

impl BuiltinFunction {
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

#[derive(Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    BuiltinFunction(BuiltinFunction),
    // DynamicObject(Gc<GcCell<DynamicObject>>),
    Function(Function),
    Integer(i64),
    Module(LoadedModule),
    String(GcPtr<String>),
}

impl Value {
    pub fn make_function(loaded_function: LoadedFunction, parent: Option<Closure>) -> Self {
        Value::Function(Function {
            loaded_function,
            parent,
        })
    }

    pub fn make_builtin_function<V: Fn(Vec<Value>) -> Value + 'static>(call_target: V) -> Self {
        let builtin_function = BuiltinFunction::new(Rc::new(call_target));
        Value::BuiltinFunction(builtin_function)
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
                let name = function.loaded_function.qualified_name();
                write!(f, "Function({})", name)
            }
            Integer(value) => write!(f, "{}", value),
            Module(module) => write!(f, "Module({})", module.name()),
            String(value) => {
                let string = &**value;
                write!(f, "{:?}", string)
            }
        }
    }
}

impl GcManaged for Value {}

impl GcTrace for Value {
    fn trace(&self) {
        use Value::*;
        match self {
            Function(function) => {
                if let Some(parent) = &function.parent {
                    parent.trace();
                }
            }
            String(value) => value.mark(),
            _ => (),
        }
    }
}

impl GcManaged for String {}
