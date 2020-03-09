use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use super::super::super::frontend::Module as FrontendModule;
use super::super::super::type_ast::{self as ast};
use super::super::path_to_name::path_to_name;
use super::super::vecs_equal::vecs_equal;
use super::typ::{RealType, TupleType, Type};
use super::typer::Typer;
use super::value::{AbstractValue, FuncValue, LocalValue, StaticValue, Value, ValueId};
use super::{Container, Func, Module, Root};

pub struct BasicBlockManager {
    next_value_id: Cell<usize>,
    current_index: Cell<Option<usize>>,
    pub basic_blocks: RefCell<Vec<BasicBlock>>,
}

// Pointer to a basic block.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct BasicBlockIndex(usize);

impl BasicBlockManager {
    pub fn new() -> Self {
        Self {
            next_value_id: Cell::new(0),
            current_index: Cell::new(None),
            basic_blocks: RefCell::new(vec![]),
        }
    }

    fn append_basic_block(&self, name: Option<&str>) -> BasicBlockIndex {
        let name = name.unwrap_or("block");
        let index = self.basic_blocks.borrow().len();
        let name = format!("{}{}", name, index);
        let mut basic_blocks = self.basic_blocks.borrow_mut();
        basic_blocks.push(BasicBlock {
            index,
            name,
            instructions: vec![],
        });
        self.current_index.set(Some(index));
        BasicBlockIndex(index)
    }

    fn current_index(&self) -> BasicBlockIndex {
        BasicBlockIndex(self.current_index.get().unwrap())
    }

    fn position_at_end(&self, index: BasicBlockIndex) {
        let index = index.0;
        self.current_index.set(Some(index));
    }

    fn push_instruction(&self, instruction: Instruction) {
        let index = self.current_index.get().expect("No basic blocks");
        let mut basic_block = RefMut::map(self.basic_blocks.borrow_mut(), |basic_blocks| {
            &mut basic_blocks[index]
        });
        basic_block.instructions.push(instruction);
    }

    fn get_next_value_id(&self) -> ValueId {
        let current = self.next_value_id.take();
        self.next_value_id.set(current + 1);
        ValueId::new(current)
    }
}

pub fn compile_modules<'m, M: Iterator<Item = &'m FrontendModule>>(
    frontend_modules: M,
    entry_frontend_module: &FrontendModule,
) -> Root {
    let typer = Typer::new(None);
    let root = Root::new(typer.clone());

    // TODO: Build a root `TypeScope` will all the builtins.

    for frontend_module in frontend_modules {
        define_module(frontend_module, root.clone());
    }

    // Find and compile the main func.
    let entry = root.find_module_by_id(entry_frontend_module.id()).unwrap();
    let main_func = entry
        .find_func_by_name("main")
        .expect("Missing 'main' func");

    if !is_immediately_specializable(&main_func) {
        panic!("Main func cannot be specialized");
    }
    let ast_func = &main_func.0.ast_func;
    let parameters = ast_func
        .arguments
        .iter()
        .map(|argument| typer.build_type(&argument.typ))
        .collect::<Vec<_>>();
    let retrn = typer.build_type(&ast_func.typ.unwrap_func().retrn.borrow());
    let func_value = compile_func_specialization(main_func, parameters, retrn);
    // Mark is as the main func for the target compiler.
    func_value.set_main(true);

    root
}

fn compile_func_specialization(func: Func, parameters: Vec<Type>, retrn: Type) -> FuncValue {
    // Ensure all the parameters are real types.
    let parameters = parameters
        .into_iter()
        .map(|parameter| parameter.into_real())
        .collect::<Vec<_>>();
    let retrn = retrn.into_real();

    let func_value = func.get_or_insert_specialization(parameters, retrn);

    // `create_basic_blocks` returns a `Some` if it just created the basic
    // block manager (indicating this specialization hasn't been compiled).
    if let Some(basic_blocks) = func_value.create_basic_blocks() {
        let builder = Builder {
            buildable: Box::new(func_value.clone()),
            basic_blocks,
        };
        compile_func_body(&builder, func_value.clone(), &func.0.ast_func.body);
    }

    func_value
}

// Abstraction so that we can compile both funcs and closures with the
// same functions.
pub trait Buildable: Container {
    /// Search for a func defined within this scope (including the
    /// current func).
    fn find_func(&self, name: &str) -> Option<Func>;

    fn find_local(&self, name: &str) -> Option<(usize, RealType)>;

    fn build_type(&self, ast_type: &ast::Type) -> Type {
        self.get_typer().build_type(ast_type)
    }
}

struct Builder<'a> {
    buildable: Box<dyn Buildable>,
    basic_blocks: Ref<'a, BasicBlockManager>,
}

