#[macro_use]
extern crate gc;

use std::process::exit;
use std::{env, fs};

mod ast;
mod parser;
mod vm;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 2 {
        println!("Usage: hummingbird [file]");
        exit(-1);
    }
    let filename = &args[1];
    let source = fs::read_to_string(filename).expect("Unable to read source file");

    let program = parser::parse(source.clone());

    println!("AST:");
    let mut printer = ast::printer::Printer::new(std::io::stdout());
    printer
        .print_module(program.clone())
        .expect("Unable to print AST");

    let mut vm = vm::Vm::new();
    vm.eval_source(source);
}
