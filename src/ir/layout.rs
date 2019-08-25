use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

// We have to share a lot of things around, so use reference-counting to keep
// track of them.
pub type SharedBasicBlock = Rc<RefCell<BasicBlock>>;
pub type SharedFunction = Rc<RefCell<Function>>;
pub type SharedValue = Rc<RefCell<Value>>;

#[derive(Debug)]
pub enum Import {
    // Just the source.
    Default(String),
    // Source and the name.
    Named(String, String),
}

#[derive(Debug)]
pub struct Module {
    pub locals: Vec<String>,
    pub functions: Vec<SharedFunction>,
    pub imports: HashMap<String, Import>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            locals: vec![],
            functions: vec![Rc::new(RefCell::new(Function::new(0, "main")))],
            imports: HashMap::new(),
        }
    }

    pub fn main_function(&self) -> SharedFunction {
        self.functions[0].clone()
    }

    pub fn new_named_function(&mut self, name: String) -> SharedFunction {
        let id = self.functions.len();
        let function = Rc::new(RefCell::new(Function::new(id as u16, name)));
        self.functions.push(function.clone());
        function
    }

    pub fn new_anonymous_function(&mut self, enclosing_function: SharedFunction) -> SharedFunction {
        let id = self.functions.len();
        let name = format!("{}.{}", enclosing_function.borrow().name, id);
        let function = Rc::new(RefCell::new(Function::new(id as u16, name)));
        self.functions.push(function.clone());
        function
    }
}

pub type Address = u32;

pub type ValueId = u32;

#[derive(Debug)]
pub struct Value {
    pub id: ValueId,
    // List of all the instructions that read this value.
    pub dependents: Vec<Address>,
}

impl Value {
    pub fn new(id: ValueId) -> Self {
        Value {
            id,
            dependents: vec![],
        }
    }

    fn null() -> Self {
        Value {
            id: 0,
            dependents: vec![],
        }
    }

    pub fn is_null(&self) -> bool {
        self.id == 0
    }

    // Call this to track that an instruction uses this value.
    pub fn used_by(&mut self, address: Address) {
        // The null register can be used freely.
        if self.is_null() {
            return;
        }
        if !self.dependents.contains(&address) {
            self.dependents.push(address)
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, PartialEq)]
pub struct Function {
    pub id: u16,
    pub name: String,
    // Vector which is used like a set (except with fixed insertion order).
    pub locals: Vec<String>,
    pub bindings: HashSet<String>,
    pub parent_bindings: bool,
    // TODO: Make this precise instead of just grabbing everything.
    pub lexical_captures: bool,

    // Always keep track of where we entered.
    entry: SharedBasicBlock,
    // Keep track of where we are.
    pub current: SharedBasicBlock,
    // Used for compilation.
    pub basic_blocks: Vec<SharedBasicBlock>,

    // All the values allocated within the function.
    pub values: Vec<SharedValue>,
    // Use to generate monotonically-increasing instruction addresses.
    pub instruction_counter: u32,
}

impl Function {
    fn new<V: Into<String>>(id: u16, name: V) -> Self {
        // NOTE: Normally we'd want to call `self.next_basic_block_id()`, but
        //   we don't exist yet to we have to hard-code the 0.
        let entry = Rc::new(RefCell::new(BasicBlock::new(0, "entry")));
        Self {
            id,
            name: name.into(),
            locals: vec![],
            bindings: HashSet::new(),
            parent_bindings: false,
            lexical_captures: false,
            entry: entry.clone(),
            current: entry.clone(),
            basic_blocks: vec![entry],
            values: vec![],
            instruction_counter: 0,
        }
    }

    pub fn null_value(&self) -> SharedValue {
        Rc::new(RefCell::new(Value::null()))
    }

    pub fn have_local(&self, local: &String) -> bool {
        self.locals.contains(local)
    }

    // pub fn get_local(&self, local: &String) -> u8 {
    //     self.locals
    //         .iter()
    //         .position(|existing| existing == local)
    //         .expect("Local not found") as u8
    // }

