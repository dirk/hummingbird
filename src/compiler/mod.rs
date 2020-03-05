use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::{Builder as InkBuilder, Builder};
use inkwell::context::Context as InkContext;
use inkwell::module::Module as InkModule;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use inkwell::types::FunctionType;
use inkwell::types::{AnyType, BasicType};
use inkwell::values::{BasicValue, FunctionValue, IntValue};
use inkwell::OptimizationLevel;

use super::frontend::Module as FrontendModule;
use super::type_ast::{self as ast, TypeId};

mod opaque;

use opaque::*;

/// Methods that compile (top-level), func, and block contexts all need
/// to provide.
trait Context<'ctx> {
    fn get_ink_context(&self) -> &'ctx InkContext;
    fn get_ink_module(&self) -> &InkModule<'ctx>;
    fn get_ink_builder(&self) -> &InkBuilder<'ctx>;

    /// Add a func to the specialization tracker so that we can build
    /// implementations later when we call the func.
    fn stub_func(&self, ast_func: ast::Func) -> UnspecializedFuncValue;

    /// NOTE: You *must* call `stub_func` with the func's AST node before
    ///   you can build specializations.
    fn get_or_build_func_specialization(
        &self,
        typ: ast::Type,
        args: Vec<Type>,
        retrn: Type,
    ) -> FuncValue;
}

struct FuncSpecializations {
    /// The implementation source that we'll recompile for specializations.
    ast_func: ast::Func,
    specializations: Vec<FuncValue>,
}

struct CompileContext<'ctx> {
    ink_ctx: &'ctx InkContext,
    /// Since specialization of functions is a key part of the language
    /// we can't really build multiple modules since we'd have to constantly
    /// reopen modules to add new specializations.
    ink_module: InkModule<'ctx>,
    ink_builder: InkBuilder<'ctx>,
    /// Track the specializations of functions to provide the correct
    /// implementation. We'll use the func's type's ID to for lookups.
    func_specializations: RefCell<HashMap<TypeId, FuncSpecializations>>,
}

impl<'ctx> Context<'ctx> for CompileContext<'ctx> {
    fn get_ink_context(&self) -> &'ctx InkContext {
        self.ink_ctx
    }

    fn get_ink_module(&self) -> &InkModule<'ctx> {
        &self.ink_module
    }

    fn get_ink_builder(&self) -> &Builder<'ctx> {
        &self.ink_builder
    }

    fn stub_func(&self, ast_func: ast::Func) -> UnspecializedFuncValue {
        let typ = ast_func.typ.clone();
        let id = typ.id();
        let mut tracker = self.func_specializations.borrow_mut();
        if tracker.contains_key(&id) {
            panic!(
                "Already have specializations for: {} ({:?})",
                id,
                typ.unwrap_func().name
            )
        }
        tracker.insert(
            id,
            FuncSpecializations {
                ast_func,
                specializations: vec![],
            },
        );
        UnspecializedFuncValue { typ }
    }

    fn get_or_build_func_specialization(
        &self,
        typ: ast::Type,
        args: Vec<Type>,
        retrn: Type,
    ) -> FuncValue {
        let typ = typ.unwrap_func();
        let id = typ.id;
        let mut specializations = {
            let mut tracker = self.func_specializations.borrow_mut();
            RefMut::map(tracker, |tracker| {
                tracker
                    .get_mut(&id)
                    .expect(&format!("Missing func: {} ({:?})", id, typ.name))
            })
        };

        // TODO: Actually iterate through the specializations looking for a
        //   matching one.

        let builder = self.get_ink_builder();
        // So that we can restore the builder after we build the func.
        let current_basic_block = builder.get_insert_block();
        let func_value = build_func_specialization(self, &specializations.ast_func, args, retrn);
        // Add the specialization so that we can eventually reuse it.
        specializations.specializations.push(func_value.clone());
        // Reposition the builder so as to not make life harder for our caller.
        if let Some(previous_basic_block) = current_basic_block {
            builder.position_at_end(previous_basic_block);
        }
        func_value
    }
}

