use std::fs::File;

use super::super::super::errors::VmError;
use super::super::super::gc::GcAllocator;
use super::super::super::loader::LoadedModule;
use super::super::super::value::{BuiltinObject, Value};
use super::expect::*;

pub fn load() -> LoadedModule {
    let module = LoadedModule::builtin("file".to_string());
    module.set_export("open", Value::make_builtin_function(open));
    module
}

fn open(arguments: Vec<Value>, gc: &mut GcAllocator) -> Result<Value, VmError> {
    expect_arg_len!(&arguments, 1);
    let path = expect_arg_type!(&arguments[0], Value::String(string) => &**string);
    let file = File::open(path).unwrap();
    // Make the GC take ownership of the file handle.
    let file = gc.allocate(file);
    Ok(Value::BuiltinObject(BuiltinObject::File(file)))
}
