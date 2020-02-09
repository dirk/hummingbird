use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::{Closable, Type, TypeError, TypeResult};

/// Proxy so that we can share different kinds of scopes.
#[derive(Clone, Debug)]
pub enum Scope {
    Func(Rc<RefCell<FuncScope>>),
    Module(Rc<RefCell<ModuleScope>>),
}

impl Scope {
    pub fn get_local(&self, name: &str) -> TypeResult<Type> {
        use Scope::*;
        match self {
            Func(func) => func.borrow_mut().get_local(name),
            Module(module) => module.borrow_mut().get_local(name),
        }
    }

    /// Called by a child scope to get its parent's local.
    ///
    ///     return self.parent.get_local_from_parent(name);
    ///
    pub fn get_local_from_parent(&self, name: &str) -> TypeResult<(Type, ParentResolution)> {
        use Scope::*;
        match self {
            Func(func) => func.borrow_mut().get_local_as_parent(name),
            Module(module) => module.borrow_mut().get_local_as_parent(name),
        }
    }

    pub fn add_local(&self, name: &str, typ: Type) -> TypeResult<()> {
        use Scope::*;
        match self {
            Func(func) => func.borrow_mut().add_local(name, typ),
            Module(module) => module.borrow_mut().add_local(name, typ),
        }
    }
}

impl Closable for Scope {
    fn close(self) -> TypeResult<Self> {
        use Scope::*;
        Ok(match self {
            Func(shared) => {
                let replacement = {
                    let func = shared.borrow();
                    let mut locals = HashMap::new();
                    for (name, typ) in func.locals.iter() {
                        locals.insert(name.clone(), typ.clone().close()?);
                    }
                    FuncScope {
                        locals,
                        parent: func.parent.clone(),
                        captures: func.captures,
                        captured: func.captured,
                        captured_locals: func.captured_locals.clone(),
                    }
                };
                shared.replace(replacement);
                Func(shared)
            }
            Module(shared) => {
                let replacement = {
                    let module = shared.borrow();
                    let mut statics = HashMap::new();
                    for (name, typ) in module.statics.iter() {
                        statics.insert(name.clone(), typ.clone().close()?);
                    }
                    ModuleScope {
                        statics,
                        captured_statics: module.captured_statics.clone(),
                    }
                };
                shared.replace(replacement);
                Module(shared)
            }
        })
    }
}

/// When getting locals from parent scopes we need to know if it was a local
/// that was ultimately read (need to set up closures) or if it was a static
/// (no need for closures).
pub enum ParentResolution {
    Local,
    Static,
}

pub trait ScopeLike {
    /// Consume oneself to produce a shareable `Scope`.
    fn into_scope(self) -> Scope;

    fn get_local(&mut self, name: &str) -> TypeResult<Type>;

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<(Type, ParentResolution)>;

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()>;
}

#[derive(Debug)]
pub struct FuncScope {
    locals: HashMap<String, Type>,
    parent: Option<Scope>,
    /// Whether or not this scope captures its parent scope.
    captures: bool,
    /// Whether or not this scope (or one of its parent scopes) is captured
    /// by child scopes (closures).
    captured: bool,
    /// Locals in this scope which are captured by closures.
    captured_locals: HashSet<String>,
}

impl FuncScope {
    pub fn new(parent: Option<Scope>) -> Self {
        Self {
            locals: HashMap::new(),
            parent,
            captures: false,
            captured: false,
            captured_locals: HashSet::new(),
        }
    }
}

impl ScopeLike for FuncScope {
    fn into_scope(self) -> Scope {
        Scope::Func(Rc::new(RefCell::new(self)))
    }

    fn get_local(&mut self, name: &str) -> Result<Type, TypeError> {
        if let Some(typ) = self.locals.get(name) {
            return Ok(typ.clone());
        }
        if let Some(parent) = &self.parent {
            return match parent.get_local_from_parent(name) {
                Ok((typ, resolution)) => {
                    match resolution {
                        // If the ultimate resolution was as a local variable
                        // then we need to mark ourselves as capturing our
                        // parent's scope.
                        ParentResolution::Local => self.captures = true,
                        _ => (),
                    }
                    Ok(typ)
                }
                Err(err) => Err(err),
            };
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<(Type, ParentResolution)> {
        if let Some(typ) = self.locals.get(name) {
            // If we found it in ourselves.
            self.captured = true;
            self.captured_locals.insert(name.to_string());
            return Ok((typ.clone(), ParentResolution::Local));
        }

        if let Some(parent) = &self.parent {
            return match parent.get_local_from_parent(name) {
                Ok((typ, resolution)) => {
                    match resolution {
                        // If the ultimate resolution was as a local variable
                        // then we need to mark ourselves as both captured and
                        // as capturing.
                        ParentResolution::Local => {
                            self.captures = true;
                            self.captured = true;
                        }
                        _ => (),
                    }
                    Ok((typ, resolution))
                }
                err @ Err(_) => err,
            };
        }

        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn add_local(&mut self, name: &str, typ: Type) -> Result<(), TypeError> {
        if self.locals.contains_key(name) {
            return Err(TypeError::LocalAlreadyDefined {
                name: name.to_string(),
            });
        }
        self.locals.insert(name.to_string(), typ);
        Ok(())
    }
}

#[derive(Debug)]
pub struct ModuleScope {
    statics: HashMap<String, Type>,
    captured_statics: HashSet<String>,
}

impl ModuleScope {
    pub fn new() -> Self {
        Self {
            statics: HashMap::new(),
            captured_statics: HashSet::new(),
        }
    }
}

impl ScopeLike for ModuleScope {
    fn into_scope(self) -> Scope {
        Scope::Module(Rc::new(RefCell::new(self)))
    }

    fn get_local(&mut self, name: &str) -> Result<Type, TypeError> {
        if let Some(typ) = self.statics.get(name) {
            return Ok(typ.clone());
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn get_local_as_parent(&mut self, name: &str) -> Result<(Type, ParentResolution), TypeError> {
        if let Some(typ) = self.statics.get(name) {
            self.captured_statics.insert(name.to_string());
            return Ok((typ.clone(), ParentResolution::Static));
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn add_local(&mut self, name: &str, typ: Type) -> Result<(), TypeError> {
        if self.statics.contains_key(name) {
            return Err(TypeError::LocalAlreadyDefined {
                name: name.to_string(),
            });
        }
        self.statics.insert(name.to_string(), typ);
        Ok(())
    }
}
