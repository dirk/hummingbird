use std::fs::File;
use std::io::Read;

use super::super::super::errors::VmError;
use super::super::super::gc::{GcAllocator, GcPtr};
use super::super::super::loader::LoadedModule;
use super::super::super::value::{BuiltinMethodFn, BuiltinObject, Value};
use super::expect::*;

pub fn load() -> LoadedModule {
    let module = LoadedModule::builtin("file".to_string());
    module.set_export("open", Value::make_builtin_function(open));
    module
}

fn open(arguments: Vec<Value>, gc: &mut GcAllocator) -> Result<Value, VmError> {
    expect_len!(&arguments, 1);
    let path = expect_type!(&arguments[0], Value::String(string) => &**string);
    let file = File::open(path).unwrap();
    // Make the GC take ownership of the file handle.
    let file = gc.allocate(file);
    Ok(Value::BuiltinObject(BuiltinObject::File(file, method_lut)))
}

fn method_lut(_this: &GcPtr<File>, value: &str) -> Option<BuiltinMethodFn> {
    match value {
        "read" => Some(method_read),
        _ => None,
    }
}

fn method_read(this: Value, arguments: Vec<Value>, gc: &mut GcAllocator) -> Result<Value, VmError> {
    expect_len!(&arguments, 0);
    let this = expect_builtin_object(this)?;
    let mut file = expect_type!(this, BuiltinObject::File(file, _) => file);

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .expect("Unable to read file");

    Ok(Value::make_string(buffer, gc))
}
