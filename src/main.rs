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

use type_ast::TypeError;

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
            let mut token_stream = parser::TokenStream::from_string(source.clone());
            let parse_ast = parser::parse_module(&mut token_stream).unwrap();
            let type_ast = match type_ast::translate_module(parse_ast) {
                Ok(module) => module,
                Err(type_error) => {
                    print_type_error(type_error, filename.clone(), source);
                    return;
                }
            };
            let printer = type_ast::Printer::new(std::io::stdout());
            printer.print_module(type_ast).unwrap();
        }
        _ => unreachable!(),
    }
}

fn print_type_error(error: TypeError, filename: String, source: String) {
    use codespan::{Files, Span as CodeSpan};
    use codespan_reporting::diagnostic::{Diagnostic, Label};

    use TypeError::*;

    let (error, span) = (error.unwrap(), error.span());

    if let Some(span) = span {
        let mut files = Files::new();
        let file_id = files.add(filename, source);

        let diagnostic = Diagnostic::new_error(
            error.short_message(),
            Label::new(
                file_id,
                CodeSpan::new(span.start.index, span.end.index),
                error.label_message(),
            ),
        )
        .with_notes(vec![format!("{:?}", error)]);

        let config = codespan_reporting::term::Config::default();
        let mut writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
        codespan_reporting::term::emit(&mut writer, &config, &files, &diagnostic).unwrap();
    } else {
        // If we don't have a span then just report the error.
        println!("{:?}", error);
    }
    exit(1)
}
