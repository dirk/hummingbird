use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use super::super::super::target::bytecode;
use super::super::frame::Closure;
use super::super::value::Value;
use super::LoadedFunction;

pub struct InnerLoadedModule {
    /// The name of the module. This should almost always be the canonicalized
    /// path to the source file.
    name: String,
    functions: Vec<LoadedFunction>,
    /// The module's static scope; it holds:
    ///   - Imports
    ///   - Bound or exported variables
    ///   - Bound or exported functions
    static_closure: Closure,
    // A module loaded into memory is uninitialized. Only after it has been
    // evaluated (and imports and exports resolved) is it initialized.
    initialized: bool,
    imports: Imports,
    exports: Exports,
}

impl InnerLoadedModule {
    fn empty(name: String, builtins_closure: Option<Closure>) -> Self {
        Self {
            name,
            functions: vec![],
            static_closure: Closure::new_static(builtins_closure),
            initialized: false,
            imports: Imports::new(),
            exports: Exports::new(),
        }
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct LoadedModule(Rc<RefCell<InnerLoadedModule>>);

impl LoadedModule {
    pub fn from_bytecode(
        module: bytecode::layout::Module,
        name: String,
        builtins_closure: Option<Closure>,
    ) -> Self {
        let inner = Rc::new(RefCell::new(InnerLoadedModule::empty(
            name,
            builtins_closure,
        )));
        let functions = module
            .functions
            .into_iter()
            .map(|function| {
                let weak_loaded_module = Rc::downgrade(&inner);
                LoadedFunction::new(weak_loaded_module, function)
            })
            .collect::<Vec<LoadedFunction>>();
        inner.borrow_mut().functions = functions;
        Self(inner)
    }

    pub fn name(&self) -> String {
        self.0.borrow().name.clone()
    }

    pub fn main(&self) -> LoadedFunction {
        self.0.borrow().functions[0].clone()
    }

    pub fn static_closure(&self) -> Closure {
        self.0.borrow().static_closure.clone()
    }

    /// Should only be called by the REPL!
    pub fn override_static_closure(&self, static_closure: Closure) {
        self.0.borrow_mut().static_closure = static_closure;
    }

    pub fn function(&self, id: u16) -> LoadedFunction {
        self.0
            .borrow()
            .functions
            .iter()
            .find(|&function| function.id() == id)
            .expect("Function not found")
            .clone()
    }

    pub fn get_exports(&self) -> HashMap<String, Option<Value>> {
        self.0.borrow().exports.exports.to_owned()
    }

    pub fn insert_export<N: Into<String>>(&self, name: N, value: Value) {
        self.0
            .borrow_mut()
            .exports
            .exports
            .insert(name.into(), Some(value));
    }
}

pub type WeakLoadedModule = Weak<RefCell<InnerLoadedModule>>;

impl TryInto<LoadedModule> for WeakLoadedModule {
    type Error = ();

    fn try_into(self) -> Result<LoadedModule, Self::Error> {
        match self.upgrade() {
            Some(inner) => Ok(LoadedModule(inner)),
            None => Err(()),
        }
    }
}

struct Imports {
    /// Maps canonicalized import paths to the corresponding module.
    imports: HashMap<PathBuf, LoadedModule>,
}

impl Imports {
    fn new() -> Self {
        Self {
            imports: HashMap::new(),
        }
    }
}

struct Exports {
    /// Exports start out as `None`s and are then filled in as the module
    /// is initialized.
    exports: HashMap<String, Option<Value>>,
}

impl Exports {
    fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }
}
