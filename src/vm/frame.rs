use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

use super::super::target::bytecode::layout::{Instruction, Reg};

use super::loader::{BytecodeFunction, LoadedFunction, LoadedModule};
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

    fn get(&self, name: &String) -> Option<Value> {
        let inner = &self.0.borrow();
        if let Some(value) = inner.locals.get(name) {
            return match value {
                initialized @ Some(_) => initialized.clone(),
                None => unreachable!("ERROR: Uninitialized closure variable: {}", name),
            };
        }
        if let Some(parent) = &inner.parent {
            return parent.get(name);
        }
        unreachable!("ERROR: Couldn't find closure variable: {}", name)
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
        if inner.is_static {
            inner.locals.insert(name, Some(value));
            return true;
        }
        if let Some(parent) = &inner.parent {
            // If we found a parent with this name and were able to set the
            // value then all is good.
            if parent.set_as_parent(name.clone(), value) {
                return true;
            }
        }
        return false;
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
}

pub enum Frame {
    Bytecode(BytecodeFrame),
}

impl FrameApi for Frame {
    fn run(&mut self) -> Action {
        match self {
            Frame::Bytecode(frame) => frame.run(),
        }
    }

    fn get_lexical(&self, name: &String) -> Value {
        match self {
            Frame::Bytecode(frame) => frame.get_lexical(name),
        }
    }

    fn set_lexical(&mut self, name: &String, value: Value) {
        match self {
            Frame::Bytecode(frame) => frame.set_lexical(name, value),
        }
    }

    fn receive_return(&mut self, value: Value) {
        match self {
            Frame::Bytecode(frame) => frame.receive_return(value),
        }
    }
}

pub trait FrameApi {
    // Run the frame's fetch-execute loop. Will be different depending on if
    // it's a bytecode or native frame.
    fn run(&mut self) -> Action;

    fn get_lexical(&self, name: &String) -> Value;

    fn set_lexical(&mut self, name: &String, value: Value);

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
            function,
            bytecode,
            locals: vec![Value::Null; locals as usize],
            closure,
            return_register: None,
            registers: vec![Value::Null; registers as usize],
            current_address: 0,
        }
    }

    pub fn unit(&self) -> LoadedModule {
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

    #[inline]
    fn offset_register(index: Reg) -> usize {
        (index as usize) - 1
    }

    pub fn read_register(&self, index: Reg) -> Value {
        self.registers[BytecodeFrame::offset_register(index)].clone()
    }

    fn write_register(&mut self, index: Reg, value: Value) {
        if index == 0 {
            return;
        }
        self.registers[BytecodeFrame::offset_register(index)] = value;
    }

    pub fn get_constant<N: AsRef<str>>(&self, name: N) -> Value {
        self.function.module().get_constant(name.as_ref())
    }

    pub fn get_local(&self, index: u8) -> Value {
        self.locals[index as usize].clone()
    }

    pub fn set_local(&mut self, index: u8, value: Value) {
        self.locals[index as usize] = value;
    }

    fn get_index_of_local(&self, name: &String) -> Option<usize> {
        self.bytecode
            .locals_names()
            .iter()
            .position(|local| local == name)
    }
}

impl FrameApi for BytecodeFrame {
    fn run(&mut self) -> Action {
        loop {
            let instruction = self.current();

            match &instruction {
                Instruction::GetConstant(lval, name) => {
                    self.write_register(*lval, self.get_constant(name));
                    self.advance();
                }
                Instruction::GetLocal(lval, index) => {
                    self.write_register(*lval, self.get_local(*index));
                    self.advance();
                }
                Instruction::GetLocalLexical(lval, name) => {
                    self.write_register(*lval, self.get_lexical(name));
                    self.advance();
                }
                Instruction::SetLocal(index, rval) => {
                    self.set_local(*index, self.read_register(*rval));
                    self.advance();
                }
                Instruction::SetLocalLexical(name, rval) => {
                    self.set_lexical(name, self.read_register(*rval));
                    self.advance();
                }
                Instruction::MakeFunction(lval, id) => {
                    let function = self.unit().function(*id);
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
                Instruction::Branch(destination) => {
                    self.current_address = *destination as usize;
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
                            let closure = if function.binds_on_call() {
                                let bindings = function.bindings();
                                let maybe_bindings = if !bindings.is_empty() {
                                    Some(bindings)
                                } else {
                                    None
                                };
                                Some(Closure::new(maybe_bindings, parent_closure))
                            } else {
                                None
                            };
                            let frame = Frame::Bytecode(BytecodeFrame::new(function, closure));
                            // Be at the next instruction when control flow returns to us.
                            self.advance();
                            return Action::Call(frame);
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
                    return Action::Return(value);
                }
                Instruction::ReturnNull => {
                    return Action::Return(Value::Null);
                }
            }
        }
    }

    fn get_lexical(&self, name: &String) -> Value {
        if let Some(index) = self.get_index_of_local(name) {
            return self.locals[index].clone();
        }
        if let Some(closure) = &self.closure {
            return closure.get(name).expect(&format!("Not found: {}", name));
        }
        panic!("Not found: {}", name)
    }

    fn set_lexical(&mut self, name: &String, value: Value) {
        if let Some(closure) = &self.closure {
            if closure.try_set(name.clone(), value.clone()) {
                return;
            }
        }
        if let Some(index) = self.get_index_of_local(name) {
            self.locals[index] = value;
        } else {
            panic!("Not found: {}\nFrame: {:?}", name, self)
        }
    }

    fn receive_return(&mut self, value: Value) {
        let return_register = self.return_register.expect("Return register not set");
        self.write_register(return_register, value);
        self.return_register = None;
    }
}

impl Debug for BytecodeFrame {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(
            f,
            "BytecodeFrame {{ locals: {:?}, closure: {:?} }}",
            self.locals.len(),
            self.closure
        )
    }
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
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
