use super::super::super::type_ast::{self as ast, ScopeId};
use super::{RealType, Value};

pub trait Frame {
    fn get_local(&self, name: &str) -> (usize, RealType);

    /// Search for a static in the scope matching `scope_id`, which should be
    /// either this frame or one if its parents.
    ///
    /// This MUST NOT return a `Value::Local`.
    fn get_static(&self, name: &str, scope_id: ScopeId) -> Value;
}
