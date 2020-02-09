#![allow(unused_imports)]
#![allow(unused_macros)]

extern crate codespan;
extern crate codespan_reporting;
#[macro_use]
extern crate lazy_static;
extern crate termcolor;

use std::env;
use std::process::exit;

mod parse_ast;
mod parser;
mod type_ast;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 1 || args.len() > 2 {
        println!("Usage: hummingbird [file]?");
        exit(-1);
    }
    match args.len() {
        2 => {
            let filename = &args[1];
            let source = std::fs::read_to_string(filename).unwrap();
            let mut token_stream = parser::TokenStream::from_string(source);
            let parse_ast = parser::parse_module(&mut token_stream).unwrap();
            let type_ast = type_ast::translate_module(parse_ast).unwrap();
            let printer = type_ast::Printer::new(std::io::stdout());
            printer.print_module(type_ast).unwrap();
        }
        _ => unreachable!(),
    }
}
