use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use inkwell::context::Context;
use inkwell::module::Module as InkModule;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicType, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, OptimizationLevel};

use super::ir::{
    self as ir, Func, FuncId, FuncValue, Instruction, Module, StaticValue, Value, ValueId,
};

mod builtins;

/// Discover all of the funcs in the modules.
fn collect_all_func_values(modules: &Vec<Module>) -> Vec<FuncValue> {
    let mut funcs = vec![];
    for module in modules.iter() {
        funcs.extend(collect_module_func_values(module));
    }
    funcs
}

fn collect_module_func_values(module: &Module) -> Vec<FuncValue> {
    let mut funcs = vec![];
    for func in module.borrow_funcs().iter() {
        funcs.extend(collect_func_func_values(func));
    }
    funcs
}

fn collect_func_func_values(func: &Func) -> Vec<FuncValue> {
    let mut funcs = vec![];
    let specializations = func.borrow_specializations();
    for specialization in specializations.iter() {
        funcs.push(specialization.clone());
        // Recursively collect funcs defined within this specialization.
        let inner_funcs = specialization.borrow_funcs();
        for inner_func in inner_funcs.iter() {
            funcs.extend(collect_func_func_values(inner_func));
        }
    }
    funcs
}

struct TypeTracker<'ctx> {
    ctx: &'ctx Context,
    real_types: Vec<(ir::RealType, BasicTypeEnum<'ctx>)>,
}

impl<'ctx> TypeTracker<'ctx> {
    fn new(ctx: &'ctx Context) -> Self {
        Self {
            ctx,
            real_types: vec![],
        }
    }

    fn get_type<'a>(&'a mut self, real_type: &ir::RealType) -> BasicTypeEnum<'ctx> {
        use ir::RealType::*;
        for (cached_real_typ, typ) in self.real_types.iter() {
            if cached_real_typ.is_equal(real_type) {
                return typ.clone();
            }
        }
        let typ: BasicTypeEnum = match real_type {
            FuncPtr(func_ptr_type) => {
                let parameters = func_ptr_type
                    .parameters
                    .iter()
                    .map(|real_type| self.get_type(real_type))
                    .collect::<Vec<_>>();
                let retrn = self.get_type(&*func_ptr_type.retrn);
                let function_type: FunctionType = retrn.fn_type(parameters.as_slice(), false);
                function_type.ptr_type(AddressSpace::Generic).into()
            }
            Int64 => self.ctx.i64_type().into(),
            // FIXME: Actually build tuple types.
            Tuple(_) => self.ctx.i64_type().into(),
        };
        self.real_types.push((real_type.clone(), typ.clone()));
        typ
    }
}

