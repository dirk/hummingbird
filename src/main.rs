#![allow(dead_code)]
#![allow(private_in_public)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unused_variables)]

extern crate codespan;
extern crate codespan_reporting;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate paste;
extern crate regex;
extern crate termcolor;

use std::env;
use std::path::PathBuf;
use std::process::exit;

mod compiler;
mod frontend;
mod parse_ast;
mod parser;
mod type_ast;

use frontend::CompileError;
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
    println!("Usage: hummingbird [command] [file] [options]");
    println!();
    println!("Commands:");
    println!("  compile  Build an executable from the file.");
    println!("  ast      Print the typed AST of a file.");
    println!();
    println!("Options:");
    println!("  --print-pointers  Include pointers in debugging output");
}

fn handle_compile_error(error: CompileError) {
    match error {
        CompileError::Type(type_error, path, source) => {
            print_type_error(type_error, path.to_str().unwrap().to_string(), source)
        }
        other @ _ => panic!("{:#?}", other),
    }
    exit(-1);
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() <= 1 {
        print_usage();
        exit(-1);
    }
    // Remove the first argument (ourselves).
    let called = args[0].clone();
    let args = args[1..].to_vec();
    let (args, print_pointers) = extract_option(args, "--print-pointers");

    // Turn them into `&str`s so that we can match against them.
    let arg0 = args.get(0).map(|arg| arg.as_str());
    let arg1 = args.get(1).map(|arg| arg.as_str());

    match (arg0, arg1) {
        (Some("help"), None) => {
            print_usage();
            exit(0);
        }
        (Some("compile"), Some(filename)) => {
            match frontend::Manager::compile_main(filename.into()) {
                Ok(_) => (),
                Err(error) => handle_compile_error(error),
            }
        }
        (Some("ast"), Some(filename)) => {
            let manager = frontend::Manager::new();
            match manager.load(filename.into()) {
                Ok(module) => {
                    let printer = Printer::new_with_options(
                        std::io::stdout(),
                        PrinterOptions { print_pointers },
                    );
                    let ast = module.unwrap_ast();
                    printer.print_module(&ast).unwrap();
                }
                Err(error) => handle_compile_error(error),
            }
        }
        _ => {
            eprintln!("Invalid args: {:?}", args);
            eprintln!();
            eprintln!("See '{} help' for usage details.", called);
            exit(-1);
        }
    }

    // let filename = &args[0];
    //
    // let source = std::fs::read_to_string(filename).unwrap();
    // let mut token_stream = parser::TokenStream::from_string(source.clone());
    // let parse_ast = parser::parse_module(&mut token_stream).unwrap();
    // let type_ast = match type_ast::translate_module(parse_ast) {
    //     Ok(module) => module,
    //     Err(type_error) => {
    //         print_type_error(type_error, filename.clone(), source);
    //         return;
    //     }
    // };
    // let printer = Printer::new_with_options(std::io::stdout(), PrinterOptions { print_pointers });
    // printer.print_module(type_ast).unwrap();
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
        let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);
        codespan_reporting::term::emit(&mut writer, &config, &files, &diagnostic).unwrap();
    } else {
        // If we don't have a span then just report the error.
        eprintln!("{:#?}", error);
    }
    exit(-1)
}
