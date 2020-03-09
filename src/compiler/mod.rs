use std::collections::HashMap;
use std::rc::Rc;

use inkwell::context::Context as InkContext;
use inkwell::module::Module as InkModule;
use inkwell::types::{AnyType, BasicType, FunctionType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::AddressSpace;

pub mod ir;
mod opaque;
mod path_to_name;
pub mod target;
pub mod value_ir;
mod vecs_equal;

use opaque::*;

enum Type {
    Int64(OpaqueIntType),
    Func(OpaqueFunctionType),
}

impl Type {
    fn get_ink_any_type<'ctx>(&self) -> Box<dyn AnyType<'ctx> + 'ctx> {
        match self {
            Type::Int64(typ) => Box::new(typ.open().clone()),
            Type::Func(typ) => Box::new(typ.open().clone()),
        }
    }

    fn get_ink_basic_type<'ctx>(&self) -> Box<dyn BasicType<'ctx> + 'ctx> {
        match self {
            Type::Int64(typ) => Box::new(typ.open().clone()),
            Type::Func(typ) => {
                let typ: &FunctionType = typ.open();
                Box::new(typ.ptr_type(AddressSpace::Generic))
            }
        }
    }
}

// fn to_basic_type<'ctx>(typ: &'ctx dyn AnyType<'ctx>) -> Box<dyn BasicType<'ctx> + 'ctx> {
//     let typ_enum = typ.as_any_type_enum();
//     if typ_enum.is_float_type() {
//         Box::new(typ_enum.into_float_type())
//     } else if typ_enum.is_int_type() {
//         Box::new(typ_enum.into_int_type())
//     } else {
//         unreachable!("Cannot convert to a basic type: {:?}", typ)
//     }
// }

/// Converts IR types into native types; caches conversions so that we're
/// not flooding LLVM with types.
struct Shaper {
    int64_type: OpaqueIntType,
    func_types: Vec<(Rc<ir::FuncType>, OpaqueFunctionType)>,
    // cache: Vec<(ir::Type)>,
}

impl Shaper {
    fn get_type(&mut self, typ: &ir::Type) -> Type {
        match typ {
            ir::Type::Int64 => Type::Int64(self.int64_type.clone()),
            ir::Type::Func(func_type) => Type::Func(self.get_func_type(func_type)),
            other @ _ => unreachable!("Cannot shape type: {:?}", other),
        }
    }

    fn get_func_type(&mut self, func_type: &Rc<ir::FuncType>) -> OpaqueFunctionType {
        // Rewrap so that we can call comparison method.
        let typ = ir::Type::Func(func_type.clone());
        for (existing_func, opaque) in self.func_types.iter() {
            let existing = ir::Type::Func(existing_func.clone());
            if existing.shape_equals(&typ) {
                return opaque.clone();
            }
        }
        let mut arguments = vec![];
        for argument in func_type.arguments.iter() {
            arguments.push(self.get_type(argument));
        }
        let retrn = self.get_type(&func_type.retrn);

        // Convert to LLVM types and build the function.
        let argument_basic_types = arguments
            .iter()
            .map(|argument| argument.get_ink_basic_type().as_basic_type_enum())
            .collect::<Vec<_>>();
        let retrn_basic_type = retrn.get_ink_basic_type().as_basic_type_enum();
        let ink_func_type = retrn_basic_type.fn_type(argument_basic_types.as_slice(), false);

        // Make it opaque and save it.
        let typ = OpaqueFunctionType::close(ink_func_type);
        self.func_types.push((func_type.clone(), typ.clone()));
        typ
    }
}

struct Context<'ctx> {
    ink_ctx: &'ctx InkContext,
    ink_module: InkModule<'ctx>,
    shaper: &'ctx mut Shaper,
    // Keep track of every function we've built so that we can fetch the
    // values later when calling them.
    funcs: HashMap<String, OpaqueFunctionValue>,
}

fn define_func(ctx: &mut Context, func: &Rc<ir::FuncValue>) {
    let typ = ctx.shaper.get_func_type(&func.typ);
    let value = ctx
        .ink_module
        .add_function(&func.name, typ.open().clone(), None);
    ctx.funcs
        .insert(func.name.clone(), OpaqueFunctionValue::close(value));
}