pub fn compile_modules(modules: &Vec<Module>) {
    let ctx = Context::create();
    let module = ctx.create_module("main");

    // Initialize the builtins LLVM module and link it into this one.
    module.link_in_module(builtins::initialize(&ctx)).unwrap();

    let mut type_tracker = TypeTracker::new(&ctx);
    let mut function_tracker = HashMap::new();

    let funcs = collect_all_func_values(modules);
    // Forward-define all of the functions.
    for func in funcs.iter() {
        let name = if func.is_main() {
            "main"
        } else {
            func.get_qualified_name()
        };
        let parameters = func
            .get_parameters()
            .iter()
            .map(|(name, real_type)| type_tracker.get_type(real_type))
            .collect::<Vec<_>>();
        let retrn = type_tracker.get_type(&func.get_retrn());
        let function_type = retrn.fn_type(parameters.as_slice(), false);
        let function_value = module.add_function(name, function_type, None);
        function_tracker.insert(func.id(), function_value);
    }

    // Then build the function implementations.
    let builder = ctx.create_builder();
    for func in funcs.iter() {
        let function_value = function_tracker
            .get(&func.id())
            .expect("Function not defined");

        // Map IR basic block IDs to LLVM basic blocks.
        let mut basic_block_tracker = HashMap::new();

        let ir_basic_block_manager = func.borrow_basic_blocks();
        let ir_basic_blocks = ir_basic_block_manager.basic_blocks.borrow();
        if ir_basic_blocks.is_empty() {
            panic!("Cannot compile an empty function");
        }
        let mut first_basic_block = None;
        // Forward-define the basic blocks so that we can resolve indices later
        // when translating IR branches into LLVM branches.
        for (index, ir_basic_block) in ir_basic_blocks.iter().enumerate() {
            let basic_block = ctx.append_basic_block(function_value.clone(), &ir_basic_block.name);
            basic_block_tracker.insert(ir_basic_block.get_index(), basic_block.clone());
            if index == 0 {
                first_basic_block = Some(basic_block);
            }
        }

        // Resolve IR values (local SSA and statics) to LLVM values.
        let value_resolver = ValueResolver::new(&ctx, &function_tracker);
        // Map local indices to LLVM stack pointer values.
        let mut local_tracker = HashMap::new();

        // Start by allocating locals and copying parameters.
        let entry_basic_block = ctx.prepend_basic_block(first_basic_block.unwrap(), "entry");
        builder.position_at_end(entry_basic_block);
        for (index, (name, real_type)) in func.get_stack_frame().iter().enumerate() {
            let typ = type_tracker.get_type(real_type);
            let ptr = builder.build_alloca(typ, name);
            local_tracker.insert(index, ptr);
            // Search for a parameter matching this local's name and build a
            // move if it does.
            for (parameter_index, (parameter_name, parameter_real_type)) in
                func.get_parameters().iter().enumerate()
            {
                if parameter_name != name {
                    continue;
                }
                if !parameter_real_type.is_equal(real_type) {
                    panic!(
                        "Type of parameter '{}' ({:?}) does not type in stack frame for '{}' ({:?})",
                        parameter_name,
                        parameter_real_type,
                        name,
                        real_type,
                    );
                }
                let value = function_value
                    .get_nth_param(parameter_index as u32)
                    .expect("Missing parameter");
                builder.build_store(ptr, value);
            }
        }
        builder.build_unconditional_branch(first_basic_block.unwrap());

        for ir_basic_block in ir_basic_blocks.iter() {
            let basic_block = basic_block_tracker
                .get(&ir_basic_block.get_index())
                .unwrap()
                .clone();
            builder.position_at_end(basic_block);

            for instruction in ir_basic_block.instructions.iter() {
                use Instruction::*;
                match instruction {
                    CallBuiltinFunc(ir_retrn, builtin_func_name, ir_arguments) => {
                        let arguments = ir_arguments
                            .iter()
                            .map(|ir_argument| value_resolver.get(ir_argument))
                            .collect::<Vec<_>>();
                        let function_value =
                            builtins::get_builtin_func(&module, builtin_func_name, &arguments);
                        let call_site =
                            builder.build_call(function_value, arguments.as_slice(), "println");
                        let retrn = call_site.try_as_basic_value().left().unwrap();
                        value_resolver.set(ir_retrn, retrn);
                    }
                    CallFunc(ir_retrn, func_value, ir_arguments) => {
                        let arguments = ir_arguments
                            .iter()
                            .map(|ir_argument| value_resolver.get(ir_argument))
                            .collect::<Vec<_>>();
                        let function_value = function_tracker
                            .get(&func_value.id())
                            .expect("Function not defined")
                            .clone();
                        let call_site =
                            builder.build_call(function_value, arguments.as_slice(), "");
                        let retrn = call_site.try_as_basic_value().left().unwrap();
                        value_resolver.set(ir_retrn, retrn);
                    }
                    CallFuncPtr(ir_retrn, ir_value, ir_arguments) => {
                        let arguments = ir_arguments
                            .iter()
                            .map(|ir_argument| value_resolver.get(ir_argument))
                            .collect::<Vec<_>>();
                        // `into_pointer_value` will panic if it's not a
                        // function pointer, so we need to be sure that it's
                        // going to be one.
                        let value = value_resolver
                            .get(&Value::Local(ir_value.clone()))
                            .into_pointer_value();
                        let call_site = builder.build_call(value, arguments.as_slice(), "");
                        let retrn = call_site.try_as_basic_value().left().unwrap();
                        value_resolver.set(ir_retrn, retrn);
                    }
                    GetLocal(ir_value, index) => {
                        let ptr = local_tracker.get(index).expect("Local not defined").clone();
                        let value = builder.build_load(ptr, "");
                        value_resolver.set(ir_value, value);
                    }
                    Return(ir_value) => {
                        let value = value_resolver.get(ir_value);
                        builder.build_return(Some(&value));
                    }
                }
            }
        }
    }

    let print_to_stderr = true;
    if print_to_stderr {
        module.print_to_stderr();
    }

    generate_module(module);
}

