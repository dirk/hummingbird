use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::{Closable, RecursionTracker, TypeError, TypeResult};

lazy_static! {
    static ref UID: AtomicUsize = AtomicUsize::new(0);
}

/// Returns an ID that is guaranteed to be unique amongst all threads.
pub fn next_uid() -> usize {
    UID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Clone, Debug)]
pub enum Type {
    /// A callable function; it is fixed and cannot be mutated.
    ///
    /// It is *only* mutable in order to support forward declaration. We need
    /// a shareable `Rc` before we know the closed argument and return types.
    Func(Rc<RefCell<Func>>),
    /// A generic defined by the user; it is fixed and cannot be mutated.
    ///
    /// It is *only* mutable in order to support forward declaration. We need
    /// to build a shareable `Rc` before we actually know the closed type.
    Generic(Rc<RefCell<Generic>>),
    Object(Rc<Object>),
    // Used to make writing tests easier.
    Phantom {
        id: usize,
    },
    Tuple(Tuple),
    // A type whose entire identity can change.
    Variable(Rc<RefCell<Variable>>),
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        use Type::*;
        match (self, other) {
            (Func(self_func), Func(other_func)) => self_func == other_func,
            (Generic(self_generic), Generic(other_generic)) => self_generic == other_generic,
            (Object(self_object), Object(other_object)) => self_object == other_object,
            (Phantom { id: self_id }, Phantom { id: other_id }) => self_id == other_id,
            (Tuple(self_tuple), Tuple(other_tuple)) => self_tuple == other_tuple,
            (Variable(self_variable), Variable(other_variable)) => self_variable == other_variable,
            _ => false,
        }
    }
}

impl Type {
    pub fn new_func(name: Option<String>, arguments: Vec<Type>, retrn: Type) -> Self {
        Type::Func(Rc::new(RefCell::new(Func {
            id: next_uid(),
            name,
            arguments,
            retrn: Box::new(retrn),
        })))
    }

    pub fn new_object(class: Class) -> Self {
        Type::Object(Rc::new(Object {
            id: next_uid(),
            class,
        }))
    }

    pub fn new_phantom() -> Self {
        Type::Phantom { id: next_uid() }
    }

    pub fn new_substitute(typ: Type) -> Self {
        let variable = Variable::Substitute(Box::new(typ));
        Type::Variable(Rc::new(RefCell::new(variable)))
    }

    pub fn new_empty_tuple() -> Self {
        Type::Tuple(Tuple {
            id: next_uid(),
            members: vec![],
        })
    }

    pub fn new_unbound() -> Self {
        let variable = Variable::Unbound { id: next_uid() };
        Self::new_variable(variable)
    }

    pub fn new_variable(variable: Variable) -> Self {
        Type::Variable(Rc::new(RefCell::new(variable)))
    }

