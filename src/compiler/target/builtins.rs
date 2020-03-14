use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::module::{Module, Linkage};
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::builder::Builder;

use super::super::opaque::OpaqueModule;

pub fn initialize(ctx: &Context) -> Module {
    let data_ir = include_bytes!("builtins.ll");
    let memory_buffer = MemoryBuffer::create_from_memory_range_copy(data_ir, "builtins");
    match ctx.create_module_from_ir(memory_buffer) {
        Ok(module) => module,
        Err(string) => {
            panic!("Error initializing builtins module from LLVM IR:\n{}", string.to_string())
        }
    }
}

pub fn get_builtin_func<'ctx>(
    module: &Module<'ctx>,
    name: &str,
    arguments: &Vec<BasicValueEnum>,
) -> FunctionValue<'ctx> {
    // Uncomment to see the layout of the module we've loaded:
    //   module.print_to_stderr();
    let function_name = match name {
        "println" => get_println_specialization(arguments),
        _ => unreachable!("Invalid builtin func name: {}", name),
    };
    module.get_function(function_name).unwrap()
}

pub fn get_println_specialization(arguments: &Vec<BasicValueEnum>) -> &'static str {
    if arguments.len() != 1 {
        unreachable!("println accepts 1 argument, got {}", arguments.len())
    }
    let argument = &arguments[0];
    match argument {
        BasicValueEnum::IntValue(_) => "builtin_println_int64",
        _ => {
            let typ = argument.get_type();
            unreachable!("println cannot be called with: {:?}", typ)
        }
    }
}

// The following implementation will work if we DON'T link the builtins module
// into the main module at the beginning of compilation; not doing so implies
// we will link in the builtins object file when building the binary:
//
//   pub fn get_builtin_func<'ctx>(
//       module: &Module<'ctx>,
//       builtins_module: &Module<'ctx>,
//       name: &str,
//   ) -> FunctionValue<'ctx> {
//       if let Some(already_defined) = module.get_function(name) {
//           return already_defined
//       }
//       // Uncomment to see the layout of the module we've loaded:
//       //   builtins_module.print_to_stderr();
//       let builtin_name = match name {
//           "println" => "builtin_println_int64",
//           _ => unreachable!("Invalid builtin func name: {}", name),
//       };
//       let typ = builtins_module.get_function(builtin_name).unwrap().get_type();
//       module.add_function(builtin_name, typ, Some(Linkage::AvailableExternally))
//   }
