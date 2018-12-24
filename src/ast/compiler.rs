use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use super::super::ir::layout::{
    Address, Instruction, InstructionBuilder, Module, SharedFunction, SharedValue,
};
use super::nodes::*;
use super::super::vm::prelude::is_in_prelude;

#[derive(Debug)]
enum ScopeResolution {
    Local(u8),
    Lexical(String),
    Constant(String),
    NotFound(String),
}

trait Scope {
    fn resolve(&mut self, name: &String) -> ScopeResolution;
}

struct FunctionScope<'a> {
    parent: &'a mut Scope,
    function: SharedFunction,
}

impl<'a> FunctionScope<'a> {
    fn new(parent: &'a mut Scope, function: SharedFunction) -> Self {
        Self { parent, function }
    }
}

impl<'a> Scope for FunctionScope<'a> {
    fn resolve(&mut self, name: &String) -> ScopeResolution {
        if self.function.borrow().have_local(name) {
            ScopeResolution::Local(self.function.borrow().get_local(name))
        } else {
            self.parent.resolve(name)
        }
    }
}

struct ModuleScope {
    module: Rc<RefCell<Module>>,
    // Map an import name to its source.
    imports: HashMap<String, String>,
}

impl Scope for ModuleScope {
    fn resolve(&mut self, name: &String) -> ScopeResolution {
        if is_in_prelude(name) {
            self.imports.insert(name.to_owned(), "prelude".to_owned());
            ScopeResolution::Constant(name.to_owned())
        } else {
            ScopeResolution::NotFound(name.to_owned())
        }
    }
}

struct Compiler {
    unit: Rc<RefCell<Module>>,
    current: SharedFunction,
}

impl Compiler {
    fn new() -> Self {
        let unit = Rc::new(RefCell::new(Module::new()));
        let current = unit.borrow().main_function();
        Self { unit, current }
    }

    fn compile_program(&mut self, program: &Program) {
        let mut module_scope = ModuleScope {
            module: self.unit.clone(),
            imports: HashMap::new(),
        };

        // We should start in the main function.
        assert_eq!(self.current, self.unit.borrow().main_function());

        let mut scope = FunctionScope::new(&mut module_scope, self.current.clone());

        for node in program.nodes.iter() {
            self.compile_node(node, &mut scope);
            // We should always end up back in the main function.
            assert_eq!(self.current, self.unit.borrow().main_function());
        }
    }

    fn compile_node(&mut self, node: &Node, scope: &mut Scope) -> SharedValue {
        match node {
            &Node::Assignment(ref assignment) => self.compile_assignment(assignment, scope),
            &Node::Block(ref block) => self.compile_anonymous_block(block, scope),
            &Node::Function(ref function) => self.compile_function(function, scope),
            &Node::Identifier(ref identifier) => self.compile_identifier(identifier, scope),
            &Node::Integer(ref integer) => self.compile_integer(integer, scope),
            &Node::PostfixCall(ref call) => self.compile_postfix_call(call, scope),
            &Node::Return(ref ret) => self.compile_return(ret, scope),
            &Node::Var(ref var) => self.compile_var(var, scope),
            _ => panic!("Cannot compile node: {:?}", node),
        }
    }

    fn compile_assignment(&mut self, assignment: &Assignment, scope: &mut Scope) -> SharedValue {
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

        let rval = self.compile_node(assignment.rhs.deref(), scope);
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

    fn compile_anonymous_block(&mut self, block: &Block, scope: &mut Scope) -> SharedValue {
        // Push a new basic block and branch to it.
        self.current.borrow_mut().push_basic_block(true);

        let mut implicit_return = self.null_value();
        for node in block.nodes.iter() {
            implicit_return = self.compile_node(node, scope);
        }

        // Exit from the current block to the new block.
        self.current.borrow_mut().push_basic_block(true);

        implicit_return
    }

    fn compile_function(&mut self, function: &Function, scope: &mut Scope) -> SharedValue {
        let name = function.name.to_owned();
        let outer_function = self.current.clone();
        let new_function = self.unit.borrow_mut().new_function(name.clone());
        self.current = new_function.clone();

        let mut function_scope = FunctionScope::new(scope, self.current.clone());
        for node in function.block.nodes.iter() {
            self.compile_node(node, &mut function_scope);
        }
        self.build_return_null();

        self.current = outer_function;
        let lval = self.build_make_function(new_function);
        let index = self.get_or_add_local(name);
        self.build_set_local(index, lval);
        self.null_value()
    }

    fn compile_identifier(&mut self, identifier: &Identifier, scope: &mut Scope) -> SharedValue {
        let local = &identifier.value;
        let resolution = scope.resolve(local);
        match resolution {
            ScopeResolution::Local(index) => self.build_get_local(index),
            ScopeResolution::Lexical(name) => self.build_get_local_lexical(name),
            _ => panic!("Cannot handle resolution: {:?}", resolution),
        }
    }

    fn compile_integer(&mut self, integer: &Integer, scope: &mut Scope) -> SharedValue {
        self.build_make_integer(integer.value)
    }

    fn compile_postfix_call(&mut self, call: &PostfixCall, scope: &mut Scope) -> SharedValue {
        let target = self.compile_node(call.target.deref(), scope);
        let mut arguments = vec![];
        for argument in call.arguments.iter() {
            arguments.push(self.compile_node(argument, scope));
        }
        self.build_call(target, arguments)
    }

    fn compile_return(&mut self, ret: &Return, scope: &mut Scope) -> SharedValue {
        if let Some(ref rhs) = ret.rhs {
            let rval = self.compile_node(&rhs, scope);
            self.build_return(rval);
        } else {
            self.build_return_null();
        }
        self.null_value()
    }

    fn compile_var(&mut self, var: &Var, scope: &mut Scope) -> SharedValue {
        let index = self.get_or_add_local(var.lhs.value.clone());
        if let Some(ref rhs) = var.rhs {
            let rval = self.compile_node(rhs.deref(), scope);
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

pub fn compile(program: &Program) -> Module {
    let mut compiler = Compiler::new();
    compiler.compile_program(program);
    // The reference count should be 1 at this point.
    let module_cell = Rc::try_unwrap(compiler.unit).unwrap();
    module_cell.into_inner()
}
