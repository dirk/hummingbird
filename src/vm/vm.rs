use std::path::Path;

use super::frame::{Action, BytecodeFrame, Frame, FrameApi};
use super::loader::Loader;
use super::prelude::build_prelude;
use crate::vm::frame::Closure;

pub struct Vm {
    stack: Vec<Frame>,
    loader: Loader,
}

impl Vm {
    pub fn run_file<P: AsRef<Path>>(path: P) {
        let mut vm = Self {
            stack: vec![],
            loader: Loader::new(),
        };

        let prelude = build_prelude();
        let module = vm.loader.load_file(path).expect("Unable to read file");

        // FIXME: Actually do imports on request instead of just copying the
        //   whole prelude.
        for (name, export) in prelude.get_named_exports().iter() {
            if let Some(export) = export {
                module.set_import(name.to_owned(), export.clone());
            }
        }

        let main = module.main();
        let bindings = main.bindings();
        let maybe_bindings = if !bindings.is_empty() {
            Some(bindings)
        } else {
            None
        };
        let closure = Some(Closure::new(maybe_bindings, None));
        let frame = BytecodeFrame::new(module.main(), closure);

        vm.stack.push(Frame::Bytecode(frame));
        vm.run();
    }

    fn run(&mut self) {
        loop {
            let action = {
                let top = self.stack.last_mut().expect("Empty stack");
                top.run()
            };

            match action {
                Action::Call(frame) => self.stack.push(frame),
                Action::Return(return_value) => {
                    self.stack.pop().expect("Empty stack");
                    match self.stack.last_mut() {
                        Option::Some(new_top) => {
                            new_top.receive_return(return_value);
                        }
                        Option::None => return,
                    }
                }
            }
        }
    }
}
