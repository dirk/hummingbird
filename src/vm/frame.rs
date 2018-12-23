use std::cell::RefCell;
use std::rc::Rc;

use super::super::target::bytecode::layout::{Instruction, Reg};

use super::loader::{LoadedFunction, LoadedModule};
use super::value::Value;

// Frames can live outside of the stack (eg. closures) and can be mutated from
// places outside the stack (again, closures), therefore we need reference
// counting (`Rc`) and interior mutability (`RefCell`).
pub type SharedFrame = Rc<RefCell<Frame>>;

// An action for the VM to do.
pub enum Action {
    Call(SharedFrame),
    Return(Reg, Value),
}

pub trait Frame {
    // Run the frame's fetch-execute loop. Will be different depending on if
    // it's a bytecode or native frame.
    fn run(&mut self) -> Action;

    fn write_register(&mut self, index: Reg, value: Value);

    fn get_local_lexical(&self, name: &String) -> Value;
}

// Frame evaluating a bytecode function.
//
// The first two fields should *not* be changed after the frame
// is initialized.
pub struct BytecodeFrame {
    // TODO: Replace `LoadedFunction` with an abstraction that can support
    //   specialized instruction sequences.
    function: LoadedFunction,
    lexical_parent: Option<SharedFrame>,
    pub return_register: Reg,
    registers: Vec<Value>,
    locals: Vec<Value>,
    current_address: usize,
}

impl BytecodeFrame {
    pub fn new(
        function: LoadedFunction,
        lexical_parent: Option<SharedFrame>,
        return_register: Reg,
    ) -> Self {
        let registers = function.registers();
        let locals = function.locals();
        Self {
            function,
            lexical_parent,
            return_register,
            registers: vec![Value::Null; registers as usize],
            locals: vec![Value::Null; locals as usize],
            current_address: 0,
        }
    }

    pub fn unit(&self) -> LoadedModule {
        self.function.unit()
    }

    #[inline]
    pub fn current(&self) -> Instruction {
        self.function.instruction(self.current_address)
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
        self.registers[BytecodeFrame::offset_register(index)].clone()
    }

    pub fn get_local(&self, index: u8) -> Value {
        self.locals[index as usize].clone()
    }

    pub fn set_local(&mut self, index: u8, value: Value) {
        self.locals[index as usize] = value;
    }
}

impl Frame for BytecodeFrame {
    fn run(&mut self) -> Action {
        loop {
            let instruction = self.current();

            match &instruction {
                Instruction::GetLocal(lval, index) => {
                    self.write_register(*lval, self.get_local(*index));
                    self.advance();
                }
                Instruction::GetLocalLexical(lval, name) => {
                    self.write_register(*lval, self.get_local_lexical(name));
                    self.advance();
                }
                Instruction::SetLocal(index, rval) => {
                    self.set_local(*index, self.read_register(*rval));
                    self.advance();
                }
                Instruction::MakeFunction(lval, id) => {
                    let function = self.unit().function(*id);
                    let value = Value::from_dynamic_function(function);
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::MakeInteger(lval, value) => {
                    self.write_register(*lval, Value::Integer(*value));
                    self.advance();
                }
                Instruction::Branch(destination) => {
                    self.current_address = *destination as usize;
                }
                Instruction::Call(lval, target, arguments) => {
                    let return_register = *lval;
                    let target = self.read_register(*target);
                    let arguments = arguments
                        .iter()
                        .map(|argument| self.read_register(*argument))
                        .collect::<Vec<Value>>();
                    match target {
                        Value::DynamicFunction(dynamic_function) => {
                            // TODO: Make `CallTarget` able to do specialization.
                            let function = dynamic_function.call_target.function;
                            let frame = Rc::new(RefCell::new(BytecodeFrame::new(
                                function,
                                Option::None,
                                return_register,
                            )));
                            // Be at the next instruction when control flow returns to us.
                            self.advance();
                            return Action::Call(frame);
                        }
                        Value::NativeFunction(native_function) => {
                            let result = native_function.call(arguments);
                            self.write_register(return_register, result);
                            self.advance();
                        }
                        _ => panic!("Cannot call"),
                    }
                }
                Instruction::Return(rval) => {
                    let value = self.read_register(*rval);
                    return Action::Return(self.return_register, value);
                }
                Instruction::ReturnNull => {
                    return Action::Return(self.return_register, Value::Null);
                }
                _ => panic!("Cannot dispatch: {:?}", instruction),
            }
        }
    }

    fn get_local_lexical(&self, name: &String) -> Value {
        let index = self
            .function
            .locals_names()
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

    fn write_register(&mut self, index: Reg, value: Value) {
        if index == 0 {
            return;
        }
        self.registers[BytecodeFrame::offset_register(index)] = value;
    }
}
