use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::ops::Range;
use std::rc::Rc;

use super::ast::nodes::*;
use super::ir::layout::{
    Address, Instruction, InstructionBuilder, Module as IrModule, SharedFunction, SharedSlot,
    SharedValue,
};
use super::parser::Span;

#[derive(PartialEq)]
enum ScopeFlags {
    /// Evaluate scoping rules normally.
    None,
    /// Evaluate scoping rules for use in a REPL (ie. promote all locals
    /// to statics).
    Repl,
}

impl Into<ScopeFlags> for CompilationFlags {
    fn into(self) -> ScopeFlags {
        match self {
            CompilationFlags::None => ScopeFlags::None,
            CompilationFlags::Repl => ScopeFlags::Repl,
        }
    }
}

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
    parent_bindings: HashMap<String, SharedSlot>,
}

impl<'a> FunctionScope<'a> {
    fn new(parent: &'a mut dyn Scope, function: SharedFunction) -> Self {
        Self {
            parent,
            function,
            locals: HashMap::new(),
            bindings: HashSet::new(),
            parent_bindings: HashMap::new(),
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
        if let Some(resolved) = self.locals.get(name) {
            resolved.clone()
        } else {
            let slot = self.parent.resolve_as_binding(name);
            self.parent_bindings.insert(name.clone(), slot.clone());
            slot
        }
    }

    fn resolve_as_binding(&mut self, name: &String) -> SharedSlot {
        if let Some(resolved) = self.locals.get(name) {
            // Since this was called from a lower function scope we now know
            // we need to bind it for that lower scope.
            self.bindings.insert(name.clone());
            resolved.clone()
        } else {
            let slot = self.parent.resolve_as_binding(name);
            self.parent_bindings.insert(name.clone(), slot.clone());
            slot
        }
    }
}

struct ModuleScope {
    module: Rc<RefCell<IrModule>>,
    bindings: HashMap<String, SharedSlot>,
}

impl Scope for ModuleScope {
    fn add_local(&mut self, name: &String) -> SharedSlot {
        let slot = SharedSlot::new_static(name.clone());
        self.bindings.insert(name.clone(), slot.clone());
        slot
    }

