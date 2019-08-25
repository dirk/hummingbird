use std::cell::{Ref, RefCell};
use std::collections::HashMap;

use super::super::target::bytecode::layout as bytecode;
use super::layout as ir;

struct BasicBlockTracker {
    // Map basic block IDs to the addresses where they start.
    starts: HashMap<u16, usize>,
    // Track branch instructions that need to be filled in after we know all
    // the addresses.
    branches: Vec<(usize, u16)>,
}

impl BasicBlockTracker {
    fn new() -> Self {
        Self {
            starts: HashMap::new(),
            branches: vec![],
        }
    }

    fn track_start(&mut self, basic_block: &ir::BasicBlock, address: usize) {
        self.starts.insert(basic_block.id, address);
    }

    fn track_branch(&mut self, address: usize, basic_block: &ir::BasicBlock) {
        self.branches.push((address, basic_block.id));
    }
}

struct RegisterAllocator {
    registers: Vec<Option<ir::ValueId>>,
    allocated: Vec<ir::ValueId>,
    live_dependents: HashMap<ir::ValueId, Vec<ir::Address>>,
}

impl RegisterAllocator {
    fn new() -> Self {
        Self {
            registers: vec![],
            allocated: vec![],
            live_dependents: HashMap::new(),
        }
    }

    fn allocate(&mut self, value: &ir::Value) -> bytecode::Reg {
        if value.is_null() {
            return 0;
        }

        // Allocation can only happen once since IR values are SSA-form.
        if self.allocated.contains(&value.id) {
            panic!("Value already allocated: ${}", value.id);
        }
        self.allocated.push(value.id);

        // If no one is going to use us then we can "allocate" as the
        // null register.
        if value.dependents.len() == 0 {
            return 0;
        }

        // All values start off with their dependencies being live.
        self.live_dependents
            .insert(value.id, value.dependents.clone());

        // Find the first available register.
        for (index, register) in self.registers.iter_mut().enumerate() {
            if *register == None {
                *register = Some(value.id);
                return RegisterAllocator::offset_register(index);
            }
        }

        self.registers.push(Some(value.id));
        RegisterAllocator::offset_register(self.registers.len() - 1)
    }

    fn read(&mut self, value: &ir::Value, address: &ir::Address) -> bytecode::Reg {
        if value.is_null() {
            return 0;
        }

        let dependents = self
            .live_dependents
            .get_mut(&value.id)
            .expect(&format!("Value not live: ${}", value.id));

        let dependent_index = dependents
            .iter()
            .position(|dependent| dependent == address)
            .expect(&format!("Dependent not found: {:04}", address));

        // Free the dependent now that we've used it.
        dependents.remove(dependent_index);

        let register_index = self
            .registers
            .iter()
            .position(|register| *register == Some(value.id))
            .expect(&format!("Register not found: ${}", value.id));

        // If all dependents have been freed then we can free the register.
        if dependents.len() == 0 {
            self.registers[register_index] = None;
        }

        RegisterAllocator::offset_register(register_index)
    }

    fn registers_required(&self) -> usize {
        self.registers.len()
    }

    // Applies an "algorithm" to properly offset an index in `registers` to
    // leave r0 free because it's the null register.
    fn offset_register(register: usize) -> bytecode::Reg {
        (register as bytecode::Reg) + 1
    }
}

struct Compiler {}

impl Compiler {
    fn compile_module(module: &ir::Module) -> bytecode::Module {
        let functions = module
            .functions
            .iter()
            .map(|function| Compiler::compile_function(function.borrow()))
            .collect::<Vec<bytecode::Function>>();

        bytecode::Module {
            functions: functions,
        }
    }

