use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use super::super::super::type_ast::{self as ast};
use super::{FuncPtrType, IrError, RealType, TupleType, Type};

/// Used to apply generics and cache AST-type-to-IR-type translations.
#[derive(Clone)]
pub struct Typer(Rc<InnerTyper>);

struct InnerTyper {
    parent: Option<Typer>,
    types: RefCell<HashMap<ast::TypeId, Type>>,
}

impl Typer {
    pub fn new(parent: Option<Typer>) -> Self {
        Self(Rc::new(InnerTyper {
            parent,
            types: RefCell::new(HashMap::new()),
        }))
    }

    /// Searches itself and its parent for a type.
    fn lookup_type(&self, ast_type: &ast::Type) -> Option<Type> {
        let id = ast_type.id();
        if let Some(existing) = self.0.types.borrow().get(&id) {
            return Some(existing.clone());
        }
        if let Some(parent) = &self.0.parent {
            parent.lookup_type(ast_type)
        } else {
            // If we don't have a parent then we're the root type and we
            // can do a builtins lookup.
            Self::lookup_builtin(ast_type)
        }
    }

    fn lookup_builtin(ast_type: &ast::Type) -> Option<Type> {
        let intrinsic = match ast_type {
            ast::Type::Object(object) => match &object.class {
                ast::Class::Intrinsic(intrinsic) => intrinsic,
                ast::Class::Derived(_) => return None,
            },
            _ => return None,
        };
        BuiltinsCache::lookup_intrinsic(intrinsic)
    }

    pub fn build_type(&self, ast_type: &ast::Type) -> Type {
        if let Some(existing) = self.lookup_type(ast_type) {
            return existing;
        }
        let typ = {
            match ast_type {
                ast::Type::Tuple(ast_tuple) => {
                    let members = ast_tuple
                        .members
                        .iter()
                        .map(|ast_member| self.build_type(ast_member).into_real())
                        .collect::<Vec<_>>();
                    Type::Real(RealType::Tuple(TupleType::new(members)))
                }
                ast::Type::Func(func) => {
                    let parameters = func
                        .arguments
                        .borrow()
                        .iter()
                        .map(|argument| self.build_type(argument).into_real())
                        .collect::<Vec<_>>();
                    let retrn = self.build_type(&func.retrn.borrow()).into_real();
                    Type::Real(RealType::FuncPtr(FuncPtrType::new(parameters, retrn)))
                }
                other @ _ => unreachable!("Cannot build IR Type from AST Type: {:#?}", other),
            }
        };
        let id = ast_type.id();
        let mut types = self.0.types.borrow_mut();
        types.insert(id, typ.clone());
        typ
    }

    /// Used when building a func specialization to save the resolution of
    /// possibly-generic parameter types to their appropriate specialization.
    /// Also used by `compile_modules` to add entries for the builtin types.
    pub fn set_type(&self, ast_type: &ast::Type, typ: Type) -> Result<(), IrError> {
        let id = ast_type.id();
        {
            let types = self.0.types.borrow();
            if let Some(existing) = types.get(&id) {
                // Check if the type we've saved is the same as the one we're
                // trying to re-save. If they're not that means we have a
                // problem with type flow through the AST.
                if !existing.is_equal(&typ) {
                    return Err(IrError::TypeMismatch {
                        expected: existing.clone(),
                        got: typ,
                    });
                }
                // If they're equal then we don't need to re-save.
                return Ok(());
            }
        }
        let mut types = self.0.types.borrow_mut();
        types.insert(id, typ);
        Ok(())
    }
}

struct BuiltinsCache(Arc<InnerBuiltinsCache>);

struct InnerBuiltinsCache {
    /// Map the intrinsic class type IDs to real types.
    intrinsics: HashMap<ast::TypeId, Type>,
}

impl BuiltinsCache {
    #[allow(non_snake_case)]
    fn new() -> Self {
        let mut intrinsics = HashMap::new();

        let Int = ast::Builtins::get("Int");
        intrinsics.insert(Int.id(), Type::Real(RealType::Int64));

        // Check for completeness.
        let all = ast::Builtins::get_all();
        for (name, class) in all.iter() {
            if intrinsics.contains_key(&class.id()) {
                continue;
            }
            panic!("Builtin not mapped: {}({})", name, class.id());
        }

        BuiltinsCache(Arc::new(InnerBuiltinsCache { intrinsics }))
    }

    fn lookup_intrinsic(intrinsic: &Arc<ast::IntrinsicClass>) -> Option<Type> {
        let id = &intrinsic.id;
        BUILTINS_CACHE.0.intrinsics.get(id).map(|typ| typ.clone())
    }
}

lazy_static! {
    static ref BUILTINS_CACHE: BuiltinsCache = BuiltinsCache::new();
}
