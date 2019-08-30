use std::cell::RefCell;

use super::errors::VmError;
use super::frame::Closure;
use super::gc::GcAllocator;
use super::loader::LoadedModule;
use super::symbol::desymbolicate;
use super::value::Value;

mod stdlib;

thread_local! {
    static FILE: Loader = Loader::new(stdlib::file::load);
}

// Handy abstraction for memoized module loading.
struct Loader {
    cell: RefCell<Option<LoadedModule>>,
    load: fn() -> LoadedModule,
}

impl Loader {
    fn new(load: fn() -> LoadedModule) -> Self {
        Self {
            cell: RefCell::new(None),
            load,
        }
    }

    fn load_once(&self) -> LoadedModule {
        if let Some(loaded) = &*self.cell.borrow() {
            return loaded.clone();
        }
        let loaded = (self.load)();
        *self.cell.borrow_mut() = Some(loaded.clone());
        loaded
    }
}

pub fn try_load_stdlib(name: &str) -> Option<LoadedModule> {
    let module = match name {
        "file" => FILE.with(Loader::load_once),
        _ => return None,
    };
    Some(module)
}

/// Creates a new builtins closure that is suitable for being the root closure
/// (closure of last resort for resolution) of all loaded modules.
pub fn build_closure() -> Closure {
    let closure = Closure::new_builtins();

    let functions = vec![("println".to_owned(), builtin_println)];

    for (name, function) in functions.into_iter() {
        closure.set_directly(name, Value::make_builtin_function(function));
    }

    closure
}

fn builtin_println(arguments: Vec<Value>, _: &mut GcAllocator) -> Result<Value, VmError> {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Boolean(value) => println!("{:?}", value),
            Value::BuiltinObject(_) => println!("BuiltinObject"),
            Value::BuiltinFunction(_) => println!("BuiltinFunction"),
            Value::Function(function) => {
                println!("Function({})", function.loaded_function.qualified_name())
            }
            Value::Integer(value) => println!("{}", value),
            Value::Module(module) => println!("Module({})", module.name()),
            Value::Null => println!("null"),
            Value::String(value) => {
                let string = &**value;
                println!("{}", string)
            }
            Value::Symbol(symbol) => {
                let string = desymbolicate(symbol).expect("Symbol not found");
                println!(":{}", string)
            }
        }
    };
    Ok(Value::Null)
}
