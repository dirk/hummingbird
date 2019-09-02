use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::sync::{Arc, RwLock};

lazy_static! {
    static ref SYMBOLICATOR: Symbolicator = Symbolicator::new();
}

pub fn symbolicate<S: Into<String>>(string: S) -> Symbol {
    (*SYMBOLICATOR).symbolicate(string)
}

pub fn desymbolicate<S: AsRef<Symbol>>(symbol: &S) -> Option<String> {
    (*SYMBOLICATOR).desymbolicate(symbol)
}

#[derive(Clone)]
struct Symbolicator(Arc<RwLock<InnerSymbolicator>>);

impl Symbolicator {
    fn new() -> Self {
        let inner = InnerSymbolicator::new();
        Self(Arc::new(RwLock::new(inner)))
    }

    pub fn symbolicate<S: Into<String>>(&self, string: S) -> Symbol {
        let string = string.into();
        {
            let reader = self.0.read().unwrap();
            if let Some(existing_id) = reader.symbols.get(&string) {
                return Symbol(*existing_id);
            }
        }
        let mut writer = self.0.write().unwrap();
        let next_id = writer.symbols.len() as u32;
        if next_id == UNINITIALIZED_ID {
            panic!("Exhausted symbol space");
        }
        writer.symbols.insert(string, next_id);
        Symbol(next_id)
    }

    pub fn desymbolicate<S: AsRef<Symbol>>(&self, symbol: &S) -> Option<String> {
        let reader = self.0.read().unwrap();
        let symbol = symbol.as_ref();
        if *symbol == Symbol::uninitialized() {
            panic!("Attempting to desymbolicate uninitialized symbol");
        }
        let id = symbol.0;
        for (key, value) in reader.symbols.iter() {
            if *value == id {
                return Some(key.clone());
            }
        }
        None
    }
}

type SymbolId = u32;

const UNINITIALIZED_ID: SymbolId = std::u32::MAX;

#[derive(Clone, Copy, PartialEq)]
pub struct Symbol(SymbolId);

impl Symbol {
    pub const fn uninitialized() -> Symbol {
        Symbol(UNINITIALIZED_ID)
    }

    pub fn id(&self) -> SymbolId {
        self.0
    }
}

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
        (0..10)
            .into_iter()
            .map(|_| {
                let movable = symbolicator.clone();
                thread::spawn(move || {
                    assert_eq!(movable.symbolicate("foo"), Symbol(0));
                })
            })
            .map(|handle| handle.join().unwrap())
            .for_each(drop);

        // Spawn 20 of threads that will all race to symbolicate and desymbolicate.
        (0..20)
            .into_iter()
            .map(|index| {
                let movable = symbolicator.clone();
                thread::spawn(move || {
                    if index % 2 == 1 {
                        assert_eq!(movable.symbolicate("foo"), Symbol(0));
                    } else {
                        assert_eq!(movable.desymbolicate(&Symbol(0)), Some("foo".to_string()));
                    }
                })
            })
            .map(|handle| handle.join().unwrap())
            .for_each(drop);
    }

    #[test]
    #[should_panic]
    fn it_doesnt_desymbolicate_unitialized() {
        let symbol = Symbol::uninitialized();
        let symbolicator = Symbolicator::new();
        symbolicator.desymbolicate(&symbol);
    }
}
