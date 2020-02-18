use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Error, Formatter};
use std::rc::Rc;

use super::{Closable, RecursionTracker, Type, TypeError, TypeResult};

/// Proxy so that we can share different kinds of scopes.
#[derive(Clone, Debug)]
pub enum Scope {
    Closure(Rc<RefCell<ClosureScope>>),
    Func(Rc<RefCell<FuncScope>>),
    Module(Rc<RefCell<ModuleScope>>),
}

impl Scope {
    pub fn get_local(&self, name: &str) -> TypeResult<ScopeResolution> {
        use Scope::*;
        match self {
            Closure(closure) => closure.borrow_mut().get_local(name),
            Func(func) => func.borrow_mut().get_local(name),
            Module(module) => module.borrow_mut().get_local(name),
        }
    }

    /// Called by a child scope to get its parent's local.
    ///
    ///     return self.parent.get_local_from_parent(name);
    ///
    pub fn get_local_from_parent(&self, name: &str) -> TypeResult<ScopeResolution> {
        use Scope::*;
        let resolution = match self {
            Closure(closure) => closure.borrow_mut().get_local_as_parent(name),
            Func(func) => func.borrow_mut().get_local_as_parent(name),
            Module(module) => module.borrow_mut().get_local_as_parent(name),
        };
        // Add ourselves (the parent) onto the end of the scope chain.
        resolution.map(|resolution| resolution.add_scope(self.clone()))
    }

    pub fn add_local(&self, name: &str, typ: Type) -> TypeResult<()> {
        use Scope::*;
        match self {
            Closure(closure) => closure.borrow_mut().add_local(name, typ),
            Func(func) => func.borrow_mut().add_local(name, typ),
            Module(module) => module.borrow_mut().add_local(name, typ),
        }
    }

    fn get_parent(&self) -> Option<Scope> {
        use Scope::*;
        match self {
            Closure(closure) => closure.borrow().get_parent(),
            Func(func) => func.borrow().get_parent(),
            Module(module) => module.borrow().get_parent(),
        }
    }

    // Returns true if the other scope is a parent (or parent's parent, etc.)
    // of this scope. Also returns true if they're the same scope.
    pub fn within(&self, other: &Scope) -> bool {
        let mut parent = self.clone();
        loop {
            if &parent == other {
                return true;
            }
            parent = if let Some(next) = parent.get_parent() {
                next
            } else {
                return false;
            }
        }
    }
}

