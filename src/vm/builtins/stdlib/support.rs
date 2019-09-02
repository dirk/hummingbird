use std::cell::UnsafeCell;
use std::mem;
use std::ops::Deref;

use super::super::super::errors::VmError;
use super::super::super::symbol::{symbolicate, Symbol};
use super::super::super::value::{BuiltinObject, Value};

/// This is a slightly-unsafe utility type to make it easier to declare static
/// symbols in a native module.
///
/// ```
/// static READ: StaticSymbol = StaticSymbol::new();
///
/// pub fn load() -> LoadedModule {
///   READ.initialize("read");
/// }
///
/// fn my_other_function() {
///   *READ // Will be a `&Symbol`.
/// }
/// ```
///
/// If you deref a `StaticSymbol` before it is initialized you will get an
/// uninitialized symbol.
pub struct StaticSymbol {
    cell: UnsafeCell<Symbol>,
}

impl StaticSymbol {
    pub const fn new() -> Self {
        Self {
            cell: UnsafeCell::new(Symbol::uninitialized()),
        }
    }

    pub fn initialize(&self, value: &str) {
        let cell = self.cell.get();
        unsafe {
            *cell = symbolicate(value);
        }
    }
}

impl Deref for StaticSymbol {
    type Target = Symbol;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.get() }
    }
}

// Mark it as safe to share between threads.
unsafe impl Sync for StaticSymbol {}

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
    use super::super::super::super::symbol::{symbolicate, Symbol};
    use super::StaticSymbol;

    #[test]
    fn it_begins_uninitialized_and_can_be_initialized() {
        let static_symbol = StaticSymbol::new();
        assert_eq!(*static_symbol, Symbol::uninitialized());

        static_symbol.initialize("initialized");
        assert_eq!(*static_symbol, symbolicate("initialized"));
    }
}
