use std::ops::Deref;

use super::super::ir::layout::{
    Address, Instruction, InstructionBuilder, SharedFunction, SharedValue, Unit,
};
use super::nodes::*;

struct Compiler {
    unit: Unit,
    current: SharedFunction,
}

impl Compiler {
    fn new() -> Self {
        let unit = Unit::new();
        let current = unit.main_function();
        Self { unit, current }
    }

    fn compile_program(&mut self, program: &Program) {
        // We should start in the main function.
        assert_eq!(self.current, self.unit.main_function());

        for node in program.nodes.iter() {
            self.compile_node(node);
            // We should always end up back in the main function.
            assert_eq!(self.current, self.unit.main_function());
        }
    }

    fn compile_node(&mut self, node: &Node) -> SharedValue {
        match node {
            &Node::Integer(ref integer) => self.compile_integer(integer),
            &Node::Var(ref var) => self.compile_var(var),
            _ => self.null_value(),
        }
    }

    fn compile_integer(&mut self, integer: &Integer) -> SharedValue {
        self.build_make_integer(integer.value)
    }

    fn compile_var(&mut self, var: &Var) -> SharedValue {
        let index = self.get_or_add_local(var.lhs.value.clone());
        if let Some(ref rhs) = var.rhs {
            let rval = self.compile_node(rhs.deref());
            self.build_set_local(index, rval);
        }
        self.null_value()
    }

    // Convenience proxy methods to the current function:

    fn get_or_add_local(&mut self, local: String) -> u8 {
        self.current.borrow_mut().get_or_add_local(local)
    }

    fn null_value(&self) -> SharedValue {
        self.current.borrow().null_value()
    }
}

// Forward all the heavy lifting to the current function. This allows us to do
// stuff like `self.build_get_local()` above inside of `Compiler`.
impl InstructionBuilder for Compiler {
    fn new_value(&mut self) -> SharedValue {
        self.current.borrow_mut().new_value()
    }

    fn push(&mut self, instruction: Instruction) -> Address {
        self.current.borrow_mut().push(instruction)
    }

    fn track(&mut self, rval: SharedValue, address: Address) {
        self.current.borrow_mut().track(rval, address)
    }
}

pub fn compile(program: &Program) -> Unit {
    let mut compiler = Compiler::new();
    compiler.compile_program(program);
    compiler.unit
}
