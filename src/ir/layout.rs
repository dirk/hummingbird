use std::cell::RefCell;
use std::rc::Rc;

// We have to share a lot of things around, so use reference-counting to keep
// track of them.
pub type SharedBasicBlock = Rc<RefCell<BasicBlock>>;
pub type SharedFunction = Rc<RefCell<Function>>;
pub type SharedValue = Rc<RefCell<Value>>;

#[derive(Debug)]
pub struct Unit {
    pub locals: Vec<String>,
    pub functions: Vec<SharedFunction>,
}

impl Unit {
    pub fn new() -> Self {
        Self {
            locals: vec![],
            functions: vec![Rc::new(RefCell::new(Function::new(0, "main")))],
        }
    }

    pub fn main_function(&self) -> SharedFunction {
        self.functions[0].clone()
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
    fn new(id: ValueId) -> Self {
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
    fn used_by(&mut self, address: Address) {
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
    pub locals: Vec<String>,

    // Always keep track of where we entered.
    entry: SharedBasicBlock,
    // Keep track of where we are.
    current: SharedBasicBlock,
    // Used for compilation.
    pub basic_blocks: Vec<SharedBasicBlock>,

    // All the values allocated within the function.
    pub values: Vec<SharedValue>,
    // Use to generate monotonically-increasing instruction addresses.
    pub instruction_counter: u32,
}

impl Function {
    fn new<V: Into<String>>(id: u16, name: V) -> Self {
        let entry = Rc::new(RefCell::new(BasicBlock::new("entry")));
        Self {
            id,
            name: name.into(),
            locals: vec![],
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

    pub fn get_local(&self, local: &String) -> u8 {
        self.locals
            .iter()
            .position(|existing| existing == local)
            .expect("Local not found") as u8
    }

    pub fn get_or_add_local(&mut self, local: String) -> u8 {
        let position = self.locals.iter().position(|existing| existing == &local);
        match position {
            Some(index) => index as u8,
            None => {
                self.locals.push(local);
                (self.locals.len() - 1) as u8
            }
        }
    }

    fn next_address(&mut self) -> Address {
        let address = self.instruction_counter;
        self.instruction_counter += 1;
        address
    }
}

pub trait InstructionBuilder {
    // Build a new `Value`.
    fn new_value(&mut self) -> SharedValue;

    // Push an instruction: returns the address of the new instruction.
    fn push(&mut self, instruction: Instruction) -> Address;

    // Add an instruction address to the value's list of dependents.
    fn track(&mut self, rval: SharedValue, address: Address);

    fn build_get_local(&mut self, index: u8) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::GetLocal(lval.clone(), index));
        lval
    }

    fn build_get_local_lexical(&mut self, name: String) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::GetLocalLexical(lval.clone(), name));
        lval
    }

    fn build_set_local(&mut self, index: u8, rval: SharedValue) {
        let address = self.push(Instruction::SetLocal(index, rval.clone()));
        self.track(rval, address);
    }

    fn build_set_local_lexical(&mut self, name: String, rval: SharedValue) {
        let address = self.push(Instruction::SetLocalLexical(name, rval.clone()));
        self.track(rval, address);
    }

    fn build_make_integer(&mut self, value: i64) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::MakeInteger(lval.clone(), value));
        lval
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

#[derive(Debug, PartialEq)]
pub enum Instruction {
    GetLocal(SharedValue, u8),
    GetLocalLexical(SharedValue, String),
    SetLocal(u8, SharedValue),
    SetLocalLexical(String, SharedValue),
    MakeInteger(SharedValue, i64),
    Call(SharedValue, SharedValue, Vec<SharedValue>),
}

#[derive(Debug, PartialEq)]
pub struct BasicBlock {
    pub name: String,
    pub instructions: Vec<(Address, Instruction)>,
}

impl BasicBlock {
    fn new<V: Into<String>>(name: V) -> Self {
        Self {
            name: name.into(),
            instructions: vec![],
        }
    }
}
