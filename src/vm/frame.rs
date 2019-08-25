use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use super::super::target::bytecode::layout::{Instruction, Reg};

use super::errors::UndefinedNameError;
use super::loader::{BytecodeFunction, LoadedFunction, LoadedModule};
use super::operators;
use super::value::Value;

struct InnerClosure {
    locals: HashMap<String, Option<Value>>,
    parent: Option<Closure>,
    /// If this closure is the static closure environment for a module.
    is_static: bool,
}

#[derive(Clone)]
pub struct Closure(Rc<RefCell<InnerClosure>>);

impl Closure {
    pub fn new(bindings: Option<HashSet<String>>, parent: Option<Closure>) -> Self {
        let mut locals = HashMap::<String, Option<Value>>::new();
        if let Some(bindings) = bindings {
            for binding in bindings.iter() {
                locals.insert(binding.clone(), None);
            }
        };
        Self(Rc::new(RefCell::new(InnerClosure {
            locals,
            parent,
            is_static: false,
        })))
    }

    pub fn new_static() -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: None,
            is_static: true,
        })))
    }

    fn get(&self, name: &String) -> Result<Value, Box<dyn error::Error>> {
        let inner = &self.0.borrow();
        if let Some(value) = inner.locals.get(name) {
            return match value {
                Some(initialized) => Ok(initialized.clone()),
                None => unreachable!("ERROR: Uninitialized closure variable: {}", name),
            };
        }
        if let Some(parent) = &inner.parent {
            return parent.get(name);
        }
        Err(Box::new(UndefinedNameError::new(name.to_owned())))
    }

    /// Returns true if it found a closure in which to set the variable,
    /// false if not.
    pub fn try_set(&self, name: String, value: Value) -> bool {
        let inner = &mut self.0.borrow_mut();
        if let Some(exists) = inner.locals.get_mut(&name) {
            *exists = Some(value);
            return true;
        }
        // If it's static then we can create new locals at will.
        // if inner.is_static {
        //     inner.locals.insert(name, Some(value));
        //     return true;
        // }
        if let Some(parent) = &inner.parent {
            // If we found a parent with this name and were able to set the
            // value then all is good.
            if parent.set_as_parent(name.clone(), value) {
                return true;
            }
        }
        return false;
    }

    /// Set a local directly into this exact closure. This should only be used
    /// by the VM when initializing a module's closure with imports.
    pub fn set_directly(&self, name: String, value: Value) {
        let inner = &mut self.0.borrow_mut();
        inner.locals.insert(name, Some(value));
    }

    // Recursive call to set in parent closures. Returns true if it found
    // a local and set, false if not.
    fn set_as_parent(&self, name: String, value: Value) -> bool {
        let inner = &mut self.0.borrow_mut();
        if inner.locals.contains_key(&name) {
            inner.locals.insert(name.clone(), Some(value));
            return true;
        }
        if let Some(parent) = &inner.parent {
            parent.set_as_parent(name, value)
        } else {
            false
        }
    }
}

// An action for the VM to do.
pub enum Action {
    Call(Frame),
    Return(Value),
    Error(Box<dyn error::Error>),
}

pub enum Frame {
    Bytecode(BytecodeFrame),
    Module(ModuleFrame),
}

impl Frame {
    pub fn is_module(&self) -> bool {
        match self {
            Frame::Module(_) => true,
            _ => false,
        }
    }

    /// Returns a description of the frame suitable for printing in a
    /// stack trace.
    pub fn stack_description(&self) -> String {
        match self {
            Frame::Bytecode(frame) => frame.stack_description(),
            Frame::Module(_) => unreachable!("Cannot get a stack description for a module"),
        }
    }
}

impl FrameApi for Frame {
    fn run(&mut self) -> Action {
        match self {
            Frame::Bytecode(frame) => frame.run(),
            Frame::Module(frame) => frame.run(),
        }
    }

    fn receive_return(&mut self, value: Value) {
        match self {
            Frame::Bytecode(frame) => frame.receive_return(value),
            Frame::Module(frame) => frame.receive_return(value),
        }
    }
}

pub trait FrameApi {
    // Run the frame's fetch-execute loop. Will be different depending on if
    // it's a bytecode or native frame.
    fn run(&mut self) -> Action;

