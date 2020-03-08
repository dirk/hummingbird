use std::cell::{Cell, Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

use regex::Regex;

use super::super::frontend::Module as FrontendModule;
use super::super::type_ast::{self as ast, TypeId};

#[derive(Clone, Debug)]
pub enum Type {
    Func(Rc<FuncType>),
    Int64,
    /// Used when we're forward-defining a func's frame and don't yet have a
    /// type ready; specifically used with unspecialized func types.
    Placeholder,
    Tuple(TupleType),
    UnspecializedFunc(Rc<UnspecializedFuncType>),
}

impl Type {
    fn to_value<G: FnOnce() -> ValueId>(&self, get_value_id: G) -> Value {
        use Type::*;
        match self {
            Func(func) => {
                let unspecialized_func_type = func.unspecialized.upgrade().unwrap();
                Value::UnspecializedFunc(unspecialized_func_type)
            }
            Int64 => Value::Int64(get_value_id(), None),
            Placeholder => unreachable!(),
            Tuple(tuple_type) => Value::Tuple(get_value_id(), tuple_type.clone()),
            UnspecializedFunc(unspecialized_func_type) => {
                Value::UnspecializedFunc(unspecialized_func_type.clone())
            }
        }
    }

    pub fn is_unspecialized(&self) -> bool {
        use Type::*;
        match self {
            UnspecializedFunc(_) => true,
            _ => false,
        }
    }

    /// Compare whether two types have the same shape (ie. can be used in the
    /// same place).
    pub fn shape_equals(&self, other: &Type) -> bool {
        use Type::*;
        match (self, other) {
            (Func(self_func), Func(other_func)) => {
                for (index, self_argument) in self_func.arguments.iter().enumerate() {
                    let other_argument = &other_func.arguments[index];
                    if !self_argument.shape_equals(other_argument) {
                        return false;
                    }
                }
                self_func.retrn.shape_equals(&other_func.retrn)
            }
            (Int64, Int64) => true,
            (Tuple(self_tuple_type), Tuple(other_tuple_type)) => vecs_equal(
                &self_tuple_type.members,
                &other_tuple_type.members,
                |self_member, other_member| self_member.shape_equals(other_member),
            ),
            (UnspecializedFunc(self_unspecialized), UnspecializedFunc(other_unspecialized)) => {
                self_unspecialized.name == other_unspecialized.name
            }
            _ => false,
        }
    }
}

/// Compare two `Vec`s for equality using a custom comparator function to
/// check equality of each element.
fn vecs_equal<T, F: Fn(&T, &T) -> bool>(left: &Vec<T>, right: &Vec<T>, cmp: F) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (index, left_element) in left.iter().enumerate() {
        let right_element = &right[index];
        if !cmp(left_element, right_element) {
            return false;
        }
    }
    true
}

impl Type {
    fn unwrap_unspecialized_func(&self) -> &Rc<UnspecializedFuncType> {
        use Type::*;
        match self {
            UnspecializedFunc(unspecialized_func_type) => unspecialized_func_type,
            other @ _ => unreachable!("Not an UnspecializedFunc: {:?}", other),
        }
    }
}

#[derive(Debug)]
pub struct FuncType {
    // The function that this is a specialized implementation of.
    unspecialized: Weak<UnspecializedFuncType>,
    pub arguments: Vec<Type>,
    pub retrn: Type,
}

#[derive(Clone, Debug)]
struct TupleType {
    members: Vec<Type>,
}

impl TupleType {
    fn new(members: Vec<Type>) -> Self {
        Self { members }
    }

    fn unit() -> Self {
        Self::new(vec![])
    }
}

#[derive(Debug)]
struct UnspecializedFuncType {
    /// The context it was defined in so that we can define specializations
    /// in the right place.
    ctx: Rc<ModuleContext>,
    /// The fully-qualified name of this func. Must be unique!
    name: String,
    specializations: RefCell<Vec<(Rc<FuncType>, Rc<FuncValue>)>>,
    ast_typ: ast::Type,
    ast_func: RefCell<Option<ast::Func>>,
}

impl UnspecializedFuncType {
    fn needs_specialization(&self) -> bool {
        self.get_ast_func().typ.contains_generics()
    }

    fn set_ast_func(&self, ast_func: ast::Func) {
        let mut mutable = self.ast_func.borrow_mut();
        *mutable = Some(ast_func);
    }

