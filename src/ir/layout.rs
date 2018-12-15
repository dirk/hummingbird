use std::cell::RefCell;
use std::rc::Rc;

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

type Address = u32;

#[derive(Debug)]
pub struct Value {
    pub id: u32,
    // List of all the instructions that read this value.
    pub dependencies: Vec<Address>,
}

impl Value {
    fn new(id: u32) -> Self {
        Value {
            id,
            dependencies: vec![],
        }
    }

    fn null() -> Self {
        Value {
            id: 0,
            dependencies: vec![],
        }
    }

    fn is_null(&self) -> bool {
        self.id == 0
    }

    // Call this to track that an instruction uses this value.
    fn used_by(&mut self, address: Address) {
        // The null register can be used freely.
        if self.is_null() {
            return;
        }
        if !self.dependencies.contains(&address) {
            self.dependencies.push(address)
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
    pub values: Vec<SharedValue>,
    pub instruction_counter: u32,
}

impl Function {
    fn new<V: Into<String>>(id: u16, name: V) -> Self {
        Self {
            id,
            name: name.into(),
            locals: vec![],
            values: vec![],
            instruction_counter: 0,
        }
    }

    fn new_value(&mut self) -> SharedValue {
        let id = (self.values.len() as u32) + 1;
        let value = Rc::new(RefCell::new(Value::new(id)));
        self.values.push(value.clone());
        value
    }

    fn next_address(&mut self) -> Address {
        let address = self.instruction_counter;
        self.instruction_counter += 1;
        address
    }
}

enum Instruction {
    GetLocal(SharedValue, u8),
    SetLocal(u8, SharedValue),
}

struct BasicBlock {
    parent: RefCell<Function>,
    name: String,
    instructions: Vec<(Address, Instruction)>,
}

impl BasicBlock {
    fn build_get_local(&mut self, index: u8) -> SharedValue {
        let lval = self.new_value();
        self.push(Instruction::GetLocal(lval.clone(), index));
        lval
    }

    fn build_set_local(&mut self, index: u8, rval: SharedValue) {
        let address = self.push(Instruction::SetLocal(index, rval.clone()));
        BasicBlock::track(rval, address);
    }

    fn new_value(&mut self) -> SharedValue {
        self.parent.borrow_mut().new_value()
    }

    fn next_address(&mut self) -> Address {
        self.parent.borrow_mut().next_address()
    }

    fn push(&mut self, instruction: Instruction) -> Address {
        let address = self.next_address();
        self.instructions.push((address, instruction));
        address
    }

    fn track(rval: SharedValue, address: Address) {
        rval.borrow_mut().used_by(address)
    }
}
