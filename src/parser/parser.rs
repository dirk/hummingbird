use std::fmt::{Debug, Display, Error, Formatter};

use super::super::ast::nodes::{
    Assignment, Block, Export, Function, Identifier, If, Infix, Integer, Let, Module, Node,
    PostfixCall, PostfixProperty, Return, Var, While,
};

use super::super::ast::{Import, ImportBindings, StringLiteral, SymbolLiteral};
use super::lexer::{Token, TokenStream};
use super::location::{Location, Span};

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Unexpected {
        expected: Vec<String>,
        got: String,
        location: Option<Location>,
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
                if let Some(location) = location {
                    write!(f, " at line {} column {}", location.line, location.column)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {}

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

/// Using a macro instead of a method so that we can stringify the pattern.
macro_rules! expect_to_read2 {
    ($i:ident, $p:pat => $m:expr) => {
        match $i.read() {
            $p => $m,
            got @ _ => {
                return Err(ParseError::new_unexpected(
                    vec![stringify!($p).to_string()],
                    got,
                ));
            }
        }
    };
}

/// Parsing entry point.
pub fn parse_module(input: &mut TokenStream) -> Result<Module, ParseError> {
    let mut nodes: Vec<Node> = Vec::new();
    while input.peek() != Token::EOF {
        let mut statements = parse_statements(input, Token::EOF, StatementContext::Module)?;
        nodes.append(&mut statements);
    }
    Ok(Module::new(nodes))
}

// `import` and `export` statements are allowed at the module-level but not at
// the block-level.
#[derive(Copy, Clone, Debug)]
enum StatementContext {
    Module,
    Block,
}

fn parse_statements(
    input: &mut TokenStream,
    terminator: Token,
    ctx: StatementContext,
) -> Result<Vec<Node>, ParseError> {
    let mut nodes = vec![];
    loop {
        consume_terminals(input);
        if input.peek() == terminator {
            break;
        }

        let node = parse_statement(input, terminator.clone(), ctx)?;
        nodes.push(node);

        consume_terminals(input);
        if input.peek() == terminator {
            break;
        }
    }
    Ok(nodes)
}

// `terminator` is a pseudo-terminal (eg. a closing brace) that can act like a
// terminal. However, when it is encountered it will be peeked, not read, from
// the input.
fn parse_statement(
    input: &mut TokenStream,
    terminator: Token,
    ctx: StatementContext,
) -> Result<Node, ParseError> {
    let next = input.peek();
    let maybe_named_function = match next {
        Token::Identifier(_, _) => try_parse_named_function(input),
        _ => None,
    };
    // If we found a named function then that's our node, otherwise try the
    // other statement nodes or just a plain expression.
    let node = if let Some(named_function) = maybe_named_function {
        named_function
    } else {
        match next {
            Token::Export => match ctx {
                StatementContext::Module => expect_export(input)?,
                other @ _ => unreachable!("Cannot parse export in {:?}", other),
            },
            Token::Import(_) => match ctx {
                StatementContext::Module => expect_import(input)?,
                other @ _ => unreachable!("Cannot parse import in {:?}", other),
            },
            Token::Let(_) => parse_let_and_var(input)?,
            Token::Return => parse_return(input, terminator.clone())?,
            Token::Var(_) => parse_let_and_var(input)?,
            _ => parse_expression(input)?,
        }
    };
    // Treat the statement as terminated if we encounter the terminator (but
    // don't try to consume it).
    if input.peek() == terminator {
        return Ok(node);
    }
    let next = input.peek();
    if let Token::Terminal(_) = next {
        return Ok(node);
    }
    Err(ParseError::new_unexpected(
        vec!["Terminal".to_string()],
        next,
    ))
}

fn try_parse_named_function(input: &mut TokenStream) -> Option<Node> {
    let savepoint = input.clone();
    let name = match input.read() {
        Token::Identifier(name, _) => name.clone(),
        other @ _ => {
            panic_unexpected_names(other, "Identifier");
            unreachable!()
        }
    };
    if let Token::ParenthesesLeft = input.peek() {
        if let Some((_, body)) = try_parse_function(input) {
            return Some(Node::Function(Function::new_named(name, body)));
        }
    };
    // If we fall through that means we didn't find a named function and
    // need to backtrack.
    input.backtrack(&savepoint);
    None
}

fn expect_export(input: &mut TokenStream) -> Result<Node, ParseError> {
    expect_to_read(input, Token::Export);
    expect_to_read(input, Token::BraceLeft);
    let mut identifiers: Vec<Identifier> = vec![];
    loop {
        if input.peek() == Token::BraceRight {
            input.read();
            break;
        }
        let identifier = expect_identifier(input)?;
        identifiers.push(identifier);
        match input.read() {
            Token::BraceRight => break,
            Token::Comma => continue,
            other @ _ => {
                panic_unexpected_names(other, "BraceRight or Comma");
                unreachable!()
            }
        }
    }
    Ok(Node::Export(Export::new(identifiers)))
}

fn expect_import(input: &mut TokenStream) -> Result<Node, ParseError> {
    let start = expect_to_read2!(input, Token::Import(location) => location);
    let bindings = match input.peek() {
        Token::Star => {
            input.read();
            ImportBindings::AllExports
        }
        Token::String(_, _) => ImportBindings::Module,
        other @ _ => unreachable!("Cannot yet parse in import: {:?}", other),
    };
    let (name, end) = expect_to_read2!(input, Token::String(value, span) => {
        (value, span.end)
    });
    Ok(Node::Import(Import::new(
        name,
        bindings,
        Span::new(start, end),
    )))
}

fn parse_let_and_var(input: &mut TokenStream) -> Result<Node, ParseError> {
    // Consume the `let` or `var`.
    let keyword = input.read();

    let lhs = expect_identifier(input)?;

    let mut rhs = None;
    if input.peek() == Token::Equal {
        expect_to_read(input, Token::Equal);
        rhs = Some(parse_expression(input)?);
    }

    match keyword {
        Token::Let(location) => {
            let mut let_ = Let::new(lhs, rhs);
            let_.location = Some(location);
            Ok(Node::Let(let_))
        }
        Token::Var(location) => {
            let mut var = Var::new(lhs, rhs);
            var.location = Some(location);
            Ok(Node::Var(var))
        }
        _ => {
            panic_unexpected_names(keyword, "Let or Var");
            unreachable!()
        }
    }
}

fn expect_if(input: &mut TokenStream) -> Result<Node, ParseError> {
    expect_to_read2!(input, Token::If(_) => ());
    let condition = parse_statement(input, Token::BraceLeft, StatementContext::Block)?;
    let block = expect_block(input)?;
    Ok(Node::If(If::new(condition, block)))
}

fn expect_while(input: &mut TokenStream) -> Result<Node, ParseError> {
    expect_to_read2!(input, Token::While(_) => ());
    let condition = parse_statement(input, Token::BraceLeft, StatementContext::Block)?;
    let block = expect_block(input)?;
    Ok(Node::While(While::new(condition, block)))
}

fn parse_return(input: &mut TokenStream, terminator: Token) -> Result<Node, ParseError> {
    expect_to_read(input, Token::Return);
    let mut rhs = None;
    let next = input.peek();
    if let Token::Terminal(_) = next {
        // Do nothing.
    } else if next == terminator {
        // Also do nothing.
    } else {
        // We got an expression!
        rhs = Some(parse_expression(input)?);
    }
    Ok(Node::Return(Return::new(rhs)))
}

fn parse_expression(input: &mut TokenStream) -> Result<Node, ParseError> {
    match input.peek() {
        Token::If(_) => expect_if(input),
        Token::While(_) => expect_while(input),
        _ => parse_infix(input),
    }
}

fn parse_infix(input: &mut TokenStream) -> Result<Node, ParseError> {
    #[derive(Debug)]
    enum Subnode {
        Node(Node),
        Op(Token),
    }

    impl From<Subnode> for Node {
        fn from(subnode: Subnode) -> Node {
            match subnode {
                Subnode::Node(node) => node,
                _ => unreachable!(),
            }
        }
    }

    fn reduce_subnodes(subnodes: &mut Vec<Subnode>, reduce_token: Token) {
        let mut index = 1;
        while index < subnodes.len() {
            let subnode = &subnodes[index];
            if let Subnode::Op(token) = subnode {
                if *token == reduce_token {
                    // Take out the operation and the nodes on either side.
                    let mut removed_nodes: Vec<Subnode> =
                        subnodes.drain((index - 1)..=(index + 1)).collect();
                    assert_eq!(removed_nodes.len(), 3);

                    // Then turn that 3-vector into stuff we can work with.
                    let rhs = removed_nodes.pop().unwrap();
                    let op = match removed_nodes.pop().unwrap() {
                        Subnode::Op(token) => token,
                        unexpected @ _ => {
                            panic_unexpected_names(unexpected, "Op");
                            unreachable!()
                        }
                    };
                    let lhs = removed_nodes.pop().unwrap();

                    let node = Node::Infix(Infix::new(lhs.into(), op, rhs.into()));
                    subnodes.insert(index - 1, Subnode::Node(node));
                    // Since we replaced things in-place we should repeat
                    // from where we are rather than advancing.
                    continue;
                }
            }
            index += 1;
        }
    }

    let mut subnodes = vec![Subnode::Node(parse_block(input)?)];
    while infix(input.peek()) {
        subnodes.push(Subnode::Op(input.read()));
        subnodes.push(Subnode::Node(parse_block(input)?));
    }
    // Implement associativity by reducing around operators. The earlier
    // reductions have higher associativity than later ones.
    reduce_subnodes(&mut subnodes, Token::Star);
    reduce_subnodes(&mut subnodes, Token::Plus);
    reduce_subnodes(&mut subnodes, Token::Minus);
    reduce_subnodes(&mut subnodes, Token::AngleLeft);
    reduce_subnodes(&mut subnodes, Token::DoubleEqual);
    // It better have fully reduced!
    assert_eq!(subnodes.len(), 1);
    Ok(subnodes.remove(0).into())
}

fn infix(token: Token) -> bool {
    match token {
        Token::AngleLeft | Token::DoubleEqual | Token::Minus | Token::Plus | Token::Star => true,
        _ => false,
    }
}

fn parse_block(input: &mut TokenStream) -> Result<Node, ParseError> {
    if let Token::BraceLeft = input.peek() {
        Ok(Node::Block(expect_block(input)?))
    } else {
        parse_anonymous_function(input)
    }
}

fn expect_block(input: &mut TokenStream) -> Result<Block, ParseError> {
    expect_to_read(input, Token::BraceLeft); // Opening brace
    let nodes = parse_statements(input, Token::BraceRight, StatementContext::Block)?;
    expect_to_read(input, Token::BraceRight); // Closing brace
    Ok(Block { nodes })
}

fn parse_anonymous_function(input: &mut TokenStream) -> Result<Node, ParseError> {
    if let Some((_, body)) = try_parse_function(input) {
        return Ok(Node::Function(Function::new_anonymous(body)));
    }
    parse_assignment(input)
}

/// Tries to parse the main components of a function:
///   - The parameters (eg. `(a, b)`)
///   - The arrow (`->`)
///   - The body (an expression)
///
/// If successful it returns a tuple of the parameters and body, if unsuccessful
/// it backtracks the input and returns `None`.
fn try_parse_function(input: &mut TokenStream) -> Option<((), Box<Node>)> {
    let savepoint = input.clone();
    // Making a closure so that we can easily `return None` anywhere to
    // interrupt parsing and backtrack.
    let mut inner = || {
        input.read_if(Token::ParenthesesLeft)?;
        // TODO: Parse parameters list.
        input.read_if(Token::ParenthesesRight)?;
        // TODO: Parse return type if present.
        input.read_if(Token::Arrow)?;
        // FIXME: At this point we know it's a function so failure to parse
        //   the expression should be returned as an error.
        let body = parse_expression(input).ok()?;
        Some(((), Box::new(body)))
    };
    match inner() {
        None => {
            input.backtrack(&savepoint);
            None
        }
        Some(pair) => Some(pair),
    }
}

fn parse_assignment(input: &mut TokenStream) -> Result<Node, ParseError> {
    let lhs = parse_postfix(input)?;
    if input.peek() == Token::Equal {
        input.read(); // Equals sign
        let rhs = parse_expression(input)?;
        // FIXME: Check that assignment left-hand-side doesn't end with a call.
        Ok(Node::Assignment(Assignment::new(lhs, rhs)))
    } else {
        Ok(lhs)
    }
}

fn parse_postfix(input: &mut TokenStream) -> Result<Node, ParseError> {
    let mut target = parse_parentheses(input)?;
    loop {
        if let Some(new_target) = try_parse_postfix_property(input, &target) {
            target = new_target;
            continue;
        }
        if input.peek() == Token::ParenthesesLeft {
            target = parse_postfix_call(input, target)?;
            continue;
        }
        break;
    }
    Ok(target)
}

fn try_parse_postfix_property(input: &mut TokenStream, target: &Node) -> Option<Node> {
    let mut needs_backtrack = false;
    let savepoint = input.clone();

    // Lookahead to for subsequent-line properties, eg:
    //   myCoolVariable
    //     .myCoolProperty
    if input.peek().newline() {
        input.read(); // Newline
        needs_backtrack = true;
    }

    if let Token::Dot(dot_start) = input.peek() {
        input.read(); // Dot
        needs_backtrack = true;

        if let Token::Identifier(value, identifier_span) = input.peek() {
            // Identifier
            input.read();
            // Build a span holding both the dot and the identifier.
            let span = Span::new(dot_start, identifier_span.end);
            return Some(Node::PostfixProperty(PostfixProperty::new(
                target.to_owned(),
                value,
                span,
            )));
        }
    }

    if needs_backtrack {
        input.backtrack(&savepoint);
    }
    None
}

fn parse_postfix_call(input: &mut TokenStream, target: Node) -> Result<Node, ParseError> {
    expect_to_read(input, Token::ParenthesesLeft);
    let mut arguments = vec![];
    if input.peek() != Token::ParenthesesRight {
        loop {
            let argument = parse_expression(input)?;
            arguments.push(argument);
            let next = input.peek();
            if next == Token::Comma {
                expect_to_read(input, Token::Comma);
                // Allow a trailing comma before the closing parentheses.
                if input.peek() == Token::ParenthesesRight {
                    break;
                }
                // Otherwise consume the next argument.
                continue;
            } else if next == Token::ParenthesesRight {
                break;
            } else {
                panic_unexpected(next, Some(vec![Token::Comma, Token::ParenthesesRight]));
            }
        }
    }
    expect_to_read(input, Token::ParenthesesRight);
    Ok(Node::PostfixCall(PostfixCall::new(target, arguments)))
}

fn parse_parentheses(input: &mut TokenStream) -> Result<Node, ParseError> {
    if let Token::ParenthesesLeft = input.peek() {
        input.read(); // Opening parentheses
        let node = parse_expression(input);
        expect_to_read(input, Token::ParenthesesRight); // Closing parentheses
        node
    } else {
        parse_atom(input)
    }
}

fn parse_atom(input: &mut TokenStream) -> Result<Node, ParseError> {
    let next = input.peek();
    match next {
        Token::Identifier(_, _) => {
            let identifier = expect_identifier(input)?;
            Ok(Node::Identifier(identifier))
        }
        Token::Integer(value) => {
            input.read();
            Ok(Node::Integer(Integer { value }))
        }
        Token::String(value, _) => {
            input.read();
            Ok(Node::String(StringLiteral { value }))
        }
        Token::Symbol(value, _) => {
            input.read();
            Ok(Node::Symbol(SymbolLiteral::new(value)))
        }
        _ => Err(ParseError::new_unexpected(vec![], next)),
    }
}

fn expect_identifier(input: &mut TokenStream) -> Result<Identifier, ParseError> {
    Ok(expect_to_read2!(input, Token::Identifier(value, span) => {
        Identifier::new(value, span)
    }))
}

fn consume_terminals(input: &mut TokenStream) {
    while let Token::Terminal(_) = input.peek() {
        input.read();
    }
}

fn expect_to_read(input: &mut TokenStream, token: Token) -> Token {
    let next = input.read();
    if next != token {
        panic_unexpected(next.clone(), Some(vec![token]));
    }
    next
}

fn panic_unexpected(token: Token, expected_tokens: Option<Vec<Token>>) {
    let expected = match expected_tokens {
        Some(tokens) => format!(" (expected: {:?})", tokens),
        None => "".to_string(),
    };
    panic!("Unexpected token: {:?}{}", token, expected)
}

fn panic_unexpected_names<T: Debug>(token: T, expected_names: &str) {
    panic!(
        "Unexpected token: {:?} (expected {})",
        token, expected_names
    )
}

#[cfg(test)]
mod tests {
    use super::super::super::ast::nodes::{
        Assignment, Block, Export, Function, Identifier, If, Import, ImportBindings, Infix,
        Integer, Let, Module, Node, PostfixCall, PostfixProperty, Return, StringLiteral,
        SymbolLiteral, While,
    };

    use super::super::lexer::{Token, TokenStream};
    use super::super::{Location, Span};

    use super::{base_name, parse_block, parse_infix, parse_module, parse_postfix};

    fn input(input: &str) -> TokenStream {
        TokenStream::from_string(input.to_string())
    }

    fn parse_complete(program: &str) -> Vec<Node> {
        let mut token_stream = input(program);
        let module = parse_module(&mut token_stream);
        module.unwrap().nodes
    }

    #[test]
    fn it_parses_program() {
        // Test handling of multiple newlines (terminals).
        assert_eq!(
            parse_module(&mut input(
                "
                1

                2

            "
            )),
            Ok(Module::new(vec![
                Node::Integer(Integer { value: 1 }),
                Node::Integer(Integer { value: 2 }),
            ])),
        );
    }

    #[test]
    fn it_parses_let() {
        let mut nodes = parse_complete("let a = 1");
        assert_eq!(
            nodes,
            vec![Node::Let(Let::new(
                Identifier::new("a", Span::unknown()),
                Some(Node::Integer(Integer { value: 1 })),
            ))],
        );
        let node = nodes.remove(0);
        match node {
            Node::Let(let_) => {
                assert_eq!(let_.location, Some(Location::new(0, 1, 1)));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_parses_export() {
        let nodes = parse_complete("export { }");
        assert_eq!(nodes, vec![Node::Export(Export::new(vec![]))],);
        let nodes = parse_complete("export { foo }");
        assert_eq!(
            nodes,
            vec![Node::Export(Export::new(vec![Identifier::new(
                "foo",
                Span::unknown()
            )]))],
        );
        let nodes = parse_complete("export { foo, }");
        assert_eq!(
            nodes,
            vec![Node::Export(Export::new(vec![Identifier::new(
                "foo",
                Span::unknown()
            )]))],
        );
        let nodes = parse_complete("export { foo, bar }");
        assert_eq!(
            nodes,
            vec![Node::Export(Export::new(vec![
                Identifier::new("foo", Span::unknown()),
                Identifier::new("bar", Span::unknown())
            ]))],
        );
        let nodes = parse_complete("export { foo, bar, }");
        assert_eq!(
            nodes,
            vec![Node::Export(Export::new(vec![
                Identifier::new("foo", Span::unknown()),
                Identifier::new("bar", Span::unknown())
            ]))],
        );
    }

    #[test]
    fn it_parses_import() {
        let nodes = parse_complete("import * \"foo\"");
        assert_eq!(
            nodes,
            vec![Node::Import(Import::new(
                "foo".to_string(),
                ImportBindings::AllExports,
                Span::unknown(),
            ))],
        );
    }

    #[test]
    fn it_parses_return() {
        assert_eq!(
            parse_complete("return"),
            vec![Node::Return(Return::new(None))],
        );
        assert_eq!(
            parse_complete("return\n"),
            vec![Node::Return(Return::new(None))],
        );
        assert_eq!(
            parse_complete("return;"),
            vec![Node::Return(Return::new(None))],
        );
        assert_eq!(
            parse_complete("return 1"),
            vec![Node::Return(Return::new(Some(Node::Integer(Integer {
                value: 1
            }))))],
        );
        assert_eq!(
            parse_complete("{ return 1 }"),
            vec![Node::Block(Block {
                nodes: vec![Node::Return(Return::new(Some(Node::Integer(Integer {
                    value: 1
                }))))],
            })],
        );
        assert_eq!(
            parse_complete("{ return 1; }"),
            vec![Node::Block(Block {
                nodes: vec![Node::Return(Return::new(Some(Node::Integer(Integer {
                    value: 1
                }))))],
            })],
        );
    }

    #[test]
    fn it_parses_anonymous_function() {
        assert_eq!(
            parse_complete("() -> { 123 }"),
            vec![Node::Function(Function::new_anonymous(Box::new(
                Node::Block(Block {
                    nodes: vec![Node::Integer(Integer { value: 123 })],
                }),
            )))],
        );
        assert_eq!(
            parse_complete("foo = () -> 123"),
            vec![Node::Assignment(Assignment::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                Node::Function(Function::new_anonymous(Box::new(Node::Integer(Integer {
                    value: 123
                })))),
            ))],
        );
        assert_eq!(
            parse_complete("foo = (() -> 123)()"),
            vec![Node::Assignment(Assignment::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                Node::PostfixCall(PostfixCall::new(
                    Node::Function(Function::new_anonymous(Box::new(Node::Integer(Integer {
                        value: 123
                    })))),
                    vec![],
                )),
            ))],
        );
        assert_eq!(
            parse_complete("foo(() -> 123)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![Node::Function(Function::new_anonymous(Box::new(
                    Node::Integer(Integer { value: 123 })
                )))],
            ))],
        );
    }

    #[test]
    fn it_parses_nested_functions() {
        // Nested anonymous.
        assert_eq!(
            parse_complete("() -> () -> 123"),
            vec![Node::Function(Function::new_anonymous(Box::new(
                Node::Function(Function::new_anonymous(Box::new(Node::Integer(Integer {
                    value: 123
                }))))
            )))],
        );
        // Nested anonymous as right-hand-side of assignment.
        assert_eq!(
            parse_complete("let a = () -> () -> 123"),
            vec![Node::Let(Let::new(
                Identifier::new("a", Span::unknown()),
                Some(Node::Function(Function::new_anonymous(Box::new(
                    Node::Function(Function::new_anonymous(Box::new(Node::Integer(Integer {
                        value: 123
                    })))),
                )))),
            ))],
        );
        // Nested anonymous within named.
        assert_eq!(
            parse_complete("foo() -> () -> 123"),
            vec![Node::Function(Function::new_named(
                "foo".to_string(),
                Box::new(Node::Function(Function::new_anonymous(Box::new(
                    Node::Integer(Integer { value: 123 })
                )))),
            ))],
        );
        // Nested named within blocks.
        assert_eq!(
            parse_complete("foo() -> { bar() -> 123 \n bar }"),
            vec![Node::Function(Function::new_named(
                "foo".to_string(),
                Box::new(Node::Block(Block {
                    nodes: vec![
                        Node::Function(Function::new_named(
                            "bar".to_string(),
                            Box::new(Node::Integer(Integer { value: 123 })),
                        )),
                        Node::Identifier(Identifier::new("bar", Span::unknown())),
                    ],
                })),
            ))],
        );
    }

    // Named functions are statements so they cannot be nested as expressions.
    #[test]
    #[should_panic]
    fn it_doesnt_parse_nested_named_functions() {
        parse_complete("a() -> b() -> 123");
    }

    #[test]
    fn it_parses_named_function() {
        assert_eq!(
            parse_complete("foo() -> { 123 }"),
            vec![Node::Function(Function::new_named(
                "foo".to_string(),
                Box::new(Node::Block(Block {
                    nodes: vec![Node::Integer(Integer { value: 123 })],
                })),
            ))],
        );
        // Check that `;` terminates the expression.
        assert_eq!(
            parse_complete("foo() -> 456; \"not in function\""),
            vec![
                Node::Function(Function::new_named(
                    "foo".to_string(),
                    Box::new(Node::Integer(Integer { value: 456 }),)
                )),
                Node::String(StringLiteral::new("not in function".to_owned())),
            ],
        );
    }

    #[test]
    fn it_parses_atom() {
        let parsed = parse_complete("/* */\n  foo");
        assert_eq!(
            parsed,
            vec![Node::Identifier(Identifier::new("foo", Span::unknown(),))],
        );
        match &parsed[0] {
            Node::Identifier(identifier) => assert_eq!(
                identifier.span,
                Span::new(Location::new(8, 2, 3), Location::new(11, 2, 6))
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_parses_block() {
        assert_eq!(
            parse_block(&mut input("{}")),
            Ok(Node::Block(Block { nodes: vec![] })),
        );
        assert_eq!(
            parse_block(&mut input("{ 1 }")),
            Ok(Node::Block(Block {
                nodes: vec![Node::Integer(Integer { value: 1 })],
            })),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; }")),
            Ok(Node::Block(Block {
                nodes: vec![Node::Integer(Integer { value: 1 })],
            })),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; 2 }")),
            Ok(Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            })),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; 2; }")),
            Ok(Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            })),
        );
        assert_eq!(
            parse_block(&mut input("{\n  1\n  2;\n}")),
            Ok(Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            })),
        );
    }

    #[test]
    fn it_parses_if() {
        assert_eq!(
            parse_complete("if 1 { 2 }"),
            vec![Node::If(If::new(
                Node::Integer(Integer { value: 1 }),
                Block {
                    nodes: vec![Node::Integer(Integer { value: 2 })],
                },
            ))],
        );
        assert_eq!(
            parse_complete("let a = if 1 { 2 }"),
            vec![Node::Let(Let::new(
                Identifier::new("a", Span::unknown()),
                Some(Node::If(If::new(
                    Node::Integer(Integer { value: 1 }),
                    Block {
                        nodes: vec![Node::Integer(Integer { value: 2 })],
                    },
                ))),
            ))],
        );
    }

    #[test]
    fn it_parses_infix() {
        assert_eq!(
            parse_infix(&mut input("1 + 2")),
            Ok(Node::Infix(Infix::new(
                Node::Integer(Integer { value: 1 }),
                Token::Plus,
                Node::Integer(Integer { value: 2 }),
            ))),
        );
        assert_eq!(
            parse_infix(&mut input("1 * 2 * 3")),
            Ok(Node::Infix(Infix::new(
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 1 }),
                    Token::Star,
                    Node::Integer(Integer { value: 2 }),
                )),
                Token::Star,
                Node::Integer(Integer { value: 3 }),
            ))),
        );
        assert_eq!(
            parse_infix(&mut input("1 + 3 == 2 * 2")),
            Ok(Node::Infix(Infix::new(
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 1 }),
                    Token::Plus,
                    Node::Integer(Integer { value: 3 }),
                )),
                Token::DoubleEqual,
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 2 }),
                    Token::Star,
                    Node::Integer(Integer { value: 2 }),
                )),
            ))),
        );
        // Now with associativity!
        assert_eq!(
            parse_infix(&mut input("1 * 2 + 3 * 4")),
            Ok(Node::Infix(Infix::new(
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 1 }),
                    Token::Star,
                    Node::Integer(Integer { value: 2 }),
                )),
                Token::Plus,
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 3 }),
                    Token::Star,
                    Node::Integer(Integer { value: 4 }),
                )),
            ))),
        );
        // Now with parentheses!
        assert_eq!(
            parse_infix(&mut input("1 * (2 + 3) * 4")),
            Ok(Node::Infix(Infix::new(
                Node::Infix(Infix::new(
                    Node::Integer(Integer { value: 1 }),
                    Token::Star,
                    Node::Infix(Infix::new(
                        Node::Integer(Integer { value: 2 }),
                        Token::Plus,
                        Node::Integer(Integer { value: 3 }),
                    )),
                )),
                Token::Star,
                Node::Integer(Integer { value: 4 }),
            ))),
        );
    }

    #[test]
    fn it_parses_postfix_property() {
        assert_eq!(
            parse_postfix(&mut input("foo.bar")),
            Ok(Node::PostfixProperty(PostfixProperty::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                "bar".to_string(),
                Span::unknown(),
            ))),
        );
    }

    #[test]
    fn it_parses_postfix_call() {
        assert_eq!(
            parse_complete("foo()"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![],
            ))],
        );
        assert_eq!(
            parse_complete("foo(1)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![Node::Integer(Integer { value: 1 }),],
            ))],
        );
        assert_eq!(
            parse_complete("foo(1,)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![Node::Integer(Integer { value: 1 }),],
            ))],
        );
        assert_eq!(
            parse_complete("foo(1, 2)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            ))],
        );
        assert_eq!(
            parse_complete("foo(1, 2,)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            ))],
        );
        assert_eq!(
            parse_complete("foo(bar())"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo", Span::unknown())),
                vec![Node::PostfixCall(PostfixCall::new(
                    Node::Identifier(Identifier::new("bar", Span::unknown())),
                    vec![],
                )),],
            )),]
        );
    }

    #[test]
    fn it_parses_chained_postfix_call() {
        assert_eq!(
            parse_complete("foo(1)(2)"),
            vec![Node::PostfixCall(PostfixCall::new(
                Node::PostfixCall(PostfixCall::new(
                    Node::Identifier(Identifier::new("foo", Span::unknown())),
                    vec![Node::Integer(Integer { value: 1 })],
                )),
                vec![Node::Integer(Integer { value: 2 })],
            ))],
        );
    }

    #[test]
    fn it_parses_symbol() {
        assert_eq!(
            parse_complete(":foo"),
            vec![Node::Symbol(SymbolLiteral::new("foo".to_string()))],
        );
    }

    #[test]
    fn it_parses_while() {
        assert_eq!(
            parse_complete("while 1 { 2 }"),
            vec![Node::While(While::new(
                Node::Integer(Integer { value: 1 }),
                Block {
                    nodes: vec![Node::Integer(Integer { value: 2 })],
                },
            ))],
        );
    }

    #[test]
    fn it_base_names_formatted_strings() {
        assert_eq!(base_name("Value::String(string)"), "String");
        assert_eq!(base_name("Integer(1)"), "Integer");
        assert_eq!(base_name("Export"), "Export");
    }
}