    fn resolve(&mut self, name: &String) -> SharedSlot {
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

    fn compile_module(&mut self, module: &Module, flags: CompilationFlags) {
        let mut module_scope = ModuleScope {
            module: self.module.clone(),
            bindings: HashMap::new(),
        };

        // We should start in the main function.
        assert_eq!(self.current, self.module.borrow().main_function());

        let mut scope = FunctionScope::new(&mut module_scope, self.current.clone());
        let mut implicit_return = self.null_value();
        for node in module.nodes.iter() {
            implicit_return = self.compile_node(node, &mut scope);
            // We should always end up back in the main function.
            assert_eq!(self.current, self.module.borrow().main_function());
        }
        match flags {
            CompilationFlags::Repl => {
                // If compiling for a REPL then we want to implicit return the
                // last statement.
                self.build_return(implicit_return);
            }
            _ => {
                // Insert a return at the end to make sure empty modules don't crash.
                self.build_return_null();
            }
        };
        Compiler::finalize_function(self.current.clone(), &scope, flags.into());
    }

    fn compile_node(&mut self, node: &Node, scope: &mut dyn Scope) -> SharedValue {
        match node {
            &Node::Assignment(ref assignment) => self.compile_assignment(assignment, scope),
            &Node::Block(ref block) => self.compile_anonymous_block(block, scope),
            &Node::Export(ref export) => self.compile_export(export, scope),
            &Node::Function(ref function) => self.compile_function(function, scope),
            &Node::Identifier(ref identifier) => self.compile_identifier(identifier, scope),
            &Node::If(ref if_) => self.compile_if(if_, scope),
            &Node::Import(ref import) => self.compile_import(import, scope),
            &Node::Infix(ref infix) => self.compile_infix(infix, scope),
            &Node::Integer(ref integer) => self.compile_integer(integer, scope),
            &Node::PostfixCall(ref call) => self.compile_postfix_call(call, scope),
            &Node::PostfixProperty(ref property) => self.compile_postfix_property(property, scope),
            &Node::Return(ref ret) => self.compile_return(ret, scope),
            &Node::String(ref string_literal) => self.compile_string(string_literal, scope),
            &Node::Symbol(ref symbol_literal) => self.compile_symbol(symbol_literal, scope),
            &Node::Var(ref var) => self.compile_var(var, scope),
            &Node::While(ref while_) => self.compile_while(while_, scope),
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
        self.current
            .borrow_mut()
            .push_basic_block("anonymous", true);

        let mut implicit_return = self.null_value();
        for node in block.nodes.iter() {
            implicit_return = self.compile_node(node, scope);
        }

        // Exit from the current block to the new block.
        self.current
            .borrow_mut()
            .push_basic_block("anonymous", true);

        implicit_return
    }

    fn compile_export(&mut self, export: &Export, scope: &mut dyn Scope) -> SharedValue {
        for identifier in export.identifiers.iter() {
            let rval = self.compile_identifier(identifier, scope);
            let name = identifier.value.clone();
            self.build_export(name, rval);
        }
        self.null_value()
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
        let lval = match body {
            // If it's a block then compile all the nodes ourselves. This
            // avoids creating the additional inner and successor blocks that
            // `compile_anonymous_block` generates.
            Node::Block(block) => {
                let mut implicit_return = self.null_value();
                for node in block.nodes.iter() {
                    implicit_return = self.compile_node(node, &mut function_scope);
                }
                implicit_return
            }
            other @ _ => self.compile_node(other, &mut function_scope),
        };
        self.build_return(lval);
        // Save the bindings so that we know how to build the closure.
        Compiler::finalize_function(new_function.clone(), &function_scope, ScopeFlags::None);

        self.current = enclosing_function;
        let lval = self.build_make_function(new_function);
        if let Some(slot) = slot {
            self.build_set(slot, lval.clone());
        };
        lval
    }

    fn finalize_function(function: SharedFunction, scope: &FunctionScope, flags: ScopeFlags) {
        let mut function = function.borrow_mut();

        // If we're in a REPL then promote everything to lexical and always
        // capture our parent bindings.
        if flags == ScopeFlags::Repl {
            for (name, slot) in scope.locals.iter() {
                slot.promote_from_local_to_lexical(name.clone());
            }
            function.locals = vec![];
            function.bindings = scope.bindings.clone();
            function.parent_bindings = true;
            return;
        }

        let mut all_locals = scope.locals.clone();
        // The bindings will have been filled in by nested functions within
        // this function. We therefore now need to remove any local that's
        // been bound and promote it to lexical.
        for binding in scope.bindings.iter() {
            let slot = all_locals
                .remove(binding)
                .expect(&format!("Missing local for binding: {}", binding));
            slot.promote_from_local_to_lexical(binding.clone());
        }
        // Remaining entries in `all_locals` will now only be unbound locals
        // (ie. ones that won't be captured in a closure).
        let unbound_locals = all_locals;

        let mut locals = vec![];
        for (index, (name, slot)) in unbound_locals.iter().enumerate() {
            locals.push(name.clone());
            slot.set_local_index(index as u8);
        }

        function.locals = locals;
        function.bindings = scope.bindings.clone();
        // We only need to capture our parent's bindings if we use non-static
        // ones (eg. lexical slots or local slots that will be promoted
        // to lexical).
        function.parent_bindings = scope.parent_bindings.values().any(|slot| !slot.is_static());
    }

    fn compile_identifier(
        &mut self,
        identifier: &Identifier,
        scope: &mut dyn Scope,
    ) -> SharedValue {
        self.set_mappings(identifier.span.clone(), |this| {
            let local = &identifier.value;
            this.build_get(scope.resolve(local))
        })
    }

    fn compile_if(&mut self, if_: &If, scope: &mut dyn Scope) -> SharedValue {
        // Make a block for the condition and branch to it.
        let condition_block = self.current.borrow_mut().push_basic_block(true);
        // The block to evaluate if the condition is true.
        let true_block = self.current.borrow_mut().push_basic_block(false);
        // The block after the statement.
        let successor_block = self.current.borrow_mut().push_basic_block(false);

        // Temporary local variable to hold the result of the branches.
        let lval = scope.add_local(&format!(".{}", condition_block.borrow().name));

        self.current
            .borrow_mut()
            .set_current_basic_block(condition_block.clone());
        let condition_value = self.compile_node(&if_.condition, scope);
        self.build_branch_if(true_block.clone(), condition_value);
        // TODO: Implement else and else-if.
        let rhs = self.null_value();
        self.build_set(lval.clone(), rhs);
        self.build_branch(successor_block.clone());

        self.current
            .borrow_mut()
            .set_current_basic_block(true_block.clone());
        let mut lhs = self.null_value();
        for node in if_.block.nodes.iter() {
            lhs = self.compile_node(node, scope);
        }
        self.build_set(lval.clone(), lhs);
        // After executing the block go to the successor.
        self.build_branch(successor_block.clone());

        self.current
            .borrow_mut()
            .set_current_basic_block(successor_block.clone());
        self.build_get(lval)
    }

    fn compile_import(&mut self, import: &Import, _scope: &mut dyn Scope) -> SharedValue {
        self.set_mappings(import.span.clone(), |this| {
            match &import.bindings {
                ImportBindings::Module => {
                    let alias = import
                        .path()
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .expect("Couldn't get file name of import")
                        .to_owned();
                    this.build_import(alias, import.name.clone());
                }
                other @ _ => println!("Cannot compile import binding: {:?}", other),
            };
            this.null_value()
        })
    }

    fn compile_infix(&mut self, infix: &Infix, scope: &mut dyn Scope) -> SharedValue {
        let lhs = self.compile_node(&infix.lhs, scope);
        let rhs = self.compile_node(&infix.rhs, scope);
        match &infix.op {
            &InfixOp::Add => self.build_op_add(lhs, rhs),
            &InfixOp::Equality => self.build_op_equality(lhs, rhs),
            &InfixOp::LessThan => self.build_op_less_than(lhs, rhs),
            other @ _ => panic!("Cannot compile infix op: {:?}", other),
        }
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

    fn compile_postfix_property(
        &mut self,
        property: &PostfixProperty,
        scope: &mut dyn Scope,
    ) -> SharedValue {
        let target = self.compile_node(&property.target, scope);
        self.set_mappings(property.span.clone(), |this| {
            this.build_op_property(target, property.value.clone())
        })
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

    fn compile_string(
        &mut self,
        string_literal: &StringLiteral,
        _scope: &mut dyn Scope,
    ) -> SharedValue {
        self.build_make_string(string_literal.value.clone())
    }

    fn compile_symbol(
        &mut self,
        symbol_literal: &SymbolLiteral,
        _scope: &mut dyn Scope,
    ) -> SharedValue {
        self.build_make_symbol(symbol_literal.value.clone())
    }

    fn compile_var(&mut self, var: &Var, scope: &mut dyn Scope) -> SharedValue {
        let slot = scope.add_local(&var.lhs.value);
        if let Some(ref rhs) = var.rhs {
            let rval = self.compile_node(rhs, scope);
            self.build_set(slot, rval);
        }
        self.null_value()
    }

    fn compile_while(&mut self, while_: &While, scope: &mut dyn Scope) -> SharedValue {
        // Make a block for the condition and branch to it.
        let condition_block = self
            .current
            .borrow_mut()
            .push_basic_block("while.condition", true);
        // The block for the loop to run in.
        let loop_block = self
            .current
            .borrow_mut()
            .push_basic_block("while.loop", false);
        // The block after the while.
        let successor_block = self
            .current
            .borrow_mut()
            .push_basic_block("while.successor", false);

        self.current
            .borrow_mut()
            .set_current_basic_block(condition_block.clone());
        let condition_value = self.compile_node(&while_.condition, scope);
        // If it's true branch to the loop block, else branch to the after block.
        self.build_branch_if(loop_block.clone(), condition_value);
        self.build_branch(successor_block.clone());

        self.current
            .borrow_mut()
            .set_current_basic_block(loop_block.clone());
        for node in while_.block.nodes.iter() {
            self.compile_node(node, scope);
        }
        // After executing the block go back to the condition.
        self.build_branch(condition_block.clone());

        self.current
            .borrow_mut()
            .set_current_basic_block(successor_block.clone());
        self.null_value()
    }

    // Convenience proxy methods to the current function:

    fn null_value(&self) -> SharedValue {
        self.current.borrow().null_value()
    }

    fn address(&self) -> Address {
        self.current.borrow().address()
    }

    fn add_mapping(&self, address: Address, span: Span) {
        self.current.borrow_mut().add_mapping(address, span);
    }

    fn set_mappings<T, F: FnOnce(&mut Compiler) -> T>(&mut self, span: Span, f: F) -> T {
        let start = self.address();

        let value = f(self);

        let range = Range {
            start,
            end: self.address(),
        };
        for address in range {
            self.add_mapping(address, span.clone());
        }
        value
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

pub enum CompilationFlags {
    None,
    Repl,
}

impl Default for CompilationFlags {
    fn default() -> Self {
        Self::None
    }
}

pub fn compile(module: &Module, flags: CompilationFlags) -> IrModule {
    let mut compiler = Compiler::new();
    compiler.compile_module(module, flags);
    // The reference count should be 1 at this point.
    let module_cell = Rc::try_unwrap(compiler.module).unwrap();
    module_cell.into_inner()
}
