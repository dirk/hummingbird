use std::cell::RefCell;
use std::path::Path;
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

impl Vm {
    pub fn run_file<P: AsRef<Path>>(path: P) {
        let mut vm = Self {
            stack: vec![],
            loader: Loader::new(),
        };

        let module = vm.loader.load_file(path).expect("Unable to read file");
        let frame = BytecodeFrame::new(module.main(), None, 0);

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
