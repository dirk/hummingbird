use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt::{self, Debug, Formatter};
use std::io::Write;
use std::rc::Rc;

use super::super::target::bytecode::layout::{Instruction, Reg};

use super::super::ast_to_ir;
use super::super::parser;
use super::errors::UndefinedNameError;
use super::loader::{self, BytecodeFunction, LoadedFunction, LoadedModule};
use super::operators;
use super::value::Value;

struct InnerClosure {
    locals: HashMap<String, Option<Value>>,
    parent: Option<Closure>,
    /// If this closure is for a REPL. It allows us to set new variables at
    /// will.
    repl: bool,
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
            repl: false,
        })))
    }

    pub fn new_static() -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: None,
            repl: false,
        })))
    }

    pub fn new_repl() -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: None,
            repl: true,
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
        // If it's for a REPL then we can create new locals at will.
        if inner.repl {
            inner.locals.insert(name, Some(value));
            return true;
        }
        if let Some(parent) = &inner.parent {
            return parent.try_set(name.clone(), value);
        }
        return false;
    }

    /// Set a local directly into this exact closure. This should only be used
    /// by the VM when initializing a module's closure with imports.
    pub fn set_directly(&self, name: String, value: Value) {
        let inner = &mut self.0.borrow_mut();
        inner.locals.insert(name, Some(value));
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
    Repl(ReplFrame),
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

    fn can_catch_error(&self, error: &Box<dyn error::Error>) -> bool {
        match self {
            Frame::Bytecode(frame) => frame.can_catch_error(error),
            Frame::Module(frame) => frame.can_catch_error(error),
            Frame::Repl(frame) => frame.can_catch_error(error),
        }
    }

    fn catch_error(&mut self, error: Box<dyn error::Error>) {
        match self {
            Frame::Bytecode(frame) => frame.catch_error(error),
            Frame::Module(frame) => frame.catch_error(error),
            Frame::Repl(frame) => frame.catch_error(error),
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
    fn can_catch_error(&self, _error: &Box<dyn error::Error>) -> bool {
        false
    }

    /// This method should not do any evaluation. Instead it should merely
    /// prepare for evaluation to resume in this frame.
    fn catch_error(&mut self, _error: Box<dyn error::Error>) {
        unreachable!()
    }
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
            "Closure {{ locals: {:?}, parent: {:?}, repl: {:?} }}",
            locals, inner.parent, inner.repl
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

pub struct ReplFrame {
    // FIXME: Set up a shared loader to hold loaded modules in memory.
    //   Eventually they should be part of the GC graph so that they're freed
    //   when there are no longer live references to them.
    loaded_modules: Vec<LoadedModule>,
    counter: u16,
    static_closure: Closure,
    // The result of the last expression's evaluation.
    last_result: Option<Value>,
    last_error: Option<Box<dyn error::Error>>,
}

impl ReplFrame {
    pub fn new() -> Self {
        Self {
            loaded_modules: vec![],
            counter: 0,
            static_closure: Closure::new_repl(),
            last_result: None,
            last_error: None,
        }
    }

    pub fn closure(&self) -> Closure {
        self.static_closure.clone()
    }

    fn compile_line(&mut self, line: String, counter: u16) -> LoadedModule {
        let name = format!("repl[{}]", counter);

        let ast_module = parser::parse(line);
        let loaded_module =
            loader::compile_ast_into_module(&ast_module, name, ast_to_ir::CompilationFlags::Repl)
                .expect("Couldn't compile line");
        // Hold it in ourselves so that it doesn't get dropped.
        self.loaded_modules.push(loaded_module.clone());
        // Make all the loaded modules share the same static closure so that
        // they see all the same defined variables.
        loaded_module.override_static_closure(self.static_closure.clone());
        loaded_module
    }
}

impl FrameApi for ReplFrame {
    fn run(&mut self) -> Action {
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
                    let module = self.compile_line(buffer, counter);
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
    fn can_catch_error(&self, _error: &Box<dyn error::Error>) -> bool {
        true
    }

    fn catch_error(&mut self, error: Box<dyn error::Error>) {
        println!("{}", error);
        self.last_result = None;
        self.last_error = Some(error);
    }
}
