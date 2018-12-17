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
            &Node::Assignment(ref assignment) => self.compile_assignment(assignment),
            &Node::Identifier(ref identifier) => self.compile_identifier(identifier),
            &Node::Integer(ref integer) => self.compile_integer(integer),
            &Node::PostfixCall(ref call) => self.compile_postfix_call(call),
            &Node::Return(ref ret) => self.compile_return(ret),
            &Node::Var(ref var) => self.compile_var(var),
            _ => panic!("Cannot compile node: {:?}", node),
        }
    }

    fn compile_assignment(&mut self, assignment: &Assignment) -> SharedValue {
        let lhs = assignment.lhs.deref();

        enum Assigner {
            Local(u8),
            LexicalLocal(String),
        }

        let assigner = match lhs {
            &Node::Identifier(ref identifier) => {
                let local = &identifier.value;
                if self.current.borrow().have_local(&local) {
                    let index = self.current.borrow().get_local(&local);
                    Assigner::Local(index)
                } else {
                    Assigner::LexicalLocal(local.to_string())
                }
            }
            _ => panic!("Cannot assign to: {:?}", lhs),
        };

        let rval = self.compile_node(assignment.rhs.deref());
        match assigner {
            Assigner::Local(index) => {
                self.build_set_local(index, rval.clone());
                rval
            }
            Assigner::LexicalLocal(local) => {
                self.build_set_local_lexical(local, rval.clone());
                rval
            }
        }
    }

    fn compile_identifier(&mut self, identifier: &Identifier) -> SharedValue {
        let local = &identifier.value;
        if self.current.borrow().have_local(local) {
            let index = self.current.borrow().get_local(local);
            self.build_get_local(index)
        } else {
            self.build_get_local_lexical(local.clone())
        }
    }

    fn compile_integer(&mut self, integer: &Integer) -> SharedValue {
        self.build_make_integer(integer.value)
    }

    fn compile_postfix_call(&mut self, call: &PostfixCall) -> SharedValue {
        let target = self.compile_node(call.target.deref());
        let mut arguments = vec![];
        for argument in call.arguments.iter() {
            arguments.push(self.compile_node(argument));
        }
        self.build_call(target, arguments)
    }

    fn compile_return(&mut self, ret: &Return) -> SharedValue {
        if let Some(ref rhs) = ret.rhs {
            let rval = self.compile_node(&rhs);
            self.build_return(rval);
        } else {
            self.build_return_null();
        }
        self.null_value()
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
