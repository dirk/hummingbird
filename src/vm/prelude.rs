use super::loader::LoadedModule;
use super::value::Value;

pub fn build_prelude() -> LoadedModule {
    let module = LoadedModule::empty();
    {
        let mut module = module.borrow_mut();
        module.add_named_export("println", Value::make_native_function(prelude_println));
    }
    module
}

pub fn is_in_prelude<N: AsRef<str>>(name: N) -> bool {
    match name.as_ref() {
        "println" => true,
        _ => false,
    }
}

fn prelude_println(arguments: Vec<Value>) -> Value {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Integer(value) => println!("{}", value),
            _ => unreachable!(),
        }
    };
    Value::Null
}
