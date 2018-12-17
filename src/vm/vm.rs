use std::cell::RefCell;
use std::rc::Rc;

use super::super::target::bytecode::layout::{Function, Instruction, Reg, Unit};

use super::value::{NativeFunction, Value};

// Frames can live outside of the stack (eg. closures) and can be mutated from
// places outside the stack (again, closures), therefore we need referencing
// counting (`Rc`) and interior mutability (`RefCell`).
type SharedFrame = Rc<RefCell<Frame>>;

// The first three fields should *not* be changed after the frame
// is initialized.
struct Frame {
    unit: Rc<Unit>,
    function: Rc<Function>,
    lexical_parent: Option<SharedFrame>,
    return_register: Reg,
    registers: Vec<Value>,
    locals: Vec<Value>,
    current_block: usize,
    current_address: usize,
}

impl Frame {
    fn new(
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
    fn current(&self) -> Instruction {
        let block = &self.function.basic_blocks[self.current_block];
        block.instructions[self.current_address].clone()
    }

    #[inline]
    fn offset_register(index: Reg) -> usize {
        (index as usize) - 1
    }

    fn advance(&mut self) {
        self.current_address += 1;
    }

    fn read_register(&self, index: Reg) -> Value {
        self.registers[Frame::offset_register(index)].clone()
    }

    fn write_register(&mut self, index: Reg, value: Value) {
        if index == 0 {
            return;
        }
        self.registers[Frame::offset_register(index)] = value;
    }

    fn get_local(&self, index: u8) -> Value {
        self.locals[index as usize].clone()
    }

    fn get_local_lexical(&self, name: &String) -> Value {
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

    fn set_local(&mut self, index: u8, value: Value) {
        self.locals[index as usize] = value;
    }
}

pub struct Vm {
    stack: Vec<SharedFrame>,
}

enum Action {
    // Advance to the next instruction.
    Advance,
    // Execute a dynamic function call (ie. push onto the stack).
    CallDynamic(Reg, Value, Vec<Value>),
    CallNative(Reg, NativeFunction, Vec<Value>),
    Return(Value),
}

fn prelude_println(arguments: Vec<Value>) -> Value {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Integer(value) => println!("{}", value),
            _ => unreachable!(),
        }
    };
    Value::Null
}

impl Vm {
    pub fn run_main(unit: Rc<Unit>) {
        let prelude = Vm::build_prelude();
        let main_function = Rc::new(unit.functions[0].clone());
        let frame = Frame::new(unit, main_function, Some(prelude), 0);
        let mut vm = Self {
            stack: vec![Rc::new(RefCell::new(frame))],
        };
        vm.run();
    }

    fn build_prelude() -> SharedFrame {
        let prelude_unit = Unit {
            functions: vec![Function {
                id: 0,
                name: "prelude".to_string(),
                registers: 0,
                basic_blocks: vec![],
                locals: 1,
                locals_names: vec!["println".to_string()],
            }],
        };
        let prelude_main_function = prelude_unit.functions[0].clone();
        let mut prelude = Frame::new(
            Rc::new(prelude_unit),
            Rc::new(prelude_main_function),
            None,
            0,
        );
        prelude.set_local(
            0,
            Value::NativeFunction(NativeFunction::new(Rc::new(prelude_println))),
        );
        Rc::new(RefCell::new(prelude))
    }

    fn run(&mut self) {
        loop {
            let top = self.stack.last().expect("Empty stack");
            let instruction = top.borrow().current();
            let action = Vm::dispatch(&instruction, &mut top.borrow_mut());
            match action {
                Action::Advance => top.borrow_mut().advance(),
                // Due to borrow-checker rules we cannot have the side effects
                // of a call happen within `dispatch`.
                Action::CallDynamic(return_register, target, arguments) => {
                    let (unit, function) = target.dynamic_function().unwrap();
                    let frame = Rc::new(RefCell::new(Frame::new(
                        unit,
                        function,
                        None,
                        return_register,
                    )));
                    self.stack.push(frame)
                }
                Action::CallNative(return_register, target, arguments) => {
                    let result = target.call(arguments);
                    let mut top = top.borrow_mut();
                    top.write_register(return_register, result);
                    top.advance();
                }
                Action::Return(value) => {
                    let popped = self.stack.pop().expect("Empty stack");
                    let return_register = popped.borrow().return_register;
                    match self.stack.last() {
                        Some(top) => {
                            let mut top = top.borrow_mut();
                            top.write_register(return_register, value);
                            top.advance();
                        }
                        // Returning from the top frame.
                        None => return,
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    // Decodes the instruction, applies any frame-level side effects, and
    // returns an `Action` describing stack/VM-level side effects.
    #[inline]
    fn dispatch(instruction: &Instruction, top: &mut Frame) -> Action {
        match instruction {
            Instruction::GetLocal(lval, index) => {
                top.write_register(*lval, top.get_local(*index));
                Action::Advance
            }
            Instruction::GetLocalLexical(lval, name) => {
                top.write_register(*lval, top.get_local_lexical(name));
                Action::Advance
            }
            Instruction::SetLocal(index, rval) => {
                top.set_local(*index, top.read_register(*rval));
                Action::Advance
            }
            Instruction::MakeInteger(lval, value) => {
                top.write_register(*lval, Value::Integer(*value));
                Action::Advance
            }
            Instruction::Call(lval, target, arguments) => {
                let target = top.read_register(*target);
                let arguments = arguments
                    .iter()
                    .map(|argument| top.read_register(*argument))
                    .collect::<Vec<Value>>();
                match target {
                    Value::DynamicFunction(_, _) => Action::CallDynamic(*lval, target, arguments),
                    Value::NativeFunction(native_function) => {
                        Action::CallNative(*lval, native_function, arguments)
                    }
                    _ => panic!("Cannot call"),
                }
            }
            Instruction::ReturnNull => Action::Return(Value::Null),
            _ => panic!("Cannot dispatch: {:?}", instruction),
        }
    }
}
