use std::cell::{Ref, Cell};
use std::path::Path;
use std::process::Command;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::{Builder as InkBuilder, Builder};
use inkwell::context::{Context as InkContext, Context};
use inkwell::module::Module as InkModule;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use inkwell::types::{AnyType, BasicType};
use inkwell::types::FunctionType;
use inkwell::values::{BasicValue, FunctionValue, IntValue};
use inkwell::OptimizationLevel;

use super::frontend::Module as FrontendModule;
use super::type_ast::{self as ast, TypeId};

mod opaque;

use opaque::*;

trait GetInk<'ctx> {
    fn get_ink_context(&self) -> &'ctx InkContext;
    fn get_ink_builder(&self) -> &InkBuilder<'ctx>;
}

struct CompileContext<'ctx> {
    ink_ctx: &'ctx InkContext,
    /// Since specialization of functions is a key part of the language
    /// we can't really build multiple modules since we'd have to constantly
    /// reopen modules to add new specializations.
    ink_module: InkModule<'ctx>,
    ink_builder: InkBuilder<'ctx>,
}

impl<'ctx> GetInk<'ctx> for &CompileContext<'ctx> {
    fn get_ink_context(&self) -> &'ctx Context {
        self.ink_ctx
    }

    fn get_ink_builder(&self) -> &Builder<'ctx> {
        &self.ink_builder
    }
}

pub fn build_main(frontend_module: &FrontendModule) {
    let ink_ctx = InkContext::create();

    let compile_ctx = CompileContext {
        ink_ctx: &ink_ctx,
        ink_module: ink_ctx.create_module("main"),
        ink_builder: ink_ctx.create_builder(),
    };

    let main_func = find_main_func(frontend_module);
    build_func(&compile_ctx, &main_func, vec![], Type::UInt64);

    // Set up the paths we'll emit to.
    let object = Path::new("./build/out.o");
    let executable = Path::new("./build/out");

    // let optimization_level = OptimizationLevel::Default;
    let optimization_level = OptimizationLevel::None;
    let reloc_mode = RelocMode::Default;
    let code_model = CodeModel::Default;
    Target::initialize_x86(&InitializationConfig::default());
    let target = Target::from_name("x86-64").unwrap();
    let target_machine = target
        .create_target_machine(
            "x86_64-apple-darwin19.3.0",
            "x86-64",
            "",
            optimization_level,
            reloc_mode,
            code_model,
        )
        .unwrap();

    target_machine
        .write_to_file(&compile_ctx.ink_module, FileType::Object, &object)
        .unwrap();

    // Link the object file into an executable.
    Command::new("clang")
        .args(&[object.to_str().unwrap(), "-o", executable.to_str().unwrap()])
        .output()
        .unwrap();
}

#[derive(Debug)]
enum Type {
    UInt64,
}

impl Type {
    fn get_ink_type<'ctx, C: GetInk<'ctx>>(&self, ctx: &C) -> Box<dyn BasicType<'ctx> + 'ctx> {
        use Type::*;
        let ink_context = ctx.get_ink_context();
        match self {
            UInt64 => Box::new(ink_context.i64_type()),
        }
    }
}

#[derive(Debug)]
enum Value {
    Tuple(TupleValue),
    UInt64(OpaqueIntValue),
}

#[derive(Debug)]
struct TupleValue {
    // TODO: Include the struct or void pointer in the tuple.
    value: OpaqueIntValue,
    typs: Vec<Type>,
}

impl Value {
    fn unit<'ctx, C: GetInk<'ctx>>(ctx: &C) -> Self {
        let ink_context = ctx.get_ink_context();
        let int_type = ink_context.bool_type();
        let value = int_type.const_zero();
        Value::Tuple(TupleValue {
            value: OpaqueIntValue::wrap(value),
            typs: vec![],
        })
    }

    fn get_ink_value<'ctx>(&self) -> Box<dyn BasicValue<'ctx> + 'ctx> {
        use Value::*;
        match self {
            Tuple(tuple) => {
                Box::new(tuple.value.unwrap().clone())
            },
            UInt64(value) => Box::new(value.unwrap().clone()),
        }
    }
}

struct FunctionContext<'ctx, 'pctx> {
    parent: Option<&'ctx dyn GetInk<'pctx>>,
    function_value: FunctionValue<'ctx>,
    // TODO: Manage stack and heap frames.
}

impl<'ctx, 'pctx> GetInk<'pctx> for FunctionContext<'ctx, 'pctx> {
    fn get_ink_context(&self) -> &'pctx Context {
        self.parent.unwrap().get_ink_context()
    }

    fn get_ink_builder(&self) -> &Builder<'pctx> {
        self.parent.unwrap().get_ink_builder()
    }
}

trait GetFunctionValue<'ctx> {
    fn get_function_value(&self) -> FunctionValue<'ctx>;
}

impl<'ctx, 'pctx> GetFunctionValue<'ctx> for FunctionContext<'ctx, 'pctx> {
    fn get_function_value(&self) -> FunctionValue<'ctx> {
        self.function_value.clone()
    }
}

