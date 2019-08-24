use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::ast::nodes::*;
use super::ir::layout::{
    Address, Import, Instruction, InstructionBuilder, Module as IrModule, SharedFunction,
    SharedSlot, SharedValue,
};
use super::vm::prelude::is_in_prelude;

trait Scope {
    fn add_local(&mut self, name: &String) -> SharedSlot;

    fn resolve(&mut self, name: &String) -> SharedSlot;

    /// Acts the same as `resolve`, but records bindings along the way.
    fn resolve_as_binding(&mut self, name: &String) -> SharedSlot;
}

struct FunctionScope<'a> {
    parent: &'a mut dyn Scope,
    function: SharedFunction,
    // Keep track of every variable we've seen. Some of them may end up bound,
    // so we'll actually make them proper local variables or bound variables
    // when we traverse upwards.
    locals: HashMap<String, SharedSlot>,
    // Keep track of which variables need to be bound and come from parent
    // bindings.
    bindings: HashSet<String>,
    parent_bindings: HashSet<String>,
}

impl<'a> FunctionScope<'a> {
    fn new(parent: &'a mut dyn Scope, function: SharedFunction) -> Self {
        Self {
            parent,
            function,
            locals: HashMap::new(),
            bindings: HashSet::new(),
            parent_bindings: HashSet::new(),
        }
    }
}

impl<'a> Scope for FunctionScope<'a> {
    fn add_local(&mut self, name: &String) -> SharedSlot {
        let slot = SharedSlot::new_local(name.clone());
        self.locals.insert(name.clone(), slot.clone());
        slot
    }

    fn resolve(&mut self, name: &String) -> SharedSlot {
        if let Some(resolution) = self.locals.get(name) {
            resolution.clone()
        } else {
            self.parent_bindings.insert(name.clone());
            self.parent.resolve_as_binding(name)
        }
    }

    fn resolve_as_binding(&mut self, name: &String) -> SharedSlot {
        if let Some(resolution) = self.locals.get(name) {
            // Since this was called from a lower function scope we now know
            // we need to bind it for that lower scope.
            self.bindings.insert(name.clone());
            resolution.clone()
        } else {
            self.parent_bindings.insert(name.clone());
            self.parent.resolve_as_binding(name)
        }
    }
}

struct ModuleScope {
    module: Rc<RefCell<IrModule>>,
    bindings: HashMap<String, SharedSlot>,
    // Map an import name to its source.
    imports: HashMap<String, Import>,
}

impl Scope for ModuleScope {
    fn add_local(&mut self, name: &String) -> SharedSlot {
        let slot = SharedSlot::new_static(name.clone());
        self.bindings.insert(name.clone(), slot.clone());
        slot
    }

    fn resolve(&mut self, name: &String) -> SharedSlot {
        if is_in_prelude(name) {
            self.imports.insert(
                name.to_owned(),
                Import::Named("prelude".to_owned(), name.to_owned()),
            );
            return SharedSlot::new_static(name.clone());
        }
        if let Some(slot) = self.bindings.get(name) {
            slot.clone()
        } else {
            // Automatically add it because it might be imported via
            // an `import *`.
            self.add_local(name)
        }
    }

    fn resolve_as_binding(&mut self, name: &String) -> SharedSlot {
        self.resolve(name)
    }
}

struct Compiler {
    module: Rc<RefCell<IrModule>>,
    current: SharedFunction,
}

impl Compiler {
    fn new() -> Self {
        let module = Rc::new(RefCell::new(IrModule::new()));
        let current = module.borrow().main_function();
        Self { module, current }
    }

    fn compile_module(&mut self, module: &Module) {
        let mut module_scope = ModuleScope {
            module: self.module.clone(),
            bindings: HashMap::new(),
            imports: HashMap::new(),
        };

        // We should start in the main function.
        assert_eq!(self.current, self.module.borrow().main_function());

        let mut scope = FunctionScope::new(&mut module_scope, self.current.clone());
        for node in module.nodes.iter() {
            self.compile_node(node, &mut scope);
            // We should always end up back in the main function.
            assert_eq!(self.current, self.module.borrow().main_function());
        }
        Compiler::finalize_function(self.current.clone(), &scope);

        // Now that we've visited the whole program we can write out the
        // imports we've found.
        self.module.borrow_mut().imports = module_scope.imports;
    }

