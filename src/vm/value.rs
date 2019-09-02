use std::fmt::{Debug, Error, Formatter};
use std::fs::File;
use std::mem;
use std::rc::Rc;

use super::errors::VmError;
use super::frame::Closure;
use super::gc::{GcAllocator, GcManaged, GcPtr, GcTrace};
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

type BuiltinFunctionFn = fn(Vec<Value>, &mut GcAllocator) -> Result<Value, VmError>;

#[derive(Clone)]
pub struct BuiltinFunction {
    call_target: Rc<BuiltinFunctionFn>,
}

impl BuiltinFunction {
    pub fn new(call_target: Rc<BuiltinFunctionFn>) -> Self {
        Self { call_target }
    }

    pub fn call(&self, arguments: Vec<Value>, gc: &mut GcAllocator) -> Result<Value, VmError> {
        (self.call_target)(arguments, gc)
    }
}

// pub struct DynamicObject {
//     properties: HashMap<String, Value>,
// }

/// If a builtin object supports properties then it will include a static
/// function to look up a property. Using generics here since we know the LUT
/// will receive the variant of the object (eg. a file if it's the LUT for
/// a file builtin object).
type BuiltinObjectPropertyLUT<T> = fn(&T, &str) -> Option<Value>;

/// Similar to the property LUT but more concise to make method LUT functions
/// shorter to write.
pub type BuiltinObjectMethodLUT<T> = fn(&T, Symbol) -> Option<BuiltinMethodFn>;

/// Specialized container for builtin objects used by the native stdlib
/// (see `builtins::stdlib`).
#[derive(Clone)]
pub enum BuiltinObject {
    File(GcPtr<File>, BuiltinObjectMethodLUT<GcPtr<File>>),
}

impl BuiltinObject {
    pub fn get_property(&self, value: Symbol) -> Option<Value> {
        match self {
            BuiltinObject::File(this, method_lut) => {
                self.execute_method_lut(method_lut, this, value)
            }
        }
    }

    /// Calls the given method LUT. If it returns a method function pointer
    /// then it builds a bound method and returns that.
    #[inline]
    fn execute_method_lut<T>(
        &self,
        method_lut: &BuiltinObjectMethodLUT<T>,
        this: &T,
        value: Symbol,
    ) -> Option<Value> {
        match method_lut(this, value) {
            Some(method) => {
                // Convert ourselves back into a value to be the receiver
                // of the method.
                let receiver = Value::BuiltinObject(Box::new(self.clone()));
                Some(Value::make_builtin_bound_method(receiver, method))
            }
            None => None,
        }
    }
}

impl Debug for BuiltinObject {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "BuiltinObject(")?;
        match self {
            BuiltinObject::File(_, _) => write!(f, "File")?,
        }
        write!(f, ")")
    }
}

impl GcTrace for BuiltinObject {
    fn trace(&self) {
        match self {
            BuiltinObject::File(file, _) => file.mark(),
        }
    }
}

pub type BuiltinMethodFn = fn(Value, Vec<Value>, &mut GcAllocator) -> Result<Value, VmError>;

#[derive(Clone)]
pub enum BoundMethod {
    Builtin(Value, BuiltinMethodFn),
}

#[derive(Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    BoundMethod(Box<BoundMethod>),
    BuiltinFunction(BuiltinFunction),
    BuiltinObject(Box<BuiltinObject>),
    // DynamicObject(Gc<GcCell<DynamicObject>>),
    Function(Box<Function>),
    Integer(i64),
    Module(LoadedModule),
    String(GcPtr<String>),
    Symbol(Symbol),
}

impl Value {
    pub fn make_function(loaded_function: LoadedFunction, parent: Option<Closure>) -> Self {
        Value::Function(Box::new(Function {
            loaded_function,
            parent,
        }))
    }

    pub fn make_builtin_bound_method(receiver: Value, call_target: BuiltinMethodFn) -> Self {
        let method = BoundMethod::Builtin(receiver, call_target);
        Value::BoundMethod(Box::new(method))
    }

    pub fn make_builtin_function(call_target: BuiltinFunctionFn) -> Self {
        let builtin_function = BuiltinFunction::new(Rc::new(call_target));
        Value::BuiltinFunction(builtin_function)
    }

    pub fn make_string(string: String, gc: &mut GcAllocator) -> Self {
        let allocated = gc.allocate(string);
        Value::String(allocated)
    }

    pub fn type_name(&self) -> &str {
        match self {
            Value::Null => "Null",
            Value::Boolean(_) => "Boolean",
            Value::BoundMethod(_) => "BoundMethod",
            Value::BuiltinFunction(_) => "BuiltinFunction",
            Value::BuiltinObject(_) => "BuiltinObject",
            Value::Function(_) => "Function",
            Value::Integer(_) => "Integer",
            Value::Module(_) => "Module",
            Value::String(_) => "String",
            Value::Symbol(_) => "Symbol",
        }
    }
}

/// On 64-bit platforms the `Value` should be 2 words in size.
#[cfg(target_pointer_width = "64")]
const BYTE_SIZE_OF_VALUE: usize = 16;

/// Hack to statically check the size of the `Value`.
#[allow(dead_code, unreachable_code)]
fn static_assert_value_size() {
    unsafe { mem::transmute::<Value, [u8; BYTE_SIZE_OF_VALUE]>(unreachable!()) };
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Value::Null => write!(f, "null"),
            Value::Boolean(value) => write!(f, "{:?}", value),
            Value::BoundMethod(_) => write!(f, "BoundMethod"),
            Value::BuiltinFunction(_) => write!(f, "BuiltinFunction"),
            Value::BuiltinObject(object) => write!(f, "{:?}", object),
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
