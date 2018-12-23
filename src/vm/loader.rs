use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::rc::{Rc, Weak};

use super::super::ast;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;

// Manages loading IR units and compiling them into callable functions.
//
// NOTE: Eventually this should probably handle loading source files. Really
//   the whole continuum between source and specialized native code.
pub struct Loader {
    units: Vec<LoadedModule>,
}

impl Loader {
    pub fn new() -> Self {
        Self { units: vec![] }
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

        self.units.push(loaded_module.clone());
        Ok(loaded_module)
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct LoadedModule(Rc<RefCell<InnerLoadedModule>>);

type WeakLoadedModule = Weak<RefCell<InnerLoadedModule>>;

impl LoadedModule {
    fn empty() -> Self {
        InnerLoadedModule { functions: vec![] }.into()
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
}

impl From<InnerLoadedModule> for LoadedModule {
    fn from(loaded_unit: InnerLoadedModule) -> LoadedModule {
        LoadedModule(Rc::new(RefCell::new(loaded_unit)))
    }
}

// TODO: Hold on to all stages of compilation.
pub struct InnerLoadedModule {
    functions: Vec<LoadedFunction>,
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
    pub fn instruction(
        &self,
        basic_block_address: usize,
        instruction_address: usize,
    ) -> bytecode::layout::Instruction {
        let block = &self.function.basic_blocks[basic_block_address];
        block.instructions[instruction_address].clone()
    }

    pub fn locals_names(&self) -> Vec<String> {
        self.function.locals_names.clone()
    }

    pub fn unit(&self) -> LoadedModule {
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