    fn compile_node(&mut self, node: &Node, scope: &mut dyn Scope) -> SharedValue {
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

    fn compile_assignment(
        &mut self,
        assignment: &Assignment,
        scope: &mut dyn Scope,
    ) -> SharedValue {
        let lhs = &*assignment.lhs;
        let assignee = match &lhs {
            &Node::Identifier(identifier) => {
                let local = &identifier.value;
                scope.resolve(local)
            }
            _ => unreachable!("Cannot assign to: {:?}", lhs),
        };

        let rval = self.compile_node(&assignment.rhs, scope);
        self.build_set(assignee, rval.clone());
        rval
    }

    fn compile_anonymous_block(&mut self, block: &Block, scope: &mut dyn Scope) -> SharedValue {
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

    fn compile_function(&mut self, function: &Function, scope: &mut dyn Scope) -> SharedValue {
        let name = function.name.to_owned();
        let enclosing_function = self.current.clone();
        let new_function = match &name {
            Some(name) => self.module.borrow_mut().new_named_function(name.clone()),
            None => self
                .module
                .borrow_mut()
                .new_anonymous_function(enclosing_function.clone()),
        };
        self.current = new_function.clone();

        let slot = name.map(|name| scope.add_local(&name));
        let mut function_scope = FunctionScope::new(scope, self.current.clone());
        let body = &*function.body;
        let lval = self.compile_node(body, &mut function_scope);
        match body {
            Node::Block(_) => self.build_return_null(),
            _ => self.build_return(lval),
        };
        // Save the bindings so that we know how to build the closure.
        Compiler::finalize_function(new_function.clone(), &function_scope);

        self.current = enclosing_function;
        let lval = self.build_make_function(new_function);
        if let Some(slot) = slot {
            self.build_set(slot, lval.clone());
        };
        lval
    }

    fn finalize_function(function: SharedFunction, scope: &FunctionScope) {
        let mut function = function.borrow_mut();

        let mut unbound_locals = scope.locals.clone();
        for binding in scope.bindings.iter() {
            let slot = unbound_locals
                .remove(binding)
                .expect(&format!("Missing local for binding: {}", binding));
            slot.promote_from_local_to_lexical(binding.clone());
        }

        let mut locals = vec![];
        for (index, (name, slot)) in unbound_locals.iter().enumerate() {
            locals.push(name.clone());
            slot.set_local_index(index as u8);
        }

        function.locals = locals;
        function.bindings = scope.bindings.clone();
        function.parent_bindings = !scope.parent_bindings.is_empty();
    }

    fn compile_identifier(
        &mut self,
        identifier: &Identifier,
        scope: &mut dyn Scope,
    ) -> SharedValue {
        let local = &identifier.value;
        self.build_get(scope.resolve(local))
    }

    fn compile_integer(&mut self, integer: &Integer, _scope: &mut dyn Scope) -> SharedValue {
        self.build_make_integer(integer.value)
    }

    fn compile_postfix_call(&mut self, call: &PostfixCall, scope: &mut dyn Scope) -> SharedValue {
        let target = self.compile_node(&call.target, scope);
        let mut arguments = vec![];
        for argument in call.arguments.iter() {
            arguments.push(self.compile_node(argument, scope));
        }
        self.build_call(target, arguments)
    }

    fn compile_return(&mut self, ret: &Return, scope: &mut dyn Scope) -> SharedValue {
        if let Some(ref rhs) = ret.rhs {
            let rval = self.compile_node(&rhs, scope);
            self.build_return(rval);
        } else {
            self.build_return_null();
        }
        self.null_value()
    }

    fn compile_var(&mut self, var: &Var, scope: &mut dyn Scope) -> SharedValue {
        let slot = scope.add_local(&var.lhs.value);
        if let Some(ref rhs) = var.rhs {
            let rval = self.compile_node(rhs, scope);
            self.build_set(slot, rval);
        }
        self.null_value()
    }

    // Convenience proxy methods to the current function:

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

pub fn compile(module: &Module) -> IrModule {
    let mut compiler = Compiler::new();
    compiler.compile_module(module);
    // The reference count should be 1 at this point.
    let module_cell = Rc::try_unwrap(compiler.module).unwrap();
    module_cell.into_inner()
}