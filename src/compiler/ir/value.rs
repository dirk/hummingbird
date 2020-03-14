use std::cell::{Cell, Ref, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::super::super::type_ast::{self as ast, ScopeId};
use super::compile::{BasicBlock, BasicBlockManager, Buildable};
use super::frame::Frame;
use super::typ::{AbstractType, FuncPtrType, RealType, TupleType, Type};
use super::typer::Typer;
use super::{Container, Func, FuncParent, InnerFunc};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ValueId(usize);

impl ValueId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone)]
pub enum Value {
    Local(LocalValue),
    Static(StaticValue),
    Abstract(AbstractValue),
}

impl Value {
    pub fn is_real(&self) -> bool {
        match self {
            Value::Abstract(_) => false,
            _ => true,
        }
    }

    pub fn value_id(&self) -> ValueId {
        match self {
            Value::Local(local_value) => local_value.value_id(),
            other @ _ => unreachable!("Cannot get SSA Value ID for Static or Abstract Values"),
        }
    }

    pub fn typ(&self) -> Type {
        match self {
            Value::Local(local_value) => local_value.typ(),
            Value::Static(static_value) => match static_value {
                StaticValue::Func(func_value) => {
                    let parameters = func_value
                        .get_parameters()
                        .iter()
                        .map(|(name, typ)| typ.clone())
                        .collect::<Vec<_>>();
                    Type::Real(RealType::FuncPtr(FuncPtrType {
                        parameters,
                        retrn: Box::new(func_value.0.retrn.clone()),
                    }))
                }
            },
            Value::Abstract(abstract_value) => match abstract_value {
                AbstractValue::UnspecializedFunc(func) => {
                    Type::Abstract(AbstractType::UnspecializedFunc(func.clone()))
                }
                AbstractValue::SpecializableBuiltinFunc(_, retrn) => Type::Real(retrn.clone()),
            },
        }
    }
}

#[derive(Clone)]
pub enum LocalValue {
    FuncPtr(ValueId, FuncPtrType),
    Int64(ValueId, Option<u64>),
    Tuple(ValueId, TupleType),
}

impl LocalValue {
    pub fn value_id(&self) -> ValueId {
        match self {
            LocalValue::FuncPtr(id, _) => id.clone(),
            LocalValue::Int64(id, _) => id.clone(),
            LocalValue::Tuple(id, _) => id.clone(),
        }
    }

    pub fn typ(&self) -> Type {
        match self {
            LocalValue::FuncPtr(_, func_ptr_type) => {
                Type::Real(RealType::FuncPtr(func_ptr_type.clone()))
            }
            LocalValue::Int64(_, _) => Type::Real(RealType::Int64),
            LocalValue::Tuple(_, tuple_type) => Type::Real(RealType::Tuple(tuple_type.clone())),
        }
    }
}

#[derive(Clone)]
pub enum StaticValue {
    Func(FuncValue),
}

/// Sibling to `AbstractType`.
#[derive(Clone)]
pub enum AbstractValue {
    UnspecializedFunc(Func),
    /// A builtin func that can be dynamically specialized when generating
    /// target code.
    SpecializableBuiltinFunc(String, RealType),
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct FuncId(usize);

lazy_static! {
    static ref FUNC_ID: AtomicUsize = AtomicUsize::new(0);
}

/// Returns an ID that is guaranteed to be unique amongst all threads.
fn next_func_id() -> FuncId {
    FuncId(FUNC_ID.fetch_add(1, Ordering::SeqCst))
}

#[derive(Clone)]
pub struct FuncValue(Rc<InnerFuncValue>);

struct InnerFuncValue {
    id: FuncId,
    /// The fully-qualified unique name of this func.
    qualified_name: String,
    /// If this should be compiled as the main func.
    main: Cell<bool>,
    /// Used to resolve generics.
    typer: Typer,
    /// The func that this value is a specialization of.
    func: Weak<InnerFunc>,
    parameters: Vec<(String, RealType)>,
    retrn: RealType,
    /// The frame to be built on the stack for local variables.
    stack_frame: Vec<(String, RealType)>,
    // TODO: Heap frame for locals that are captured.
    /// Funcs declared within this func. This will produce a lot of bloat when
    /// nesting functions that don't depend on generics from their parents, but
    /// LLVM will merge duplicate functions for us.
    funcs: RefCell<Vec<Func>>,
    /// The actual body of the func; is `None` until it's actually compiled.
    basic_blocks: RefCell<Option<BasicBlockManager>>,
}

impl FuncValue {
    pub fn new(
        qualified_name: String,
        func: Func,
        parameters: Vec<RealType>,
        retrn: RealType,
    ) -> Self {
        let ast_func = &func.0.ast_func;
        let typer = Typer::new(ast_func.scope.id(), Some(func.get_parent().get_typer()));

        // Store the mappings of AST parameter and retrn types to the
        // corresponding real types in this specialization.
        let mut parameter_pairs = vec![];
        for (index, parameter) in parameters.into_iter().enumerate() {
            let ast_argument = &ast_func.arguments[index];
            typer
                .set_type(&ast_argument.typ, Type::Real(parameter.clone()))
                .expect("Parameter type mismatch");
            // Also save the names of the parameters so that we can do
            // index resolution when building instructions.
            parameter_pairs.push((ast_argument.name.clone(), parameter));
        }
        let ast_retrn = &ast_func.typ.unwrap_func().retrn.borrow();
        typer
            .set_type(ast_retrn, Type::Real(retrn.clone()))
            .expect("Return type mismatch");

        let mut stack_frame = vec![];
        let scope = ast_func.scope.unwrap_func();
        for (name, ast_typ) in scope.locals.iter() {
            // Skip funcs since they can't be built into real types.
            // TODO: Separate funcs from locals in `FuncScope`s.
            if ast_typ.is_func() {
                continue;
            }
            let typ = typer.build_type(ast_typ).into_real();
            stack_frame.push((name.clone(), typ));
        }

        Self(Rc::new(InnerFuncValue {
            id: next_func_id(),
            qualified_name,
            main: Cell::new(false),
            typer,
            func: Rc::downgrade(&func.0),
            parameters: parameter_pairs,
            retrn,
            stack_frame,
            funcs: RefCell::new(vec![]),
            // This is `None` since this func value hasn't actually be compiled yet.
            basic_blocks: RefCell::new(None),
        }))
    }

