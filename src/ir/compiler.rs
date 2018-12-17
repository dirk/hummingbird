use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::ops::Deref;

use super::super::target::bytecode::layout as bytecode;
use super::layout as ir;

struct BasicBlockFinder {
    indices: HashMap<u16, usize>,
}

impl BasicBlockFinder {
    fn new(basic_blocks: &Vec<ir::SharedBasicBlock>) -> Self {
        let mut indices = HashMap::new();
        for (index, basic_block) in basic_blocks.iter().enumerate() {
            let basic_block = basic_block.deref().borrow();
            indices.insert(basic_block.id, index);
        }
        Self { indices }
    }

    fn find(&self, basic_block: &ir::BasicBlock) -> usize {
        *self.indices.get(&basic_block.id).unwrap()
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
    fn compile_unit(unit: &ir::Unit) -> bytecode::Unit {
        let functions = unit
            .functions
            .iter()
            .map(|function| Compiler::compile_function(function.deref().borrow()))
            .collect::<Vec<bytecode::Function>>();

        bytecode::Unit {
            functions: functions,
        }
    }

    fn compile_function(function: Ref<ir::Function>) -> bytecode::Function {
        let basic_block_finder = BasicBlockFinder::new(&function.basic_blocks);
        let mut register_allocator = RegisterAllocator::new();

        let basic_blocks = function
            .basic_blocks
            .iter()
            .map(|basic_block| {
                let basic_block = basic_block.deref().borrow();
                Compiler::compile_basic_block(
                    &basic_block,
                    &basic_block_finder,
                    &mut register_allocator,
                )
            })
            .collect::<Vec<bytecode::BasicBlock>>();

        bytecode::Function {
            id: function.id,
            name: function.name.clone(),
            registers: register_allocator.registers_required() as u8,
            basic_blocks: basic_blocks,
            locals: function.locals.len() as u8,
            locals_names: function.locals.clone(),
        }
    }

    fn compile_basic_block(
        basic_block: &ir::BasicBlock,
        basic_block_finder: &BasicBlockFinder,
        register_allocator: &mut RegisterAllocator,
    ) -> bytecode::BasicBlock {
        let id = basic_block_finder.find(&basic_block);

        // Build a shared mutable cell so that we can mutate the register
        // allocator from our `allocate` and `read` convenience closures.
        let register_allocator = RefCell::new(register_allocator);
        let allocate = |lval: &ir::SharedValue| {
            register_allocator
                .borrow_mut()
                .allocate(&lval.deref().borrow())
        };
        let read = |rval: &ir::SharedValue, address| {
            register_allocator
                .borrow_mut()
                .read(&rval.deref().borrow(), address)
        };

        let mut instructions = vec![];
        for (address, instruction) in basic_block.instructions.iter() {
            let bytecode_instruction = match instruction {
                ir::Instruction::GetLocal(lval, index) => {
                    bytecode::Instruction::GetLocal(allocate(lval), *index)
                }
                ir::Instruction::GetLocalLexical(lval, name) => {
                    bytecode::Instruction::GetLocalLexical(allocate(lval), name.clone())
                }
                ir::Instruction::SetLocal(index, rval) => {
                    bytecode::Instruction::SetLocal(*index, read(rval, address))
                }
                ir::Instruction::MakeFunction(lval, function) => {
                    let id = function.borrow().id;
                    bytecode::Instruction::MakeFunction(allocate(lval), id)
                }
                ir::Instruction::MakeInteger(lval, value) => {
                    bytecode::Instruction::MakeInteger(allocate(lval), *value)
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
                _ => panic!("Cannot compile instruction: {:?}", instruction),
            };
            instructions.push(bytecode_instruction)
        }

        bytecode::BasicBlock {
            id: id as u8,
            name: basic_block.name.clone(),
            instructions,
        }
    }
}

pub fn compile(unit: &ir::Unit) -> bytecode::Unit {
    Compiler::compile_unit(unit)
}
