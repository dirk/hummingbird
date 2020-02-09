use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::{Closable, TypeError, TypeResult};

lazy_static! {
    static ref UID: AtomicUsize = AtomicUsize::new(0);
}

/// Returns an ID that is guaranteed to be unique amongst all threads.
pub fn next_uid() -> usize {
    UID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Clone, Debug)]
pub enum Type {
    // A callable function.
    Func(Rc<Func>),
    // A generic defined by the user; it is fixed and cannot be mutated.
    Generic(Rc<Generic>),
    Object(Rc<Object>),
    // Used to make writing tests easier.
    Phantom { id: usize },
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
        Type::Func(Rc::new(Func {
            id: next_uid(),
            name,
            arguments,
            retrn: Box::new(retrn),
        }))
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
        Type::Tuple(Tuple { id: next_uid(), members: vec![] })
    }

    pub fn new_unbound() -> Self {
        let variable = Variable::Unbound { id: next_uid() };
        Type::Variable(Rc::new(RefCell::new(variable)))
    }

    pub fn id(&self) -> usize {
        use Type::*;
        match self {
            Func(func) => func.id,
            Generic(generic) => generic.id,
            Object(object) => object.id,
            Phantom { id } => *id,
            Tuple(tuple) => tuple.id,
            Variable(variable) => variable.borrow().id(),
        }
    }

    pub fn is_unbound(&self) -> bool {
        if let Type::Variable(variable) = self {
            return variable.borrow().is_unbound()
        }
        false
    }

    pub fn close_func(self) -> TypeResult<Self> {
        match self {
            Type::Func(func) => {
                let func = &*func;
                let mut arguments = vec![];
                // First convert any unbound (ie. unused) arguments into open
                // generics. We need to do this in one pass in case earlier
                // arguments depend on later ones.
                for argument in func.arguments.iter() {
                    // Convert any unbound (ie. unused) arguments into open
                    // generics.
                    if let Type::Variable(variable) = argument {
                        if variable.borrow().is_unbound() {
                            let mut inner = variable.borrow_mut();
                            // Use a substitution so that any substitutions of
                            // this variable are followed to the *same*
                            // generic. We should never have multiple generics
                            // (or any type for that matter) with the same ID.
                            let generic = Type::Generic(Rc::new(Generic::new()));
                            *inner = Variable::Substitute(Box::new(generic));
                        }
                    }
                }
                for argument in func.arguments.iter() {
                    arguments.push(argument.clone().close()?);
                }
                let retrn = func.retrn.clone().close()?;
                Ok(Type::Func(Rc::new(Func {
                    id: func.id,
                    name: func.name.clone(),
                    arguments,
                    retrn: Box::new(retrn),
                })))
            }
            other @ _ => unreachable!("Called close_func on non-Func: {:?}", other),
        }
    }
}

impl Closable for Type {
    /// When unification is done we need to turn all the `Variable` types into
    /// closed, fixed types.
    ///
    /// TODO: Make closed types a separate type so that we get compile-time
    ///   guarantees that we're only working with a closed type.
    fn close(self) -> TypeResult<Self> {
        match self {
            Type::Func(_) => self.close_func(),
            Type::Variable(variable) => {
                // Uncomment to see types pre-closing:
                //   return Ok(Type::Variable(variable));
                match &*variable.borrow() {
                    Variable::Generic(generic) => Ok(Type::Generic(Rc::new(generic.clone()))),
                    // Substitutions can be unboxed into the underlying type.
                    // Note the `close()` to recursively resolve nested
                    // substitutions.
                    Variable::Substitute(typ) => Ok(typ.clone().close()?),
                    Variable::Unbound { id } => {
                        // panic!("Unexpected unbound: {}", id);
                        Err(TypeError::UnexpectedUnbound { id: *id })
                    }
                }
            }
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericConstraint {
    /// The type can be called with the given arguments and return type.
    Callable {
        arguments: Vec<Type>,
        retrn: Type,
    },
    Property {
        name: String,
        typ: Type,
    },
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

    pub fn is_unbound(&self) -> bool {
        match self {
            Variable::Unbound { .. } => true,
            _ => false,
        }
    }
}
