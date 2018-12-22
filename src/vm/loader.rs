use std::cell::RefCell;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use super::super::ir;
use super::super::target::bytecode;

// Manages loading IR units and compiling them into callable functions.
//
// NOTE: Eventually this should probably handle loading source files. Really
//   the whole continuum between source and specialized native code.
pub struct Loader {
    units: Vec<LoadedUnit>,
}

impl Loader {
    pub fn new() -> Self {
        Self { units: vec![] }
    }

    pub fn load(&mut self, ir_unit: ir::layout::Unit) -> LoadedUnit {
        let loaded_unit = LoadedUnit::empty();
        let functions = ir::compiler::compile(&ir_unit)
            .functions
            .into_iter()
            .map(|function| {
                LoadedFunction(Rc::new(InnerLoadedFunction {
                    unit: Rc::downgrade(&loaded_unit.0),
                    function,
                }))
            })
            .collect::<Vec<LoadedFunction>>();
        loaded_unit.0.borrow_mut().functions = functions;

        self.units.push(loaded_unit.clone());
        loaded_unit
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct LoadedUnit(Rc<RefCell<InnerLoadedUnit>>);

type WeakLoadedUnit = Weak<RefCell<InnerLoadedUnit>>;

impl LoadedUnit {
    fn empty() -> Self {
        InnerLoadedUnit { functions: vec![] }.into()
    }

    pub fn main(&self) -> LoadedFunction {
        self.0.borrow().functions[0].clone()
    }

    pub fn function(&self, id: u16) -> LoadedFunction {
        let this = &self.0;
        this.borrow()
            .functions
            .iter()
            .find(|&function| function.id() == id)
            .expect("Function not found")
            .clone()
    }
}

impl From<InnerLoadedUnit> for LoadedUnit {
    fn from(loaded_unit: InnerLoadedUnit) -> LoadedUnit {
        LoadedUnit(Rc::new(RefCell::new(loaded_unit)))
    }
}

pub struct InnerLoadedUnit {
    functions: Vec<LoadedFunction>,
}

// Handle to a loaded bytecode function and its unit. Used by `Frame`.
#[derive(Clone)]
pub struct InnerLoadedFunction {
    unit: WeakLoadedUnit,
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

    pub fn unit(&self) -> LoadedUnit {
        let upgraded = self.unit.upgrade().expect("Unit has been dropped");
        LoadedUnit(upgraded)
    }
}

#[derive(Clone)]
pub struct LoadedFunction(Rc<InnerLoadedFunction>);

impl Deref for LoadedFunction {
    type Target = InnerLoadedFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