impl<'a> Builder<'a> {
    fn define_func(&self, ast_func: ast::Func) -> Func {
        self.buildable.define_func(ast_func)
    }

    fn find_func(&self, name: &str) -> Option<Func> {
        self.buildable.find_func(name)
    }

    fn find_local(&self, name: &str) -> Option<(usize, RealType)> {
        self.buildable.find_local(name)
    }

    fn build_type(&self, ast_type: &ast::Type) -> Type {
        self.buildable.get_typer().build_type(ast_type)
    }

    fn build_value(&self, typ: Type) -> Value {
        match typ {
            Type::Real(real_type) => match real_type {
                RealType::FuncPtr(func_ptr_type) => {
                    Value::Local(LocalValue::FuncPtr(self.get_next_value_id(), func_ptr_type))
                }
                RealType::Int64 => Value::Local(LocalValue::Int64(self.get_next_value_id(), None)),
                RealType::Tuple(tuple_type) => {
                    Value::Local(LocalValue::Tuple(self.get_next_value_id(), tuple_type))
                }
            },
            _ => unreachable!("Cannot build an SSA Value for an Abstract type"),
        }
    }

    fn append_basic_block(&self, name: Option<&str>) -> BasicBlockIndex {
        self.basic_blocks.append_basic_block(name)
    }

    fn get_next_value_id(&self) -> ValueId {
        self.basic_blocks.get_next_value_id()
    }

    fn push_instruction(&self, instruction: Instruction) {
        self.basic_blocks.push_instruction(instruction);
    }

    fn const_unit(&self) -> Value {
        let id = self.get_next_value_id();
        Value::Local(LocalValue::Tuple(id, TupleType::unit()))
    }

    fn const_int64(&self, const_value: u64) -> Value {
        let id = self.get_next_value_id();
        Value::Local(LocalValue::Int64(id, Some(const_value)))
    }

    fn build_call_func(
        &self,
        retrn: RealType,
        func_value: FuncValue,
        arguments: Vec<Value>,
    ) -> Value {
        let retrn = self.build_value(Type::Real(retrn));
        self.push_instruction(Instruction::CallFunc(retrn.clone(), func_value, arguments));
        retrn
    }

    fn build_call_func_ptr(
        &self,
        retrn: RealType,
        value: LocalValue,
        arguments: Vec<Value>,
    ) -> Value {
        let retrn = self.build_value(Type::Real(retrn));
        self.push_instruction(Instruction::CallFuncPtr(retrn.clone(), value, arguments));
        retrn
    }

    fn build_get_local(&self, index: usize, real_type: RealType) -> Value {
        let value = self.build_value(Type::Real(real_type));
        self.push_instruction(Instruction::GetLocal(value.clone(), index));
        value
    }

    fn build_return(&self, value: Value) {
        self.push_instruction(Instruction::Return(value));
    }
}

fn compile_func_body(builder: &Builder, func_value: FuncValue, body: &ast::FuncBody) {
    builder.append_basic_block(Some("entry"));
    let implicit_retrn = match body {
        ast::FuncBody::Block(block) => compile_block(builder, block),
    };
    builder.build_return(implicit_retrn);
}

fn compile_block(builder: &Builder, block: &ast::Block) -> Value {
    // Implicit return is the unit tuple or the value produced by the
    // last statement.
    let mut implicit_value = builder.const_unit();
    if !block.statements.is_empty() {
        let last_index = block.statements.len() - 1;
        for (index, statement) in block.statements.iter().enumerate() {
            let value = match statement {
                ast::BlockStatement::Expression(expression) => {
                    compile_expression(builder, expression)
                }
                ast::BlockStatement::Func(func) => {
                    let func = builder.define_func(func.clone());
                    Value::Abstract(AbstractValue::UnspecializedFunc(func))
                }
                other @ _ => unreachable!("Cannot compile BlockStatement: {:?}", other),
            };
            if index == last_index {
                implicit_value = value
            }
        }
    }

    implicit_value
}

fn compile_expression(builder: &Builder, expression: &ast::Expression) -> Value {
    match expression {
        ast::Expression::Identifier(identifier) => compile_identifier(builder, identifier),
        ast::Expression::LiteralInt(literal) => builder.const_int64(literal.value as u64),
        ast::Expression::PostfixCall(call) => compile_postfix_call(builder, call),
        other @ _ => unreachable!("Cannot compile Expression: {:?}", other),
    }
}