    /// Returns a Some(arguments, retrn) if the type is some kind of callable
    /// (func or callable generic constraint).
    pub fn maybe_callable(&self) -> Option<(Vec<Type>, Type)> {
        fn generic_to_callable(generic: &Generic) -> Option<(Vec<Type>, Type)> {
            generic
                .get_callable()
                .map(|constraint| (constraint.arguments.clone(), constraint.retrn.clone()))
        }

        match self {
            Type::Func(func) => {
                let func = &*func.borrow();
                Some((func.arguments.clone(), (*func.retrn).clone()))
            }
            Type::Generic(generic) => {
                let generic = &*generic.borrow();
                generic_to_callable(generic)
            }
            Type::Variable(variable) => {
                let variable = &*variable.borrow();
                match variable {
                    Variable::Generic(generic) => generic_to_callable(generic),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn id(&self) -> usize {
        use Type::*;
        match self {
            Func(func) => func.borrow().id,
            Generic(generic) => generic.borrow().id,
            Object(object) => object.id,
            Phantom { id } => *id,
            Tuple(tuple) => tuple.id,
            Variable(variable) => variable.borrow().id(),
        }
    }

    pub fn is_unbound(&self) -> bool {
        if let Type::Variable(variable) = self {
            return variable.borrow().is_unbound();
        }
        false
    }

    /// Called on a function's arguments to recursively convert any unbound
    /// types into open generics. Also does some checks for unbounds being
    /// where they're not supposed to be.
    fn genericize(&self) -> TypeResult<()> {
        match self {
            Type::Variable(variable) => {
                let replacement = match &*variable.borrow() {
                    Variable::Substitute(substitute) => {
                        substitute.genericize()?;
                        None
                    }
                    Variable::Unbound { .. } => {
                        let generic = Type::Generic(Rc::new(RefCell::new(Generic::new())));
                        // Use a substitution so that any substitutions of this
                        // variable are followed to the *same* generic. We
                        // should never have multiple generics (or any type for
                        // that matter) with the same ID.
                        Some(Type::new_variable(Variable::Substitute(Box::new(generic))))
                    }
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    let mut inner = variable.borrow_mut();
                    *inner = Variable::Substitute(Box::new(replacement));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn close_func(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        let func = match self {
            Type::Func(func) => func,
            other @ _ => unreachable!("Called close_func on non-Func: {:?}", other),
        };
        let id = func.borrow().id;
        // Check if the function's already been built.
        if let Some(known) = tracker.check(&id) {
            return Ok(known);
        }
        tracker.add(id, Type::Func(func.clone()));
        // Genericize and close the arguments and return types.
        let (arguments, retrn) = {
            let func = func.borrow();
            let mut arguments = vec![];
            let retrn = *func.retrn.clone();
            // First convert any unbound (ie. unused) arguments into open
            // generics. We need to do this in one pass in case earlier
            // arguments depend on later ones.
            for argument in func.arguments.iter() {
                argument.genericize()?;
            }
            retrn.genericize()?;
            // Then close the arguments and return.
            for argument in func.arguments.iter() {
                arguments.push(argument.clone().close(tracker)?);
            }
            let retrn = retrn.close(tracker)?;
            (arguments, retrn)
        };
        // Then write them back to the function to make it closed.
        {
            let mut mutable = func.borrow_mut();
            mutable.arguments = arguments;
            mutable.retrn = Box::new(retrn);
        }
        Ok(Type::Func(func))
    }

    fn close_variable(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        let variable = match self {
            Type::Variable(variable) => variable,
            other @ _ => unreachable!("Called close_variable on non-Variable: {:?}", other),
        };
        // Uncomment to see types pre-closing:
        //   return Ok(Type::Variable(variable));
        let replacement = match &*variable.borrow() {
            Variable::Generic(open) => {
                // Check if the generic is already being built (dealing with
                // recursive types), otherwise add the forward declaration.
                if let Some(known) = tracker.check(&open.id) {
                    return Ok(known);
                }
                let closed = Rc::new(RefCell::new(Generic::new()));
                // Note that we register with the old open generic's ID
                // since that's what's going to be in any nested types
                // that haven't been closed yet.
                tracker.add(open.id, Type::Generic(closed.clone()));
                // Copy the constraints from the open to the closed generics,
                // closing their types along the way.
                for constraint in open.constraints.iter() {
                    use GenericConstraint::*;
                    let mut mutable = closed.borrow_mut();
                    match constraint {
                        Property(property) => mutable.add_property_constraint(
                            property.name.clone(),
                            property.typ.clone().close(tracker)?,
                        ),
                        Callable(callable) => {
                            let mut arguments = vec![];
                            for argument in callable.arguments.iter() {
                                arguments.push(argument.clone().close(tracker)?);
                            }
                            mutable.add_callable_constraint(
                                arguments,
                                callable.retrn.clone().close(tracker)?,
                            )
                        }
                        other @ _ => unreachable!("Cannot close constraint: {:?}", other),
                    }
                }
                // Use substitution so that other uses will be updated.
                Some(Variable::Substitute(Box::new(Type::Generic(closed))))
            }
            // Turn unbounds into closed-but-unconstrained generics.
            // If that's incorrect it will be caught by codegen.
            Variable::Unbound { .. } => {
                let closed = Generic::new();
                // Use substitution so that other uses will be updated.
                Some(Variable::Substitute(Box::new(Type::Generic(Rc::new(
                    RefCell::new(closed),
                )))))
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            let mut mutable = variable.borrow_mut();
            *mutable = replacement;
        }
        let result = match &*variable.borrow() {
            Variable::Generic(_) => unreachable!("Generic should have been replaced"),
            // Substitutions can be unboxed into the underlying type.
            // Note the `close()` to recursively resolve nested
            // substitutions.
            Variable::Substitute(typ) => Ok(typ.clone().close(tracker)?),
            Variable::Unbound { id } => {
                // panic!("Unexpected unbound: {}", id);
                // Err(TypeError::UnexpectedUnbound { id: *id })
                unreachable!("Unbound should have been replaced: {}", id)
            }
        };
        result
    }
}

impl Closable for Type {
    /// When unification is done we need to turn all the `Variable` types into
    /// closed, fixed types.
    ///
    /// TODO: Make closed types a separate type so that we get compile-time
    ///   guarantees that we're only working with a closed type.
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        match self {
            Type::Func(_) => self.close_func(tracker),
            Type::Variable(_) => self.close_variable(tracker),
            other @ _ => Ok(other),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub id: usize,
    // Included for debugging.
    pub name: Option<String>,
    pub arguments: Vec<Type>,
    pub retrn: Box<Type>,
}

impl Func {
    pub fn arity(&self) -> usize {
        self.arguments.len()
    }
}

impl PartialEq for Func {
    /// Functions are equal if either:
    ///   - Their ID is the same (which guarantees the rest of their fields are).
    ///   - They have the same arguments and return types.
    fn eq(&self, other: &Self) -> bool {
        // Short-circuit for identity equality.
        if self.id == other.id {
            return true;
        }
        if self.arguments.len() != other.arguments.len() {
            return false;
        }
        for (self_argument, other_argument) in self.arguments.iter().zip(other.arguments.iter()) {
            if self_argument != other_argument {
                return false;
            }
        }
        self.retrn == other.retrn
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Generic {
    pub id: usize,
    // TODO: Name
    pub constraints: Vec<GenericConstraint>,
}

impl Generic {
    pub fn new() -> Self {
        Self {
            id: next_uid(),
            constraints: vec![],
        }
    }

    pub fn new_with_constraints(constraints: Vec<GenericConstraint>) -> Self {
        Self {
            id: next_uid(),
            constraints,
        }
    }

    pub fn add_callable_constraint(&mut self, arguments: Vec<Type>, retrn: Type) {
        self.constraints
            .push(GenericConstraint::Callable(CallableConstraint {
                arguments,
                retrn,
            }))
    }

    pub fn get_callable(&self) -> Option<&CallableConstraint> {
        use GenericConstraint::*;
        for constraint in self.constraints.iter() {
            match constraint {
                Callable(callable) => return Some(callable),
                _ => (),
            }
        }
        None
    }

    pub fn add_property_constraint(&mut self, name: String, typ: Type) {
        self.constraints
            .push(GenericConstraint::Property(PropertyConstraint {
                name,
                typ,
            }))
    }

    pub fn get_property<S: AsRef<str>>(&self, name: S) -> Option<&PropertyConstraint> {
        use GenericConstraint::*;
        for constraint in self.constraints.iter() {
            match constraint {
                Property(property) => {
                    if &property.name == name.as_ref() {
                        return Some(property);
                    }
                }
                _ => (),
            }
        }
        None
    }
}

/// The type can be called with the given arguments and return type.
#[derive(Clone, Debug, PartialEq)]
pub struct CallableConstraint {
    pub arguments: Vec<Type>,
    pub retrn: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyConstraint {
    pub name: String,
    pub typ: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericConstraint {
    Callable(CallableConstraint),
    Property(PropertyConstraint),
}

#[derive(Clone, Debug)]
pub struct Object {
    pub id: usize,
    pub class: Class,
    // TODO: Parameterize
}

impl PartialEq for Object {
    /// Two objects are equal if they:
    ///   - Have the same ID.
    ///   - Have the same class (and eventually generic parameters)
    fn eq(&self, other: &Self) -> bool {
        if self.id == other.id {
            return true;
        }
        self.class == other.class
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Class {
    Intrinsic(Arc<IntrinsicClass>),
    Derived(Arc<DerivedClass>),
}

impl Class {
    pub fn name(&self) -> String {
        use Class::*;
        match self {
            Intrinsic(intrinsic) => intrinsic.name.clone(),
            Derived(derived) => derived.name.clone(),
        }
    }
}

/// A class built into the language.
#[derive(Clone, Debug, PartialEq)]
pub struct IntrinsicClass {
    pub id: usize,
    pub name: String,
}

/// A user-defined class.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedClass {
    pub id: usize,
    pub name: String,
    // TODO: Parameters
}

#[derive(Clone, Debug)]
pub struct Tuple {
    pub id: usize,
    pub members: Vec<Type>,
}

impl PartialEq for Tuple {
    fn eq(&self, other: &Self) -> bool {
        if self.id == other.id {
            return true;
        }
        if self.members.len() != other.members.len() {
            return false;
        }
        for (self_member, other_member) in self.members.iter().zip(other.members.iter()) {
            if self_member != other_member {
                return false;
            }
        }
        true
    }
}

/// A type which can change during unification.
#[derive(Clone, Debug, PartialEq)]
pub enum Variable {
    /// An auto-generated generic whose constraints can be mutated by virtue
    /// of being stored in a `Variable`.
    Generic(Generic),
    /// Substitute this type with another type.
    Substitute(Box<Type>),
    /// We don't know what it is yet. It is an error for any `Unbound`s to
    /// make it to the end of unification.
    Unbound { id: usize },
}

impl Variable {
    pub fn id(&self) -> usize {
        use Variable::*;
        match self {
            Generic(generic) => generic.id,
            Substitute(typ) => typ.id(),
            Unbound { id } => *id,
        }
    }

    pub fn is_generic(&self) -> bool {
        match self {
            Variable::Generic(_) => true,
            _ => false,
        }
    }

    pub fn is_unbound(&self) -> bool {
        match self {
            Variable::Unbound { .. } => true,
            _ => false,
        }
    }

    pub fn unwrap_substitute(&self) -> &Type {
        match self {
            Variable::Substitute(substitute) => &**substitute,
            other @ _ => unreachable!("Not a Substitute: {:?}", other),
        }
    }

    pub fn unwrap_generic(&self) -> &Generic {
        match self {
            Variable::Generic(generic) => generic,
            other @ _ => unreachable!("Not a Generic: {:?}", other),
        }
    }

    pub fn unwrap_mut_generic(&mut self) -> &mut Generic {
        match self {
            Variable::Generic(generic) => generic,
            other @ _ => unreachable!("Not a Generic: {:?}", other),
        }
    }
}
