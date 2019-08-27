use std::collections::HashMap;
use std::error;
use std::path::{Path, PathBuf};

use super::errors::AnnotatedError;
use super::frame::{Action, Closure, Frame, FrameApi, ModuleFrame, ReplFrame};
use super::loader::{self, LoadedModule, Loader};
use super::prelude;

pub type StackSnapshot = Vec<(u16, String)>;

pub struct Vm {
    loader: Loader,
    builtins_closure: Closure,
    stack: Vec<Frame>,
}

impl Vm {
    pub fn new() -> Self {
        let builtins_closure = prelude::build_prelude();
        Self {
            loader: Loader::new(builtins_closure.clone()),
            builtins_closure,
            stack: vec![],
        }
    }

    pub fn run_file<P: AsRef<Path>>(path: P) {
        let mut vm = Self::new();
        let (module, _already_loaded) = vm.loader.load_file(path).expect("Unable to read file");
        vm.stack.push(Frame::Module(ModuleFrame::new(module)));
        vm.run();
    }

    pub fn run_repl() {
        let mut vm = Self::new();
        let frame = ReplFrame::new(vm.builtins_closure.clone());
        vm.stack.push(Frame::Repl(frame));
        vm.run();
    }

    fn run(&mut self) {
        loop {
            let action = {
                if let Some(top) = self.stack.last_mut() {
                    top.run()
                } else {
                    // If the stack's empty then we have nothing more to do!
                    return;
                }
            };

            let result = self.process_action(action);

            // If we had a problem processing the action then start unwinding.
            if let Err(error) = result {
                let annotated = AnnotatedError::new(error, self.snapshot_stack());
                // Get a formatted string for the error before it's
                // consumed by the call to `error_unwind`.
                let formatted = format!("{}", annotated);
                if !self.error_unwind(Box::new(annotated)) {
                    // If we weren't able to catch the error then print
                    // what went wrong and exit.
                    println!("{}", formatted);
                    return;
                }
            }
        }
    }

    fn process_action(&mut self, action: Action) -> Result<(), Box<dyn error::Error>> {
        match action {
            Action::Import(name, relative_import_path) => {
                let (module, already_loaded) =
                    self.loader.load_file_by_name(name, relative_import_path)?;
                if already_loaded {
                    let top = self.stack.last_mut().unwrap();
                    top.receive_import(module);
                } else {
                    self.stack.push(Frame::Module(ModuleFrame::new(module)));
                }
                Ok(())
            }
            Action::Call(frame) => {
                self.stack.push(frame);
                Ok(())
            }
            Action::Return(return_value) => {
                let snapshot = self.snapshot_stack();
                let returning_from = self.stack.pop().expect("Empty stack");
                if let Some(returning_to) = self.stack.last_mut() {
                    // Processing imports happens through a dedicated path
                    // rather than through regular returns. Maybe in the future
                    // it can be handled like a normal call and return.
                    if let Some(module) = returning_from.initializing_module() {
                        // If we just popped a module frame then we need to
                        // mark that module as initialized.
                        module.set_initialized();
                        returning_to.receive_import(module)?;
                    } else {
                        returning_to.receive_return(return_value);
                    }
                }
                Ok(())
            }
            Action::Error(error) => Err(error),
        }
    }

    // Returns true if it found a frame to catch the error, false if not.
    fn error_unwind(&mut self, error: Box<dyn error::Error>) -> bool {
        loop {
            let can_catch_error = {
                let top = match self.stack.last() {
                    Some(frame) => frame,
                    None => {
                        // Out of stack frames to unwind from.
                        return false;
                    }
                };
                top.can_catch_error(&error)
            };

            if can_catch_error {
                let top = self.stack.last_mut().unwrap();
                top.catch_error(error);
                return true;
            } else {
                // If this frame didn't catch the error then keep on
                // unwinding.
                self.stack.pop();
            }
        }
    }

    fn snapshot_stack(&self) -> StackSnapshot {
        let mut index = 0u16;
        let mut captured: StackSnapshot = vec![];
        for frame in self.stack.iter().rev() {
            if frame.is_module() {
                continue;
            }
            captured.push((index, frame.stack_description()));
            index += 1;
        }
        captured
    }
}
