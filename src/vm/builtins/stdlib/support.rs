use std::sync::atomic::{AtomicU32, Ordering};

use super::super::super::errors::VmError;
use super::super::super::symbol::{symbolicate, Symbol, UNINITIALIZED_ID};
use super::super::super::value::{BuiltinObject, Value};

/// This is a utility type to make it easier to declare static symbols in a
/// native module.
///
/// ```
/// static READ: StaticSymbol = StaticSymbol::new("read");
///
/// fn later() {
///   READ.get() // Will be a `Symbol`.
/// }
/// ```
///
/// It uses an `AtomicU32` under the hood to keep track of the symbol ID. It
/// starts out uninitialized, and on the first get it will symbolicate its
/// value and store the resulting symbol ID.
pub struct StaticSymbol {
    id: AtomicU32,
    value: &'static str,
}

impl StaticSymbol {
    pub const fn new(value: &'static str) -> Self {
        Self {
            id: AtomicU32::new(UNINITIALIZED_ID),
            value,
        }
    }

    /// If this static symbol hasn't been initialized it will symbolicate the
    /// value and store it in the `id` field.
    #[inline]
    pub fn get(&self) -> Symbol {
        let mut id = self.id.load(Ordering::Relaxed);
        if id == UNINITIALIZED_ID {
            let symbol = symbolicate(self.value);
            id = symbol.id();
            self.id.store(id, Ordering::Relaxed);
        }
        Symbol::new(id)
    }
}

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

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::super::super::super::symbol::{symbolicate, UNINITIALIZED_ID};
    use super::StaticSymbol;

    #[test]
    fn it_begins_uninitialized_and_can_be_initialized() {
        let static_symbol = StaticSymbol::new("initialized");
        assert_eq!(static_symbol.id.load(Ordering::SeqCst), UNINITIALIZED_ID);
        assert_eq!(static_symbol.get(), symbolicate("initialized"));
    }
}