impl Closable for Scope {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use Scope::*;
        Ok(match self {
            Closure(shared) => {
                let replacement = {
                    let closure = shared.borrow();
                    let mut locals = HashMap::new();
                    for (name, typ) in closure.locals.iter() {
                        locals.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
                    }
                    ClosureScope {
                        locals,
                        parent: closure.parent.clone(),
                        captures: closure.captures,
                        captured: closure.captured,
                        captured_locals: closure.captured_locals.clone(),
                    }
                };
                shared.replace(replacement);
                Closure(shared)
            }
            Func(shared) => {
                let replacement = {
                    let func = shared.borrow();
                    let mut locals = HashMap::new();
                    for (name, typ) in func.locals.iter() {
                        locals.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
                    }
                    FuncScope {
                        locals,
                        parent: func.parent.clone(),
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
                        statics.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
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

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        use Scope::*;
        match (self, other) {
            (Closure(self_func), Closure(other_func)) => self_func.as_ptr() == other_func.as_ptr(),
            (Module(self_module), Module(other_module)) => {
                self_module.as_ptr() == other_module.as_ptr()
            }
            _ => false,
        }
    }
}

/// When getting locals from parent scopes we need to know how the local
/// was read:
///   - Local: in the current scope
///   - Closure: in a higher up func and/or block scope
///   - Static
#[derive(Debug, PartialEq)]
pub enum ScopeResolution {
    Local(String, Type),
    /// A local that was found in a func and/or block scope above the current
    /// scope. The `Vec<Scope>` lists the chain of scopes traversed: the first
    /// is the highest/farthest and the last is the lowest/nearest to the
    /// current scope.
    Closure(String, Type, Vec<Scope>),
    // TODO: Add the module, import, or class the static was found on.
    Static(String, Type),
}

impl ScopeResolution {
    pub fn name(&self) -> String {
        use ScopeResolution::*;
        match self {
            Local(name, _) | Closure(name, _, _) | Static(name, _) => name.clone(),
        }
    }

    pub fn typ(&self) -> Type {
        use ScopeResolution::*;
        match self {
            Local(_, typ) | Closure(_, typ, _) | Static(_, typ) => typ.clone(),
        }
    }

    /// Add a scope to the end of the scope chain if it's a closure resolution,
    /// otherwise this is a no-op identity function.
    pub fn add_scope(self, scope: Scope) -> Self {
        use ScopeResolution::*;
        match self {
            Closure(name, typ, scopes) => {
                let mut new_scopes = scopes.clone();
                new_scopes.push(scope);
                Closure(name, typ, new_scopes)
            }
            other @ _ => other,
        }
    }

    /// Ensure that the resolution is not a `Closure`; will return an `Err` if
    /// it is. Useful with `Result#and_then`:
    ///
    ///     resolution.and_then(ScopeResolution::disallow_closure)
    ///
    fn disallow_closure(resolution: Self) -> TypeResult<Self> {
        use ScopeResolution::*;
        match resolution {
            Closure(name, _, _) => Err(TypeError::CannotCapture {
                name: name.to_string(),
            }),
            other @ _ => Ok(other),
        }
    }

    /// Useful for ensuring that the resolution is not `Local`.
    fn assert_not_local(resolution: Self) -> TypeResult<Self> {
        use ScopeResolution::*;
        match resolution {
            local @ Local(_, _) => panic!("Unexpected local resolution: {:?}", local),
            other @ _ => Ok(other),
        }
    }
}

impl Closable for ScopeResolution {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use ScopeResolution::*;
        Ok(match self {
            Local(name, typ) => Local(name, typ.close(tracker, scope)?),
            Closure(name, typ, chain) => Closure(name, typ.close(tracker, scope)?, chain),
            Static(name, typ) => Static(name, typ.close(tracker, scope)?),
        })
    }
}

pub trait ScopeLike {
    /// Consume oneself to produce a shareable `Scope`.
    fn into_scope(self) -> Scope;

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution>;

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<ScopeResolution>;

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()>;

    fn get_parent(&self) -> Option<Scope>;
}

pub struct ClosureScope {
    pub locals: HashMap<String, Type>,
    parent: Option<Scope>,
    /// Whether or not this scope captures its parent scope.
    captures: bool,
    /// Whether or not this scope (or one of its parent scopes) is captured
    /// by child scopes.
    captured: bool,
    /// Locals in this scope which are captured by closures.
    captured_locals: HashSet<String>,
}

impl ClosureScope {
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

impl ScopeLike for ClosureScope {
    fn into_scope(self) -> Scope {
        Scope::Closure(Rc::new(RefCell::new(self)))
    }

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            return Ok(Local(name.to_string(), typ.clone()));
        }
        if let Some(parent) = &self.parent {
            return parent
                .get_local_from_parent(name)
                .and_then(ScopeResolution::assert_not_local);
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            // If we found it in ourselves.
            self.captured = true;
            self.captured_locals.insert(name.to_string());
            // Our caller (`get_local_from_parent`) will add ourselves onto the
            // scope chain `Vec`.
            return Ok(Closure(name.to_string(), typ.clone(), vec![]));
        }
        if let Some(parent) = &self.parent {
            return parent
                .get_local_from_parent(name)
                .and_then(ScopeResolution::assert_not_local);
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

    fn get_parent(&self) -> Option<Scope> {
        self.parent.clone()
    }
}

impl std::fmt::Debug for ClosureScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "ClosureScope({:p})", self)
    }
}

pub struct FuncScope {
    locals: HashMap<String, Type>,
    /// Only static resolutions are allowed through the parent.
    parent: Option<Scope>,
    /// Whether or not this scope is captured by child scopes (closures).
    captured: bool,
    captured_locals: HashSet<String>,
}

impl FuncScope {
    pub fn new(parent: Option<Scope>) -> Self {
        Self {
            locals: HashMap::new(),
            parent,
            captured: false,
            captured_locals: HashSet::new(),
        }
    }
}

impl ScopeLike for FuncScope {
    fn into_scope(self) -> Scope {
        Scope::Func(Rc::new(RefCell::new(self)))
    }

