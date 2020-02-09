use std::collections::HashMap;
use std::sync::Arc;

use super::typ::{next_uid, Class, IntrinsicClass};

pub struct Builtins(Arc<HashMap<String, Class>>);

lazy_static! {
    static ref BUILTINS: Builtins = {
        let mut builtins = HashMap::new();

        let mut instrinsic = |name: &str| {
            let class = IntrinsicClass {
                id: next_uid(),
                name: name.to_string(),
            };
            builtins.insert(name.to_string(), Class::Intrinsic(Arc::new(class)));
        };
        instrinsic("Int");

        Builtins(Arc::new(builtins))
    };
}

impl Builtins {
    pub fn get<S: AsRef<str>>(name: S) -> Class {
        let builtins = &BUILTINS.0;
        builtins
            .get(name.as_ref())
            .expect(&format!("Builtin not found: {}", name.as_ref()))
            .clone()
    }
}
