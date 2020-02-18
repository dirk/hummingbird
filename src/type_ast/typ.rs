use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::scope::Scope;
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
    /// A callable function or closure; it is fixed and cannot be mutated once
    /// it is closed at the end of creation.
    Func(Rc<Func>),
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
    /// A type whose entire identity can change.
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
    // `scope` is the scope that the function was defined in, not its own
    // internal scope.
    pub fn new_func(name: Option<String>, arguments: Vec<Type>, retrn: Type, scope: Scope) -> Self {
        Type::Func(Rc::new(Func {
            id: next_uid(),
            scope,
            closed: RefCell::new(false),
            name,
            arguments: RefCell::new(arguments),
            retrn: RefCell::new(retrn),
        }))
    }

    pub fn new_object(class: Class, scope: Scope) -> Self {
        Type::Object(Rc::new(Object {
            id: next_uid(),
            scope,
            class,
        }))
    }

    #[cfg(test)]
    pub fn new_phantom() -> Self {
        Type::Phantom { id: next_uid() }
    }

    #[cfg(not(test))]
    pub fn new_phantom() -> Self {
        unreachable!("Phantom types cannot be constructed outside of tests")
    }

    pub fn new_substitute(typ: Type, scope: Scope) -> Self {
        let variable = Variable::Substitute {
            scope,
            substitute: Box::new(typ),
        };
        Type::Variable(Rc::new(RefCell::new(variable)))
    }

    pub fn new_empty_tuple(scope: Scope) -> Self {
        Type::Tuple(Tuple {
            id: next_uid(),
            scope,
            members: vec![],
        })
    }

    pub fn new_unbound(scope: Scope) -> Self {
        let variable = Variable::Unbound {
            id: next_uid(),
            scope,
        };
        Self::new_variable(variable)
    }

    pub fn new_variable(variable: Variable) -> Self {
        Type::Variable(Rc::new(RefCell::new(variable)))
    }

    pub fn unwrap_variable(&self) -> &Rc<RefCell<Variable>> {
        match self {
            Type::Variable(variable) => variable,
            other @ _ => unreachable!("Not a Variable: {:?}", other),
        }
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
                let arguments = func.arguments.borrow().clone();
                let retrn = func.retrn.borrow().clone();
                Some((arguments, retrn))
            }
            Type::Generic(generic) => {
                let generic = &*generic.borrow();
                generic_to_callable(generic)
            }
            Type::Variable(variable) => {
                let variable = &*variable.borrow();
                match variable {
                    Variable::Generic { generic, .. } => generic_to_callable(generic),
                    Variable::Substitute { substitute, .. } => substitute.maybe_callable(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn id(&self) -> usize {
        use Type::*;
        match self {
            Func(func) => func.id,
            Generic(generic) => generic.borrow().id,
            Object(object) => object.id,
            Phantom { id } => *id,
            Tuple(tuple) => tuple.id,
            Variable(variable) => variable.borrow().id(),
        }
    }

    pub fn scope(&self) -> Scope {
        use Type::*;
        match self {
            Func(func) => func.scope.clone(),
            Generic(generic) => generic.borrow().scope.clone(),
            Object(object) => object.scope.clone(),
            Phantom { id } => unreachable!(),
            Tuple(tuple) => tuple.scope.clone(),
            Variable(variable) => variable.borrow().scope(),
        }
    }

    /// Return an open (ie. variable) duplicate of oneself. Only really applies
    /// to generics. We use this when calling functions so that unbounds don't
    /// become substituted for closed immutable types.
    ///
    /// We use a `RecursionTracker` so that we can return the same duplicate
    /// if we see it multiple times. This way links between argument and return
    /// types are preserved.
    pub fn open_duplicate(&self, tracker: &mut RecursionTracker, scope: Scope) -> Type {
        if let Some(known) = tracker.check(&self.id()) {
            return known;
        }
        match self {
            Type::Generic(generic) => {
                let constraints = generic
                    .borrow()
                    .constraints
                    .iter()
                    .map(|constraint| {
                        use GenericConstraint::*;
                        match constraint {
                            Callable(callable) => {
                                let mut arguments = vec![];
                                for argument in callable.arguments.iter() {
                                    arguments.push(argument.open_duplicate(tracker, scope.clone()));
                                }
                                Callable(CallableConstraint {
                                    arguments,
                                    retrn: callable.retrn.open_duplicate(tracker, scope.clone()),
                                })
                            }
                            Property(property) => Property(PropertyConstraint {
                                name: property.name.clone(),
                                typ: property.typ.open_duplicate(tracker, scope.clone()),
                            }),
                        }
                    })
                    .collect::<Vec<_>>();
                let open = Type::Variable(Rc::new(RefCell::new(Variable::Generic {
                    scope: scope.clone(),
                    generic: Generic::new_with_constraints(constraints, scope),
                })));
                // Add the opened version to the track with the closed's ID so
                // that it will be returned if the closed is encountered again.
                tracker.add(self.id(), open.clone());
                open
            }
            _ => self.clone(),
        }
    }

    pub fn is_unbound(&self) -> bool {
        if let Type::Variable(variable) = self {
            return variable.borrow().is_unbound();
        }
        false
    }

    /// Called on a function's arguments to recursively convert any unbound
    /// types into open generics.
    fn genericize(&self, scope: Scope) -> TypeResult<()> {
        // Skip genericizing if this variable wasn't in the scope being closed.
        if !self.scope().within(&scope) {
            return Ok(());
        }
        match self {
            Type::Variable(variable) => {
                let replacement = match &*variable.borrow() {
                    Variable::Substitute { substitute, .. } => {
                        substitute.genericize(scope.clone())?;
                        None
                    }
                    Variable::Unbound {
                        scope: originating_scope,
                        ..
                    } => {
                        let generic = Type::Generic(Rc::new(RefCell::new(Generic::new(
                            originating_scope.clone(),
                        ))));
                        // Use a substitution so that any substitutions of this
                        // variable are followed to the *same* generic. We
                        // should never have multiple generics (or any type for
                        // that matter) with the same ID.
                        Some(Variable::Substitute {
                            scope: originating_scope.clone(),
                            substitute: Box::new(generic),
                        })
                    }
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    let mut mutable = variable.borrow_mut();
                    *mutable = replacement;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn close_func(typ: Type, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Type> {
        let func = match typ {
            Type::Func(func) => func,
            other @ _ => unreachable!("Called close_func on non-Func: {:?}", other),
        };
        // Don't reclose.
        if *func.closed.borrow() {
            return Ok(Type::Func(func));
        }
        // Check if the function's already been built.
        if let Some(known) = tracker.check(&func.id) {
            return Ok(known);
        }
        tracker.add(func.id, Type::Func(func.clone()));
        // Genericize and close the arguments and return types.
        let (arguments, retrn) = {
            let mut arguments = vec![];
            let retrn = func.retrn.borrow().clone();
            // First convert any unbound (ie. unused) arguments into open
            // generics. We need to do this in one pass in case earlier
            // arguments depend on later ones.
            for argument in func.arguments.borrow().iter() {
                argument.genericize(scope.clone())?;
            }
            retrn.genericize(scope.clone())?;
            // Then close the arguments and return.
            for argument in func.arguments.borrow().iter() {
                arguments.push(argument.clone().close(tracker, scope.clone())?);
            }
            let retrn = retrn.close(tracker, scope)?;
            (arguments, retrn)
        };
        // Then write them back to the function to make it closed.
        {
            let mut mutable_closed = func.closed.borrow_mut();
            *mutable_closed = true;
            let mut mutable_arguments = func.arguments.borrow_mut();
            *mutable_arguments = arguments;
            let mut mutable_retrn = func.retrn.borrow_mut();
            *mutable_retrn = retrn;
        }
        Ok(Type::Func(func))
    }

    fn close_variable(typ: Type, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let variable = match typ {
            Type::Variable(variable) => variable,
            other @ _ => unreachable!("Called close_variable on non-Variable: {:?}", other),
        };
        // Uncomment to see types pre-closing:
        //   return Ok(Type::Variable(variable));
        let replacement = match &*variable.borrow() {
            Variable::Generic { generic: open, .. } => {
                // Check if the generic is already being built (dealing with
                // recursive types), otherwise add the forward declaration.
                if let Some(known) = tracker.check(&open.id) {
                    return Ok(known);
                }
                let closed = Rc::new(RefCell::new(Generic::new(scope.clone())));
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
                            property.typ.clone().close(tracker, scope.clone())?,
                        ),
                        Callable(callable) => {
                            let mut arguments = vec![];
                            for argument in callable.arguments.iter() {
                                arguments.push(argument.clone().close(tracker, scope.clone())?);
                            }
                            mutable.add_callable_constraint(
                                arguments,
                                callable.retrn.clone().close(tracker, scope.clone())?,
                            )
                        }
                        other @ _ => unreachable!("Cannot close constraint: {:?}", other),
                    }
                }
                // Use substitution so that other uses will be updated.
                Some(Variable::Substitute {
                    scope: scope.clone(),
                    substitute: Box::new(Type::Generic(closed)),
                })
            }
            // Turn unbounds into closed-but-unconstrained generics.
            // If that's incorrect it will be caught by codegen.
            Variable::Unbound { .. } => {
                let closed = Generic::new(scope.clone());
                // Use substitution so that other uses will be updated.
                Some(Variable::Substitute {
                    scope: scope.clone(),
                    substitute: Box::new(Type::Generic(Rc::new(RefCell::new(closed)))),
                })
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            let mut mutable = variable.borrow_mut();
            *mutable = replacement;
        }
        let result = match &*variable.borrow() {
            Variable::Generic { .. } => unreachable!("Generic should have been replaced"),
            // Substitutions can be unboxed into the underlying type.
            // Note the `close()` to recursively resolve nested
            // substitutions.
            Variable::Substitute { substitute, .. } => {
                Ok(substitute.clone().close(tracker, scope)?)
            }
            Variable::Unbound { id, .. } => {
                // panic!("Unexpected unbound: {}", id);
                // Err(TypeError::UnexpectedUnbound { id: *id })
                unreachable!("Unbound should have been replaced: {}", id)
            }
        };
        result
    }
}

impl Closable for Type {
    /// When translation and unification is done we need to turn all the
    /// `Variable` types into closed, fixed types.
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        // Skip closing if this variable isn't in the scope being closed.
        if !self.scope().within(&scope) {
            return Ok(self);
        }
        match self {
            Type::Func(_) => Type::close_func(self, tracker, scope),
            Type::Variable(_) => Type::close_variable(self, tracker, scope),
            other @ _ => Ok(other),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub id: usize,
    /// The scope that this function was defined in, not the actual scope of
    /// its body. If you want that see the `type_ast::Func.scope`.
    pub scope: Scope,
    /// False while the type is being built (forward declaration), true
    /// afterwards. Should not be mutated once closed!
    pub closed: RefCell<bool>,
    /// Included for debugging.
    pub name: Option<String>,
    // The arguments and return types are only mutable in order to support
    // forward declaration (for closures and recursion).
    pub arguments: RefCell<Vec<Type>>,
    pub retrn: RefCell<Type>,
}

impl Func {
    pub fn arity(&self) -> usize {
        self.arguments.borrow().len()
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
        if self.arity() != other.arity() {
            return false;
        }
        let self_arguments = self.arguments.borrow();
        let other_arguments = other.arguments.borrow();
        for (self_argument, other_argument) in self_arguments.iter().zip(other_arguments.iter()) {
            if self_argument != other_argument {
                return false;
            }
        }
        let self_retrn = &*self.retrn.borrow();
        let other_retrn = &*other.retrn.borrow();
        self_retrn == other_retrn
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Generic {
    pub id: usize,
    pub scope: Scope,
    // TODO: Name
    pub constraints: Vec<GenericConstraint>,
}

impl Generic {
    pub fn new(scope: Scope) -> Self {
        Self {
            id: next_uid(),
            scope,
            constraints: vec![],
        }
    }

    pub fn new_with_constraints(constraints: Vec<GenericConstraint>, scope: Scope) -> Self {
        Self {
            id: next_uid(),
            scope,
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
    pub scope: Scope,
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
    pub scope: Scope,
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
    Generic { scope: Scope, generic: Generic },
    /// Substitute this type with another type.
    Substitute { scope: Scope, substitute: Box<Type> },
    /// We don't know what it is yet. It is an error for any `Unbound`s to
    /// make it to the end of unification.
    Unbound { id: usize, scope: Scope },
}

impl Variable {
    pub fn id(&self) -> usize {
        use Variable::*;
        match self {
            Generic { generic, .. } => generic.id,
            Substitute { substitute, .. } => substitute.id(),
            Unbound { id, .. } => *id,
        }
    }

    pub fn scope(&self) -> Scope {
        use Variable::*;
        match self {
            Generic { scope, .. } => scope.clone(),
            Substitute { scope, .. } => scope.clone(),
            Unbound { scope, .. } => scope.clone(),
        }
    }

    pub fn is_generic(&self) -> bool {
        match self {
            Variable::Generic { .. } => true,
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
            Variable::Substitute { substitute, .. } => &**substitute,
            other @ _ => unreachable!("Not a Substitute: {:?}", other),
        }
    }

    pub fn unwrap_generic(&self) -> &Generic {
        match self {
            Variable::Generic { generic, .. } => generic,
            other @ _ => unreachable!("Not a Generic: {:?}", other),
        }
    }

    pub fn unwrap_mut_generic(&mut self) -> &mut Generic {
        match self {
            Variable::Generic { generic, .. } => generic,
            other @ _ => unreachable!("Not a Generic: {:?}", other),
        }
    }
}
