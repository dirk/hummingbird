use super::vm::{BuiltinFunction, Function};

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    BuiltinFunction(BuiltinFunction),
    Function(Function),
    Integer(i64),
    String(String),
}

impl From<BuiltinFunction> for Value {
    fn from(builtin_function: BuiltinFunction) -> Self {
        Self::BuiltinFunction(builtin_function)
    }
}
