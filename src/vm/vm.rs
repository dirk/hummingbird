use std::collections::HashMap;
use std::error;
use std::path::{Path, PathBuf};

use super::errors::AnnotatedError;
use super::frame::{Action, Closure, Frame, FrameApi, ModuleFrame, ReplFrame};
use super::loader::{self, LoadedModule};
use super::prelude;

pub type StackSnapshot = Vec<(u16, String)>;

pub struct Vm {
    loaded_modules: HashMap<PathBuf, LoadedModule>,
    builtins_closure: Closure,
    stack: Vec<Frame>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            loaded_modules: HashMap::new(),
            builtins_closure: prelude::build_prelude(),
            stack: vec![],
        }
    }

    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<LoadedModule, Box<dyn error::Error>> {
        let canonicalized = path
            .as_ref()
            .canonicalize()
            .expect("Could not canonicalize path");

        if self.loaded_modules.contains_key(&canonicalized) {
            panic!("Module already loaded: {:?}", canonicalized);
        }

        let loaded_module = loader::load_file(path, Some(self.builtins_closure.clone()))?;
        self.loaded_modules
            .insert(canonicalized, loaded_module.clone());
        Ok(loaded_module)
    }

    pub fn run_file<P: AsRef<Path>>(path: P) {
        let mut vm = Self::new();
        let module = vm.load_file(path).expect("Unable to read file");
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
                Action::Error(error) => {
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
