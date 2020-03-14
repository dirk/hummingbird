use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::super::type_ast::{self as ast, ScopeId};
use super::vecs_equal::vecs_equal;

mod compile;
mod error;
mod frame;
mod typ;
mod typer;
mod value;

use frame::Frame;
use typ::*;
use typer::Typer;

pub use compile::{compile_modules, Instruction};
pub use error::IrError;
pub use typ::RealType;
pub use value::{AbstractValue, FuncId, FuncValue, LocalValue, StaticValue, Value, ValueId};

/// Something which:
///   - Has a fully-qualified name.
///   - Can have funcs defined within it.
///   - Has a typer to resolve generics.
///
/// The current containers are `Module`s and `Func`s.
trait Container {
    fn get_qualified_name(&self) -> String;

    fn get_typer(&self) -> Typer;

    /// Add an unspecialized func to the container.
    fn define_func(&self, ast_func: ast::Func) -> Func;
}

#[derive(Clone)]
pub struct Root(Rc<InnerRoot>);

struct InnerRoot {
    modules: RefCell<Vec<Module>>,
    typer: Typer,
    statics: RefCell<HashMap<String, Value>>,
}

impl Root {
    fn new(typer: Typer) -> Self {
        Self(Rc::new(InnerRoot {
            modules: RefCell::new(vec![]),
            typer,
            statics: RefCell::new(HashMap::new()),
        }))
    }

    fn add_module(
        &self,
        id: usize,
        qualified_name: String,
        ast_module: Ref<ast::Module>,
    ) -> Module {
        let module = Module(Rc::new(InnerModule {
            id,
            qualified_name,
            parent: self.clone(),
            typer: Typer::new(ast_module.scope.id(), Some(self.0.typer.clone())),
            scope_id: ast_module.scope.id(),
            funcs: RefCell::new(vec![]),
        }));
        let mut modules = self.0.modules.borrow_mut();
        modules.push(module.clone());
        module
    }

    fn borrow_modules(&self) -> Ref<Vec<Module>> {
        self.0.modules.borrow()
    }

    pub fn get_modules(&self) -> Vec<Module> {
        self.0.modules.borrow().clone()
    }

    fn set_static(&self, name: &str, value: Value) {
        let mut statics = self.0.statics.borrow_mut();
        statics.insert(name.to_string(), value);
    }

    fn find_module_by_id(&self, id: usize) -> Option<Module> {
        let modules = self.0.modules.borrow();
        modules
            .iter()
            .find(|module| module.0.id == id)
            .map(|module| module.clone())
    }
}

impl Frame for Root {
    fn get_local(&self, name: &str) -> (usize, RealType) {
        panic!("Cannot get_local on Root")
    }

    fn get_static(&self, name: &str, scope_id: ScopeId) -> Value {
        if !scope_id.is_builtin() {
            panic!(
                "Exhausted frames searching for static: name: {}, scope_id: {:?}",
                name, scope_id
            )
        }
        let statics = self.0.statics.borrow();
        for (static_name, value) in statics.iter() {
            if static_name == name {
                return value.clone();
            }
        }
        panic!("Static not found: {}", name)
    }
}

#[derive(Clone)]
pub struct Module(Rc<InnerModule>);

struct InnerModule {
    /// The unique module identifier from the frontend.
    id: usize,
    qualified_name: String,
    parent: Root,
    typer: Typer,
    /// The scope ID for this module's root scope from the AST. We need this
    /// to precisely look up statics from `ScopeResolution`s.
    scope_id: ScopeId,
    funcs: RefCell<Vec<Func>>,
}

impl Module {
    fn find_func_by_name(&self, name: &str) -> Option<Func> {
        let funcs = self.0.funcs.borrow();
        funcs
            .iter()
            .find(|func| func.name() == name)
            .map(|func| func.clone())
    }

    pub fn borrow_funcs(&self) -> Ref<Vec<Func>> {
        self.0.funcs.borrow()
    }
}

impl Container for Module {
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

impl Frame for Module {
    fn get_local(&self, name: &str) -> (usize, RealType) {
        panic!("Cannot get_local on Module")
    }

    fn get_static(&self, name: &str, scope_id: ScopeId) -> Value {
        if self.0.scope_id == scope_id {
            // Current only funcs are supported as statics.
            let funcs = self.0.funcs.borrow();
            for func in funcs.iter() {
                if func.name() == name {
                    return Value::Abstract(AbstractValue::UnspecializedFunc(func.clone()));
                }
            }
            panic!("Static not found in Module: {}", name)
        } else {
            self.0.parent.get_static(name, scope_id)
        }
    }
}

impl FuncParent for Module {}

/// AbstractType::UnspecializedFunc -> Func -> FuncValue
#[derive(Clone)]
pub struct Func(Rc<InnerFunc>);

impl Func {
    /// Two `Func`s are equal if and only if they are same `Rc`.
    pub fn is_equal(&self, other: &Func) -> bool {
        let self_ptr = &self.0 as *const Rc<InnerFunc>;
        let other_ptr = &other.0 as *const Rc<InnerFunc>;
        self_ptr == other_ptr
    }
}

trait FuncParent: Container + Frame {}

struct InnerFunc {
    /// The parent that this func is declared in; should be one of:
    ///   - `Module` for root-level funcs.
    ///   - `FuncValue` for a func nested within another func.
    parent: Box<dyn FuncParent>,
    specializations: RefCell<Vec<FuncValue>>,
    ast_func: ast::Func,
}

impl Func {
    fn new(ast_func: ast::Func, parent: Box<dyn FuncParent>) -> Self {
        Self(Rc::new(InnerFunc {
            parent,
            specializations: RefCell::new(vec![]),
            ast_func,
        }))
    }

    fn upgrade(weak: &Weak<InnerFunc>) -> Option<Func> {
        weak.upgrade().map(|inner| Self(inner))
    }

    fn get_or_insert_specialization(
        &self,
        parameters: Vec<RealType>,
        retrn: RealType,
    ) -> FuncValue {
        // Search for an existing matching specialization first.
        {
            let specializations = self.borrow_specializations();
            for specialization in specializations.iter() {
                let existing_parameters = specialization
                    .get_parameters()
                    .iter()
                    .map(|(name, typ)| typ.clone())
                    .collect::<Vec<_>>();
                let parameters_match =
                    vecs_equal(&existing_parameters, &parameters, RealType::is_equal);
                // If the parameters and return match then we have a usable
                // existing specialization.
                let existing_retrn = specialization.get_retrn();
                if parameters_match && existing_retrn.is_equal(&retrn) {
                    return specialization.clone();
                }
            }
        }
        let qualified_name = format!(
            "{}_{}{}",
            self.0.parent.get_qualified_name(),
            self.name(),
            self.0.specializations.borrow().len(),
        );
        let func = FuncValue::new(qualified_name, self.clone(), parameters, retrn);
        // Save the specialization for future reference.
        let mut specializations = self.0.specializations.borrow_mut();
        specializations.push(func.clone());
        func
    }

    fn name(&self) -> &str {
        &self.0.ast_func.name
    }

    fn scope_id(&self) -> ScopeId {
        self.0.ast_func.scope.id()
    }

    fn get_parent(&self) -> &Box<dyn FuncParent> {
        &self.0.parent
    }

    pub fn borrow_specializations(&self) -> Ref<Vec<FuncValue>> {
        self.0.specializations.borrow()
    }
}
