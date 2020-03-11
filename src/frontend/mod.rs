use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::compiler;
use super::parser::{self, ParseError, TokenStream};
use super::type_ast::{self, Module as TModule, TypeError};

/// Combines underlying parsing and type errors with the canonicalized path and
/// source code of the module that produced the error.
#[derive(Debug)]
pub enum CompileError {
    Parse(ParseError, PathBuf, String),
    Type(TypeError, PathBuf, String),
    CircularDependency(PathBuf),
}

#[derive(Clone)]
pub struct Manager(Rc<ManagerInner>);

struct ManagerInner {
    modules: RefCell<HashSet<Module>>,
    /// Keep track of what modules are being actively loaded.
    loading: RefCell<HashSet<PathBuf>>,
}

impl Manager {
    pub fn new() -> Self {
        Self(Rc::new(ManagerInner {
            modules: RefCell::new(HashSet::new()),
            loading: RefCell::new(HashSet::new()),
        }))
    }

    pub fn compile_main(entry_path: PathBuf) -> Result<Self, CompileError> {
        let manager = Self::new();
        let entry = manager.load(entry_path)?;

        // let modules = compiler::ir::compile_modules(manager.0.modules.borrow().iter());
        // compiler::compile_modules(modules);

        let ir_modules =
            compiler::ir::compile_modules(manager.0.modules.borrow().iter(), &entry).get_modules();

        compiler::target::compile_modules(&ir_modules);

        Ok(manager)
    }

    fn start_loading(&self, path: PathBuf) -> Result<(), CompileError> {
        {
            let modules = self.0.modules.borrow();
            for module in modules.iter() {
                if module.path() == path {
                    return Err(CompileError::CircularDependency(path));
                }
            }
        }
        {
            let loading = self.0.loading.borrow();
            if loading.contains(&path) {
                return Err(CompileError::CircularDependency(path));
            }
        }
        let mut loading = self.0.loading.borrow_mut();
        loading.insert(path);
        Ok(())
    }

    fn finish_loading(&self, module: Module) {
        let path = module.path().to_path_buf();
        {
            let mut loading = self.0.loading.borrow_mut();
            if !loading.remove(&path) {
                unreachable!(
                    "Module was not in the loading set: {}",
                    path.to_str().unwrap()
                )
            }
        }
        let mut modules = self.0.modules.borrow_mut();
        modules.insert(module);
    }

    pub fn load(&self, path: PathBuf) -> Result<Module, CompileError> {
        let path = path.canonicalize().unwrap();
        // Check for circular dependencies and track that this module is being
        // actively loaded in `loading`.
        self.start_loading(path.clone())?;
        let next_id = {
            let modules = self.0.modules.borrow();
            modules.len()
        };
        let module = Module::new(next_id, path);
        module.load(self.clone())?;
        // Remove the module from `loading` and add it to the loaded `modules`.
        self.finish_loading(module.clone());
        Ok(module)
    }
}

#[derive(Clone)]
pub struct Module(Rc<ModuleInner>);

impl Module {
    pub fn new(id: usize, path: PathBuf) -> Self {
        Self(Rc::new(ModuleInner {
            id,
            path,
            typed: RefCell::new(None),
        }))
    }

    pub fn id(&self) -> usize {
        self.0.id
    }

    pub fn path(&self) -> &Path {
        self.0.path.as_path()
    }

    pub fn borrow_typed(&self) -> Ref<TModule> {
        Ref::map(self.0.typed.borrow(), |module| module.as_ref().unwrap())
    }

    pub fn load(&self, manager: Manager) -> Result<(), CompileError> {
        let path = self.0.path.clone();
        let source = std::fs::read_to_string(path.clone()).unwrap();

        let mut token_stream = TokenStream::from_string(source.clone());
        let parsed = parser::parse_module(&mut token_stream)
            .map_err(|err| CompileError::Parse(err, path.clone(), source.clone()))?;

        let typed = type_ast::translate_module(parsed)
            .map_err(|err| CompileError::Type(err, path.clone(), source.clone()))?;

        {
            let mut mutable = self.0.typed.borrow_mut();
            *mutable = Some(typed);
        }
        Ok(())
    }

    /// Consumes oneself returning the typed AST within.
    pub fn unwrap_ast(&self) -> Ref<TModule> {
        Ref::map(self.0.typed.borrow(), |option| option.as_ref().unwrap())
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Module {}

impl Hash for Module {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state)
    }
}

struct ModuleInner {
    id: usize,
    /// Canonicalized path of the module's source file.
    path: PathBuf,
    /// Will be filled in once the module is finished initializing.
    typed: RefCell<Option<TModule>>,
}
