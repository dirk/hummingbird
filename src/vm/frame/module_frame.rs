use super::super::errors::DebugSource;
use super::super::gc::{GcAllocator, GcTrace};
use super::super::loader::LoadedModule;
use super::super::value::Value;
use super::{Action, BytecodeFrame, Frame, FrameApi};

enum ModuleFrameState {
    Entering,
    Leaving,
    Reentering,
}

pub struct ModuleFrame {
    module: LoadedModule,
    state: ModuleFrameState,
}

impl ModuleFrame {
    pub fn new(module: LoadedModule) -> Self {
        Self {
            module,
            state: ModuleFrameState::Entering,
        }
    }

    pub fn module(&self) -> LoadedModule {
        self.module.clone()
    }
}

impl FrameApi for ModuleFrame {
    fn run(&mut self, _gc: &mut GcAllocator) -> Action {
        use ModuleFrameState::*;
        match self.state {
            // When the VM first runs us we call our main function and set our
            // state to leaving for the next execution (when the main
            // function returns).
            Entering => {
                self.state = Leaving;

                let main = self.module.main();
                let closure = main.build_closure_for_call(Some(self.module.static_closure()));
                let frame = BytecodeFrame::new(main, closure);
                self.state = Leaving;
                Action::Call(Frame::Bytecode(frame))
            }
            // The second time we're executed should be when control returns to
            // us from the main function.
            Leaving => {
                // FIXME: Return the module as a value suitable for
                //   `import` statements.
                self.state = Reentering;
                Action::Return(Value::Null)
            }
            Reentering => {
                unreachable!("Cannot reenter a module frame which has been entered and left")
            }
        }
    }

    fn receive_return(&mut self, _value: Value) {
        // No-op. Our return will always be the module as a value.
    }

    fn debug_source(&self) -> DebugSource {
        DebugSource::new(self.module.clone(), None, None)
    }
}

impl GcTrace for ModuleFrame {
    fn trace(&self) -> () {
        self.module.static_closure().trace();
    }
}
