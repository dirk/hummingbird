/// The goal of inference and unification is to take an AST of variable types
/// (unbound types) and resolve them all into real (funcs, objects) or generic
/// types.
///
/// At the end of the process there should be no `Type::Variable` variants
/// remaining in the AST.
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::parser::{Span, Token, Word};
use super::{parse_ast as past, StageError};

mod builtins;
mod nodes;
mod printer;
mod scope;
mod translate;
mod typ;
mod unify;

pub use builtins::Builtins;
pub use nodes::*;
pub use printer::{Printer, PrinterOptions};
pub use scope::{ClosureScope, ModuleScope, Scope, ScopeLike, ScopeResolution};
pub use translate::translate_module;
pub use typ::{
    Class, Func as TFunc, Generic, GenericConstraint, IntrinsicClass, PropertyConstraint, Type,
    TypeId, Variable,
};
pub use unify::unify;

type TypeResult<T> = Result<T, TypeError>;

#[derive(Clone, Debug)]
pub enum TypeError {
    LocalAlreadyDefined {
        name: String,
    },
    LocalNotFound {
        name: String,
    },
    CannotCapture {
        name: String,
    },
    // PropertyAlreadyDefined {
    //     name: String,
    // },
    CannotUnify {
        first: Type,
        second: Type,
    },
    TypeMismatch {
        expected: Type,
        got: Type,
    },
    ArgumentLengthMismatch {
        expected: Vec<Type>,
        got: Vec<Type>,
    },
    /// When we try to `Type#close` and run into an `Unbound`.
    UnexpectedUnbound {
        id: usize,
    },
    InternalError {
        message: String,
    },
    WithSpan {
        wrapped: Box<TypeError>,
        span: Span,
    },
}

impl TypeError {
    /// Reverse the sides of two-sided errors.
    fn reverse(self) -> Self {
        use TypeError::*;
        match self {
            CannotUnify { first, second } => CannotUnify {
                first: second,
                second: first,
            },
            TypeMismatch { expected, got } => TypeMismatch {
                expected: got,
                got: expected,
            },
            other @ _ => other,
        }
    }

    pub fn short_message(&self) -> String {
        use TypeError::*;
        let message = match self.unwrap() {
            LocalAlreadyDefined { .. } => "LocalAlreadyDefined",
            LocalNotFound { .. } => "LocalNotFound",
            CannotCapture { .. } => "CannotCapture",
            // PropertyAlreadyDefined { .. } => "PropertyAlreadyDefined",
            CannotUnify { .. } => "CannotUnify",
            TypeMismatch { .. } => "TypeMismatch",
            ArgumentLengthMismatch { .. } => "ArgumentLengthMismatch",
            UnexpectedUnbound { .. } => "UnexpectedUnbound",
            InternalError { .. } => "InternalError",
            WithSpan { .. } => unreachable!(),
        };
        message.to_string()
    }

    pub fn label_message(&self) -> String {
        use TypeError::*;
        match self.unwrap() {
            CannotCapture { .. } => "Attempted to capture here".to_string(),
            CannotUnify { .. } => "Trying to unify here".to_string(),
            TypeMismatch { .. } => "Mismatch occurred here".to_string(),
            InternalError { message } => message.clone(),
            WithSpan { .. } => unreachable!(),
            _ => "Here".to_string(),
        }
    }

    pub fn notes(&self) -> Option<String> {
        use TypeError::*;
        match self.unwrap() {
            CannotCapture { .. } => {
                return Some(
                    "Functions cannot capture local variables from higher scopes.\n\
                    \n\
                    To avoid this error convert the function to a closure."
                        .to_string(),
                )
            }
            _ => (),
        }
        None
    }

    pub fn span(&self) -> Option<Span> {
        use TypeError::*;
        match self {
            WithSpan { span, .. } => Some(span.clone()),
            _ => None,
        }
    }

    /// Returns the underlying error with all metadata layers peeled off.
    pub fn unwrap(&self) -> TypeError {
        use TypeError::*;
        match self {
            WithSpan { wrapped, .. } => wrapped.unwrap(),
            other @ _ => other.clone(),
        }
    }

    /// Add a span to mark the location of the error. If it already has a span
    /// it does *not* overwrite the span.
    pub fn with_span(self, span: Span) -> Self {
        if self.span().is_some() {
            return self;
        }
        TypeError::WithSpan {
            wrapped: Box::new(self),
            span,
        }
    }

    pub fn into_stage_error(self, path: &PathBuf, source: &String) -> StageError {
        StageError::Type(self, path.clone(), source.clone())
    }
}

/// To safely handle recursive types we must register forward declarations
/// of types while closing them. That way when reclosing the same type we
/// return the forward declaration rather than actually closing again.
pub struct RecursionTracker(HashMap<usize, Type>);

impl RecursionTracker {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn check(&self, id: &usize) -> Option<Type> {
        if let Some(known) = self.0.get(id) {
            return Some(known.clone());
        }
        None
    }

    pub fn add(&mut self, id: usize, typ: Type) {
        self.0.insert(id, typ);
    }
}

/// Nodes/types which consume themselves to produce a new node/type where all
/// the types in themselves and their children have been closed.
trait Closable {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::parse_ast as past;
    use super::super::parser::{Location, Span, Token, Word};
    use super::scope::{ClosureScope, Scope};
    use super::{unify, Builtins, Closable, ScopeLike, Type, TypeError, Variable};

    fn new_scope() -> Scope {
        ClosureScope::new(None).into_scope()
    }

    #[test]
    fn test_unify_unbound() -> Result<(), TypeError> {
        // Check with unbound on the left.
        let scope = new_scope();
        let phantom = Type::new_phantom();
        let unbound = Type::new_unbound(scope.clone());
        unify(&unbound, &phantom, scope)?;
        assert_eq!(
            &phantom,
            unbound.unwrap_variable().borrow().unwrap_substitute()
        );

        // And with unbound on the right.
        let scope = new_scope();
        let phantom = Type::new_phantom();
        let unbound = Type::new_unbound(scope.clone());
        unify(&phantom, &unbound, scope)?;
        assert_eq!(
            &phantom,
            unbound.unwrap_variable().borrow().unwrap_substitute()
        );

        Ok(())
    }
}