    // pub fn get_or_add_local(&mut self, local: String) -> u8 {
    //     let position = self.locals.iter().position(|existing| existing == &local);
    //     match position {
    //         Some(index) => index as u8,
    //         None => {
    //             self.locals.push(local);
    //             (self.locals.len() - 1) as u8
    //         }
    //     }
    // }

    pub fn next_address(&mut self) -> Address {
        let address = self.instruction_counter;
        self.instruction_counter += 1;
        address
    }

    pub fn push_basic_block(&mut self, build_branch: bool) -> SharedBasicBlock {
        let id = self.basic_blocks.len();
        let basic_block = Rc::new(RefCell::new(BasicBlock::new(
            id as u16,
            format!("anonymous.{}", id),
        )));
        self.basic_blocks.push(basic_block.clone());
        // If requested to, automatically build the branch from the current
        // block to the new block.
        if build_branch {
            self.build_branch(basic_block.clone());
        }
        self.current = basic_block.clone();
        basic_block
    }

    pub fn set_current_basic_block(&mut self, basic_block: SharedBasicBlock) {
        self.current = basic_block;
    }
}

/// Describes how a given variable slot was resolved.
#[derive(Clone, PartialEq)]
pub enum Slot {
    /// Module-level static slot.
    Static(String),
    /// Local variable slot.
    Local(String, Option<u8>),
    /// Lexical scope through bindings (closures). This is the fallback if we
    /// can't determine if it's local or static.
    Lexical(String),
}

#[derive(Clone, PartialEq)]
pub struct SharedSlot(Rc<RefCell<Slot>>);

impl SharedSlot {
    /// Return a new un-numbered local.
    pub fn new_local(name: String) -> Self {
        Self(Rc::new(RefCell::new(Slot::Local(name, None))))
    }

    pub fn new_lexical(name: String) -> Self {
        Self(Rc::new(RefCell::new(Slot::Lexical(name))))
    }

    pub fn new_static(name: String) -> Self {
        Self(Rc::new(RefCell::new(Slot::Static(name))))
    }

    // pub fn set(&self, value: Slot) {
    //     match self.0.borrow().deref() {
    //         Slot::Unknown(_) => (),
    //         other @ _ => unreachable!("Trying to set an already-known name: {:?}", other),
    //     }
    //     *self.0.borrow_mut() = value;
    // }

    pub fn is_static(&self) -> bool {
        let inner = &*self.0.borrow();
        match inner {
            &Slot::Static(_) => true,
            _ => false,
        }
    }

    pub fn promote_from_local_to_lexical(&self, name: String) {
        let existing_name = match &*self.0.borrow() {
            Slot::Local(name, _) => name.clone(),
            other @ _ => unreachable!("Cannot promote to lexical from non-local: {:?}", other),
        };
        assert_eq!(existing_name, name);
        *self.0.borrow_mut() = Slot::Lexical(name);
    }

    // Sets the index of a local variable.
    pub fn set_local_index(&self, index: u8) {
        let (name, existing_index) = match &*self.0.borrow() {
            Slot::Local(name, existing_index) => (name.clone(), existing_index.clone()),
            other @ _ => unreachable!("Cannot set index on non-local: {:?}", other),
        };
        if let Some(existing_index) = existing_index {
            panic!(
                "Cannot set index as one is already set: existing={}, new={}",
                existing_index, index
            )
        }
        *self.0.borrow_mut() = Slot::Local(name.clone(), Some(index));
    }

    pub fn copy_inner(&self) -> Slot {
        self.0.borrow().clone()
    }
}

impl Debug for Slot {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Slot::Local(name, index) => write!(f, "Local({}, {:?})", name, index)?,
            Slot::Lexical(name) => write!(f, "Lexical({})", name)?,
            Slot::Static(name) => write!(f, "Static({})", name)?,
        };
        Ok(())
    }
}

impl Debug for SharedSlot {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let slot = &*self.0.borrow();
        write!(f, "{:?}", slot)
    }
}

