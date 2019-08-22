use super::ast::Module;

mod lexer;
mod location;
mod parser;

pub use self::lexer::Token;
pub use self::location::{Location, Span};

pub fn parse<I: ToString>(input: I) -> Module {
    let mut token_stream = lexer::TokenStream::from_string(input.to_string());
    parser::parse_module(&mut token_stream)
}