fn compile_func(ctx: &mut Context, func: &Rc<ir::FuncValue>) {
    let value: &FunctionValue = ctx.funcs.get(&func.name).unwrap().open();

    // TODO: Basic block tracker to build jumps and such.

    let builder = ctx.ink_ctx.create_builder();

    // Keep track of stack frame slot pointers.
    let mut frame_slots = HashMap::new();

    for (index, ir_bb) in func.basic_blocks.borrow().iter().enumerate() {
        let ink_bb = ctx.ink_ctx.append_basic_block(value.clone(), &ir_bb.name);
        builder.position_at_end(ink_bb);

        // If we're building the first block then start by allocating stack
        // frame slots.
        if index == 0 {
            for (name, typ) in func.stack_frame.borrow().iter() {
                // FIXME: Don't store functions in the IR frame.
                if typ.is_unspecialized() {
                    continue;
                }
                let typ = ctx.shaper.get_type(typ);
                let basic_type = typ.get_ink_basic_type().as_basic_type_enum();
                let ptr = builder.build_alloca(basic_type, name);
                frame_slots.insert(name.clone(), OpaquePointerValue::close(ptr));
            }
        }

        // Map IR SSA values to LLVM SSA values.
        let mut value_tracker = ValueTracker::new(ctx);

        for ir_instruction in ir_bb.instructions.iter() {
            use ir::Instruction::*;
            match ir_instruction {
                Call(ir_retrn, ir_target, ir_arguments) => {
                    let arguments = ir_arguments
                        .iter()
                        .map(|ir_argument| value_tracker.get(ir_argument))
                        .collect::<Vec<_>>();

                    let call_site = match ir_target {
                        ir::Value::Func(func_value) => {
                            let target: &FunctionValue = ctx
                                .funcs
                                .get(&func_value.name)
                                .expect(&format!("Func not defined: {:?}", &func.name))
                                .open();
                            builder.build_call(target.clone(), arguments.as_slice(), "")
                        }
                        other @ _ => panic!("Cannot build call to Value: {:?}", other),
                    };

                    let value = call_site
                        .try_as_basic_value()
                        .left()
                        .expect("Unexpected void return");
                    value_tracker.set(ir_retrn, value);
                }
                GetLocal(ir_value, name) => {
                    let ptr: PointerValue = frame_slots.get(name).unwrap().open().clone();
                    let value = builder.build_load(ptr, "");
                    value_tracker.set(ir_value, value);
                }
                Return(ir_value) => {
                    let value = value_tracker.get(ir_value);
                    builder.build_return(Some(&value));
                }
            }
        }
    }

    value.print_to_stderr();
}

struct ValueTracker<'a, 'ctx> {
    ctx: &'a Context<'ctx>,
    values: HashMap<ir::ValueId, BasicValueEnum<'ctx>>,
}

impl<'a, 'ctx> ValueTracker<'a, 'ctx> {
    fn new(ctx: &'a Context<'ctx>) -> Self {
        Self {
            ctx,
            values: HashMap::new(),
        }
    }

    fn get(&self, ir_value: &ir::Value) -> BasicValueEnum<'ctx> {
        match ir_value {
            ir::Value::Int64(_, Some(const_value)) => {
                let typ: &IntType = self.ctx.shaper.int64_type.open();
                return typ.const_int(*const_value, false).into();
            }
            _ => (),
        }
        self.values
            .get(&ir_value.value_id())
            .expect(&format!("Value not tracked: {:?}", ir_value.value_id()))
            .clone()
    }

    fn set(&mut self, ir_value: &ir::Value, value: BasicValueEnum<'ctx>) {
        let id = ir_value.value_id();
        if self.values.contains_key(&id) {
            panic!(
                "Value has already been set, re-setting would violate SSA: {:?}",
                id
            );
        }
        self.values.insert(id, value);
    }
}

pub fn compile_modules(modules: Vec<Rc<ir::Module>>) {
    let ink_ctx = InkContext::create();
    let ink_module = ink_ctx.create_module("main");
    let mut shaper = Shaper {
        int64_type: OpaqueIntType::close(ink_ctx.i64_type()),
        func_types: vec![],
    };

    let mut ctx = Context {
        ink_ctx: &ink_ctx,
        ink_module,
        shaper: &mut shaper,
        funcs: HashMap::new(),
    };

    // Forward-define all the funcs.
    for module in modules.iter() {
        for func in module.func_values.borrow().iter() {
            define_func(&mut ctx, func);
        }
    }

    // Then actually compile them all.
    for module in modules.iter() {
        for func in module.func_values.borrow().iter() {
            compile_func(&mut ctx, func);
        }
    }
}
