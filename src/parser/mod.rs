use super::ast::{Node, Program};

mod lexer;
mod location;
mod parser;

pub use self::lexer::Token;
pub use self::location::Location;

pub fn parse<I: ToString>(input: I) -> Program {
    let mut token_stream = lexer::TokenStream::from_string(input.to_string());
    let node = parser::parse_program(&mut token_stream);
    match node {
        Node::Program(program) => program,
        _ => panic!("Not a Program: {:?}", node),
    }
}
