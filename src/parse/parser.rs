use super::{
    ast::{Import, ImportWhole, Module},
    lexer::{PeekableTokenStream, Token, TokenStream},
    location::{Location, Span},
};

type ParseResult<T> = Result<T, ParseError>;

/// Using a macro instead of a method so that we can stringify the pattern.
macro_rules! expect_to_read {
    ($i:ident, { $($p:pat => $m:tt),+ $(,)* }) => {
        match $i.read() {
            $($p => $m,)+
            got @ _ => {
                return Err(ParseError::new_unexpected(
                    vec![$(stringify!($p)),+],
                    got,
                ));
            }
        }
    };
}

pub fn parse_module(input: &mut PeekableTokenStream) -> ParseResult<Module> {
    let mut imports: Vec<Import> = vec![];

    loop {
        match input.peek() {
            Token::EOF(_) => break,
            Token::Import(_) => {
                let import = parse_import(input)?;
                imports.push(import);
            }
            _ => {
                return Err(ParseError::new_unexpected(vec!["import"], input.read()));
            }
        }
    }

    Ok(Module { imports: vec![] })
}

pub fn parse_import(input: &mut PeekableTokenStream) -> ParseResult<Import> {
    let start = expect_to_read!(input, {
        Token::Import(start) => start,
    });
    let mut more = !input.peek().is_literal_string();
    let mut whole = None;
    let mut members = vec![];
    while more {
        match input.peek() {
            Token::Word(word) => {
                input.read();
                let alias = match input.peek() {
                    Token::As(_) => {
                        input.read();
                        let alias = expect_to_read!(input, {
                            Token::Word(alias) => alias,
                        });
                        Some(alias)
                    }
                    _ => None,
                };
                whole = Some(ImportWhole {
                    name: word.clone(),
                    alias,
                });
            }
            _ => break,
        };
        more = input.peek().is_comma();
    }
    Ok(Import {
        whole,
        members,
        source: "".to_string(),
        span: Span::new(start.clone(), start),
    })
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Unexpected {
        expected: Vec<String>,
        got: String,
        location: Location,
    },
}

impl ParseError {
    fn new_unexpected(expected: Vec<&str>, got: Token) -> Self {
        let location = got.location();
        Self::Unexpected {
            expected: expected
                .into_iter()
                .map(|name| base_name(name).to_string())
                .collect::<Vec<_>>(),
            got: base_name(&format!("{:?}", got)).to_string(),
            location,
        }
    }
}

/// Takes a string from:
///   - `stringify!` on a pattern
///   - A debug-formatted struct or enum
///
/// And returns just the relevant name portion.
///
/// ```
/// base_name("Value::String(string)") // => "String"
/// base_name("Integer(1)") // => "Integer"
/// base_name("Export") // => "Exports"
/// ```
pub fn base_name(name: &str) -> &str {
    let left = name.rfind("::").map(|index| index + 2);
    let right = name.find("(");
    match (left, right) {
        (Some(left), Some(right)) => &name[left..right],
        (None, Some(right)) => &name[..right],
        (Some(left), None) => &name[left..],
        (None, None) => name,
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::Module;
    use super::{super::lexer::PeekableTokenStream, ParseResult};

    fn parse(input: &str) -> ParseResult<Module> {
        super::parse_module(&mut PeekableTokenStream::new_from_string(input))
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(
            parse(""),
            Ok(Module {
                imports: vec![],
            })
        );
        assert_eq!(
            parse("// Just a comment"),
            Ok(Module {
                imports: vec![],
            })
        );
    }
}
