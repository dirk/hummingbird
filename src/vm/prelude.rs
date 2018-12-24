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

fn prelude_println(arguments: Vec<Value>) -> Value {
    if let Some(argument) = arguments.first() {
        match argument {
            Value::Integer(value) => println!("{}", value),
            _ => unreachable!(),
        }
    };
    Value::Null
}
