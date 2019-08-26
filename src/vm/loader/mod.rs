use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

use super::super::ast;
use super::super::ast_to_ir;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;
use super::frame::Closure;

mod loaded_module;

pub use loaded_module::LoadedModule;
use loaded_module::WeakLoadedModule;
use std::convert::TryInto;

lazy_static! {
    static ref DEBUG_ALL: bool = env::var("DEBUG_ALL").is_ok();
    static ref DEBUG_AST: bool = (*DEBUG_ALL || env::var("DEBUG_AST").is_ok());
    static ref DEBUG_IR: bool = (*DEBUG_ALL || env::var("DEBUG_IR").is_ok());
    static ref DEBUG_BYTECODE: bool = (*DEBUG_ALL || env::var("DEBUG_BYTECODE").is_ok());
}

fn read_and_parse_file<P: AsRef<Path>>(path: P) -> Result<ast::Module, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    Ok(parser::parse(source))
}

pub fn compile_ast_into_module(
    ast_module: &ast::Module,
    name: String,
    ast_flags: ast_to_ir::CompilationFlags,
) -> Result<LoadedModule, Box<dyn Error>> {
    let ir_module = ast_to_ir::compile(ast_module, ast_flags);
    if *DEBUG_IR {
        println!("IR({}):", name);
        ir::printer::Printer::new(std::io::stdout()).print_module(&ir_module)?;
        println!();
    }

    let bytecode_module = ir::compiler::compile(&ir_module);
    if *DEBUG_BYTECODE {
        println!("Bytecode({}):", name);
        bytecode::printer::Printer::new(std::io::stdout()).print_module(&bytecode_module)?;
    }

    let loaded_module = LoadedModule::from_bytecode(bytecode_module, name);
    Ok(loaded_module)
}

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<LoadedModule, Box<dyn Error>> {
    let name = path
        .as_ref()
        .to_str()
        .expect("Couldn't convert path to string")
        .to_owned();

    let ast_module = read_and_parse_file(&name)?;
    if *DEBUG_AST {
        println!("AST({}):", name);
        ast::printer::Printer::new(std::io::stdout()).print_module(ast_module.clone())?;
        println!();
    }

    compile_ast_into_module(&ast_module, name, Default::default())
}

#[derive(Clone)]
struct InnerLoadedFunction {
    module: WeakLoadedModule,
    id: u16,
    bytecode: BytecodeFunction,
}

/// Handle to a loaded function.
#[derive(Clone)]
pub struct LoadedFunction(Rc<InnerLoadedFunction>);

impl LoadedFunction {
    fn new(module: WeakLoadedModule, function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerLoadedFunction {
            module,
            id: function.id,
            bytecode: BytecodeFunction::new(function),
        }))
    }

    pub fn id(&self) -> u16 {
        self.0.id
    }

    /// Returns a string indicating the base-name of the module it was defined
    /// in and its own name.
    pub fn qualified_name(&self) -> String {
        let module_basename = Path::new(&self.module().name())
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("(unknown)")
            .to_owned();
        let own_name = self.0.bytecode.name();
        format!("{}:{}", module_basename, own_name)
    }

    pub fn bytecode(&self) -> BytecodeFunction {
        self.0.bytecode.clone()
    }

    /// Returns whether or not this function binds/captures its environment
    /// when it is created.
    pub fn binds_on_create(&self) -> bool {
        self.0.bytecode.parent_bindings()
    }

    /// Returns a closure suitable for calling the function.
    pub fn build_closure_for_call(&self, parent: Option<Closure>) -> Option<Closure> {
        let bindings = self.0.bytecode.bindings();
        // Whether or not it needs to create bindings (a closure) when called.
        let binds_on_call = !bindings.is_empty() || self.0.bytecode.parent_bindings();
        if binds_on_call {
            let bindings = if bindings.is_empty() {
                None
            } else {
                Some(bindings)
            };
            Some(Closure::new(bindings, parent))
        } else {
            None
        }
    }

    pub fn module(&self) -> LoadedModule {
        self.0
            .module
            .clone()
            .try_into()
            .expect("Module has been dropped")
    }
}

pub struct InnerBytecodeFunction {
    function: bytecode::layout::Function,
}

impl InnerBytecodeFunction {
    pub fn name(&self) -> &str {
        &self.function.name
    }

    #[inline]
    pub fn registers(&self) -> u8 {
        self.function.registers
    }

    #[inline]
    pub fn locals(&self) -> u8 {
        self.function.locals
    }

    #[inline]
    pub fn instruction(&self, instruction_address: usize) -> bytecode::layout::Instruction {
        self.function.instructions[instruction_address].clone()
    }

    pub fn locals_names(&self) -> Vec<String> {
        self.function.locals_names.clone()
    }

    pub fn has_bindings(&self) -> bool {
        !self.function.bindings.is_empty()
    }

    pub fn bindings(&self) -> HashSet<String> {
        self.function.bindings.clone()
    }

    pub fn parent_bindings(&self) -> bool {
        self.function.parent_bindings
    }
}

#[derive(Clone)]
pub struct BytecodeFunction(Rc<InnerBytecodeFunction>);

impl BytecodeFunction {
    pub fn new(function: bytecode::layout::Function) -> Self {
        Self(Rc::new(InnerBytecodeFunction { function }))
    }
}

impl Deref for BytecodeFunction {
    type Target = InnerBytecodeFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
