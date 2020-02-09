use std::fmt::{Debug, Display, Error, Formatter};

use super::super::parse_ast::*;
use super::lexer::{Token, TokenStream};
use super::{Location, Span};

type ParseResult<T> = Result<T, ParseError>;

/// Using a macro instead of a method so that we can stringify the pattern.
macro_rules! expect_to_read {
    ($i:ident, { $($p:pat => $m:tt),+ $(,)* }) => {
        match $i.read() {
            $($p => $m,)+
            got @ _ => {
                return Err(ParseError::new_unexpected(
                    vec![$(stringify!($p).to_string()),+],
                    got,
                ));
            }
        }
    };
}

pub fn parse_module(input: &mut TokenStream) -> ParseResult<Module> {
    let mut statements = vec![];
    while !input.peek().is_eof() {
        if let Some(statement) = parse_module_statement(input)? {
            statements.push(statement)
        }
        statements.append(&mut expect_module_terminals(input)?);
    }
    Ok(Module { statements })
}

/// Must receive a `Token::CommentLine`; returns a corresponding
/// `ModuleStatement::CommentLine`.
fn token_to_comment_line(token: Token) -> CommentLine {
    if let Token::CommentLine(content, span) = token {
        CommentLine { content, span }
    } else {
        unreachable!("Expected Token::CommentLine; got {:?}", token)
    }
}

/// Expect at least one module-level terminal. Returns any comments found along the way.
fn expect_module_terminals(input: &mut TokenStream) -> ParseResult<Vec<ModuleStatement>> {
    let mut comments = vec![];
    let mut found_terminal = false;
    let mut next;
    loop {
        // NOTE: If we match comments or newlines we'll call `input.read()`
        // to consume the token, but for EOFs we leave them on the input to
        // be picked up by our caller.
        next = input.peek();
        match next {
            Token::CommentLine(_, _) => comments.push(ModuleStatement::CommentLine(
                token_to_comment_line(input.read()),
            )),
            Token::EOF(_) => {
                found_terminal = true;
                break;
            }
            Token::Newline(_) => {
                input.read();
                found_terminal = true;
            }
            _ => break,
        }
    }
    if found_terminal {
        Ok(comments)
    } else {
        Err(ParseError::new_unexpected(
            vec![
                "Comment".to_string(),
                "EOF".to_string(),
                "Newline".to_string(),
            ],
            next.clone(),
        ))
    }
}

fn parse_module_statement(input: &mut TokenStream) -> ParseResult<Option<ModuleStatement>> {
    let next = input.peek();
    Ok(match next {
        token @ Token::CommentLine(_, _) => {
            Some(ModuleStatement::CommentLine(token_to_comment_line(token)))
        }
        Token::Func(_) => Some(ModuleStatement::Func(expect_func(input)?)),
        Token::Import(_) => Some(expect_import(input)?),
        Token::Newline(_) => None,
        _ => return Err(ParseError::new_unexpected(vec!["None".to_string()], next)),
    })
}

fn expect_block(input: &mut TokenStream) -> ParseResult<Block> {
    let start = expect_to_read!(input, { Token::BraceLeft(location) => location });
    let mut statements = vec![];
    while !input.peek().is_brace_right() {
        if let Some(statement) = parse_block_statement(input)? {
            statements.push(statement)
        }
        statements.append(&mut expect_block_terminals(input)?)
    }
    let end = expect_to_read!(input, { Token::BraceRight(location) => location });
    Ok(Block {
        statements,
        span: Span::new(start, end),
    })
}

fn parse_block_statement(input: &mut TokenStream) -> ParseResult<Option<BlockStatement>> {
    // TODO: If, else, etc.
    Ok(match input.peek() {
        Token::CommentLine(_, _) => Some(BlockStatement::CommentLine(
            token_to_comment_line(input.read()),
        )),
        Token::Newline(_) => None,
        Token::Func(_) => Some(BlockStatement::Func(expect_func(input)?)),
        _ => Some(BlockStatement::Expression(parse_expression(input)?)),
    })
}

/// Expect at least one module-level terminal. Returns any comments found along the way.
fn expect_block_terminals(input: &mut TokenStream) -> ParseResult<Vec<BlockStatement>> {
    let mut comments = vec![];
    let mut found_terminal = false;
    let mut next;
    loop {
        // NOTE: If we match comments or newlines we'll call `input.read()`
        // to consume the token, but for right-braces we leave them on the
        // input to be picked up by our caller.
        next = input.peek();
        match next {
            Token::CommentLine(_, _) => comments.push(BlockStatement::CommentLine(
                token_to_comment_line(input.read()),
            )),
            Token::BraceRight(_) => {
                found_terminal = true;
                break;
            }
            Token::Newline(_) => {
                input.read();
                found_terminal = true;
            }
            _ => break,
        }
    }
    if found_terminal {
        Ok(comments)
    } else {
        Err(ParseError::new_unexpected(
            vec![
                "Comment".to_string(),
                "BraceRight".to_string(),
                "Newline".to_string(),
            ],
            next.clone(),
        ))
    }
}