pub fn build_main(frontend_module: &FrontendModule) {
    let ink_ctx = InkContext::create();

    let compile_ctx = CompileContext {
        ink_ctx: &ink_ctx,
        ink_module: ink_ctx.create_module("main"),
        ink_builder: ink_ctx.create_builder(),
        func_specializations: RefCell::new(HashMap::new()),
    };

    let main_func = find_main_func(frontend_module);
    build_func_specialization(&compile_ctx, &main_func, vec![], Type::UInt64);

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

#[derive(Clone, Debug)]
enum Type {
    UInt64,
}

impl Type {
    fn get_ink_type<'ctx, C: Context<'ctx>>(&self, ctx: &C) -> Box<dyn BasicType<'ctx> + 'ctx> {
        use Type::*;
        let ink_context = ctx.get_ink_context();
        match self {
            UInt64 => Box::new(ink_context.i64_type()),
        }
    }
}

#[derive(Clone, Debug)]
enum Value {
    Func(FuncValue),
    UnspecializedFunc(UnspecializedFuncValue),
    Tuple(TupleValue),
    UInt64(OpaqueIntValue),
}

/// We don't yet know how this function is going to be called, but we still
/// need to be able to treat it like a value.
#[derive(Clone, Debug)]
struct UnspecializedFuncValue {
    typ: ast::Type,
}

#[derive(Clone, Debug)]
struct FuncValue {
    // We store this in the top-level specialization tracker so the lifetime
    // has to be able to escape the local compilation context.
    value: OpaqueFunctionValue,
    args: Vec<Type>,
    retrn: Type,
}

#[derive(Clone, Debug)]
struct TupleValue {
    // TODO: Include the struct or void pointer in the tuple.
    value: OpaqueIntValue,
    typs: Vec<Type>,
}

impl Value {
    fn unit<'ctx, C: Context<'ctx>>(ctx: &C) -> Self {
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
            Tuple(tuple) => Box::new(tuple.value.unwrap().clone()),
            UInt64(value) => Box::new(value.unwrap().clone()),
            other @ _ => unreachable!("Cannot get Ink value: {:?}", other),
        }
    }
}

struct FunctionContext<'ctx, 'pctx> {
    parent: Option<&'ctx dyn Context<'pctx>>,
    function_value: FunctionValue<'ctx>,
    // TODO: Manage stack and heap frames.
    stack: HashMap<String, Value>,
    unspecialized: RefCell<HashMap<String, UnspecializedFuncValue>>,
}

impl<'ctx, 'pctx> FunctionContext<'ctx, 'pctx> {
    fn write_unspecialized(&self, key: String, value: UnspecializedFuncValue) {
        let mut unspecialized = self.unspecialized.borrow_mut();
        unspecialized.insert(key, value);
    }

    fn build_read_local(&self, key: String) -> Value {
        if let Some(unspecialized) = self.unspecialized.borrow().get(&key) {
            return Value::UnspecializedFunc(unspecialized.clone());
        }
        self.stack
            .get(&key)
            .expect(&format!("Local not found: {}", key))
            .clone()
    }
}

