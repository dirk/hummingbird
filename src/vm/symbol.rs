use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::borrow::Borrow;

lazy_static! {
    static ref SYMBOLICATOR: Symbolicator = Symbolicator::new();
}

struct Symbolicator(Arc<Mutex<InnerSymbolicator>>);

impl Symbolicator {
    fn new() -> Self {
        let inner = InnerSymbolicator::new();
        Self(Arc::new(Mutex::new(inner)))
    }

    pub fn symbolicate<S: Into<String>>(&self, string: S) -> Symbol {
        let string = string.into();
        let mut inner = self.0.lock().unwrap();
        if let Some(existing_id) = inner.symbols.get(&string) {
            return Symbol(*existing_id)
        }
        let next_id = inner.symbols.len() as u32;
        inner.symbols.insert(string, next_id);
        Symbol(next_id)
    }
}

type SymbolId = u32;

pub struct Symbol(SymbolId);

// Trick trait to enforce that the `Symbolicator` is thread-safe.
trait MustBeSendAndSync: Send + Sync {}

impl MustBeSendAndSync for Symbolicator {}

struct InnerSymbolicator {
    symbols: HashMap<String, SymbolId>,
}

impl InnerSymbolicator {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }
}
