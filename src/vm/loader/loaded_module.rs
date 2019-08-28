use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use super::super::super::target::bytecode;
use super::super::frame::Closure;
use super::super::gc::GcTrace;
use super::super::value::Value;
use super::LoadedFunction;

pub struct InnerLoadedModule {
    /// The name of the module. This should almost always be the canonicalized
    /// path to the source file.
    name: String,
    /// The source code of the module.
    source: String,
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
    fn empty(name: String, source: String, builtins_closure: Option<Closure>) -> Self {
        Self {
            name,
            source,
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
        source: String,
        builtins_closure: Option<Closure>,
    ) -> Self {
        let inner = Rc::new(RefCell::new(InnerLoadedModule::empty(
            name,
            source,
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

    pub fn source(&self) -> String {
        self.0.borrow().source.clone()
    }

    /// Returns a path to the directory in which relative imports can be
    /// processed for this module.
    pub fn relative_import_path(&self) -> Option<PathBuf> {
        let name = self.name();
        // FIXME: Make REPL detection and handling smarter.
        if name.starts_with("repl[") {
            return env::current_dir().ok();
        }
        let path = Path::new(&name);
        if path.is_file() {
            return path.parent().map(Path::to_path_buf);
        }
        None
    }

    pub fn main(&self) -> LoadedFunction {
        self.0.borrow().functions[0].clone()
    }

    pub fn static_closure(&self) -> Closure {
        self.0.borrow().static_closure.clone()
    }

    pub fn initialized(&self) -> bool {
        self.0.borrow().initialized
    }

    pub fn set_initialized(&self) {
        let mut inner = self.0.borrow_mut();
        inner.initialized = true;
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

    pub fn get_export<N: AsRef<str>>(&self, name: N) -> Option<Value> {
        self.0
            .borrow_mut()
            .exports
            .exports
            .get(name.as_ref())
            .map(Clone::clone)
    }

    pub fn set_export<N: Into<String>>(&self, name: N, value: Value) {
        self.0
            .borrow_mut()
            .exports
            .exports
            .insert(name.into(), value);
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

impl GcTrace for LoadedModule {
    fn trace(&self) {
        let inner = self.0.borrow();
        inner.static_closure.trace();
        for export in inner.exports.exports.values() {
            export.trace();
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
    exports: HashMap<String, Value>,
}

impl Exports {
    fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }
}
