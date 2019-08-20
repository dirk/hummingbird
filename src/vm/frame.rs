use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::value::Value;
use crate::vm::vm::Function;

struct InnerClosure {
    locals: HashMap<String, Option<Value>>,
    parent: Option<Closure>,
}

/// Holds the variables in a frame that outlive that frame.
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
        Self(Rc::new(RefCell::new(InnerClosure { locals, parent })))
    }

    pub fn has(&self, name: String) -> bool {
        let inner = &self.0.borrow();
        if inner.locals.contains_key(&name) {
            return true;
        }
        if let Some(parent) = &inner.parent {
            return parent.has(name);
        }
        false
    }

    fn get(&self, name: String) -> Option<Value> {
        let inner = &self.0.borrow();
        if let Some(value) = inner.locals.get(&name) {
            return match value {
                initialized @ Some(_) => initialized.clone(),
                None => unreachable!("ERROR: Uninitialized closure variable: {}", name),
            };
        }
        if let Some(parent) = &inner.parent {
            return parent.get(name);
        }
        None
    }

    pub fn set(&self, name: String, value: Value) {
        let inner = &mut self.0.borrow_mut();
        if inner.locals.contains_key(&name) {
            inner.locals.insert(name.clone(), Some(value));
            return;
        }
        if let Some(parent) = &inner.parent {
            // If we found a parent with this name and were able to set the
            // value then all is good.
            if parent.set_as_parent(name.clone(), value) {
                return;
            }
        }
        // If we couldn't find the value to set in any parent then crash.
        unreachable!("ERROR: Couldn't find closure variable: {}", name)
    }

    // Recursive call to set in parent closures. Returns true if it found
    // a local and set, false if not.
    fn set_as_parent(&self, name: String, value: Value) -> bool {
        let inner = &mut self.0.borrow_mut();
        if inner.locals.contains_key(&name) {
            inner.locals.insert(name.clone(), Some(value));
            return true;
        }
        if let Some(parent) = &inner.parent {
            parent.set_as_parent(name, value)
        } else {
            false
        }
    }
}

/// A frame on the stack.
pub struct Frame {
    locals: HashMap<String, Value>,
    /// Locals within this frame which are closed over and therefore must
    /// outlive this frame.
    closure: Option<Closure>,
}

impl Frame {
    pub fn new_with_closure(closure: Closure) -> Self {
        Self {
            locals: HashMap::new(),
            closure: Some(closure),
        }
    }

    pub fn new_for_function(function: &Function) -> Self {
        let bindings = function.bindings();
        // If the function has its own bindings or if it uses bindings from
        // its parent then we need to set up a closure for it.
        let closure = if bindings.is_some() || function.has_parent_bindings() {
            Some(Closure::new(bindings, function.closure_cloned()))
        } else {
            None
        };
        Self {
            locals: HashMap::new(),
            closure,
        }
    }

    pub fn get(&self, name: String) -> Option<Value> {
        if let Some(value) = self.locals.get(&name) {
            return Some(value.clone());
        }
        if let Some(closure) = &self.closure {
            return closure.get(name);
        }
        None
    }

    pub fn set(&mut self, name: String, value: Value) {
        if let Some(closure) = &self.closure {
            if closure.has(name.clone()) {
                closure.set(name, value);
                return;
            }
        }
        self.locals.insert(name, value);
    }

    /// The frame owns its closure, but it's happy to give out clones.
    pub fn closure_cloned(&self) -> Option<Closure> {
        self.closure.clone()
    }
}
