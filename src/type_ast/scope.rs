use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::{Error, Formatter};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::typ::Func;
use super::{Builtins, Closable, RecursionTracker, Type, TypeError, TypeResult};

const BUILTIN_SCOPE_ID: usize = 0;

lazy_static! {
    static ref SCOPE_ID: AtomicUsize = AtomicUsize::new(BUILTIN_SCOPE_ID + 1);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeId(usize);

impl ScopeId {
    pub const fn builtin() -> Self {
        ScopeId(BUILTIN_SCOPE_ID)
    }

    pub fn get(&self) -> usize {
        self.0
    }

    pub fn is_builtin(&self) -> bool {
        self.0 == BUILTIN_SCOPE_ID
    }
}

/// Returns an ID that is guaranteed to be unique amongst all threads.
fn next_scope_id() -> ScopeId {
    ScopeId(SCOPE_ID.fetch_add(1, Ordering::SeqCst))
}

/// Proxy so that we can share different kinds of scopes.
#[derive(Clone, Debug)]
pub enum Scope {
    Builtin,
    Closure(Rc<RefCell<ClosureScope>>),
    Func(Rc<RefCell<FuncScope>>),
    Module(Rc<RefCell<ModuleScope>>),
}

impl Scope {
    pub fn id(&self) -> ScopeId {
        use Scope::*;
        match self {
            Builtin => ScopeId(BUILTIN_SCOPE_ID),
            Closure(closure) => closure.borrow().id(),
            Func(func) => func.borrow().id(),
            Module(module) => module.borrow().id(),
        }
    }

    pub fn get_local(&self, name: &str) -> TypeResult<ScopeResolution> {
        use Scope::*;
        let resolution = match self {
            // Special case for the builtin scope that can act like a scope
            // but isn't backed by any actual `ScopeLike`.
            Builtin => {
                if let Some(typ) = Builtins::try_get(name) {
                    return Ok(ScopeResolution::Static(
                        name.to_string(),
                        typ,
                        Some(Scope::Builtin),
                    ));
                }
                Err(TypeError::LocalNotFound {
                    name: name.to_string(),
                })
            }
            Closure(closure) => closure.borrow_mut().get_local(name),
            Func(func) => func.borrow_mut().get_local(name),
            Module(module) => module.borrow_mut().get_local(name),
        };
        // Add ourselves onto the end of the scope chain.
        resolution.map(|resolution| resolution.add_scope(self.clone()))
    }

    /// Called by a child scope to get its parent's local.
    ///
    ///     return self.parent.get_local_from_parent(name);
    ///
    pub fn get_local_from_parent(&self, name: &str) -> TypeResult<ScopeResolution> {
        use Scope::*;
        let resolution = match self {
            Builtin => unreachable!("Cannot get local from parent of Builtin scope"),
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
            Builtin => unreachable!("Cannot add local to Builtin scope"),
            Closure(closure) => closure.borrow_mut().add_local(name, typ),
            Func(func) => func.borrow_mut().add_local(name, typ),
            Module(module) => module.borrow_mut().add_local(name, typ),
        }
    }

    pub fn add_static(&self, name: &str, typ: Type) -> TypeResult<()> {
        use Scope::*;
        match self {
            Builtin => unreachable!("Cannot add static to Builtin scope"),
            Closure(closure) => closure.borrow_mut().add_static(name, typ),
            Func(func) => func.borrow_mut().add_static(name, typ),
            Module(module) => module.borrow_mut().add_static(name, typ),
        }
    }

    fn get_parent(&self) -> Option<Scope> {
        use Scope::*;
        match self {
            Builtin => unreachable!("Cannot get parent of Builtin scope"),
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

    pub fn unwrap_func(&self) -> Ref<FuncScope> {
        use Scope::*;
        match self {
            Func(func) => func.borrow(),
            other @ _ => unreachable!("Not a Func scope: {:?}", other),
        }
    }

    pub fn unwrap_module(&self) -> Ref<ModuleScope> {
        use Scope::*;
        match self {
            Module(module) => module.borrow(),
            other @ _ => unreachable!("Not a Module scope: {:?}", other),
        }
    }
}

impl Closable for Scope {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use Scope::*;
        Ok(match self {
            Builtin => unreachable!("Builtin can't be closed"),
            Closure(shared) => {
                let replacement = {
                    let closure = shared.borrow();
                    let mut locals = HashMap::new();
                    for (name, typ) in closure.locals.iter() {
                        locals.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
                    }
                    let mut funcs = HashMap::new();
                    for (name, typ) in closure.funcs.iter() {
                        funcs.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
                    }
                    ClosureScope {
                        id: closure.id.clone(),
                        locals,
                        funcs,
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
                    let mut funcs = HashMap::new();
                    for (name, typ) in func.funcs.iter() {
                        funcs.insert(name.clone(), typ.clone().close(tracker, scope.clone())?);
                    }
                    FuncScope {
                        id: func.id.clone(),
                        locals,
                        funcs,
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
                        id: module.id.clone(),
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
            (Closure(self_closure), Closure(other_closure)) => {
                self_closure.as_ptr() == other_closure.as_ptr()
            }
            (Func(self_func), Func(other_func)) => self_func.as_ptr() == other_func.as_ptr(),
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
#[derive(Clone, Debug, PartialEq)]
pub enum ScopeResolution {
    Local(String, Type),
    /// A local that was found in a func and/or block scope above the current
    /// scope. The `Vec<Scope>` lists the chain of scopes traversed: the first
    /// is the scope where the value lives and the last is where it's read.
    Closure(String, Type, Vec<Scope>),
    /// `Option<Scope>` is the scope where this static was defined. It's `None`
    /// initially but is filled in by `add_scope`.
    Static(String, Type, Option<Scope>),
}

impl ScopeResolution {
    pub fn name(&self) -> String {
        use ScopeResolution::*;
        match self {
            Local(name, _) | Closure(name, _, _) | Static(name, _, _) => name.clone(),
        }
    }

    pub fn typ(&self) -> Type {
        use ScopeResolution::*;
        match self {
            Local(_, typ) | Closure(_, typ, _) | Static(_, typ, _) => typ.clone(),
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
            // If the static resolution's origin hasn't been filled in then the
            // current scope can be assumed to be the defining scope.
            Static(name, typ, None) => Static(name, typ, Some(scope)),
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
            Static(name, typ, origin) => Static(name, typ.close(tracker, scope)?, origin),
        })
    }
}

pub trait ScopeLike {
    /// Consume oneself to produce a shareable `Scope`.
    fn into_scope(self) -> Scope;

    fn id(&self) -> ScopeId;

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution>;

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<ScopeResolution>;

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()>;

    fn add_static(&mut self, name: &str, typ: Type) -> TypeResult<()>;

    fn get_parent(&self) -> Option<Scope>;
}

pub struct ClosureScope {
    id: ScopeId,
    pub locals: HashMap<String, Type>,
    /// Funcs can be defined within closures.
    pub funcs: HashMap<String, Type>,
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
            id: next_scope_id(),
            locals: HashMap::new(),
            funcs: HashMap::new(),
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

    fn id(&self) -> ScopeId {
        self.id.clone()
    }

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            return Ok(Local(name.to_string(), typ.clone()));
        }
        if let Some(typ) = self.funcs.get(name) {
            return Ok(Static(name.to_string(), typ.clone(), None));
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

    fn add_static(&mut self, name: &str, typ: Type) -> TypeResult<()> {
        if self.funcs.contains_key(name) {
            return Err(TypeError::StaticAlreadyDefined {
                name: name.to_string(),
            });
        }
        match typ {
            Type::Func(_) => {
                self.funcs.insert(name.to_string(), typ.clone());
                Ok(())
            }
            _ => {
                return Err(TypeError::CannotAddStatic {
                    message: format!(
                        "Cannot add non-Func static to ClosureScope; tried adding '{}': {:?}",
                        name, typ
                    ),
                })
            }
        }
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
    id: ScopeId,
    pub locals: HashMap<String, Type>,
    /// Funcs are statics and therefore will be resolved as static.
    pub funcs: HashMap<String, Type>,
    /// Only static resolutions are allowed through the parent.
    parent: Option<Scope>,
    /// Whether or not this scope is captured by child scopes (closures).
    captured: bool,
    captured_locals: HashSet<String>,
}

impl FuncScope {
    pub fn new(parent: Option<Scope>) -> Self {
        Self {
            id: next_scope_id(),
            locals: HashMap::new(),
            funcs: HashMap::new(),
            parent,
            captured: false,
            captured_locals: HashSet::new(),
        }
    }

    pub fn get_locals(&self) -> &HashMap<String, Type> {
        &self.locals
    }
}

impl ScopeLike for FuncScope {
    fn into_scope(self) -> Scope {
        Scope::Func(Rc::new(RefCell::new(self)))
    }

    fn id(&self) -> ScopeId {
        self.id.clone()
    }

    fn get_local(&mut self, name: &str) -> Result<ScopeResolution, TypeError> {
        use ScopeResolution::*;
        if let Some(typ) = self.locals.get(name) {
            return Ok(Local(name.to_string(), typ.clone()));
        }
        if let Some(typ) = self.funcs.get(name) {
            return Ok(Static(name.to_string(), typ.clone(), None));
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
        if let Some(typ) = self.funcs.get(name) {
            return Ok(Static(name.to_string(), typ.clone(), None));
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

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()> {
        if self.locals.contains_key(name) {
            return Err(TypeError::LocalAlreadyDefined {
                name: name.to_string(),
            });
        }
        self.locals.insert(name.to_string(), typ);
        Ok(())
    }

    fn add_static(&mut self, name: &str, typ: Type) -> TypeResult<()> {
        if self.funcs.contains_key(name) {
            return Err(TypeError::StaticAlreadyDefined {
                name: name.to_string(),
            });
        }
        match typ {
            Type::Func(_) => {
                self.funcs.insert(name.to_string(), typ.clone());
                Ok(())
            }
            _ => {
                return Err(TypeError::CannotAddStatic {
                    message: format!(
                        "Cannot add non-Func static to FuncScope; tried adding '{}': {:?}",
                        name, typ
                    ),
                })
            }
        }
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
    id: ScopeId,
    pub statics: HashMap<String, Type>,
    // Keeping track of which statics are used by child scopes. Not sure why...
    captured_statics: HashSet<String>,
}

impl ModuleScope {
    pub fn new() -> Self {
        Self {
            id: next_scope_id(),
            statics: HashMap::new(),
            captured_statics: HashSet::new(),
        }
    }
}

impl ScopeLike for ModuleScope {
    fn into_scope(self) -> Scope {
        Scope::Module(Rc::new(RefCell::new(self)))
    }

    fn id(&self) -> ScopeId {
        self.id.clone()
    }

    fn get_local(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        if let Some(typ) = self.statics.get(name) {
            return Ok(ScopeResolution::Static(name.to_string(), typ.clone(), None));
        }
        Scope::Builtin.get_local(name)
    }

    fn get_local_as_parent(&mut self, name: &str) -> TypeResult<ScopeResolution> {
        if let Some(typ) = self.statics.get(name) {
            self.captured_statics.insert(name.to_string());
            return Ok(ScopeResolution::Static(name.to_string(), typ.clone(), None));
        }
        Scope::Builtin.get_local(name)
    }

    fn add_local(&mut self, name: &str, typ: Type) -> TypeResult<()> {
        Err(TypeError::CannotAddStatic {
            message: format!(
                "Cannot add local to ModuleScope; tried adding '{}': {:?}",
                name, typ
            ),
        })
    }

    fn add_static(&mut self, name: &str, typ: Type) -> TypeResult<()> {
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
        level1.add_static("static1", static1.clone()).unwrap();

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
            ScopeResolution::Static("static1".to_string(), static1, Some(level1))
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
                vec![level2.clone(), level3.clone(), level4.clone()]
            )
        );
        assert_eq!(
            level4.get_local("local3").unwrap(),
            ScopeResolution::Closure("local3".to_string(), local3, vec![level3.clone(), level4.clone()])
        );
    }
}