    fn get_ast_func(&self) -> Ref<ast::Func> {
        Ref::map(self.ast_func.borrow(), |ast_func| {
            ast_func.as_ref().expect("Missing AST func implementation")
        })
    }

    /// Try to find an already-existing specialization by comparing arguments.
    fn find_matching_specialization(&self, arguments: &Vec<Type>) -> Option<Rc<FuncValue>> {
        for (func_type, func_value) in self.specializations.borrow().iter() {
            if vecs_equal(
                &func_type.arguments,
                arguments,
                |func_argument, argument| func_argument.shape_equals(argument),
            ) {
                return Some(func_value.clone());
            }
        }
        None
    }

    fn get_or_insert_specialization(
        unspecialized: &Rc<UnspecializedFuncType>,
        arguments: Vec<Type>,
    ) -> Rc<FuncValue> {
        // First look for an existing specialization for our arguments.
        if let Some(matching) = unspecialized.find_matching_specialization(&arguments) {
            return matching;
        }
        // Otherwise start building a new specialization.
        let name = format!(
            "{}{}",
            unspecialized.name,
            unspecialized.specializations.borrow().len()
        );
        // Build a specializer to track how generic types are specialized
        // (and also cache types to save time in general).
        let mut specializer = Specializer::new(Rc::downgrade(&unspecialized.ctx), name.clone());
        // Save the generics in the specializer so that they'll be used
        // when building other types.
        let ast_type = unspecialized.ast_typ.unwrap_func();
        let ast_func = &*unspecialized.get_ast_func();
        for (index, argument_ast_type) in ast_type.arguments.borrow().iter().enumerate() {
            let argument_type = arguments[index].clone();
            specializer.cache_type(argument_ast_type, argument_type);
        }
        // Build the return type via the specializer so that specializations
        // are applied.
        let retrn = specializer.build_type(&ast_type.retrn.borrow());
        let func_typ = Rc::new(FuncType {
            unspecialized: Rc::downgrade(unspecialized),
            arguments,
            retrn,
        });
        let func_value = unspecialized
            .ctx
            .module
            .add_func_value(name, func_typ.clone());
        // Save the type and value for reuse.
        {
            let mut specializations = unspecialized.specializations.borrow_mut();
            specializations.push((func_typ.clone(), func_value.clone()));
        }
        // Now that the type and value are both saved we can safely compile
        // the body of the func.
        compile_func_specialization(
            unspecialized.ctx.clone(),
            specializer,
            func_value.clone(),
            ast_func,
        );
        func_value
    }
}

struct Builder {
    func: Rc<FuncValue>,
    index: usize,
    next_value_id: Cell<usize>,
}

struct BasicBlockPointer(usize);

impl Builder {
    fn new(func: Rc<FuncValue>) -> Self {
        Self {
            func,
            index: 0,
            next_value_id: Cell::new(0),
        }
    }

    fn append_basic_block(&self, name: Option<&str>) -> BasicBlockPointer {
        let name = name.unwrap_or("block");
        let name = format!("{}{}", name, self.func.basic_blocks.borrow().len());
        let mut basic_blocks = self.func.basic_blocks.borrow_mut();
        basic_blocks.push(BasicBlock {
            name: name.to_string(),
            instructions: vec![],
        });
        let index = basic_blocks.len() - 1;
        BasicBlockPointer(index)
    }

    fn const_int64(&self, value: u64) -> Value {
        Value::Int64(self.get_next_value_id(), Some(value))
    }

    fn const_unit(&self) -> Value {
        Value::Tuple(self.get_next_value_id(), TupleType::unit())
    }

    fn build_return(&self, value: Value) {
        self.push(Instruction::Return(value))
    }

    fn build_get_local(&self, name: String, typ: &Type) -> Value {
        let value = typ.to_value(|| self.get_next_value_id());
        self.push(Instruction::GetLocal(value.clone(), name));
        value
    }

    fn build_call(&self, target: Value, arguments: Vec<Value>, retrn: &Type) -> Value {
        let retrn = retrn.to_value(|| self.get_next_value_id());
        self.push(Instruction::Call(retrn.clone(), target, arguments));
        retrn
    }

    fn push(&self, instruction: Instruction) {
        let mut basic_blocks = self.func.basic_blocks.borrow_mut();
        let basic_block = basic_blocks.get_mut(self.index).unwrap();
        basic_block.instructions.push(instruction);
    }

