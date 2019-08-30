use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref SYMBOLICATOR: Symbolicator = Symbolicator::new();
}

#[derive(Clone)]
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
            return Symbol(*existing_id);
        }
        let next_id = inner.symbols.len() as u32;
        inner.symbols.insert(string, next_id);
        Symbol(next_id)
    }

    pub fn desymbolicate<S: AsRef<Symbol>>(&self, symbol: &S) -> Option<String> {
        let inner = self.0.lock().unwrap();
        let id = symbol.as_ref().0;
        for (key, value) in inner.symbols.iter() {
            if *value == id {
                return Some(key.clone());
            }
        }
        None
    }
}

type SymbolId = u32;

#[derive(Clone, PartialEq)]
pub struct Symbol(SymbolId);

impl AsRef<Symbol> for Symbol {
    fn as_ref(&self) -> &Symbol {
        &self
    }
}

#[cfg(not(test))]
impl Debug for Symbol {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_tuple("Symbol")
            .field(&self.0)
            .field(&(*SYMBOLICATOR).desymbolicate(&self))
            .finish()
    }
}

#[cfg(test)]
impl Debug for Symbol {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_tuple("Symbol").field(&self.0).finish()
    }
}

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

#[cfg(test)]
mod tests {
    use std::thread;

    use super::{Symbol, Symbolicator};
    use std::io::Write;

    #[test]
    fn it_symbolicates_and_desymbolicates() {
        let symbolicator = Symbolicator::new();
        let symbol = symbolicator.symbolicate("foo");
        assert_eq!(symbol, Symbol(0));
        assert_eq!(symbolicator.desymbolicate(&symbol), Some("foo".to_string()),);
        assert_eq!(symbolicator.desymbolicate(&Symbol(1)), None);
    }

    #[test]
    fn it_symbolicates_thread_safely() {
        let symbolicator = Symbolicator::new();

        // Spawn 10 of threads that will all race to symbolicate.
        let join_handles = (0..10).iter();
        for _ in 0..10 {
            let movable = symbolicator.clone();
            thread::spawn(move || {
                assert_eq!(movable.symbolicate("foo"), Symbol(0));
            });
        }

        // Spawn 10 of threads that will all race to desymbolicate.
        for _ in 0..10 {
            let movable = symbolicator.clone();
            thread::spawn(move || {
                assert_eq!(movable.desymbolicate(&Symbol(0)), Some("foo".to_string()));
            });
        }
    }
}