fn build_func(
    compile_ctx: &CompileContext,
    ast_func: &ast::Func,
    args: Vec<Type>,
    retrn: Type,
) {
    // TODO: Look for an existing specialization in the context.

    let return_ink_type = retrn.get_ink_type(&compile_ctx);
    let function_ink_type = return_ink_type.fn_type(&[], false);
    let function_value = compile_ctx.ink_module.add_function(&ast_func.name, function_ink_type, None);

    let function_ctx = FunctionContext {
        parent: Some(&compile_ctx),
        function_value,
    };

    match &ast_func.body {
        ast::FuncBody::Block(block) => build_func_body_block(&function_ctx, block),
    }

    function_value.print_to_stderr();
}

fn build_func_body_block(
    ctx: &FunctionContext,
    block: &ast::Block,
) {
    let tracker = BlockTracker::new(None);
    let value = build_block(ctx, &tracker, block, Some("entry".to_string()));
    build_return(ctx, &tracker, value);
}

fn build_return(ctx: &FunctionContext, tracker: &BlockTracker, value: Value) {
    let value = value.get_ink_value();
    let builder = ctx.get_ink_builder();
    builder.build_return(Some(&*value));
}

/// Returns the implicit return value of the block (or an empty tuple if
/// there is none).
///
/// This function guarantees that the builder will be positioned at the exit
/// block for non-returning control flow.
fn build_block(
    ctx: &FunctionContext,
    tracker: &BlockTracker,
    block: &ast::Block,
    name: Option<String>,
) -> Value {
    tracker.new_basic_block(ctx, name.unwrap_or("block".to_string()));
    let mut value = None;
    if !block.statements.is_empty() {
        let last_index = block.statements.len() - 1;
        for (index, statement) in block.statements.iter().enumerate() {
            let statement_value = match statement {
                ast::BlockStatement::Expression(expression) => {
                    Some(build_expression(ctx, tracker, expression))
                }
                other @ _ => unreachable!("Cannot build: {:?}", other),
            };
            if index == last_index {
                value = statement_value;
            }
        }
    }
    value.unwrap_or(Value::unit(ctx))
}

fn build_infix(ctx: &FunctionContext, tracker: &BlockTracker, infix: &ast::Infix) -> Value {
    let lhs = build_expression(ctx, tracker, &infix.lhs);
    let rhs = build_expression(ctx, tracker, &infix.rhs);
    match (lhs, rhs) {
        (Value::UInt64(lhs), Value::UInt64(rhs)) => build_infix_int64(ctx, lhs, rhs),
        (lhs @ _, rhs @ _) => unreachable!("Cannot infix: {:?} and {:?}", lhs, rhs),
    }
}

fn build_infix_int64(ctx: &FunctionContext, lhs: OpaqueIntValue, rhs: OpaqueIntValue) -> Value {
    let builder = ctx.get_ink_builder();
    let lhs: IntValue = *lhs.unwrap();
    let rhs: IntValue = *rhs.unwrap();
    let clear = builder.build_int_add(lhs, rhs, "");
    Value::UInt64(OpaqueIntValue::wrap(clear))
}

fn build_expression(
    ctx: &FunctionContext,
    tracker: &BlockTracker,
    expression: &ast::Expression,
) -> Value {
    match expression {
        ast::Expression::Infix(infix) => build_infix(ctx, tracker, infix),
        ast::Expression::LiteralInt(literal) => {
            let int_type = ctx.get_ink_context().i64_type();
            // FIXME: Support negative integer constants.
            let clear = int_type.const_int(literal.value as u64, false);
            Value::UInt64(OpaqueIntValue::wrap(clear))
        }
        other @ _ => unreachable!("Cannot build: {:?}", other),
    }
}

struct BlockTracker {
    index: Cell<usize>,
    /// Allows for nesting blocks (to avoid naming conflicts).
    prefix: Option<String>,
}

impl BlockTracker {
    fn new(prefix: Option<String>) -> Self {
        Self {
            index: Cell::new(0),
            prefix,
        }
    }

    /// Return a new tracker with its prefix as the name of the passed
    /// basic block.
    fn nest(&self, basic_block: BasicBlock) -> Self {
        let prefix = basic_block.get_name().to_str().unwrap().to_string();
        Self::new(Some(prefix))
    }

    /// Add a basic block to the function and position the builder at the end
    /// of that block.
    fn new_basic_block<'ictx, 'fctx, C: GetInk<'ictx> + GetFunctionValue<'fctx>>(
        &self,
        ctx: &C,
        name: String,
    ) -> BasicBlock<'ictx> {
        let name = self.next_name(name);

        let ink_context = ctx.get_ink_context();
        let ink_builder = ctx.get_ink_builder();
        let function_value = ctx.get_function_value();

        let basic_block = ink_context.append_basic_block(function_value, &name);
        ink_builder.position_at_end(basic_block);
        basic_block
    }

    fn next_name(&self, name: String) -> String {
        let index = self.index.get();
        self.index.replace(index + 1);
        if let Some(prefix) = &self.prefix {
            format!("{}_{}{}", prefix, name, index)
        } else {
            format!("{}{}", name, index)
        }
    }
}

/// Searches the given module for a `main` func.
fn find_main_func(frontend_module: &FrontendModule) -> Ref<ast::Func> {
    use ast::ModuleStatement::*;
    Ref::map(frontend_module.borrow_typed(), |module| {
        let mut main_func = None;
        for statement in module.statements.iter() {
            match statement {
                Func(func) => {
                    if &func.name == "main" {
                        main_func = Some(func);
                        break;
                    }
                }
            }
        }
        match main_func {
            Some(main_func) => main_func,
            None => unreachable!("main func doesn't exist in module"),
        }
    })
}