#[derive(Debug, PartialEq)]
pub enum Instruction {
    Get(SharedValue, SharedSlot),
    Set(SharedSlot, SharedValue),
    MakeFunction(SharedValue, SharedFunction),
    MakeInteger(SharedValue, i64),
    OpAdd(SharedValue, SharedValue, SharedValue), // $1 = $2 + $3
    OpLessThan(SharedValue, SharedValue, SharedValue), // $1 = $2 < $3
    Branch(SharedBasicBlock),
    BranchIf(SharedBasicBlock, SharedValue),
    Call(SharedValue, SharedValue, Vec<SharedValue>),
    Return(SharedValue),
    ReturnNull,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: u16,
    pub name: String,
    pub instructions: Vec<(Address, Instruction)>,
}

impl BasicBlock {
    fn new<V: Into<String>>(id: u16, name: V) -> Self {
        Self {
            id,
            name: name.into(),
            instructions: vec![],
        }
    }
}

impl PartialEq for BasicBlock {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub trait InstructionBuilder {
    // Build a new `Value`.
    fn new_value(&mut self) -> SharedValue;

    // Push an instruction: returns the address of the new instruction.
    fn push(&mut self, instruction: Instruction) -> Address;

    // Add an instruction address to the value's list of dependents.
    fn track(&mut self, rval: SharedValue, address: Address);

    fn build_get(&mut self, name: SharedSlot) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::Get(lval.clone(), name));
        lval
    }

    fn build_set(&mut self, name: SharedSlot, rval: SharedValue) {
        let address = self.push(Instruction::Set(name, rval.clone()));
        self.track(rval, address);
    }

    fn build_make_function(&mut self, function: SharedFunction) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::MakeFunction(lval.clone(), function));
        lval
    }

    fn build_make_integer(&mut self, value: i64) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::MakeInteger(lval.clone(), value));
        lval
    }

    fn build_op_add(&mut self, lhs: SharedValue, rhs: SharedValue) -> SharedValue {
        let lval = self.new_value();
        let address = self.push(Instruction::OpAdd(lval.clone(), lhs.clone(), rhs.clone()));
        self.track(lhs, address);
        self.track(rhs, address);
        lval
    }

    fn build_op_less_than(&mut self, lhs: SharedValue, rhs: SharedValue) -> SharedValue {
        let lval = self.new_value();
        let address = self.push(Instruction::OpLessThan(lval.clone(), lhs.clone(), rhs.clone()));
        self.track(lhs, address);
        self.track(rhs, address);
        lval
    }

    fn build_branch(&mut self, destination: SharedBasicBlock) {
        self.push(Instruction::Branch(destination));
    }

    fn build_branch_if(&mut self, destination: SharedBasicBlock, condition: SharedValue) {
        let address = self.push(Instruction::BranchIf(destination, condition.clone()));
        self.track(condition, address);
    }

    fn build_call(&mut self, target: SharedValue, arguments: Vec<SharedValue>) -> SharedValue {
        let lval = self.new_value();
        let address = self.push(Instruction::Call(
            lval.clone(),
            target.clone(),
            arguments.clone(),
        ));
        self.track(target, address);
        for argument in arguments {
            self.track(argument, address);
        }
        lval
    }

    fn build_return(&mut self, rval: SharedValue) {
        let address = self.push(Instruction::Return(rval.clone()));
        self.track(rval, address);
    }

    fn build_return_null(&mut self) {
        self.push(Instruction::ReturnNull);
    }
}

impl InstructionBuilder for Function {
    fn new_value(&mut self) -> SharedValue {
        let id = (self.values.len() as u32) + 1;
        let value = Rc::new(RefCell::new(Value::new(id)));
        self.values.push(value.clone());
        value
    }

    fn push(&mut self, instruction: Instruction) -> Address {
        let address = self.next_address();
        self.current
            .borrow_mut()
            .instructions
            .push((address, instruction));
        address
    }

    fn track(&mut self, rval: SharedValue, address: Address) {
        rval.borrow_mut().used_by(address)
    }
}
