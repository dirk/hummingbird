use std::process::exit;
use std::{env, fs};

mod ast;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 2 {
        println!("Usage: hummingbird [file]");
        exit(-1);
    }
    let filename = &args[1];
    let source = fs::read_to_string(filename).expect("Unable to read source file");

    let program = parser::parse(source);
    println!("Program:\n{:?}", program);
}
