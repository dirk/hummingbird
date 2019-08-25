use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::rc::{Rc, Weak};

use super::super::ast;
use super::super::ast_to_ir;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;
use super::frame::Closure;
use super::value::Value;

fn read_and_parse_file<P: AsRef<Path>>(path: P) -> Result<ast::Module, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    Ok(parser::parse(source))
}

fn compile_ast_into_module(
    ast_module: &ast::Module,
    name: String,
) -> Result<LoadedModule, Box<dyn Error>> {
    let ir_module = ast_to_ir::compile(ast_module);

    let loaded_module = LoadedModule::empty(name);
    let functions = ir::compiler::compile(&ir_module)
        .functions
        .into_iter()
        .map(|function| {
            let weak_loaded_module = Rc::downgrade(&loaded_module.0);
            LoadedFunction::new(weak_loaded_module, function)
        })
        .collect::<Vec<LoadedFunction>>();
    loaded_module.0.borrow_mut().functions = functions;

    Ok(loaded_module)
}

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<LoadedModule, Box<dyn Error>> {
    let ast_module = read_and_parse_file(path.as_ref())?;
    let name = path
        .as_ref()
        .to_str()
        .expect("Couldn't convert path to string")
        .to_owned();
    compile_ast_into_module(&ast_module, name)
}

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

type WeakLoadedModule = Weak<RefCell<InnerLoadedModule>>;

impl LoadedModule {
    pub fn empty(name: String) -> Self {
        Self(Rc::new(RefCell::new(InnerLoadedModule::empty(name))))
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

#[derive(Clone)]
struct InnerLoadedFunction {
    module: WeakLoadedModule,
    id: u16,
    bytecode: BytecodeFunction,
}

/// Handle to a loaded function.
#[derive(Clone)]
pub struct LoadedFunction(Rc<InnerLoadedFunction>);

impl LoadedFunction {
    fn new(module: WeakLoadedModule, function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerLoadedFunction {
            module,
            id: function.id,
            bytecode: BytecodeFunction::new(function),
        }))
    }

    pub fn id(&self) -> u16 {
        self.0.id
    }

    pub fn bytecode(&self) -> BytecodeFunction {
        self.0.bytecode.clone()
    }

    /// Returns whether or not this function binds/captures its environment
    /// when it is created.
    pub fn binds_on_create(&self) -> bool {
        self.0.bytecode.parent_bindings()
    }

    /// Returns a closure suitable for calling the function.
    pub fn build_closure_for_call(&self, parent: Option<Closure>) -> Option<Closure> {
        let bindings = self.0.bytecode.bindings();
        // Whether or not it needs to create bindings (a closure) when called.
        let binds_on_call = !bindings.is_empty() || self.0.bytecode.parent_bindings();
        if binds_on_call {
            let bindings = if bindings.is_empty() {
                None
            } else {
                Some(bindings)
            };
            Some(Closure::new(bindings, parent))
        } else {
            None
        }
    }

    pub fn module(&self) -> LoadedModule {
        let module = self.0.module.upgrade().expect("Module has been dropped");
        LoadedModule(module)
    }
}

pub struct InnerBytecodeFunction {
    function: bytecode::layout::Function,
}

impl InnerBytecodeFunction {
    pub fn name(&self) -> &str {
        &self.function.name
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

    pub fn has_bindings(&self) -> bool {
        !self.function.bindings.is_empty()
    }

    pub fn bindings(&self) -> HashSet<String> {
        self.function.bindings.clone()
    }

    pub fn parent_bindings(&self) -> bool {
        self.function.parent_bindings
    }
}

#[derive(Clone)]
pub struct BytecodeFunction(Rc<InnerBytecodeFunction>);

impl BytecodeFunction {
    pub fn new(function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerBytecodeFunction { function }))
    }
}

impl Deref for BytecodeFunction {
    type Target = InnerBytecodeFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
