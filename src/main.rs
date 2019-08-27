extern crate codespan;
extern crate codespan_reporting;
#[macro_use]
extern crate gc;
#[macro_use]
extern crate lazy_static;
extern crate termcolor;

use std::env;
use std::process::exit;

mod ast;
mod ast_to_ir;
mod ir;
mod parser;
mod target;
mod vm;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 1 || args.len() > 2 {
        println!("Usage: hummingbird [file]?");
        exit(-1);
    }
    match args.len() {
        1 => {
            vm::Vm::run_repl();
        }
        2 => {
            let filename = &args[1];
            vm::Vm::run_file(filename);
        }
        _ => unreachable!(),
    }
}
