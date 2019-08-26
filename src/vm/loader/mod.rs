use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use super::super::ast;
use super::super::ast_to_ir;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;

mod loaded_function;
mod loaded_module;

pub use loaded_module::LoadedModule;
use loaded_module::WeakLoadedModule;
pub use loaded_function::{BytecodeFunction, LoadedFunction};

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