    fn get_next_value_id(&self) -> usize {
        let current = self.next_value_id.take();
        self.next_value_id.set(current + 1);
        current
    }
}

#[derive(Debug)]
pub enum Instruction {
    // $1 = $2($3...)
    Call(Value, Value, Vec<Value>),
    // $1 = GetLocal($2)
    GetLocal(Value, String),
    // Return($1)
    Return(Value),
}

fn compile_func_specialization(
    parent_ctx: Rc<dyn Context>,
    specializer: Specializer,
    func_value: Rc<FuncValue>,
    ast_func: &ast::Func,
) {
    let ctx = Rc::new(FuncContext {
        parent_ctx,
        func_value: func_value.clone(),
        specializer: RefCell::new(specializer),
    });
    {
        // Build the stack frame converting types as we go. Doing it in a
        // block as we need to release the borrow before compiling the body.
        let mut stack_frame = func_value.stack_frame.borrow_mut();
        let scope = ast_func.scope.unwrap_func();
        for (name, ast_typ) in scope.locals.iter() {
            let typ = ctx.build_type(ast_typ);
            stack_frame.insert(name.clone(), typ);
        }
    }
    let builder = Builder::new(func_value.clone());
    match &ast_func.body {
        ast::FuncBody::Block(block) => compile_func_body_block(ctx, &builder, block),
    };
}

fn compile_func_body_block(ctx: Rc<FuncContext>, builder: &Builder, block: &ast::Block) {
    let implicit_retrn = compile_block(ctx, builder, block, Some("entry"));
    // Ensure there is an explicit return from the function.
    builder.build_return(implicit_retrn);
}

fn compile_block(
    ctx: Rc<FuncContext>,
    builder: &Builder,
    block: &ast::Block,
    override_name: Option<&str>,
) -> Value {
    builder.append_basic_block(override_name);
    if block.statements.is_empty() {
        // Implicit return of an empty block is the unit.
        builder.const_unit()
    } else {
        let mut implicit_retrn = None;
        let last_index = block.statements.len() - 1;
        for (index, statement) in block.statements.iter().enumerate() {
            let retrn = match statement {
                ast::BlockStatement::Expression(expression) => {
                    Some(compile_expression(ctx.clone(), builder, expression))
                }
                ast::BlockStatement::Func(func) => Some(compile_func(ctx.clone(), builder, func)),
                other @ _ => panic!("Cannot build BlockStatement: {:?}", other),
            };
            if index == last_index {
                implicit_retrn = retrn;
            }
        }
        implicit_retrn.unwrap_or_else(|| builder.const_unit())
    }
}

/// "Compile" a func; really just saving the AST in the unspecialized type so
/// that it can be specialized later.
fn compile_func(ctx: Rc<FuncContext>, builder: &Builder, func: &ast::Func) -> Value {
    let func_typ = ctx
        .get_local_type(&func.name)
        .unwrap_unspecialized_func()
        .clone();
    // Save the implementation so that we can specialize it later.
    func_typ.set_ast_func(func.clone());
    Value::UnspecializedFunc(func_typ)
}

fn compile_expression(
    ctx: Rc<FuncContext>,
    builder: &Builder,
    expression: &ast::Expression,
) -> Value {
    match expression {
        ast::Expression::Identifier(identifier) => compile_identifier(ctx, builder, identifier),
        ast::Expression::LiteralInt(literal) => builder.const_int64(literal.value as u64),
        ast::Expression::PostfixCall(call) => compile_postfix_call(ctx, builder, call),
        other @ _ => panic!("Cannot build: {:?}", other),
    }
}

fn compile_identifier(
    ctx: Rc<FuncContext>,
    builder: &Builder,
    identifier: &ast::Identifier,
) -> Value {
    use ast::ScopeResolution::*;
    match &identifier.resolution {
        Local(name, typ) => ctx.build_get_local(builder, name),
        other @ _ => unreachable!("Cannot build ScopeResolution: {:?}", other),
    }
}

