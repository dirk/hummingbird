use std::path::PathBuf;

use super::errors::{DebugSource, VmError};
use super::loader::LoadedModule;
use super::value::Value;

mod bytecode_frame;
mod closure;
mod frame_api;
mod module_frame;
mod repl_frame;

pub use bytecode_frame::BytecodeFrame;
pub use closure::Closure;
pub use frame_api::FrameApi;
pub use module_frame::ModuleFrame;
pub use repl_frame::ReplFrame;

/// An action for the VM to do.
pub enum Action {
    // (name, relative_import_path, source)
    Import(String, Option<PathBuf>, Option<DebugSource>),
    Call(Frame),
    Return(Value),
    Error(VmError),
}

pub enum Frame {
    Bytecode(BytecodeFrame),
    Module(ModuleFrame),
    Repl(ReplFrame),
}

impl Frame {
    pub fn is_module(&self) -> bool {
        match self {
            Frame::Module(_) => true,
            _ => false,
        }
    }

    /// Returns the module being initialized by this frame.
    pub fn initializing_module(&self) -> Option<LoadedModule> {
        match self {
            Frame::Module(frame) => Some(frame.module()),
            _ => None,
        }
    }

    /// Returns a description of the frame suitable for printing in a
    /// stack trace.
    pub fn stack_description(&self) -> String {
        match self {
            Frame::Bytecode(frame) => frame.stack_description(),
            Frame::Module(_) => unreachable!("Cannot get a stack description for a module"),
            Frame::Repl(_) => "(repl)".to_owned(),
        }
    }
}

impl FrameApi for Frame {
    fn run(&mut self) -> Action {
        match self {
            Frame::Bytecode(frame) => frame.run(),
            Frame::Module(frame) => frame.run(),
            Frame::Repl(frame) => frame.run(),
        }
    }

    fn receive_return(&mut self, value: Value) {
        match self {
            Frame::Bytecode(frame) => frame.receive_return(value),
            Frame::Module(frame) => frame.receive_return(value),
            Frame::Repl(frame) => frame.receive_return(value),
        }
    }

    fn receive_import(&mut self, module: LoadedModule) -> Result<(), VmError> {
        match self {
            Frame::Bytecode(frame) => frame.receive_import(module),
            Frame::Module(frame) => frame.receive_import(module),
            Frame::Repl(frame) => frame.receive_import(module),
        }
    }

    fn can_catch_error(&self, error: &VmError) -> bool {
        match self {
            Frame::Bytecode(frame) => frame.can_catch_error(error),
            Frame::Module(frame) => frame.can_catch_error(error),
            Frame::Repl(frame) => frame.can_catch_error(error),
        }
    }

    fn catch_error(&mut self, error: VmError) {
        match self {
            Frame::Bytecode(frame) => frame.catch_error(error),
            Frame::Module(frame) => frame.catch_error(error),
            Frame::Repl(frame) => frame.catch_error(error),
        }
    }

    fn debug_source(&self) -> DebugSource {
        match self {
            Frame::Bytecode(frame) => frame.debug_source(),
            Frame::Module(frame) => frame.debug_source(),
            Frame::Repl(frame) => frame.debug_source(),
        }
    }
}
