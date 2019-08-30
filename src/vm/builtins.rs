use super::frame::Closure;
use super::symbol::desymbolicate;
use super::value::Value;

pub fn build() -> Closure {
    let closure = Closure::new_builtins();

    let functions = vec![("println".to_owned(), builtin_println)];

    for (name, function) in functions.into_iter() {
        closure.set_directly(name, Value::make_builtin_function(function));
    }

    closure
}

fn builtin_println(arguments: Vec<Value>) -> Value {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Boolean(value) => println!("{:?}", value),
            Value::Function(function) => {
                println!("Function({})", function.loaded_function.qualified_name())
            }
            Value::Integer(value) => println!("{}", value),
            Value::Module(module) => println!("Module({})", module.name()),
            Value::Null => println!("null"),
            Value::String(value) => {
                let string = &**value;
                println!("{}", string)
            }
            Value::Symbol(symbol) => {
                let string = desymbolicate(symbol).expect("Symbol not found");
                println!(":{}", string)
            }
            _ => unreachable!(),
        }
    };
    Value::Null
}