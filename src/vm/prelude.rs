use super::frame::Closure;
use super::value::Value;

pub fn build_prelude() -> Closure {
    let closure = Closure::new_builtins();

    let functions = vec![("println".to_owned(), prelude_println)];

    for (name, function) in functions.into_iter() {
        closure.set_directly(name, Value::make_native_function(function));
    }

    closure
}

fn prelude_println(arguments: Vec<Value>) -> Value {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Boolean(value) => println!("{:?}", value),
            Value::Function(_) => println!("Function"),
            Value::Integer(value) => println!("{}", value),
            Value::Null => println!("null"),
            _ => unreachable!(),
        }
    };
    Value::Null
}
