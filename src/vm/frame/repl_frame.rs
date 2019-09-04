use std::io::Write;

use super::super::super::ast_to_ir;
use super::super::super::parser;
use super::super::errors::{DebugSource, VmError};
use super::super::gc::{GcAllocator, GcTrace};
use super::super::loader::{self, LoadedModule};
use super::super::value::Value;
use super::{Action, BytecodeFrame, Closure, Frame, FrameApi};

pub struct ReplFrame {
    // FIXME: Set up a shared loader to hold loaded modules in memory.
    //   Eventually they should be part of the GC graph so that they're freed
    //   when there are no longer live references to them.
    loaded_modules: Vec<LoadedModule>,
    counter: u16,
    static_closure: Closure,
    // The result of the last expression's evaluation.
    last_result: Option<Value>,
    last_error: Option<VmError>,
}

impl ReplFrame {
    pub fn new(builtins_closure: Closure) -> Self {
        Self {
            loaded_modules: vec![],
            counter: 0,
            static_closure: Closure::new_repl(builtins_closure),
            last_result: None,
            last_error: None,
        }
    }

    pub fn closure(&self) -> Closure {
        self.static_closure.clone()
    }

    fn compile_line(
        &mut self,
        line: String,
        counter: u16,
    ) -> Result<LoadedModule, parser::ParseError> {
        let name = format!("repl[{}]", counter);

        let ast_module = parser::parse(line.clone())?;
        let loaded_module = loader::compile_ast_into_module(
            &ast_module,
            name,
            line,
            ast_to_ir::CompilationFlags::Repl,
            None,
        )
        .expect("Couldn't compile line");
        // Hold it in ourselves so that it doesn't get dropped.
        self.loaded_modules.push(loaded_module.clone());
        // Make all the loaded modules share the same static closure so that
        // they see all the same defined variables.
        loaded_module.override_static_closure(self.static_closure.clone());
        Ok(loaded_module)
    }
}

impl FrameApi for ReplFrame {
    fn run(&mut self, _gc: &mut GcAllocator) -> Action {
        if let Some(result) = &self.last_result {
            println!("{:?}", result);
            self.last_result = None;
        }

        loop {
            let counter = self.counter;
            self.counter += 1;

            print!("[{}]> ", counter);
            std::io::stdout().flush().unwrap();

            let mut buffer = String::new();
            std::io::stdin()
                .read_line(&mut buffer)
                .expect("Couldn't read line");

            match buffer.as_str().trim() {
                "wtf?" => {
                    if let Some(error) = &self.last_error {
                        println!("{}", error);
                    } else {
                        println!("No recent error.")
                    }
                    continue;
                }
                _ => {
                    let module = match self.compile_line(buffer, counter) {
                        Ok(module) => module,
                        Err(parse_error) => {
                            let error = VmError::new_parse(parse_error);
                            self.catch_error(error);
                            continue;
                        }
                    };
                    // TODO: Extract and process the module's imports; that way one can do
                    //   `import` in the REPL.
                    let function = module.main();
                    let closure =
                        function.build_closure_for_call(Some(self.static_closure.clone()));
                    return Action::Call(Frame::Bytecode(BytecodeFrame::new(function, closure)));
                }
            }
        }
    }

    fn receive_return(&mut self, value: Value) {
        self.last_result = Some(value);
    }

    /// The top-level REPL frame can always catch any errors that bubble up.
    fn can_catch_error(&self, _error: &VmError) -> bool {
        true
    }

    fn catch_error(&mut self, error: VmError) {
        error.print_debug().expect("Unable to print error");
        self.last_result = None;
        self.last_error = Some(error);
    }

    fn debug_source(&self) -> DebugSource {
        DebugSource::new(
            self.loaded_modules.last().expect("Empty REPL").clone(),
            Some("(repl)".to_owned()),
            None,
        )
    }
}

impl GcTrace for ReplFrame {
    fn trace(&self) -> () {
        self.static_closure.trace();
        if let Some(value) = &self.last_result {
            value.trace();
        }
    }
}
