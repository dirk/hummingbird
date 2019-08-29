use std::fmt;

use super::super::super::parser;
use super::super::super::target::bytecode::layout::{Instruction, Reg};
use super::super::errors::{DebugSource, VmError};
use super::super::gc::{GcAllocator, GcTrace};
use super::super::loader::{BytecodeFunction, LoadedFunction, LoadedModule};
use super::super::operators;
use super::super::value::Value;
use super::{Action, Closure, Frame, FrameApi};

/// Frame evaluating a bytecode function.
///
/// The first three fields should *not* be changed after the frame
/// is initialized.
pub struct BytecodeFrame {
    // TODO: Replace `LoadedFunction` with an abstraction that can support
    //   specialized instruction sequences.
    function: LoadedFunction,
    bytecode: BytecodeFunction,
    /// The slots for the local variables (not bound to a closure).
    locals: Vec<Value>,
    closure: Option<Closure>,
    /// The static closure of the module that this function belongs to.
    static_closure: Closure,
    /// The register for the returned value to be stored in.
    return_register: Option<Reg>,
    registers: Vec<Value>,
    current_address: usize,
}

impl BytecodeFrame {
    pub fn new(function: LoadedFunction, closure: Option<Closure>) -> Self {
        let bytecode = function.bytecode();
        let registers = bytecode.registers();
        let locals = bytecode.locals();
        Self {
            function: function.clone(),
            bytecode,
            locals: vec![Value::Null; locals as usize],
            closure,
            static_closure: function.module().static_closure(),
            return_register: None,
            // Sacrifice a bit of memory space so that we don't have to do an
            // offset every time we read a register.
            registers: vec![Value::Null; (registers + 1) as usize],
            current_address: 0,
        }
    }

    pub fn stack_description(&self) -> String {
        format!("{} ({})", self.bytecode.name(), self.module().name())
    }

    pub fn module(&self) -> LoadedModule {
        self.function.module()
    }

    #[inline]
    pub fn current(&self) -> Instruction {
        self.bytecode.instruction(self.current_address)
    }

    /// Look up a source mapping for the current instruction.
    fn current_span(&self) -> Option<parser::Span> {
        self.bytecode.span(self.current_address)
    }

    #[inline]
    pub fn advance(&mut self) {
        self.current_address += 1;
    }

    pub fn read_register(&self, index: Reg) -> Value {
        self.registers[index as usize].clone()
    }

    fn write_register(&mut self, index: Reg, value: Value) {
        if index == 0 {
            return;
        }
        self.registers[index as usize] = value;
    }

    pub fn get_local(&self, index: u8) -> Value {
        self.locals[index as usize].clone()
    }

    pub fn set_local(&mut self, index: u8, value: Value) {
        self.locals[index as usize] = value;
    }

    fn get_lexical(&self, name: &String) -> Result<Value, VmError> {
        if let Some(index) = self.get_index_of_local(name) {
            return Ok(self.locals[index].clone());
        }
        if let Some(closure) = &self.closure {
            return closure.get(name);
        }
        Err(VmError::new_undefined_name(name.clone()))
    }

    fn set_lexical(&mut self, name: &String, value: Value) -> Result<(), VmError> {
        if let Some(closure) = &self.closure {
            if closure.try_set(name.clone(), value.clone()) {
                return Ok(());
            }
        }
        if let Some(index) = self.get_index_of_local(name) {
            self.locals[index] = value;
            return Ok(());
        }
        Err(VmError::new_undefined_name(name.clone()))
    }

    pub fn get_static(&self, name: &String) -> Result<Value, VmError> {
        self.static_closure.get(name)
    }

    pub fn set_static(&self, name: &String, value: Value) -> Result<(), VmError> {
        if self.static_closure.try_set(name.to_owned(), value) {
            Ok(())
        } else {
            Err(VmError::new_undefined_name(name.clone()))
        }
    }

    fn get_index_of_local(&self, name: &String) -> Option<usize> {
        self.bytecode
            .locals_names()
            .iter()
            .position(|local| local == name)
    }

