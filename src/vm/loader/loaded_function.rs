use std::collections::HashSet;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

use super::super::super::parser::Span;
use super::super::super::target::bytecode;
use super::super::frame::Closure;
use super::{LoadedModule, WeakLoadedModule};

#[derive(Clone)]
struct InnerLoadedFunction {
    module: WeakLoadedModule,
    id: u16,
    bytecode: BytecodeFunction,
}

/// Handle to a loaded function.
#[derive(Clone)]
pub struct LoadedFunction(Rc<InnerLoadedFunction>);

impl LoadedFunction {
    pub fn new(module: WeakLoadedModule, function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerLoadedFunction {
            module,
            id: function.id,
            bytecode: BytecodeFunction::new(function),
        }))
    }

    pub fn id(&self) -> u16 {
        self.0.id
    }

    /// Returns a string indicating the base-name of the module it was defined
    /// in and its own name.
    pub fn qualified_name(&self) -> String {
        let module_basename = Path::new(&self.module().name())
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("(unknown)")
            .to_owned();
        let own_name = self.0.bytecode.name();
        format!("{}:{}", module_basename, own_name)
    }

    pub fn bytecode(&self) -> BytecodeFunction {
        self.0.bytecode.clone()
    }

    /// Returns whether or not this function binds/captures its environment
    /// when it is created.
    pub fn binds_on_create(&self) -> bool {
        self.0.bytecode.parent_bindings()
    }

    /// Returns a closure suitable for calling the function.
    pub fn build_closure_for_call(&self, parent: Option<Closure>) -> Option<Closure> {
        let bindings = self.0.bytecode.bindings();
        // Whether or not it needs to create bindings (a closure) when called.
        let binds_on_call = !bindings.is_empty() || self.0.bytecode.parent_bindings();
        if binds_on_call {
            let bindings = if bindings.is_empty() {
                None
            } else {
                Some(bindings)
            };
            Some(Closure::new(bindings, parent))
        } else {
            None
        }
    }

    pub fn module(&self) -> LoadedModule {
        self.0
            .module
            .clone()
            .try_into()
            .expect("Module has been dropped")
    }
}

pub struct InnerBytecodeFunction {
    function: bytecode::layout::Function,
}

impl InnerBytecodeFunction {
    pub fn name(&self) -> &str {
        &self.function.name
    }

    #[inline]
    pub fn registers(&self) -> u8 {
        self.function.registers
    }

    #[inline]
    pub fn locals(&self) -> u8 {
        self.function.locals
    }

    #[inline]
    pub fn instruction(&self, instruction_address: usize) -> bytecode::layout::Instruction {
        self.function.instructions[instruction_address].clone()
    }

    pub fn span(&self, instruction_address: usize) -> Option<Span> {
        for (address, span) in self.function.source_mappings.iter() {
            if ((*address) as usize) == instruction_address {
                return Some(span.clone());
            }
        }
        None
    }

    pub fn locals_names(&self) -> Vec<String> {
        self.function.locals_names.clone()
    }

    pub fn has_bindings(&self) -> bool {
        !self.function.bindings.is_empty()
    }

    pub fn bindings(&self) -> HashSet<String> {
        self.function.bindings.clone()
    }

    pub fn parent_bindings(&self) -> bool {
        self.function.parent_bindings
    }
}

#[derive(Clone)]
pub struct BytecodeFunction(Rc<InnerBytecodeFunction>);

impl BytecodeFunction {
    pub fn new(function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerBytecodeFunction { function }))
    }
}

impl Deref for BytecodeFunction {
    type Target = InnerBytecodeFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
