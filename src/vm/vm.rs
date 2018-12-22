use std::cell::RefCell;
use std::process::exit;
use std::rc::Rc;

use super::super::ir::layout as ir;
use super::super::target::bytecode::layout::Instruction;

use super::frame::{Frame, SharedFrame};
use super::loader::Loader;
use super::value::{NativeFunction, Value};

pub struct Vm {
    stack: Vec<SharedFrame>,
    loader: Loader,
}

enum Action {
    None,
    Push(SharedFrame),
    Pop(Value),
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
    pub fn run_main(ir_unit: ir::Unit) {
        let mut vm = Self {
            stack: vec![],
            loader: Loader::new(),
        };

        // Declare "println" in the local scope of the main function.
        let ir_main = ir_unit.main_function();
        let index = ir_main.borrow_mut().get_or_add_local("println".to_string());

        let loaded_unit = vm.loader.load(ir_unit);

        // Then inject the println at the index we previously declared.
        let mut frame = Frame::new(loaded_unit.main(), None, 0);
        frame.set_local(index, Value::NativeFunction(NativeFunction::new(Rc::new(prelude_println))));

        vm.stack.push(Rc::new(RefCell::new(frame)));
        vm.run();
    }

    fn run(&mut self) {
        loop {
            match self.dispatch() {
                Some(code) => exit(code),
                None => (),
            }
        }
    }

    fn dispatch(&mut self) -> Option<i32> {
        use self::Action::*;

        let top = self.stack.last().expect("Empty stack");
        let instruction = top.borrow().current();

        let action = {
            // Need the `&mut Frame` to force this to be a mutable borrow.
            let top: &mut Frame = &mut top.borrow_mut();
            match &instruction {
                Instruction::GetLocal(lval, index) => {
                    top.write_register(*lval, top.get_local(*index));
                    top.advance();
                    None
                }
                Instruction::GetLocalLexical(lval, name) => {
                    top.write_register(*lval, top.get_local_lexical(name));
                    top.advance();
                    None
                }
                Instruction::SetLocal(index, rval) => {
                    top.set_local(*index, top.read_register(*rval));
                    top.advance();
                    None
                }
                Instruction::MakeFunction(lval, id) => {
                    let function = top.unit().function(*id);
                    let value = Value::from_dynamic_function(function);
                    top.write_register(*lval, value);
                    top.advance();
                    None
                }
                Instruction::MakeInteger(lval, value) => {
                    top.write_register(*lval, Value::Integer(*value));
                    top.advance();
                    None
                }
                Instruction::Call(lval, target, arguments) => {
                    let return_register = *lval;
                    let target = top.read_register(*target);
                    let arguments = arguments
                        .iter()
                        .map(|argument| top.read_register(*argument))
                        .collect::<Vec<Value>>();
                    match target {
                        Value::DynamicFunction(dynamic_function) => {
                            // TODO: Make `CallTarget` able to do specialization.
                            let function = dynamic_function.call_target.function;
                            let frame = Rc::new(RefCell::new(Frame::new(
                                function,
                                Option::None,
                                return_register,
                            )));
                            Push(frame)
                        }
                        Value::NativeFunction(native_function) => {
                            let result = native_function.call(arguments);
                            top.write_register(return_register, result);
                            top.advance();
                            None
                        }
                        _ => panic!("Cannot call"),
                    }
                }
                Instruction::Return(rval) => Pop(top.read_register(*rval)),
                Instruction::ReturnNull => Pop(Value::Null),
                _ => panic!("Cannot dispatch: {:?}", instruction),
            }
        };
        match action {
            Push(frame) => self.stack.push(frame),
            Pop(return_value) => {
                let popped = self.stack.pop().expect("Empty stack");
                let return_register = popped.borrow().return_register;
                match self.stack.last() {
                    Option::Some(top) => {
                        let mut top = top.borrow_mut();
                        top.write_register(return_register, return_value);
                        top.advance();
                    }
                    // Returning from the top frame.
                    Option::None => return Some(0),
                }
            }
            None => (),
        };
        Option::None
    }
}
