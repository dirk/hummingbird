use std::fmt::{Debug, Error, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::frame::Closure;
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
    Symbol(Symbol),
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
