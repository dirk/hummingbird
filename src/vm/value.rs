use std::fmt::{Debug, Error, Formatter};
use std::fs::File;
use std::rc::Rc;

use super::errors::VmError;
use super::frame::Closure;
use super::gc::GcAllocator;
use super::gc::{GcManaged, GcPtr, GcTrace};
use super::loader::{LoadedFunction, LoadedModule};
use super::symbol::{desymbolicate, Symbol};

#[derive(Clone)]
pub struct Function {
    pub loaded_function: LoadedFunction,
    /// The closure in which the function was originally defined.
    pub parent: Option<Closure>,
}

type BuiltinCallTarget = fn(Vec<Value>, &mut GcAllocator) -> Result<Value, VmError>;

#[derive(Clone)]
pub struct BuiltinFunction {
    call_target: Rc<BuiltinCallTarget>,
}

impl BuiltinFunction {
    pub fn new(call_target: Rc<BuiltinCallTarget>) -> Self {
        Self { call_target }
    }

    pub fn call(&self, arguments: Vec<Value>, gc: &mut GcAllocator) -> Result<Value, VmError> {
        (self.call_target)(arguments, gc)
    }
}

// pub struct DynamicObject {
//     properties: HashMap<String, Value>,
// }

/// Specialized container for builtin objects used by the native stdlib
/// (see `builtins::stdlib`).
#[derive(Clone)]
pub enum BuiltinObject {
    File(GcPtr<File>),
}

#[derive(Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    BuiltinFunction(BuiltinFunction),
    BuiltinObject(BuiltinObject),
    // DynamicObject(Gc<GcCell<DynamicObject>>),
    Function(Function),
    Integer(i64),
    Module(LoadedModule),
    String(GcPtr<String>),
    Symbol(Symbol),
}

impl Value {
    pub fn make_function(loaded_function: LoadedFunction, parent: Option<Closure>) -> Self {
        Value::Function(Function {
            loaded_function,
            parent,
        })
    }

    pub fn make_builtin_function(call_target: BuiltinCallTarget) -> Self {
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
            BuiltinObject(_) => write!(f, "BuiltinObject"),
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
            Symbol(symbol) => {
                let string = desymbolicate(symbol).unwrap_or("?".to_string());
                write!(f, "Symbol({}:{})", symbol.id(), string)
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

impl GcManaged for File {}

impl GcManaged for String {}