    /// Using an inner function so that we can use the `?` operator.
    #[inline]
    fn run_inner(&mut self, gc: &mut GcAllocator) -> Result<Action, VmError> {
        loop {
            let instruction = self.current();

            match &instruction {
                Instruction::GetLocal(lval, index) => {
                    self.write_register(*lval, self.get_local(*index));
                    self.advance();
                }
                Instruction::GetLexical(lval, name) => {
                    self.write_register(*lval, self.get_lexical(name)?);
                    self.advance();
                }
                Instruction::GetStatic(lval, name) => {
                    self.write_register(*lval, self.get_static(name)?);
                    self.advance();
                }
                Instruction::SetLocal(index, rval) => {
                    self.set_local(*index, self.read_register(*rval));
                    self.advance();
                }
                Instruction::SetLexical(name, rval) => {
                    self.set_lexical(name, self.read_register(*rval))?;
                    self.advance();
                }
                Instruction::SetStatic(name, rval) => {
                    self.set_static(name, self.read_register(*rval))?;
                    self.advance();
                }
                Instruction::MakeFunction(lval, id) => {
                    let loaded_function = self.module().function(*id);
                    // This is the parent environment in which the function's
                    // expressions will be evaluated when it is called.
                    let parent = if loaded_function.binds_on_create() {
                        self.closure.clone()
                    } else {
                        None
                    };
                    let value = Value::make_function(loaded_function, parent);
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::MakeInteger(lval, value) => {
                    self.write_register(*lval, Value::Integer(*value));
                    self.advance();
                }
                Instruction::MakeString(lval, value) => {
                    let value = gc.allocate(value.clone());
                    self.write_register(*lval, Value::String(value));
                    self.advance();
                }
                Instruction::OpAdd(lval, lhs, rhs) => {
                    let lhs = self.read_register(*lhs);
                    let rhs = self.read_register(*rhs);
                    let value = operators::op_add(lhs, rhs, gc)?;
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::OpLessThan(lval, lhs, rhs) => {
                    let lhs = self.read_register(*lhs);
                    let rhs = self.read_register(*rhs);
                    let value = operators::op_less_than(lhs, rhs)?;
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::OpProperty(lval, target, value) => {
                    let target = self.read_register(*target);
                    let value = operators::op_property(target, value.clone())?;
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::Branch(destination) => {
                    self.current_address = *destination as usize;
                }
                Instruction::BranchIf(destination, condition) => {
                    let condition = self.read_register(*condition);
                    if operators::is_truthy(condition)? {
                        self.current_address = *destination as usize;
                    } else {
                        self.advance();
                    }
                }
                Instruction::Call(lval, target, arguments) => {
                    let target = self.read_register(*target);
                    let arguments = arguments
                        .iter()
                        .map(|argument| self.read_register(*argument))
                        .collect::<Vec<Value>>();
                    match target {
                        Value::Function(function) => {
                            // Save the return register for when the VM calls `receive_return`.
                            self.return_register = Some(*lval);
                            let loaded_function = function.loaded_function;
                            let closure = loaded_function.build_closure_for_call(function.parent);
                            let frame =
                                Frame::Bytecode(BytecodeFrame::new(loaded_function, closure));
                            // Be at the next instruction when control flow returns to us.
                            self.advance();
                            return Ok(Action::Call(frame));
                        }
                        Value::BuiltinFunction(native_function) => {
                            let result = native_function.call(arguments);
                            self.write_register(*lval, result);
                            self.advance();
                        }
                        _ => panic!("Cannot call"),
                    }
                }
                Instruction::Return(rval) => {
                    let value = self.read_register(*rval);
                    return Ok(Action::Return(value));
                }
                Instruction::ReturnNull => {
                    return Ok(Action::Return(Value::Null));
                }
                Instruction::Export(name, rval) => {
                    let value = self.read_register(*rval);
                    self.module().set_export(name.clone(), value);
                    self.advance();
                }
                Instruction::Import(_alias, name) => {
                    return Ok(Action::Import(
                        name.clone(),
                        self.module().relative_import_path(),
                        Some(self.debug_source()),
                    ));
                }
            }
        }
    }
}

impl FrameApi for BytecodeFrame {
    fn run(&mut self, gc: &mut GcAllocator) -> Action {
        match self.run_inner(gc) {
            Ok(action) => action,
            Err(mut error) => {
                // Try to use our source mappings to get additional
                // debugging information for the error.
                error.set_source(self.debug_source());
                Action::Error(error)
            }
        }
    }

    fn receive_return(&mut self, value: Value) {
        let return_register = self.return_register.expect("Return register not set");
        self.write_register(return_register, value);
        self.return_register = None;
    }

    fn receive_import(&mut self, module: LoadedModule) -> Result<(), VmError> {
        let instruction = self.current();
        self.advance();

        let static_closure = self.module().static_closure();
        match instruction {
            Instruction::Import(alias, _name) => {
                static_closure.set_directly(alias, Value::Module(module));
            }
            other @ _ => {
                panic!(
                    "Cannot receive import to non-import instruction: {:?}",
                    other
                );
            }
        }
        Ok(())
    }

    fn debug_source(&self) -> DebugSource {
        DebugSource::new(
            self.module(),
            Some(self.bytecode.name().to_owned()),
            self.current_span(),
        )
    }
}

impl GcTrace for BytecodeFrame {
    fn trace(&self) -> () {
        for local_value in self.locals.iter() {
            local_value.trace();
        }
        if let Some(closure) = &self.closure {
            closure.trace();
        }
        self.static_closure.trace();
        for register_value in self.registers.iter() {
            register_value.trace();
        }
    }
}

impl fmt::Debug for BytecodeFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "BytecodeFrame {{ locals: {:?}, closure: {:?} }}",
            self.locals.len(),
            self.closure
        )
    }
}
