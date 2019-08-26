use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::{Rc, Weak};

use super::super::super::target::bytecode::layout as bytecode;
use super::super::frame::Closure;
use super::super::value::Value;
use super::LoadedFunction;

pub struct InnerLoadedModule {
    /// The name of the module. This should almost always be the canonicalized
    /// path to the source file.
    name: String,
    functions: Vec<LoadedFunction>,
    /// The closure holding the static scope that holds:
    ///   - Imports
    ///   - Bound or exported variables
    ///   - Bound or exported functions
    static_closure: Closure,
    // A module loaded into memory is uninitialized. Only after it has been
    // evaluated (and imports and exports resolved) is it initialized.
    initialized: bool,
    imports: ModuleImports,
    exports: ModuleExports,
}

impl InnerLoadedModule {
    fn empty(name: String) -> Self {
        Self {
            name,
            functions: vec![],
            static_closure: Closure::new_static(),
            initialized: false,
            imports: ModuleImports::new(),
            exports: ModuleExports::new(),
        }
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct LoadedModule(Rc<RefCell<InnerLoadedModule>>);

impl LoadedModule {
    pub fn empty(name: String) -> Self {
        Self(Rc::new(RefCell::new(InnerLoadedModule::empty(name))))
    }

    pub fn from_bytecode(module: bytecode::Module, name: String) -> Self {
        let loaded = Self::empty(name);
        let functions = module
            .functions
            .into_iter()
            .map(|function| {
                let weak_loaded_module = Rc::downgrade(&loaded.0);
                LoadedFunction::new(weak_loaded_module, function)
            })
            .collect::<Vec<LoadedFunction>>();
        loaded.0.borrow_mut().functions = functions;
        loaded
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

    pub fn get_named_exports(&self) -> HashMap<String, Option<Value>> {
        self.0.borrow().exports.exports.to_owned()
    }

    // Used by bootstrapping: see `prelude.rs`.
    pub fn add_named_export<N: Into<String>>(&self, name: N, value: Value) {
        self.0
            .borrow_mut()
            .exports
            .exports
            .insert(name.into(), Some(value));
    }

    pub fn get_constant<N: AsRef<str>>(&self, name: N) -> Value {
        self.0.borrow().imports.get_import(name.as_ref())
    }

    pub fn set_import<N: Into<String>>(&self, name: N, value: Value) {
        self.0.borrow_mut().imports.set_import(name, value)
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

struct ModuleImports {
    // Imports are resolved from `None`s into values at the beginning of
    // module initialization.
    imports: HashMap<String, Option<Value>>,
}

impl ModuleImports {
    fn new() -> Self {
        Self {
            imports: HashMap::new(),
        }
    }

    fn get_import(&self, name: &str) -> Value {
        // First look for the entry in the map.
        let import = self
            .imports
            .get(name)
            .expect(&format!("Import not found: {}", name));

        // Then check whether or not it's initialized.
        import
            .clone()
            .expect(&format!("Uninitialized import: {}", name))
    }

    fn set_import<N: Into<String>>(&mut self, name: N, value: Value) {
        self.imports.insert(name.into(), Some(value));
    }
}

pub struct ModuleExports {
    // Exports will start out as `None`s and are then filled in once the module
    // is initialized.
    pub exports: HashMap<String, Option<Value>>,
}

impl ModuleExports {
    fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }
}
