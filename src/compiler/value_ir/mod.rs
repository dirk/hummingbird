use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::super::type_ast::{self as ast};
use super::vecs_equal::vecs_equal;

mod compile;
mod typ;
mod typer;
mod value;

pub use compile::{compile_modules, Instruction};
pub use typ::RealType;
use typ::*;
use typer::Typer;
pub use value::{FuncId, FuncValue, LocalValue, StaticValue, Value, ValueId};

/// A type which:
///   - Has a fully-qualified name.
///   - Can have funcs defined within it.
///   - Has a typer to resolve generics.
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
}

impl Root {
    fn new(typer: Typer) -> Self {
        Self(Rc::new(InnerRoot {
            modules: RefCell::new(vec![]),
            typer,
        }))
    }

    fn add_module(&self, id: usize, qualified_name: String) -> Module {
        let module = Module(Rc::new(InnerModule {
            id,
            qualified_name,
            parent: self.clone(),
            typer: Typer::new(Some(self.0.typer.clone())),
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

    fn find_module_by_id(&self, id: usize) -> Option<Module> {
        let modules = self.0.modules.borrow();
        modules
            .iter()
            .find(|module| module.0.id == id)
            .map(|module| module.clone())
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

/// UnspecializedFuncType -> Func -> FuncValue
#[derive(Clone)]
pub struct Func(Rc<InnerFunc>);

struct InnerFunc {
    /// The parent that this func is declared in; should be one of:
    ///   - `Module` for root-level funcs.
    ///   - `FuncValue` for a func nested within another func.
    parent: Box<dyn Container>,
    specializations: RefCell<Vec<FuncValue>>,
    ast_func: ast::Func,
}

impl Func {
    fn new(ast_func: ast::Func, parent: Box<dyn Container>) -> Self {
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

    fn arity(&self) -> usize {
        self.0.ast_func.arguments.len()
    }

    fn get_parent_typer(&self) -> Typer {
        self.0.parent.get_typer()
    }

    pub fn borrow_specializations(&self) -> Ref<Vec<FuncValue>> {
        self.0.specializations.borrow()
    }
}