    fn get_local(&mut self, name: &str) -> Result<ScopeResolution, TypeError> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            return Ok(Local(name.to_string(), typ.clone()));
        }
        if let Some(parent) = &self.parent {
            return parent
                .get_local_from_parent(name)
                .and_then(ScopeResolution::assert_not_local)
                .and_then(ScopeResolution::disallow_closure);
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn get_local_as_parent(&mut self, name: &str) -> Result<ScopeResolution, TypeError> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            self.captured = true;
            self.captured_locals.insert(name.to_string());
            return Ok(Closure(name.to_string(), typ.clone(), vec![]));
        }
        if let Some(parent) = &self.parent {
            return parent
                .get_local_from_parent(name)
                .and_then(ScopeResolution::assert_not_local)
                .and_then(ScopeResolution::disallow_closure);
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

    fn get_parent(&self) -> Option<Scope> {
        self.parent.clone()
    }
}

impl std::fmt::Debug for FuncScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "FuncScope({:p})", self)
    }
}

pub struct ModuleScope {
    pub statics: HashMap<String, Type>,
    // Keeping track of which statics are used by child scopes. Not sure why...
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

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        if let Some(typ) = self.statics.get(name) {
            return Ok(ScopeResolution::Static(name.to_string(), typ.clone()));
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        if let Some(typ) = self.statics.get(name) {
            self.captured_statics.insert(name.to_string());
            return Ok(ScopeResolution::Static(name.to_string(), typ.clone()));
        }
        Err(TypeError::LocalNotFound {
            name: name.to_string(),
        })
    }

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()> {
        if self.statics.contains_key(name) {
            return Err(TypeError::LocalAlreadyDefined {
                name: name.to_string(),
            });
        }
        self.statics.insert(name.to_string(), typ);
        Ok(())
    }

    fn get_parent(&self) -> Option<Scope> {
        None
    }
}

impl std::fmt::Debug for ModuleScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "ModuleScope({:p})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::typ::Type;
    use super::{ClosureScope, ModuleScope, ScopeLike, ScopeResolution};

    #[test]
    fn test_scope_resolution() {
        let level1 = ModuleScope::new().into_scope();
        let static1 = Type::new_phantom();
        level1.add_local("static1", static1.clone()).unwrap();

        let level2 = ClosureScope::new(Some(level1.clone())).into_scope();
        let local2 = Type::new_phantom();
        level2.add_local("local2", local2.clone()).unwrap();

        let level3 = ClosureScope::new(Some(level2.clone())).into_scope();
        let local3 = Type::new_phantom();
        level3.add_local("local3", local3.clone()).unwrap();

        let level4 = ClosureScope::new(Some(level3.clone())).into_scope();
        let local4 = Type::new_phantom();
        level4.add_local("local4", local4.clone()).unwrap();

        // Check that module statics are resolved to static.
        assert_eq!(
            level4.get_local("static1").unwrap(),
            ScopeResolution::Static("static1".to_string(), static1)
        );

        // Check that locals are resolved to local.
        assert_eq!(
            level4.get_local("local4").unwrap(),
            ScopeResolution::Local("local4".to_string(), local4)
        );

        // And check that closed-over locals are resolved to closures with
        // correct chains.
        assert_eq!(
            level4.get_local("local2").unwrap(),
            ScopeResolution::Closure(
                "local2".to_string(),
                local2,
                vec![level2.clone(), level3.clone()]
            )
        );
        assert_eq!(
            level4.get_local("local3").unwrap(),
            ScopeResolution::Closure("local3".to_string(), local3, vec![level3.clone()])
        );
    }
}
