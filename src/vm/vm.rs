use std::error::Error;
use std::path::{Path, PathBuf};

use super::frame::{Action, Frame, FrameApi, ModuleFrame};
use super::loader::{self, LoadedModule};
use super::prelude::build_prelude;
use std::collections::HashMap;

pub struct Vm {
    stack: Vec<Frame>,
    loaded_modules: HashMap<PathBuf, LoadedModule>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            loaded_modules: HashMap::new(),
        }
    }

    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<LoadedModule, Box<dyn Error>> {
        let canonicalized = path
            .as_ref()
            .canonicalize()
            .expect("Could not canonicalize path");

        if self.loaded_modules.contains_key(&canonicalized) {
            panic!("Module already loaded: {:?}", canonicalized);
        }

        let loaded_module = loader::load_file(path)?;
        self.loaded_modules
            .insert(canonicalized, loaded_module.clone());
        Ok(loaded_module)
    }

    pub fn run_file<P: AsRef<Path>>(path: P) {
        let mut vm = Self::new();

        let prelude = build_prelude();
        let module = vm.load_file(path).expect("Unable to read file");

        // FIXME: Actually do imports on request instead of just copying the
        //   whole prelude.
        for (name, export) in prelude.get_named_exports().iter() {
            if let Some(export) = export {
                module
                    .static_closure()
                    .set_directly(name.to_owned(), export.clone())
            }
        }

        vm.stack.push(Frame::Module(ModuleFrame::new(module)));
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
                    println!("{}", error);
                    return;
                }
            }
        }
    }
}
