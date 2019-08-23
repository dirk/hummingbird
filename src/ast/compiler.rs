use std::cell::{RefCell, Cell, RefMut};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::{Rc, Weak};

use super::super::ir::layout::{
    Address, Import, Instruction, InstructionBuilder, Module as IrModule, Name, SharedFunction, SharedValue,
    SharedName,
};
use super::super::vm::prelude::is_in_prelude;
use super::nodes::*;

#[derive(Clone, Debug)]
enum ScopeResolution {
    Seen(SharedName),
    Constant(String),
    NotFound(String),
}

impl ScopeResolution {
    fn shared_name(&self) -> SharedName {
        match self {
            ScopeResolution::Seen(shared_name) => shared_name.clone(),
            other @ _ => unreachable!("Not seen: {:?}", other),
        }
    }
}

trait Scope {
    fn add(&mut self, name: &String) -> SharedName;

    fn resolve(&mut self, name: &String) -> ScopeResolution;

    /// Acts the same as `resolve`, but records bindings along the way.
    fn resolve_as_binding(&mut self, name: &String) -> ScopeResolution;
}

struct FunctionScope<'a> {
    parent: &'a mut Scope,
    function: SharedFunction,
    // Keep track of every variable we've seen. We'll fill in the
    // `SharedName`s with details as we recurse upwards.
    locals: HashMap<String, ScopeResolution>,
    // Keep track of which variables need to be bound and come from parent
    // bindings.
    bindings: HashSet<String>,
    parent_bindings: HashSet<String>,
}

impl<'a> FunctionScope<'a> {
    fn new(parent: &'a mut Scope, function: SharedFunction) -> Self {
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
    fn add(&mut self, name: &String) -> SharedName {
        let shared_name = SharedName::unknown(name.clone());
        let resolution = ScopeResolution::Seen(shared_name.clone());
        self.locals.insert(name.clone(), resolution.clone());
        shared_name
    }

    fn resolve(&mut self, name: &String) -> ScopeResolution {
        if let Some(resolution) = self.locals.get(name) {
            resolution.clone()
        } else {
            self.parent_bindings.insert(name.clone());
            self.parent.resolve_as_binding(name)
        }
    }

    fn resolve_as_binding(&mut self, name: &String) -> ScopeResolution {
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
    // Map an import name to its source.
    imports: HashMap<String, Import>,
}

impl Scope for ModuleScope {
    fn add(&mut self, name: &String) -> SharedName {
        unreachable!("Cannot add to module scope")
    }

    fn resolve(&mut self, name: &String) -> ScopeResolution {
        if is_in_prelude(name) {
            self.imports.insert(
                name.to_owned(),
                Import::Named("prelude".to_owned(), name.to_owned()),
            );
            ScopeResolution::Constant(name.clone())
        } else {
            ScopeResolution::NotFound(name.clone())
        }
    }

    fn resolve_as_binding(&mut self, name: &String) -> ScopeResolution {
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

        // Now that we've visited the whole program we can write out the
        // imports we've found.
        self.module.borrow_mut().imports = module_scope.imports;
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

        let assignee = match lhs {
            &Node::Identifier(ref identifier) => {
                let local = &identifier.value;
                match scope.resolve(local) {
                    ScopeResolution::Seen(name) => name,
                    other @ _ => unreachable!("Cannot assign to: {:?}", other),
                }
            }
            _ => unreachable!("Cannot assign to: {:?}", lhs),
        };

        let rval = self.compile_node(assignment.rhs.deref(), scope);
        self.build_set(assignee, rval.clone());
        rval
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
        let enclosing_function = self.current.clone();
        let new_function = match &name {
            Some(name) => self.module.borrow_mut().new_named_function(name.clone()),
            None => self.module.borrow_mut().new_anonymous_function(enclosing_function.clone()),
        };
        self.current = new_function.clone();

        if let Some(name) = &name {
            scope.add(name);
        }
        let mut function_scope = FunctionScope::new(scope, self.current.clone());
        let body = &*function.body;
        let lval = self.compile_node(body, &mut function_scope);
        match body {
            Node::Block(_) => self.build_return_null(),
            _ => self.build_return(lval),
        };
        // Save the bindings so that we know how to build the closure.
        Compiler::finalize_scope(new_function.clone(), &function_scope);
        {
            let mut borrowed_new_function = new_function.borrow_mut();
            borrowed_new_function.bindings = function_scope.bindings;
        }

        self.current = enclosing_function;
        let lval = self.build_make_function(new_function);
        if let Some(name) = &function.name {
            let name = scope.add(name);
            self.build_set(name, lval.clone());
        };
        lval
    }

    fn finalize_scope(function: SharedFunction, scope: &FunctionScope) {
        let mut function = function.borrow_mut();

        let mut unbound_locals = scope.locals.clone();
        for binding in scope.bindings.iter() {
            unbound_locals.remove(binding);
        }

        let mut locals = vec![];
        for (index, (name, resolution)) in unbound_locals.iter().enumerate() {
            locals.push(name.clone());
            let shared_name = resolution.shared_name();
            shared_name.set(Name::Local(index as u8));
        }

        function.locals = locals;
        function.bindings = scope.bindings.clone();
    }

    fn compile_identifier(&mut self, identifier: &Identifier, scope: &mut Scope) -> SharedValue {
        let local = &identifier.value;
        let resolution = scope.resolve(local);
        match resolution {
            ScopeResolution::Seen(shared_name) => self.build_get(shared_name),
            ScopeResolution::Constant(name) => self.build_get_constant(name),
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
        let shared_name = scope.add(&var.lhs.value);
        if let Some(ref rhs) = var.rhs {
            let rval = self.compile_node(rhs.deref(), scope);
            self.build_set(shared_name, rval);
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
