use std::path::Path;

use super::errors::{DebugSource, VmError};
use super::frame::{Action, Closure, Frame, FrameApi, ModuleFrame, ReplFrame};
use super::loader::Loader;
use super::prelude;

pub type StackSnapshot = Vec<(u16, DebugSource)>;

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
            if let Err(mut error) = result {
                let snapshot = self.snapshot_stack();
                error.set_stack(snapshot);

                if let Some(uncaught) = self.error_unwind(error) {
                    // If we weren't able to catch the error then print
                    // what went wrong and exit.
                    uncaught
                        .print_debug()
                        .expect("Unable to debug-print uncaught error");
                    return;
                }
            }
        }
    }

    fn process_action(&mut self, action: Action) -> Result<(), VmError> {
        match action {
            Action::Import(name, relative_import_path, source) => {
                let result = match self.loader.load_file_by_name(name, relative_import_path) {
                    Ok(loaded) => Ok(loaded),
                    Err(mut error) => {
                        println!("Load error!");
                        // Annotate the error with its source. Imports happen
                        // outside of the normal frame fetch-execute loop, so
                        // they instead include the source of the import with
                        // the `Action` so that it can be automatically
                        // embedded in the load error if one occurs.
                        if let Some(source) = source {
                            println!("Have source!");
                            error.set_source(source);
                        }
                        Err(error)
                    }
                };
                let (module, already_loaded) = result?;
                if already_loaded {
                    let top = self.stack.last_mut().unwrap();
                    top.receive_import(module)?;
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

    /// Tries to find a frame to catch the error. If it does then it consumes
    /// the error and returns `None`. If doesn't then it returns the error
    /// back to the caller.
    fn error_unwind(&mut self, error: VmError) -> Option<VmError> {
        loop {
            let can_catch_error = {
                let top = match self.stack.last() {
                    Some(frame) => frame,
                    None => {
                        // Out of stack frames to unwind from.
                        return Some(error);
                    }
                };
                top.can_catch_error(&error)
            };

            if can_catch_error {
                let top = self.stack.last_mut().unwrap();
                top.catch_error(error);
                return None;
            } else {
                // If this frame didn't catch the error then keep on
                // unwinding.
                let popped = self.stack.pop();
                if let Some(module) = popped.and_then(|frame| frame.initializing_module()) {
                    if !self.loader.unload(&module) {
                        println!(
                            "WARNING: Unable to unload module while unwinding: {:?}",
                            module.name()
                        );
                    }
                }
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
            captured.push((index, frame.debug_source()));
            index += 1;
        }
        captured
    }
}