    pub fn id(&self) -> FuncId {
        self.0.id.clone()
    }

    pub fn get_qualified_name(&self) -> &str {
        &self.0.qualified_name
    }

    pub fn is_main(&self) -> bool {
        self.0.main.get()
    }

    pub fn set_main(&self, main: bool) {
        self.0.main.set(main)
    }

    pub fn get_parameters(&self) -> &Vec<(String, RealType)> {
        &self.0.parameters
    }

    pub fn get_retrn(&self) -> RealType {
        self.0.retrn.clone()
    }

    pub fn get_stack_frame(&self) -> &Vec<(String, RealType)> {
        &self.0.stack_frame
    }

    pub fn borrow_funcs(&self) -> Ref<Vec<Func>> {
        self.0.funcs.borrow()
    }

    /// Returns a `Some` if it created basic blocks; `None` if the basic
    /// blocks were already defined.
    pub fn create_basic_blocks(&self) -> Option<Ref<BasicBlockManager>> {
        if self.0.basic_blocks.borrow().is_some() {
            return None;
        }
        // Create a save the new basic blocks.
        {
            let mut basic_blocks = self.0.basic_blocks.borrow_mut();
            *basic_blocks = Some(BasicBlockManager::new());
        }
        let basic_blocks = Ref::map(self.0.basic_blocks.borrow(), |basic_blocks| {
            basic_blocks.as_ref().unwrap()
        });
        Some(basic_blocks)
    }

    pub fn borrow_basic_blocks(&self) -> Ref<BasicBlockManager> {
        Ref::map(self.0.basic_blocks.borrow(), |basic_blocks| {
            basic_blocks.as_ref().unwrap()
        })
    }
}

impl Buildable for FuncValue {
    /// Find a func defined within this func (including this func).
    fn find_func(&self, name: &str) -> Option<Func> {
        for func in self.0.funcs.borrow().iter() {
            if func.name() == name {
                return Some(func.clone());
            }
        }
        // Also support finding oneself for recursive functions.
        let self_func = Func::upgrade(&self.0.func).unwrap();
        if self_func.name() == name {
            return Some(self_func);
        }
        None
    }
}

impl Container for FuncValue {
    fn get_qualified_name(&self) -> String {
        self.0.qualified_name.clone()
    }

    fn get_typer(&self) -> Typer {
        self.0.typer.clone()
    }

    fn define_func(&self, ast_func: ast::Func) -> Func {
        let func = Func::new(ast_func, Box::new(self.clone()));
        let mut funcs = self.0.funcs.borrow_mut();
        funcs.push(func.clone());
        func
    }
}

impl Frame for FuncValue {
    fn get_local(&self, name: &str) -> (usize, RealType) {
        for (index, (slot_name, typ)) in self.0.stack_frame.iter().enumerate() {
            if name == slot_name {
                return (index, typ.clone());
            }
        }
        unreachable!("Local not found in FuncValue: {}", name)
    }

    fn get_static(&self, name: &str, scope_id: ScopeId) -> Value {
        let func = Func::upgrade(&self.0.func).unwrap();
        if func.scope_id() == scope_id {
            if let Some(func) = self.find_func(name) {
                return Value::Abstract(AbstractValue::UnspecializedFunc(func));
            }
            panic!("Static not found in FuncValue: {}", name)
        } else {
            func.get_parent().get_static(name, scope_id)
        }
    }
}

impl FuncParent for FuncValue {}