fn compile_postfix_call(ctx: Rc<FuncContext>, builder: &Builder, call: &ast::PostfixCall) -> Value {
    let target = compile_expression(ctx.clone(), builder, &call.target);
    let mut arguments = vec![];
    for argument in call.arguments.iter() {
        arguments.push(compile_expression(ctx.clone(), builder, argument));
    }
    let retrn = ctx.build_type(&call.typ);
    match target.typ() {
        Type::UnspecializedFunc(unspecialized_func_type) => {
            let arguments_types = arguments
                .iter()
                .map(|argument| argument.typ())
                .collect::<Vec<_>>();
            let specialized_target =
                Value::Func(UnspecializedFuncType::get_or_insert_specialization(
                    &unspecialized_func_type,
                    arguments_types,
                ));
            builder.build_call(specialized_target, arguments, &retrn)
        }
        other @ _ => unreachable!("Cannot build call to Type: {:?}", other),
    }
}

pub type ValueId = usize;

#[derive(Clone, Debug)]
pub enum Value {
    Func(Rc<FuncValue>),
    /// If option is `Some` then this is a const value.
    Int64(ValueId, Option<u64>),
    Tuple(ValueId, TupleType),
    // Fake value to represent an unspecialized func; we'll specialize
    // just-in-time when we try to build a call.
    UnspecializedFunc(Rc<UnspecializedFuncType>),
}

impl Value {
    fn typ(&self) -> Type {
        use Value::*;
        match self {
            Func(func_value) => Type::Func(func_value.typ.clone()),
            Int64(_, _) => Type::Int64,
            Tuple(_, tuple_type) => Type::Tuple(tuple_type.clone()),
            UnspecializedFunc(unspecialized_func_type) => {
                Type::UnspecializedFunc(unspecialized_func_type.clone())
            }
        }
    }

    pub fn value_id(&self) -> ValueId {
        use Value::*;
        match self {
            Func(func_value) => func_value.id,
            Int64(value_id, _) => *value_id,
            Tuple(value_id, _) => *value_id,
            UnspecializedFunc(_) => {
                unreachable!("Unspecialized funcs can't be compiled to a Value")
            }
        }
    }
}

#[derive(Debug)]
pub struct FuncValue {
    id: ValueId,
    pub name: String,
    // The specialized type that this is an implementation of.
    pub typ: Rc<FuncType>,
    // The value needs to exist before it's fully compiled.
    pub basic_blocks: RefCell<Vec<BasicBlock>>,
    pub stack_frame: RefCell<HashMap<String, Type>>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

trait Context: Debug {
    /// Returns the fully-qualified name of the context; this should uniquely
    /// identify the context.
    fn get_qualified_name(&self) -> String;

    fn get_module_ctx(&self) -> Rc<ModuleContext>;

    fn build_type(&self, ast_type: &ast::Type) -> Type;
}

/// The top-level context that keeps track of all modules, functions,
/// types, etc.
struct CompileContext {}

impl Context for CompileContext {
    fn get_qualified_name(&self) -> String {
        unreachable!()
    }

    fn get_module_ctx(&self) -> Rc<ModuleContext> {
        unreachable!()
    }

    fn build_type(&self, ast_type: &ast::Type) -> Type {
        unreachable!()
    }
}

impl Debug for CompileContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "CompileContext({:p})", self)
    }
}

struct ModuleContext {
    compile_ctx: Rc<CompileContext>,
    module: Rc<Module>,
    name: String,
    // Little hack so that we can refer back to ourselves.
    circular: RefCell<Option<Weak<ModuleContext>>>,
    // There aren't actually specializations at the module level, but this
    // lets use reuse all the other type-building logic.
    specializer: RefCell<Option<Specializer>>,
}

impl ModuleContext {
    fn new(compile_ctx: Rc<CompileContext>, module: Rc<Module>) -> Rc<Self> {
        let name = module.name.clone();
        let ctx = Rc::new(Self {
            compile_ctx,
            module,
            name: name.clone(),
            circular: RefCell::new(None),
            specializer: RefCell::new(None),
        });
        {
            // Once we've built the pointer update the circular reference to
            // point to itself.
            let mut circular = ctx.circular.borrow_mut();
            *circular = Some(Rc::downgrade(&ctx));
            // Do the same thing with the specializer.
            let mut specializer = ctx.specializer.borrow_mut();
            *specializer = Some(Specializer::new(Rc::downgrade(&ctx), name));
        }
        ctx
    }

    fn get_static_type<S: AsRef<str>>(&self, name: S) -> Type {
        let static_frame = self.module.static_frame.borrow();
        static_frame
            .get(name.as_ref())
            .expect(&format!("Not in static frame: {}", name.as_ref()))
            .clone()
    }
}

