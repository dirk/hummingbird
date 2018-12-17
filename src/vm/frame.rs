use std::cell::RefCell;
use std::rc::Rc;

use super::super::target::bytecode::layout::{Function, Instruction, Reg, Unit};

use super::value::Value;

// Frames can live outside of the stack (eg. closures) and can be mutated from
// places outside the stack (again, closures), therefore we need reference
// counting (`Rc`) and interior mutability (`RefCell`).
pub type SharedFrame = Rc<RefCell<Frame>>;

// The first three fields should *not* be changed after the frame
// is initialized.
pub struct Frame {
    pub unit: Rc<Unit>,
    function: Rc<Function>,
    lexical_parent: Option<SharedFrame>,
    pub return_register: Reg,
    registers: Vec<Value>,
    locals: Vec<Value>,
    current_block: usize,
    current_address: usize,
}

impl Frame {
    pub fn new(
        unit: Rc<Unit>,
        function: Rc<Function>,
        lexical_parent: Option<SharedFrame>,
        return_register: Reg,
    ) -> Self {
        let registers = function.registers;
        let locals = function.locals;
        Self {
            unit,
            function,
            lexical_parent,
            return_register,
            registers: vec![Value::Null; registers as usize],
            locals: vec![Value::Null; locals as usize],
            current_block: 0,
            current_address: 0,
        }
    }

    #[inline]
    pub fn current(&self) -> Instruction {
        let block = &self.function.basic_blocks[self.current_block];
        block.instructions[self.current_address].clone()
    }

    #[inline]
    pub fn advance(&mut self) {
        self.current_address += 1;
    }

    #[inline]
    fn offset_register(index: Reg) -> usize {
        (index as usize) - 1
    }

    pub fn read_register(&self, index: Reg) -> Value {
        self.registers[Frame::offset_register(index)].clone()
    }

    pub fn write_register(&mut self, index: Reg, value: Value) {
        if index == 0 {
            return;
        }
        self.registers[Frame::offset_register(index)] = value;
    }

    pub fn get_local(&self, index: u8) -> Value {
        self.locals[index as usize].clone()
    }

    pub fn get_local_lexical(&self, name: &String) -> Value {
        let index = self
            .function
            .locals_names
            .iter()
            .position(|local| local == name);
        if let Some(index) = index {
            self.locals[index].clone()
        } else {
            if let Some(ref lexical_parent) = self.lexical_parent {
                lexical_parent.borrow().get_local_lexical(name)
            } else {
                panic!("Out of parents")
            }
        }
    }

    pub fn set_local(&mut self, index: u8, value: Value) {
        self.locals[index as usize] = value;
    }
}
