#![allow(unused_imports)]
#![allow(unused_macros)]

extern crate codespan;
extern crate codespan_reporting;
#[macro_use]
extern crate lazy_static;
extern crate termcolor;

use std::env;
use std::path::PathBuf;
use std::process::exit;

mod frontend;
mod parse_ast;
mod parser;
mod type_ast;

use type_ast::{Printer, PrinterOptions, TypeError};

fn extract_option<S: AsRef<str>>(args: Vec<String>, option: S) -> (Vec<String>, bool) {
    let mut found = false;
    let mut new = vec![];
    for arg in args.iter() {
        if arg == option.as_ref() {
            found = true
        } else {
            new.push(arg.clone())
        }
    }
    (new, found)
}

fn print_usage() {
    println!("Usage: hummingbird [file] [options]");
    println!();
    println!("  --print-pointers  Include pointers in debugging output");
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() <= 1 {
        print_usage();
        exit(-1);
    }
    // Remove the first argument (ourselves).
    let args = args[1..].to_vec();
    let (args, print_pointers) = extract_option(args, "--print-pointers");
    if args.len() != 1 {
        eprintln!("Invalid args: {:?}", args);
        exit(1);
    }
    if &args[0] == "help" {
        print_usage();
        exit(0);
    }

    let filename = &args[0];
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
    let printer = Printer::new_with_options(std::io::stdout(), PrinterOptions { print_pointers });
    printer.print_module(type_ast).unwrap();

    frontend::Manager::compile_main(filename.into()).unwrap();
}

fn print_type_error(error: TypeError, filename: String, source: String) {
    use codespan::{Files, Span as CodeSpan};
    use codespan_reporting::diagnostic::{Diagnostic, Label};

    use TypeError::*;

    let (error, span) = (error.unwrap(), error.span());

    if let Some(span) = span {
        let mut files = Files::new();
        let file_id = files.add(filename, source);

        let mut diagnostic = Diagnostic::new_error(
            error.short_message(),
            Label::new(
                file_id,
                CodeSpan::new(span.start.index, span.end.index),
                error.label_message(),
            ),
        );
        if let Some(notes) = error.notes() {
            diagnostic = diagnostic.with_notes(vec![notes]);
        }

        let config = codespan_reporting::term::Config::default();
        let mut writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
        codespan_reporting::term::emit(&mut writer, &config, &files, &diagnostic).unwrap();
    } else {
        // If we don't have a span then just report the error.
        println!("{:?}", error);
    }
    exit(1)
}