impl<'ctx, 'pctx> Context<'pctx> for FunctionContext<'ctx, 'pctx> {
    fn get_ink_context(&self) -> &'pctx InkContext {
        self.parent.unwrap().get_ink_context()
    }

    fn get_ink_module(&self) -> &InkModule<'pctx> {
        self.parent.unwrap().get_ink_module()
    }

    fn get_ink_builder(&self) -> &Builder<'pctx> {
        self.parent.unwrap().get_ink_builder()
    }

    fn stub_func(&self, ast_func: ast::Func) -> UnspecializedFuncValue {
        self.parent.unwrap().stub_func(ast_func)
    }

    fn get_or_build_func_specialization(
        &self,
        typ: ast::Type,
        args: Vec<Type>,
        retrn: Type,
    ) -> FuncValue {
        self.parent
            .unwrap()
            .get_or_build_func_specialization(typ, args, retrn)
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

/// This shouldn't usually be called directly; instead you should use
/// `Context::get_or_build_func_specialization` to get the right specialization
/// for your call.
fn build_func_specialization(
    compile_ctx: &CompileContext,
    ast_func: &ast::Func,
    args: Vec<Type>,
    retrn: Type,
) -> FuncValue {
    // TODO: Look for an existing specialization for the given `args` and
    //   `retrn` and, if found, use that.

    let return_ink_type = retrn.get_ink_type(compile_ctx);
    let function_ink_type = return_ink_type.fn_type(&[], false);
    let function_value =
        compile_ctx
            .ink_module
            .add_function(&ast_func.name, function_ink_type, None);

    let function_ctx = FunctionContext {
        parent: Some(compile_ctx),
        function_value,
        unspecialized: RefCell::new(HashMap::new()),
        stack: HashMap::new(),
    };

    match &ast_func.body {
        ast::FuncBody::Block(block) => build_func_body_block(&function_ctx, block),
    }

    // Uncomment to see the IR as it's built:
    function_value.print_to_stderr();

    FuncValue {
        value: OpaqueFunctionValue::wrap(function_value),
        args,
        retrn,
    }
}

fn build_func_body_block(ctx: &FunctionContext, block: &ast::Block) {
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
                ast::BlockStatement::Func(func) => {
                    let name = func.name.clone();
                    let unspecialized_func_value = ctx.stub_func(func.clone());
                    ctx.write_unspecialized(name, unspecialized_func_value);
                    None
                }
                other @ _ => unreachable!("Cannot build BlockStatement: {:?}", other),
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
    let value = builder.build_int_add(lhs, rhs, "");
    Value::UInt64(OpaqueIntValue::wrap(value))
}

fn build_postfix_call(
    ctx: &FunctionContext,
    tracker: &BlockTracker,
    call: &ast::PostfixCall,
) -> Value {
    let target = build_expression(ctx, tracker, &*call.target);
    let call_target = match target {
        Value::UnspecializedFunc(unspecialized) => {
            // FIXME: Convert the AST types in the call to IR types to pass to
            //   `get_or_build_func_specialization` for lookup.
            let retrn = Type::UInt64;
            ctx.get_or_build_func_specialization(unspecialized.typ.clone(), vec![], retrn)
        }
        other @ _ => unreachable!("Cannot build Call to target: {:?}", other),
    };
    Value::unit(ctx)
}

fn build_identifier(
    ctx: &FunctionContext,
    tracker: &BlockTracker,
    identifier: &ast::Identifier,
) -> Value {
    use ast::ScopeResolution::*;
    match &identifier.resolution {
        Local(name, _) => ctx.build_read_local(name.clone()),
        other @ _ => unreachable!("Cannot build Identifier with ScopeResolution: {:?}", other),
    }
}

fn build_expression(
    ctx: &FunctionContext,
    tracker: &BlockTracker,
    expression: &ast::Expression,
) -> Value {
    match expression {
        ast::Expression::Identifier(identifier) => build_identifier(ctx, tracker, identifier),
        ast::Expression::Infix(infix) => build_infix(ctx, tracker, infix),
        ast::Expression::LiteralInt(literal) => {
            let int_type = ctx.get_ink_context().i64_type();
            // FIXME: Support negative integer constants.
            let value = int_type.const_int(literal.value as u64, false);
            Value::UInt64(OpaqueIntValue::wrap(value))
        }
        ast::Expression::PostfixCall(call) => build_postfix_call(ctx, tracker, call),
        other @ _ => unreachable!("Cannot build Expression: {:?}", other),
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
    fn new_basic_block<'ictx, 'fctx, C: Context<'ictx> + GetFunctionValue<'fctx>>(
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