fn parse_expression(input: &mut TokenStream) -> ParseResult<Expression> {
    parse_infix(input)
}

/// A callable to parse a level of infix operations.
type InfixParser = Box<dyn Fn(&mut TokenStream) -> ParseResult<Expression> + Send + Sync>;

/// Construct an `InfixParser` which will parse infix expressions with the
/// given token as the operator. `next` will be called to parse the left- and
/// right-hand sides of the operator.
fn infix_parser(token: Token, next: InfixParser) -> InfixParser {
    return Box::new(move |input: &mut TokenStream| {
        let mut lhs = next(input)?;
        loop {
            if input.peek().same_variant_as(&token) {
                let op = input.read();
                let rhs = next(input)?;
                lhs = Expression::Infix(Infix {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                })
            } else {
                break;
            }
        }
        Ok(lhs)
    });
}

lazy_static! {
    // Only initialize these closures once.
    static ref PARSE_INFIX: InfixParser = {
        let atom = Box::new(parse_postfix);
        // Implement PEMDAS associativity rules.
        let mul = infix_parser(Token::Star(Location::unknown()), atom);
        let add = infix_parser(Token::Plus(Location::unknown()), mul);
        add
    };
}

/// Parse infix expressions and descendants (postfixes, groups, and atoms).
fn parse_infix(input: &mut TokenStream) -> ParseResult<Expression> {
    PARSE_INFIX(input)
}

fn parse_postfix(input: &mut TokenStream) -> ParseResult<Expression> {
    let mut target = parse_group(input)?;
    loop {
        match input.peek() {
            Token::Dot(start) => {
                input.read();
                let property = expect_to_read!(input, { Token::Word(word) => word });
                target = Expression::PostfixProperty(PostfixProperty {
                    target: Box::new(target),
                    property: property.clone(),
                    span: Span::new(start, property.span.end),
                })
            }
            _ => break,
        }
    }
    Ok(target)
}

fn parse_group(input: &mut TokenStream) -> ParseResult<Expression> {
    if let Token::ParenthesesLeft(_) = input.peek() {
        input.read();
        let expression = parse_expression(input)?;
        expect_to_read!(input, { Token::ParenthesesRight(_) => () });
        Ok(expression)
    } else {
        expect_atom(input)
    }
}

/// Parse an identifier or literal.
fn expect_atom(input: &mut TokenStream) -> ParseResult<Expression> {
    Ok(expect_to_read!(input, {
        Token::Word(word) => {
            Expression::Identifier(Identifier {
                name: word,
            })
        },
        Token::LiteralInt(literal) => {
            Expression::LiteralInt(LiteralInt {
                value: literal.value,
                span: literal.span,
            })
        },
    }))
}

fn expect_func(input: &mut TokenStream) -> ParseResult<Func> {
    let start = expect_to_read!(input, { Token::Func(start) => start });
    let name = expect_to_read!(input, { Token::Word(word) => word });
    expect_to_read!(input, { Token::ParenthesesLeft(_) => () });
    let mut arguments = vec![];
    loop {
        let next = input.peek();
        if let Token::ParenthesesRight(_) = next {
            break;
        }
        let name = expect_to_read!(input, { Token::Word(word) => word });
        arguments.push(name);
        if let Token::Comma(_) = input.peek() {
            input.read();
            continue;
        } else {
            break;
        }
    }
    expect_to_read!(input, { Token::ParenthesesRight(_) => () });
    let block = expect_block(input)?;
    let end = block.span.end.clone();
    Ok(Func {
        name,
        arguments,
        body: FuncBody::Block(block),
        span: Span::new(start, end),
    })
}

fn expect_import(input: &mut TokenStream) -> ParseResult<ModuleStatement> {
    let start = expect_to_read!(input, { Token::Import(start) => start });
    Ok(ModuleStatement::Import(Import {
        path: "test".to_string(),
        span: Span::new(start.clone(), start),
    }))
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

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Unexpected {
        expected: Vec<String>,
        got: String,
        location: Location,
    },
}