fn compile_identifier(builder: &Builder, identifier: &ast::Identifier) -> Value {
    let resolution = &identifier.resolution;
    match resolution {
        ast::ScopeResolution::Local(name, ast_typ) => {
            // First search for funcs defined in this func.
            if let Some(func) = builder.find_func(name) {
                // Use the type expected by the AST to preemptively specialize
                // if possible.
                if !ast_typ.contains_generics() {
                    // Using `unwrap` since this type better be callable,
                    // otherwise we have a huge problem in typing.
                    let (ast_parameters, ast_retrn) = ast_typ.maybe_callable().unwrap();
                    let parameters = ast_parameters
                        .iter()
                        .map(|ast_parameter| builder.build_type(ast_parameter))
                        .collect::<Vec<_>>();
                    let retrn = builder.build_type(&ast_retrn);
                    let func_value = compile_func_specialization(func, parameters, retrn);
                    return Value::Static(StaticValue::Func(func_value));
                }
                return Value::Abstract(AbstractValue::UnspecializedFunc(func));
            }
            // Then search for a slot in the stack frame.
            if let Some((index, typ)) = builder.find_local(name) {
                return builder.build_get_local(index, typ);
            }
            panic!("Local not found: {}", name)
        }
        other @ _ => unreachable!(
            "Cannot compile Identifier with ScopeResolution: {:?}",
            other
        ),
    }
}

fn compile_postfix_call(builder: &Builder, call: &ast::PostfixCall) -> Value {
    let target = compile_expression(builder, &call.target);
    let arguments = call
        .arguments
        .iter()
        .map(|argument| compile_expression(builder, argument))
        .collect::<Vec<_>>();
    // TODO: Sanity-check that the types of the values match the types
    //   built from the AST.
    let retrn_type = builder.build_type(&call.typ);
    match target {
        Value::Abstract(abstract_value) => match abstract_value {
            AbstractValue::UnspecializedFunc(func) => {
                let arguments_types = arguments
                    .iter()
                    .map(|argument| argument.typ())
                    .collect::<Vec<_>>();
                return compile_unspecialized_call(
                    builder,
                    func,
                    arguments,
                    arguments_types,
                    retrn_type,
                );
            }
        },
        Value::Static(static_value) => match static_value {
            StaticValue::Func(func_value) => {
                let retrn = func_value.get_retrn();
                return builder.build_call_func(retrn, func_value, arguments);
            }
        },
        Value::Local(local_value) => match &local_value {
            LocalValue::FuncPtr(value, func_ptr_type) => {
                // Converting these to real types because func pointers must
                // be callable with real (non-generic) types.
                let parameters = arguments
                    .iter()
                    .map(|argument| argument.typ().into_real())
                    .collect::<Vec<_>>();
                let retrn = retrn_type.into_real();

                let parameters_match =
                    vecs_equal(&parameters, &func_ptr_type.parameters, RealType::is_equal);
                if !parameters_match {
                    panic!("Tried calling function pointer but parameter types don't match\nexpected: {:?}\ngot: {:?}",
                        func_ptr_type.parameters,
                        parameters,
                    );
                }
                if !retrn.is_equal(&func_ptr_type.retrn) {
                    panic!("Tried calling function pointer but return types don't match\nexpected: {:?}\ngot: {:?}",
                        func_ptr_type.retrn,
                        parameters,
                    );
                }

                return builder.build_call_func_ptr(retrn, local_value, arguments);
            }
            _ => (),
        },
        _ => (),
    }
    unreachable!("Cannot compile Call")
}

fn compile_unspecialized_call(
    builder: &Builder,
    func: Func,
    arguments: Vec<Value>,
    arguments_types: Vec<Type>,
    retrn_type: Type,
) -> Value {
    let func_value = compile_func_specialization(func, arguments_types, retrn_type);
    let retrn = func_value.get_retrn();
    builder.build_call_func(retrn, func_value, arguments)
}

fn define_module(frontend_module: &FrontendModule, root: Root) -> Module {
    let qualified_name = path_to_name(frontend_module.path());
    let module = root.add_module(frontend_module.id(), qualified_name);
    // Walk the AST to forward-define all of the funcs.
    let ast_module = frontend_module.unwrap_ast();
    for statement in ast_module.statements.iter() {
        match statement {
            ast::ModuleStatement::Func(ast_func) => {
                module.define_func(ast_func.clone());
            }
        }
    }
    module
}

fn is_immediately_specializable(func: &Func) -> bool {
    let ast_typ = &func.0.ast_func.typ;
    ast_typ.unwrap_func(); // Assert that it's a func type.
    !ast_typ.contains_generics()
}

pub struct BasicBlock {
    index: usize,
    pub name: String,
    pub instructions: Vec<Instruction>,
}

impl BasicBlock {
    pub fn get_index(&self) -> BasicBlockIndex {
        BasicBlockIndex(self.index)
    }
}

pub enum Instruction {
    // $1 = $2($3...)
    CallFunc(Value, FuncValue, Vec<Value>),
    // $1 = $2($3...)
    CallFuncPtr(Value, LocalValue, Vec<Value>),
    // $1 = GetLocal($2)
    GetLocal(Value, usize),
    // Return($1)
    Return(Value),
}
