use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use super::super::errors::VmError;
use super::super::value::Value;

struct InnerClosure {
    locals: HashMap<String, Option<Value>>,
    parent: Option<Closure>,
    /// The builtins closure cannot be written to after it's created.
    builtins: bool,
    /// If this closure is for a REPL. It allows us to set new variables at
    /// will rather than being restricted to the established bindings.
    repl: bool,
}

#[derive(Clone)]
pub struct Closure(Rc<RefCell<InnerClosure>>);

impl Closure {
    pub fn new(bindings: Option<HashSet<String>>, parent: Option<Closure>) -> Self {
        let mut locals = HashMap::<String, Option<Value>>::new();
        if let Some(bindings) = bindings {
            for binding in bindings.iter() {
                locals.insert(binding.clone(), None);
            }
        };
        Self(Rc::new(RefCell::new(InnerClosure {
            locals,
            parent,
            builtins: false,
            repl: false,
        })))
    }

    pub fn new_builtins() -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: None,
            builtins: true,
            repl: false,
        })))
    }

    pub fn new_repl(builtins_closure: Closure) -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: Some(builtins_closure),
            builtins: false,
            repl: true,
        })))
    }

    pub fn new_static(builtins_closure: Option<Closure>) -> Self {
        Self(Rc::new(RefCell::new(InnerClosure {
            locals: HashMap::new(),
            parent: builtins_closure,
            builtins: false,
            repl: false,
        })))
    }

    pub fn get(&self, name: &String) -> Result<Value, VmError> {
        let inner = &self.0.borrow();
        if let Some(value) = inner.locals.get(name) {
            return match value {
                Some(initialized) => Ok(initialized.clone()),
                None => unreachable!("ERROR: Uninitialized closure variable: {}", name),
            };
        }
        if let Some(parent) = &inner.parent {
            return parent.get(name);
        }
        Err(VmError::new_undefined_name(name.clone()))
    }

    /// Returns true if it found a closure in which to set the variable,
    /// false if not.
    pub fn try_set(&self, name: String, value: Value) -> bool {
        let inner = &mut self.0.borrow_mut();
        // If the builtins flag is set then it cannot be mutated.
        if inner.builtins {
            return false;
        }
        if let Some(exists) = inner.locals.get_mut(&name) {
            *exists = Some(value);
            return true;
        }
        // If it's for a REPL then we can create new locals at will.
        if inner.repl {
            inner.locals.insert(name, Some(value));
            return true;
        }
        if let Some(parent) = &inner.parent {
            return parent.try_set(name.clone(), value);
        }
        return false;
    }

    /// Set a local directly into this exact closure. This should only be used
    /// by the VM when:
    ///   - Setting imports into a module's static closure.
    ///   - Setting up the builtins closure.
    pub fn set_directly(&self, name: String, value: Value) {
        let inner = &mut self.0.borrow_mut();
        inner.locals.insert(name, Some(value));
    }
}

impl fmt::Debug for Closure {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let inner = &*self.0.borrow();
        let locals = inner
            .locals
            .keys()
            .map(|k| k.to_owned())
            .collect::<Vec<String>>();
        write!(
            f,
            "Closure {{ locals: {:?}, parent: {:?}, repl: {:?} }}",
            locals, inner.parent, inner.repl
        )
    }
}
