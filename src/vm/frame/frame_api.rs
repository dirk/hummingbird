use super::super::errors::VmError;
use super::super::gc::{GcAllocator, GcTrace};
use super::super::loader::LoadedModule;
use super::super::value::Value;
use super::{Action, DebugSource};

pub trait FrameApi: GcTrace {
    // Run the frame's fetch-execute loop. Will be different depending on if
    // it's a bytecode or native frame.
    fn run(&mut self, gc: &mut GcAllocator) -> Action;

    /// Called before the frame resumes execution after the higher frame has
    /// returned a value.
    ///
    /// [0].run() -> Call
    ///   [1].run() -> Return(value)
    /// [0].receive_return(value)
    /// [0].run() -> ...
    fn receive_return(&mut self, value: Value);

    /// Called when the VM has finished initializing a module imported by this
    /// frame's module.
    fn receive_import(&mut self, _module: LoadedModule) -> Result<(), VmError> {
        unimplemented!()
    }

    /// When an error is raised the VM will call this on each frame of the
    /// stack. If the frame returns false it will be unwound off the stack.
    /// It it returns true then it must be able to immediately receive a call
    /// to `catch_error`. In pseudocode the VM's execution looks like:
    ///
    ///   loop {
    ///     if stack.top.can_catch_error(error) {
    ///       stack.catch_error(error)
    ///       break
    ///     }
    ///     stack.pop()
    ///   }
    ///
    fn can_catch_error(&self, _error: &VmError) -> bool {
        false
    }

    /// This method should not do any evaluation. Instead it should merely
    /// prepare for evaluation to resume in this frame.
    fn catch_error(&mut self, _error: VmError) {
        unreachable!()
    }

    /// Get the debug source for the current state of the frame.
    fn debug_source(&self) -> DebugSource;
}
