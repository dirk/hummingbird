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

impl PartialEq for Function {
    /// Functions are equal if they're calling the same `LoadedFunction` and
    /// using the same `Closure`.
    fn eq(&self, other: &Self) -> bool {
        let loaded_function_eq = self.loaded_function.ptr_eq(&other.loaded_function);
        let closure_eq = match (&self.parent, &other.parent) {
            (Some(own), Some(other)) => &own == &other,
            (None, None) => true,
            _ => false,
        };
        loaded_function_eq && closure_eq
    }
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

impl GcTrace for BuiltinObject {
    fn trace(&self) {
        match self {
            BuiltinObject::File(file) => file.mark(),
        }
    }
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
        match self {
            Value::Null => write!(f, "null"),
            Value::Boolean(value) => write!(f, "{:?}", value),
            Value::BuiltinFunction(_) => write!(f, "BuiltinFunction"),
            Value::BuiltinObject(object) => {
                write!(f, "BuiltinObject(")?;
                match object {
                    BuiltinObject::File(_) => write!(f, "File")?,
                }
                write!(f, ")")
            }
            Value::Function(function) => {
                let name = function.loaded_function.qualified_name();
                write!(f, "Function({})", name)
            }
            Value::Integer(value) => write!(f, "{}", value),
            Value::Module(module) => write!(f, "Module({})", module.name()),
            Value::String(value) => {
                let string = &**value;
                write!(f, "{:?}", string)
            }
            Value::Symbol(symbol) => {
                let string = desymbolicate(symbol).unwrap_or("?".to_string());
                write!(f, "Symbol({}:{})", symbol.id(), string)
            }
        }
    }
}

impl GcManaged for Value {}

impl GcTrace for Value {
    fn trace(&self) {
        match self {
            Value::BuiltinObject(object) => {
                object.trace();
            }
            Value::Function(function) => {
                if let Some(parent) = &function.parent {
                    parent.trace();
                }
            }
            Value::String(value) => value.mark(),
            _ => (),
        }
    }
}

impl GcManaged for File {}

impl GcManaged for String {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::super::target::bytecode::layout as bytecode;
    use super::super::loader::LoadedModule;
    use super::Function;

    fn empty_function(id: u16) -> bytecode::Function {
        bytecode::Function {
            id,
            name: "test".to_string(),
            registers: 0,
            instructions: vec![],
            source_mappings: vec![],
            locals: 0,
            locals_names: vec![],
            bindings: HashSet::new(),
            parent_bindings: false,
        }
    }

    #[test]
    fn it_tests_functions_for_equality() {
        let loaded_module = LoadedModule::from_bytecode(
            bytecode::Module {
                functions: vec![empty_function(0), empty_function(1)],
            },
            "test".to_string(),
            "".to_string(),
            None,
        );

        let first = Function {
            loaded_function: loaded_module.function(0),
            parent: None,
        };
        let first_clone = first.clone();
        assert!(first == first_clone);

        let second = Function {
            loaded_function: loaded_module.function(1),
            parent: None,
        };
        assert!(first != second);
    }
}
