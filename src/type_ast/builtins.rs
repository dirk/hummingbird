use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::scope::{FuncScope, Scope, ScopeLike};
use super::typ::{next_uid, Class, Func, Generic, IntrinsicClass, Type};

pub struct Builtins(BuiltinsInner);

struct BuiltinsInner {
    classes: HashMap<String, Class>,
    funcs: HashMap<String, Rc<Func>>,
}

fn new_generic(scope: Scope) -> Type {
    Type::Generic(Rc::new(RefCell::new(Generic::new(scope))))
}

lazy_static! {
    static ref BUILTINS: Builtins = {
        let mut classes = HashMap::new();
        let mut add_class = |name: &str| {
            let class = IntrinsicClass {
                id: next_uid(),
                name: name.to_string(),
            };
            classes.insert(name.to_string(), Class::Intrinsic(Rc::new(class)));
        };

        add_class("Int");

        let mut funcs = HashMap::new();
        let mut add_func = |name: &str, arguments: Vec<Type>, retrn: Type, scope: Scope| {
            let func = Func {
                id: next_uid(),
                scope,
                name: Some(name.to_string()),
                arguments: RefCell::new(arguments),
                retrn: RefCell::new(retrn),
            };
            funcs.insert(name.to_string(), Rc::new(func));
        };

        let scope = FuncScope::new(None).into_scope();
        add_func(
            "println",
            vec![new_generic(scope.clone())],
            Type::new_unit(scope.clone()),
            scope,
        );

        Builtins(BuiltinsInner { classes, funcs })
    };
}

impl Builtins {
    pub fn get_class<S: AsRef<str>>(name: S) -> Class {
        let classes = &BUILTINS.0.classes;
        classes
            .get(name.as_ref())
            .expect(&format!("Builtin not found: {}", name.as_ref()))
            .clone()
    }

    // NOTE: Only supports funcs right now.
    pub fn try_get<S: AsRef<str>>(name: S) -> Option<Type> {
        let funcs = &BUILTINS.0.funcs;
        funcs.get(name.as_ref()).map(|typ| Type::Func(typ.clone()))
    }

    pub fn get_all_classes() -> &'static HashMap<String, Class> {
        &BUILTINS.0.classes
    }
}

// Not actually safe to share between threads, but we're not currently
// multi-threaded so it's okay.
unsafe impl Send for Builtins {}
unsafe impl Sync for Builtins {}
