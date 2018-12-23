use std::cell::RefCell;
use std::rc::Rc;

use super::super::ir::layout as ir;
use super::super::target::bytecode::layout::Instruction;

use super::frame::{Action, BytecodeFrame, SharedFrame};
use super::loader::Loader;
use super::value::{NativeFunction, Value};

pub struct Vm {
    stack: Vec<SharedFrame>,
    loader: Loader,
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
        let mut frame = BytecodeFrame::new(loaded_unit.main(), None, 0);
        frame.set_local(
            index,
            Value::NativeFunction(NativeFunction::new(Rc::new(prelude_println))),
        );

        vm.stack.push(Rc::new(RefCell::new(frame)));
        vm.run();
    }

    fn run(&mut self) {
        loop {
            let action = {
                let mut top = self.stack.last().expect("Empty stack").borrow_mut();
                top.run()
            };

            match action {
                Action::Call(frame) => self.stack.push(frame),
                Action::Return(return_register, return_value) => {
                    self.stack.pop().expect("Empty stack");
                    match self.stack.last() {
                        Option::Some(new_top) => {
                            let mut new_top = new_top.borrow_mut();
                            new_top.write_register(return_register, return_value);
                        }
                        Option::None => return,
                    }
                }
            }
        }
    }
}
