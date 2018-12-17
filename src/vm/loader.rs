use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::super::ir;
use super::super::target::bytecode;

// Manages loading IR units and compiling them into callable functions.
//
// NOTE: Eventually this should probably handle loading source files. Really
//   the whole continuum between source and specialized native code.
pub struct Loader {
    units: Vec<SharedLoadedUnit>,
}

impl Loader {
    pub fn new() -> Self {
        Self { units: vec![] }
    }

    pub fn load(&mut self, ir_unit: ir::layout::Unit) -> SharedLoadedUnit {
        let bytecode_functions = ir::compiler::compile(&ir_unit)
            .functions
            .into_iter()
            .map(|function| Rc::new(function))
            .collect::<Vec<Rc<bytecode::layout::Function>>>();

        let loaded_unit: SharedLoadedUnit = LoadedUnit { bytecode_functions }.into();
        self.units.push(loaded_unit.clone());
        loaded_unit
    }
}

// Opaque wrapper around a reference-counted loaded unit.
#[derive(Clone)]
pub struct SharedLoadedUnit(Rc<RefCell<LoadedUnit>>);

impl SharedLoadedUnit {
    pub fn main(&self) -> LoadedFunctionHandle {
        let function = self.0.borrow().bytecode_functions[0].clone();
        LoadedFunctionHandle {
            unit: Rc::downgrade(&self.0),
            function,
        }
    }

    pub fn function(&self, id: u16) -> LoadedFunctionHandle {
        let this = &self.0;
        let function = this
            .borrow()
            .bytecode_functions
            .iter()
            .find(|&function| function.id == id)
            .expect("Function not found")
            .clone();
        LoadedFunctionHandle {
            unit: Rc::downgrade(this),
            function,
        }
    }
}

impl From<LoadedUnit> for SharedLoadedUnit {
    fn from(loaded_unit: LoadedUnit) -> SharedLoadedUnit {
        SharedLoadedUnit(Rc::new(RefCell::new(loaded_unit)))
    }
}

pub struct LoadedUnit {
    bytecode_functions: Vec<Rc<bytecode::layout::Function>>,
}

// TODO: Distinguish between a call target (shared, cloned in a `Value`) and
//   a handle (in a `Frame` on and off stack).
#[derive(Clone)]
pub struct LoadedFunctionHandle {
    unit: Weak<RefCell<LoadedUnit>>,
    function: Rc<bytecode::layout::Function>,
}

impl LoadedFunctionHandle {
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

    pub fn unit(&self) -> SharedLoadedUnit {
        let upgraded = self.unit.upgrade().expect("Unit has been dropped");
        SharedLoadedUnit(upgraded)
    }
}
