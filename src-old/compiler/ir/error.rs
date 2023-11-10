use super::Type;

#[derive(Debug)]
pub enum IrError {
    TypeMismatch { expected: Type, got: Type },
}
