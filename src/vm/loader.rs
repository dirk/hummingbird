use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::rc::{Rc, Weak};

use super::super::ast;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;
use super::value::Value;

// Manages loading IR units and compiling them into callable functions.
//
// NOTE: Eventually this should probably handle loading source files. Really
//   the whole continuum between source and specialized native code.
pub struct Loader {
    modules: Vec<LoadedModule>,
}

impl Loader {
    pub fn new() -> Self {
        Self { modules: vec![] }
    }

    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<LoadedModule, Box<dyn Error>> {
        let program = self.read_file_into_program(path)?;
        self.load_program(program)
    }

    // TODO: Cache translation of source into AST.
    fn read_file_into_program<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<ast::Program, Box<dyn Error>> {
        let source = fs::read_to_string(path)?;
        Ok(parser::parse(source))
    }

    fn load_program(&mut self, program: ast::Program) -> Result<LoadedModule, Box<dyn Error>> {
        let ir_unit = ast::compiler::compile(&program);
        let bytecode_unit = ir::compiler::compile(&ir_unit);

        let loaded_module = LoadedModule::empty();
        let functions = ir::compiler::compile(&ir_unit)
            .functions
            .into_iter()
            .map(|function| {
                InnerLoadedFunction {
                    unit: Rc::downgrade(&loaded_module.0),
                    function,
                }
                .into()
            })
            .collect::<Vec<LoadedFunction>>();
        loaded_module.0.borrow_mut().functions = functions;

        self.modules.push(loaded_module.clone());
        Ok(loaded_module)
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct LoadedModule(Rc<RefCell<InnerLoadedModule>>);

type WeakLoadedModule = Weak<RefCell<InnerLoadedModule>>;

impl LoadedModule {
    pub fn empty() -> Self {
        InnerLoadedModule::empty().into()
    }

    pub fn main(&self) -> LoadedFunction {
        self.0.borrow().functions[0].clone()
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

    pub fn constant<N: AsRef<str>>(&self, name: N) -> Value {
        self.0
            .borrow_mut()
            .get_constant(name)
    }
}

impl From<InnerLoadedModule> for LoadedModule {
    fn from(loaded_unit: InnerLoadedModule) -> LoadedModule {
        LoadedModule(Rc::new(RefCell::new(loaded_unit)))
    }
}

impl Deref for LoadedModule {
    type Target = RefCell<InnerLoadedModule>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO: Hold on to all stages of compilation.
pub struct InnerLoadedModule {
    functions: Vec<LoadedFunction>,
    // A module loaded into memory is uninitialized. Only after it has been
    // evaluated (and imports and exports resolved) is it initialized.
    initialized: bool,
    pub imports: ModuleImports,
    pub exports: ModuleExports,
}

impl InnerLoadedModule {
    fn empty() -> Self {
        Self {
            functions: vec![],
            initialized: false,
            imports: ModuleImports::new(),
            exports: ModuleExports::new(),
        }
    }

    // Used by bootstrapping: see `prelude.rs`.
    pub fn add_named_export<N: Into<String>>(&mut self, name: N, value: Value) {
        self.exports.named_exports.insert(name.into(), Some(value));
    }

    pub fn get_constant<N: AsRef<str>>(&mut self, name: N) -> Value {
        self.imports.get_import(name.as_ref())
    }
}

pub struct ModuleImports {
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

    pub fn set_import<N: Into<String>>(&mut self, name: N, value: Value) {
        self.imports.insert(name.into(), Some(value));
    }
}

pub struct ModuleExports {
    // Exports will start out as `None`s and are then filled in once the module
    // is initialized.
    pub named_exports: HashMap<String, Option<Value>>,
    default_export: Option<Value>,
}

impl ModuleExports {
    fn new() -> Self {
        Self {
            named_exports: HashMap::new(),
            default_export: None,
        }
    }
}

// Handle to a loaded bytecode function and its unit. Used by `Frame`.
#[derive(Clone)]
pub struct InnerLoadedFunction {
    unit: WeakLoadedModule,
    function: bytecode::layout::Function,
}

impl InnerLoadedFunction {
    #[inline]
    pub fn id(&self) -> u16 {
        self.function.id
    }

    #[inline]
    pub fn registers(&self) -> u8 {
        self.function.registers
    }

    #[inline]
    pub fn locals(&self) -> u8 {
        self.function.locals
    }

    #[inline]
    pub fn instruction(&self, instruction_address: usize) -> bytecode::layout::Instruction {
        self.function.instructions[instruction_address].clone()
    }

    pub fn locals_names(&self) -> Vec<String> {
        self.function.locals_names.clone()
    }

    pub fn module(&self) -> LoadedModule {
        let upgraded = self.unit.upgrade().expect("Unit has been dropped");
        LoadedModule(upgraded)
    }
}

#[derive(Clone)]
pub struct LoadedFunction(Rc<InnerLoadedFunction>);

impl From<InnerLoadedFunction> for LoadedFunction {
    fn from(loaded_function: InnerLoadedFunction) -> LoadedFunction {
        LoadedFunction(Rc::new(loaded_function))
    }
}

impl Deref for LoadedFunction {
    type Target = InnerLoadedFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
