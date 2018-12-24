#[macro_use]
extern crate gc;

use std::process::exit;
use std::{env, fs};

mod ast;
mod ir;
mod parser;
mod target;
mod vm;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 2 {
        println!("Usage: hummingbird [file]");
        exit(-1);
    }
    let filename = &args[1];
    let source = fs::read_to_string(filename).expect("Unable to read source file");

    let program = parser::parse(source);
    // println!("Program:\n{:?}", program);

    let mut printer = ast::printer::Printer::new(std::io::stdout());
    printer
        .print_program(program.clone())
        .expect("Unable to print AST");

    let ir_unit = ast::compiler::compile(&program);
    let mut ir_printer = ir::printer::Printer::new(std::io::stdout());
    ir_printer
        .print_module(&ir_unit)
        .expect("Unable to print IR");

    let bytecode_unit = ir::compiler::compile(&ir_unit);
    let mut bytecode_printer = target::bytecode::printer::Printer::new(std::io::stdout());
    bytecode_printer
        .print_unit(&bytecode_unit)
        .expect("Unable to print bytecode");

    vm::Vm::run_file(filename);
    // println!("Bytecode:\n{:?}", bytecode_unit);
}