impl ParseError {
    fn new_unexpected(expected: Vec<String>, got: Token) -> Self {
        let location = got.location();
        Self::Unexpected {
            expected: expected
                .into_iter()
                .map(|name| base_name(&name).to_string())
                .collect::<Vec<_>>(),
            got: base_name(&format!("{:?}", got)).to_string(),
            location,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use ParseError::*;
        match self {
            Unexpected {
                expected,
                got,
                location,
            } => {
                write!(f, "Unexpected token: got {}", got,)?;
                if !expected.is_empty() {
                    write!(f, ", expected {}", expected.join(" or "))?;
                }
                write!(f, " at line {} column {}", location.line, location.column)?;
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::super::super::parse_ast::*;
    use super::super::lexer::TokenStream;
    use super::super::{Location, Span, Token, Word};
    use super::{expect_block, parse_infix, parse_module, parse_postfix};

    fn input(input: &str) -> TokenStream {
        TokenStream::from_string(input.to_string())
    }

    #[test]
    fn test_parse_module() {
        assert_eq!(
            parse_module(&mut input("import // foo\n// bar")),
            Ok(Module {
                statements: vec![
                    ModuleStatement::Import(Import {
                        path: "test".to_string(),
                        span: Span::new(Location::new(0, 1, 1), Location::new(0, 1, 1))
                    }),
                    ModuleStatement::CommentLine(CommentLine {
                        content: "// foo".to_string(),
                        span: Span::new(Location::new(7, 1, 8), Location::new(13, 1, 14)),
                    }),
                    ModuleStatement::CommentLine(CommentLine {
                        content: "// bar".to_string(),
                        span: Span::new(Location::new(14, 2, 1), Location::new(20, 2, 7)),
                    }),
                ]
            })
        );
        assert_eq!(
            parse_module(&mut input("func foo() {}")),
            Ok(Module {
                statements: vec![ModuleStatement::Func(Func {
                    name: Word {
                        name: "foo".to_string(),
                        span: Span::new(Location::new(5, 1, 6), Location::new(8, 1, 9),)
                    },
                    arguments: vec![],
                    body: FuncBody::Block(Block {
                        statements: vec![],
                        span: Span::new(Location::new(11, 1, 12), Location::new(12, 1, 13))
                    }),
                    span: Span::new(Location::new(0, 1, 1), Location::new(12, 1, 13))
                })]
            })
        );
    }

    #[test]
    fn test_parse_infix() {
        assert_eq!(
            parse_infix(&mut input("1 * 2 + 3 * 4")),
            Ok(Expression::Infix(Infix {
                lhs: Box::new(Expression::Infix(Infix {
                    lhs: Box::new(Expression::LiteralInt(LiteralInt {
                        value: 1,
                        span: Span::unknown(),
                    })),
                    op: Token::Star(Location::unknown()),
                    rhs: Box::new(Expression::LiteralInt(LiteralInt {
                        value: 2,
                        span: Span::unknown(),
                    })),
                })),
                op: Token::Plus(Location::unknown()),
                rhs: Box::new(Expression::Infix(Infix {
                    lhs: Box::new(Expression::LiteralInt(LiteralInt {
                        value: 3,
                        span: Span::unknown(),
                    })),
                    op: Token::Star(Location::unknown()),
                    rhs: Box::new(Expression::LiteralInt(LiteralInt {
                        value: 4,
                        span: Span::unknown(),
                    })),
                }))
            }))
        );
    }

    #[test]
    fn test_parse_postfix() {
        assert_eq!(
            parse_postfix(&mut input("foo.bar")),
            Ok(Expression::PostfixProperty(PostfixProperty {
                target: Box::new(Expression::Identifier(Identifier {
                    name: Word {
                        name: "foo".to_string(),
                        span: Span::unknown(),
                    }
                })),
                property: Word {
                    name: "bar".to_string(),
                    span: Span::unknown(),
                },
                span: Span::unknown(),
            }))
        );
    }

    #[test]
    fn test_expect_block() {
        assert_eq!(
            expect_block(&mut input("{}")),
            Ok(Block {
                statements: vec![],
                span: Span::new(Location::new(0, 1, 1), Location::new(1, 1, 2)),
            }),
        );
        assert_eq!(
            expect_block(&mut input("{\n  foo\n  // bar\n  baz\n}")),
            Ok(Block {
                statements: vec![
                    BlockStatement::Expression(Expression::Identifier(Identifier {
                        name: Word {
                            name: "foo".to_string(),
                            span: Span::new(Location::new(4, 2, 3), Location::new(7, 2, 6))
                        }
                    })),
                    BlockStatement::CommentLine(CommentLine {
                        content: "// bar".to_string(),
                        span: Span::new(Location::new(10, 3, 3), Location::new(16, 3, 9)),
                    }),
                    BlockStatement::Expression(Expression::Identifier(Identifier {
                        name: Word {
                            name: "baz".to_string(),
                            span: Span::new(Location::new(19, 4, 3), Location::new(22, 4, 6))
                        }
                    }))
                ],
                span: Span::new(Location::new(0, 1, 1), Location::new(23, 5, 1)),
            }),
        );
    }

    #[test]
    fn test_parse_okay() {
        parse_module(&mut input(
            "
            // A comment about foo.
            func foo(a, b) { (a + b) * 2 }
        ",
        ))
        .unwrap();
    }
}
