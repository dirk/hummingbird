use super::super::super::errors::VmError;
use super::super::super::value::{BuiltinObject, Value};

/// Takes the result of `stringify!` on a pattern and returns just the variant.
///
/// eg. "Value::String(string)" => "String"
pub fn variant_from_pat(pat: &str) -> &str {
    let left = pat.rfind("::").expect("No \"::\" separator") + 2;
    match pat.find("(") {
        Some(right) => &pat[left..right],
        None => &pat[left..],
    }
}

macro_rules! expect_len {
    ($e:expr, $l:expr) => {
        match ($e.len(), $l) {
            ($l, $l) => (),
            (got, expected) => {
                return Err(VmError::new_argument_error(format!(
                    "Expected {} arguments, got {}",
                    expected, got,
                )))
            }
        }
    };
}

macro_rules! expect_type {
    ($e:expr, $p:pat => $m:expr) => {
        match $e {
            $p => $m,
            other @ _ => {
                return Err(VmError::new_argument_error(format!(
                    "Expected {} got {:?}",
                    variant_from_pat(stringify!($p)),
                    other,
                )))
            }
        }
    };
}

/// Shorthand for expecting a type to be a builtin object.
#[inline]
pub fn expect_builtin_object(value: Value) -> Result<BuiltinObject, VmError> {
    let object = expect_type!(value, Value::BuiltinObject(object) => object);
    Ok(*object)
}