fn generate_module(module: InkModule) {
    // Set up the paths we'll emit to.
    let object = Path::new("./build/out.o");
    let executable = Path::new("./build/out");

    // let optimization_level = OptimizationLevel::Aggressive;
    let optimization_level = OptimizationLevel::Default;
    let reloc_mode = RelocMode::Default;
    let code_model = CodeModel::Default;
    Target::initialize_x86(&InitializationConfig::default());
    let target_triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&target_triple).unwrap();
    let target_machine = target
        .create_target_machine(
            &target_triple,
            TargetMachine::get_host_cpu_name().to_str().unwrap(),
            TargetMachine::get_host_cpu_features().to_str().unwrap(),
            optimization_level,
            reloc_mode,
            code_model,
        )
        .unwrap();

    target_machine
        .write_to_file(&module, FileType::Object, &object)
        .unwrap();

    // Link the object file into an executable.
    let output = Command::new("clang")
        .args(&[object.to_str().unwrap(), "-o", executable.to_str().unwrap()])
        .output()
        .unwrap();

    if !output.status.success() {
        println!(
            "Failed to invoke to Clang (exited with {:?})",
            output.status.code()
        );
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
    }
}

struct ValueResolver<'ctx> {
    ctx: &'ctx Context,
    // Used to look up static function values.
    function_tracker: &'ctx HashMap<FuncId, FunctionValue<'ctx>>,
    // Store and look up local SSA values.
    local_tracker: RefCell<HashMap<ValueId, BasicValueEnum<'ctx>>>,
}

impl<'ctx> ValueResolver<'ctx> {
    fn new(
        ctx: &'ctx Context,
        function_tracker: &'ctx HashMap<FuncId, FunctionValue<'ctx>>,
    ) -> Self {
        Self {
            ctx,
            function_tracker,
            local_tracker: RefCell::new(HashMap::new()),
        }
    }

    fn get(&self, ir_value: &ir::Value) -> BasicValueEnum {
        // Handle constant values that haven't actually been stored.
        if let ir::Value::Local(local_value) = ir_value {
            match local_value {
                ir::LocalValue::Int64(_, Some(const_value)) => {
                    return self.ctx.i64_type().const_int(*const_value, false).into();
                }
                // Empty tuples are constant too.
                ir::LocalValue::Tuple(_, tuple_type) => {
                    if tuple_type.members.is_empty() {
                        // FIXME: Actually build the right tuple value (see TypeTracker).
                        return self.ctx.i64_type().const_int(0, false).into();
                    }
                }
                _ => (),
            }
        }
        // Handle statics by resolving them to global values and returning
        // a pointer to the global value.
        if let ir::Value::Static(static_value) = ir_value {
            return match static_value {
                ir::StaticValue::Func(func_value) => {
                    let function_value = self
                        .function_tracker
                        .get(&func_value.id())
                        .expect("Function not defined")
                        .clone();
                    let global_value = function_value.as_global_value();
                    global_value.as_pointer_value().into()
                }
            };
        }
        let id = ir_value.value_id();
        let tracker = self.local_tracker.borrow();
        tracker
            .get(&id)
            .expect(&format!("Missing value: {:?} (tracker: {:?})", id, tracker))
            .clone()
    }

    fn set(&self, ir_value: &ir::Value, value: BasicValueEnum<'ctx>) {
        let mut tracker = self.local_tracker.borrow_mut();
        tracker.insert(ir_value.value_id(), value.clone());
    }
}