impl Context for ModuleContext {
    fn get_qualified_name(&self) -> String {
        self.name.clone()
    }

    fn get_module_ctx(&self) -> Rc<ModuleContext> {
        let borrowed = self.circular.borrow();
        let circular = borrowed.as_ref().unwrap();
        circular.upgrade().unwrap()
    }

    fn build_type(&self, ast_type: &ast::Type) -> Type {
        let mut mutable = self.specializer.borrow_mut();
        let specializer = mutable.as_mut().unwrap();
        specializer.build_type(ast_type)
    }
}

impl Debug for ModuleContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "ModuleContext({:p})", self)
    }
}

struct FuncContext {
    parent_ctx: Rc<dyn Context>,
    // The func that we're building.
    func_value: Rc<FuncValue>,
    specializer: RefCell<Specializer>,
}

impl FuncContext {
    fn get_local_type<S: AsRef<str>>(&self, name: S) -> Type {
        let stack_frame = self.func_value.stack_frame.borrow();
        stack_frame
            .get(name.as_ref())
            .expect(&format!("Not in stack frame: {}", name.as_ref()))
            .clone()
    }

    fn build_get_local<S: AsRef<str>>(&self, builder: &Builder, name: S) -> Value {
        let stack_frame = self.func_value.stack_frame.borrow();
        let typ = stack_frame
            .get(name.as_ref())
            .expect(&format!("Not in stack frame: {}", name.as_ref()));
        match typ {
            Type::Int64 => builder.build_get_local(name.as_ref().to_string(), typ),
            Type::UnspecializedFunc(unspecialized_func_type) => {
                Value::UnspecializedFunc(unspecialized_func_type.clone())
            }
            other @ _ => unreachable!("Cannot build get-local of Type: {:?}", typ),
        }
    }

    fn build_set_local(&self, builder: &Builder, name: String, value: Value) {
        // TODO: Check that the type of the value we're trying to set matches
        //   the type in the frame.
        let typ = value.typ();
        match typ {
            Type::UnspecializedFunc(_) => {
                // Unspecialized funcs don't actually have a value so we don't
                // build any instructions.
            }
            other @ _ => unreachable!("Cannot build set-local of Value: {:?}", value),
        }
    }
}

impl Context for FuncContext {
    fn get_qualified_name(&self) -> String {
        self.func_value.name.clone()
    }

    fn get_module_ctx(&self) -> Rc<ModuleContext> {
        self.parent_ctx.get_module_ctx()
    }

    fn build_type(&self, ast_type: &ast::Type) -> Type {
        let mut specializer = self.specializer.borrow_mut();
        specializer.build_type(ast_type)
    }
}

impl Debug for FuncContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "FuncContext({:p})", self)
    }
}

pub struct Module {
    path: PathBuf,
    name: String,
    // Keep track of all the functions defined in this module.
    pub func_values: RefCell<Vec<Rc<FuncValue>>>,
    // Static (const) scope.
    pub static_frame: RefCell<HashMap<String, Type>>,
}

impl Module {
    fn add_func_value(&self, name: String, typ: Rc<FuncType>) -> Rc<FuncValue> {
        let id = { self.func_values.borrow().len() };
        let func_value = Rc::new(FuncValue {
            id,
            name,
            typ,
            basic_blocks: RefCell::new(vec![]),
            stack_frame: RefCell::new(HashMap::new()),
        });
        {
            let mut func_values = self.func_values.borrow_mut();
            func_values.push(func_value.clone());
        }
        func_value
    }
}

impl Debug for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Module({}, {:p})", self.name, self)
    }
}

pub fn compile_modules<'m, M: Iterator<Item = &'m FrontendModule>>(
    frontend_modules: M,
) -> Vec<Rc<Module>> {
    let ctx = Rc::new(CompileContext {});
    let mut modules = vec![];
    for frontend_module in frontend_modules {
        modules.push(compile_module(ctx.clone(), frontend_module));
    }
    println!("modules: {:#?}", modules[0].func_values);
    modules
}

