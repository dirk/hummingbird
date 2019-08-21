use std::collections::HashSet;

use super::frame::Closure;
use super::value::Value;
use super::vm::BuiltinFunction;

pub fn build() -> Closure {
    let functions = vec![("println".to_string(), builtin_println)];

    let mut bindings = HashSet::new();
    for (name, _) in functions.iter() {
        bindings.insert(name.clone());
    }

    let closure = Closure::new(Some(bindings), None);
    for (name, value) in functions.into_iter() {
        closure.set(name.clone(), BuiltinFunction::new(name, value).into());
    }
    closure
}

fn builtin_println(arguments: Vec<Value>) -> Value {
    for argument in arguments.into_iter() {
        use Value::*;
        match argument {
            BuiltinFunction(builtin_function) => println!("{:?}", builtin_function),
            Integer(value) => println!("{}", value),
            other @ _ => println!("{:?}", other),
        };
    }
    Value::Null
}