    fn compile_function(function: Ref<ir::Function>) -> bytecode::Function {
        let mut basic_block_tracker = BasicBlockTracker::new();

        // Build a shared mutable cell so that we can mutate the register
        // allocator from our `allocate` and `read` convenience closures.
        let register_allocator = RefCell::new(RegisterAllocator::new());

        let mut instructions: Vec<bytecode::Instruction> = vec![];

        for basic_block in function.basic_blocks.iter() {
            Compiler::compile_basic_block(
                &basic_block.borrow(),
                &mut instructions,
                &mut basic_block_tracker,
                &register_allocator,
            );
        }

        // Set all the branch destinations now that we know where the blocks lie.
        for (address, id) in basic_block_tracker.branches {
            let block_address = (*basic_block_tracker.starts.get(&id).unwrap()) as u8;
            let instruction = instructions.get_mut(address).unwrap();
            match instruction {
                bytecode::Instruction::Branch(ref mut destination) => {
                    *destination = block_address;
                }
                bytecode::Instruction::BranchIf(ref mut destination, _) => {
                    *destination = block_address;
                }
                _ => panic!("Unexpected instruction: {:?}", instruction),
            }
        }

        let registers = register_allocator.borrow().registers_required() as u8;

        bytecode::Function {
            id: function.id,
            name: function.name.clone(),
            registers,
            instructions,
            locals: function.locals.len() as u8,
            locals_names: function.locals.clone(),
            bindings: function.bindings.clone(),
            parent_bindings: function.parent_bindings,
        }
    }

    fn compile_basic_block(
        basic_block: &ir::BasicBlock,
        instructions: &mut Vec<bytecode::Instruction>,
        basic_block_tracker: &mut BasicBlockTracker,
        register_allocator: &RefCell<RegisterAllocator>,
    ) {
        basic_block_tracker.track_start(basic_block, instructions.len());

        let allocate =
            |lval: &ir::SharedValue| register_allocator.borrow_mut().allocate(&lval.borrow());
        let read = |rval: &ir::SharedValue, address| {
            register_allocator
                .borrow_mut()
                .read(&rval.borrow(), address)
        };

        for (address, instruction) in basic_block.instructions.iter() {
            let bytecode_instruction = match instruction {
                ir::Instruction::Get(lval, slot) => {
                    let reg = allocate(lval);
                    match slot.copy_inner() {
                        ir::Slot::Local(_name, index) => {
                            bytecode::Instruction::GetLocal(reg, index.expect("Un-indexed local"))
                        }
                        ir::Slot::Lexical(name) => bytecode::Instruction::GetLexical(reg, name),
                        ir::Slot::Static(name) => bytecode::Instruction::GetStatic(reg, name),
                    }
                }
                ir::Instruction::Set(slot, rval) => {
                    let reg = read(rval, address);
                    match slot.copy_inner() {
                        ir::Slot::Local(name, index) => bytecode::Instruction::SetLocal(
                            index.expect(&format!("Un-indexed local: {}", name)),
                            reg,
                        ),
                        ir::Slot::Lexical(name) => bytecode::Instruction::SetLexical(name, reg),
                        ir::Slot::Static(name) => bytecode::Instruction::SetStatic(name, reg),
                    }
                }
                ir::Instruction::MakeFunction(lval, function) => {
                    let id = function.borrow().id;
                    bytecode::Instruction::MakeFunction(allocate(lval), id)
                }
                ir::Instruction::MakeInteger(lval, value) => {
                    bytecode::Instruction::MakeInteger(allocate(lval), *value)
                }
                ir::Instruction::OpAdd(lval, lhs, rhs) => bytecode::Instruction::OpAdd(
                    allocate(lval),
                    read(lhs, address),
                    read(rhs, address),
                ),
                ir::Instruction::OpLessThan(lval, lhs, rhs) => bytecode::Instruction::OpLessThan(
                    allocate(lval),
                    read(lhs, address),
                    read(rhs, address),
                ),
                ir::Instruction::Branch(destination) => {
                    let bytecode_address = instructions.len();
                    basic_block_tracker.track_branch(bytecode_address, &destination.borrow());
                    bytecode::Instruction::Branch(0)
                }
                ir::Instruction::BranchIf(destination, condition) => {
                    let bytecode_address = instructions.len();
                    basic_block_tracker.track_branch(bytecode_address, &destination.borrow());
                    bytecode::Instruction::BranchIf(0, read(condition, address))
                }
                ir::Instruction::Call(lval, target, arguments) => {
                    let lval = allocate(lval);
                    let target = read(target, address);
                    let arguments = arguments
                        .iter()
                        .map(|argument| read(argument, address))
                        .collect::<Vec<bytecode::Reg>>();
                    bytecode::Instruction::Call(lval, target, arguments)
                }
                ir::Instruction::Return(rval) => bytecode::Instruction::Return(read(rval, address)),
                ir::Instruction::ReturnNull => bytecode::Instruction::ReturnNull,
            };
            instructions.push(bytecode_instruction)
        }
    }
}

pub fn compile(unit: &ir::Module) -> bytecode::Module {
    Compiler::compile_module(unit)
}