fn compile_module(compile_ctx: Rc<CompileContext>, frontend_module: &FrontendModule) -> Rc<Module> {
    let name = path_to_name(frontend_module.path());
    let module = Rc::new(Module {
        path: frontend_module.path().to_path_buf(),
        name: name.clone(),
        func_values: RefCell::new(vec![]),
        static_frame: RefCell::new(HashMap::new()),
    });
    let module_ctx = ModuleContext::new(compile_ctx, module.clone());

    let ast = frontend_module.unwrap_ast();
    // Add all the entries to the static frame.
    {
        let mut static_frame = module.static_frame.borrow_mut();
        let scope = ast.scope.unwrap_module();
        for (name, ast_typ) in scope.statics.iter() {
            let typ = module_ctx.build_type(ast_typ);
            static_frame.insert(name.clone(), typ);
        }
    }

    let mut func_names = vec![];

    // Add all the top-level funcs to the module.
    for statement in ast.statements.iter() {
        use ast::ModuleStatement::*;
        match statement {
            Func(func) => {
                let func_type = module_ctx
                    .get_static_type(&func.name)
                    .unwrap_unspecialized_func()
                    .clone();
                // Save the implementation so that we can specialize it later.
                func_type.set_ast_func(func.clone());
                func_names.push(func.name.clone());
            }
        }
    }

    // Preemptively build all of the funcs that don't need to be specialized
    // (ie. don't have generics).
    for name in func_names {
        let func_type = module_ctx
            .get_static_type(name)
            .unwrap_unspecialized_func()
            .clone();
        if func_type.needs_specialization() {
            continue;
        }
        let ast_func = func_type.get_ast_func();
        let ast_type = ast_func.typ.unwrap_func();
        let arguments = ast_type
            .arguments
            .borrow()
            .iter()
            .map(|argument| module_ctx.build_type(argument))
            .collect::<Vec<_>>();
        UnspecializedFuncType::get_or_insert_specialization(&func_type, arguments);
    }

    module
}

struct Specializer {
    module_ctx: Weak<ModuleContext>,
    /// The fully-qualified name of the module or frame context we're building
    /// in. We need this to give any generated types the correct name.
    name: String,
    // TODO: Track and resolve generics.
    cache: HashMap<TypeId, Type>,
}

impl Specializer {
    fn new(module_ctx: Weak<ModuleContext>, name: String) -> Self {
        Self {
            module_ctx,
            name,
            cache: HashMap::new(),
        }
    }

    // Save an AST-to-IR type mapping. Used to apply generics and also to
    // speed up mapping in general.
    fn cache_type(&mut self, ast_type: &ast::Type, typ: Type) {
        self.cache.insert(ast_type.id(), typ);
    }

    fn build_type(&mut self, ast_type: &ast::Type) -> Type {
        if let Some(cached) = self.cache.get(&ast_type.id()) {
            return cached.clone();
        }
        let typ = match ast_type {
            ast::Type::Func(func) => {
                let ctx = self.module_ctx.upgrade().unwrap();
                let qualified_name =
                    format!("{}_{}", self.name.clone(), func.name.clone().unwrap());
                Type::UnspecializedFunc(Rc::new(UnspecializedFuncType {
                    ctx,
                    name: qualified_name,
                    specializations: RefCell::new(vec![]),
                    ast_typ: ast_type.clone(),
                    ast_func: RefCell::new(None),
                }))
            }
            ast::Type::Object(object) => match &object.class {
                ast::Class::Intrinsic(intrinsic) => match intrinsic.name.as_str() {
                    "Int" => Type::Int64,
                    _ => unreachable!("Cannot build intrinsic: {:?}", intrinsic.name),
                },
                ast::Class::Derived(derived) => unreachable!("Cannot build derived: {:?}", derived),
            },
            other @ _ => unreachable!("Cannot build type: {:?}", other),
        };
        self.cache_type(ast_type, typ.clone());
        typ
    }
}

pub fn path_to_name<P: Into<PathBuf>>(path: P) -> String {
    let slashes = Regex::new(r"[/\\]").unwrap();
    let repeated_underscores = Regex::new(r"_+").unwrap();
    let extension = Regex::new(r"\.hb$").unwrap();
    let invalid = Regex::new(r"[^A-Za-z0-9_]").unwrap();
    let simplify = Regex::new(r"^_?(?P<inner>.+)_?$").unwrap();

    let path = path.into().to_str().unwrap().to_string();
    let without_slashes = slashes.replace_all(&path, "_");
    let condensed_underscores = repeated_underscores.replace_all(&without_slashes, "_");
    let without_extension = extension.replace(&condensed_underscores, "");
    let without_invalid = invalid.replace_all(&without_extension, "");
    let simplified = simplify.replace(&without_invalid, "$inner");

    simplified.to_string()
}