    /// Called before the frame resumes execution after the higher frame has
    /// returned a value.
    ///
    /// [0].run() -> Call
    ///   [1].run() -> Return(value)
    /// [0].receive_return(value)
    /// [0].run() -> ...
    fn receive_return(&mut self, value: Value);
}

// Frame evaluating a bytecode function.
//
// The first three fields should *not* be changed after the frame
// is initialized.
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

    fn get_lexical(&self, name: &String) -> Result<Value, Box<dyn error::Error>> {
        if let Some(index) = self.get_index_of_local(name) {
            return Ok(self.locals[index].clone());
        }
        if let Some(closure) = &self.closure {
            return closure.get(name);
        }
        Err(Box::new(UndefinedNameError::new(name.clone())))
    }

    fn set_lexical(&mut self, name: &String, value: Value) -> Result<(), Box<dyn error::Error>> {
        if let Some(closure) = &self.closure {
            if closure.try_set(name.clone(), value.clone()) {
                return Ok(());
            }
        }
        if let Some(index) = self.get_index_of_local(name) {
            self.locals[index] = value;
            return Ok(());
        }
        Err(Box::new(UndefinedNameError::new(name.clone())))
    }

    pub fn get_static(&self, name: &String) -> Result<Value, Box<dyn error::Error>> {
        self.static_closure.get(name)
    }

    pub fn set_static(&self, name: &String, value: Value) -> Result<(), Box<dyn error::Error>> {
        if self.static_closure.try_set(name.to_owned(), value) {
            Ok(())
        } else {
            Err(Box::new(UndefinedNameError::new(name.clone())))
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
    fn run_inner(&mut self) -> Result<Action, Box<dyn error::Error>> {
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
                    let function = self.module().function(*id);
                    let closure = if function.binds_on_create() {
                        self.closure.clone()
                    } else {
                        None
                    };
                    let value = Value::from_dynamic_function(function, closure);
                    self.write_register(*lval, value);
                    self.advance();
                }
                Instruction::MakeInteger(lval, value) => {
                    self.write_register(*lval, Value::Integer(*value));
                    self.advance();
                }
                Instruction::OpAdd(lval, lhs, rhs) => {
                    let lhs = self.read_register(*lhs);
                    let rhs = self.read_register(*rhs);
                    let value = operators::op_add(lhs, rhs)?;
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
                        Value::DynamicFunction(dynamic_function) => {
                            // Save the return register for when the VM calls `receive_return`.
                            self.return_register = Some(*lval);
                            // TODO: Make `CallTarget` able to do specialization.
                            let function = dynamic_function.call_target.function;
                            let parent_closure = dynamic_function.closure;
                            let closure = function.build_closure_for_call(parent_closure);
                            let frame = Frame::Bytecode(BytecodeFrame::new(function, closure));
                            // Be at the next instruction when control flow returns to us.
                            self.advance();
                            return Ok(Action::Call(frame));
                        }
                        Value::NativeFunction(native_function) => {
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
            }
        }
    }
}

impl FrameApi for BytecodeFrame {
    fn run(&mut self) -> Action {
        match self.run_inner() {
            Ok(action) => action,
            Err(error) => Action::Error(error),
        }
    }

    fn receive_return(&mut self, value: Value) {
        let return_register = self.return_register.expect("Return register not set");
        self.write_register(return_register, value);
        self.return_register = None;
    }
}

impl Debug for BytecodeFrame {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "BytecodeFrame {{ locals: {:?}, closure: {:?} }}",
            self.locals.len(),
            self.closure
        )
    }
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let inner = &*self.0.borrow();
        let locals = inner
            .locals
            .keys()
            .map(|k| k.to_owned())
            .collect::<Vec<String>>();
        write!(
            f,
            "Closure {{ locals: {:?}, parent: {:?}, is_static: {:?} }}",
            locals, inner.parent, inner.is_static
        )
    }
}

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
}

impl FrameApi for ModuleFrame {
    fn run(&mut self) -> Action {
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
                panic!("Cannot reenter a module frame which has been entered and left");
            }
        }
    }

    fn receive_return(&mut self, _value: Value) {
        // No-op. Our return will always be the module as a value.
    }
}
