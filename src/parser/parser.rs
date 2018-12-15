use super::super::ast::nodes::{
    Assignment, Block, Function, Identifier, Infix, Integer, Let, Node, PostfixCall,
    PostfixProperty, Program, Return, Var,
};

use super::lexer::{Token, TokenStream};

fn parse_program(input: &mut TokenStream) -> Node {
    let mut nodes: Vec<Node> = Vec::new();
    while input.peek() != Token::EOF {
        nodes.append(&mut parse_statements(input, Token::EOF))
    }
    Node::Program(Program { nodes })
}

fn parse_statements(input: &mut TokenStream, terminator: Token) -> Vec<Node> {
    let mut nodes = vec![];
    loop {
        consume_terminals(input);
        if input.peek() == terminator {
            break;
        }

        nodes.push(parse_statement(input, terminator.clone()));

        consume_terminals(input);
        if input.peek() == terminator {
            break;
        }
    }
    nodes
}

// `terminator` is a pseudo-terminal (eg. a closing brace) that can act like a
// terminal. However, when it is encountered it will be peeked, not read, from
// the input.
fn parse_statement(input: &mut TokenStream, terminator: Token) -> Node {
    let next = input.peek();
    let node = match next {
        Token::Func(_) => parse_function(input),
        Token::Let(_) => parse_let_and_var(input),
        Token::Return => parse_return(input, terminator.clone()),
        Token::Var(_) => parse_let_and_var(input),
        _ => parse_expression(input),
    };
    // Treat the statement as terminated if we encounter the terminator (but
    // don't try to consume it).
    if input.peek() == terminator {
        return node;
    }
    let next = input.peek();
    if let Token::Terminal(_) = next {
        return node;
    }
    panic_unexpected(
        next,
        Some(vec![Token::Terminal('\n'), Token::Terminal(';')]),
    );
    unreachable!()
}

fn parse_function(input: &mut TokenStream) -> Node {
    // Consume the `func` and get its location.
    let location = input.read().location();

    let token = input.read();
    let name_identifier: Identifier = token.into();

    expect_to_read(input, Token::ParenthesesLeft);
    expect_to_read(input, Token::ParenthesesRight);

    let token = input.peek();
    if token != Token::BraceLeft {
        panic_unexpected(token, Some(vec![Token::BraceLeft]));
    }
    let block_node = parse_block(input);

    let name = name_identifier.value;
    let block = match block_node {
        Node::Block(block) => block,
        _ => unreachable!(),
    };
    let mut function = Function::new(name, block);
    function.location = location;
    Node::Function(function)
}

fn parse_let_and_var(input: &mut TokenStream) -> Node {
    // Consume the `let` or `var`.
    let keyword = input.read();

    let lhs: Identifier = input.read().into();

    let mut rhs = None;
    if input.peek() == Token::Equals {
        expect_to_read(input, Token::Equals);
        rhs = Some(parse_expression(input));
    }

    match keyword {
        Token::Let(location) => {
            let mut let_ = Let::new(lhs, rhs);
            let_.location = Some(location);
            Node::Let(let_)
        }
        Token::Var(location) => {
            let mut var = Var::new(lhs, rhs);
            var.location = Some(location);
            Node::Var(var)
        }
        _ => {
            panic_unexpected_names(keyword, "Let or Var");
            unreachable!()
        }
    }
}

fn parse_return(input: &mut TokenStream, terminator: Token) -> Node {
    expect_to_read(input, Token::Return);
    let mut rhs = None;
    let next = input.peek();
    if let Token::Terminal(_) = next {
        // Do nothing.
    } else if next == terminator {
        // Also do nothing.
    } else {
        // We got an expression!
        rhs = Some(parse_expression(input));
    }
    Node::Return(Return::new(rhs))
}

fn parse_expression(input: &mut TokenStream) -> Node {
    parse_infix(input)
}

fn parse_infix(input: &mut TokenStream) -> Node {
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
                        _ => unreachable!(),
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

    let mut subnodes = vec![Subnode::Node(parse_block(input))];
    while infix(input.peek()) {
        subnodes.push(Subnode::Op(input.read()));
        subnodes.push(Subnode::Node(parse_block(input)));
    }
    if subnodes.len() > 1 {
        // Implement associativity by reducing around operators. The earlier
        // reductions have higher associativity than later ones.
        reduce_subnodes(&mut subnodes, Token::Star);
        reduce_subnodes(&mut subnodes, Token::Plus);
    }
    // It better have fully reduced!
    assert_eq!(subnodes.len(), 1);
    subnodes.remove(0).into()
}

fn infix(token: Token) -> bool {
    match token {
        Token::Minus | Token::Plus | Token::Star => true,
        _ => false,
    }
}

fn parse_block(input: &mut TokenStream) -> Node {
    if let Token::BraceLeft = input.peek() {
        input.read(); // Opening brace
        let nodes = parse_statements(input, Token::BraceRight);
        expect_to_read(input, Token::BraceRight); // Closing brace
        Node::Block(Block { nodes })
    } else {
        parse_parentheses(input)
    }
}

fn parse_parentheses(input: &mut TokenStream) -> Node {
    if let Token::ParenthesesLeft = input.peek() {
        input.read(); // Opening parentheses
        let node = parse_expression(input);
        expect_to_read(input, Token::ParenthesesRight); // Closing parentheses
        node
    } else {
        parse_assignment(input)
    }
}

fn parse_assignment(input: &mut TokenStream) -> Node {
    let lhs = parse_postfix(input);
    if input.peek() == Token::Equals {
        input.read(); // Equals sign
        let rhs = parse_expression(input);
        Node::Assignment(Assignment::new(lhs, rhs))
    } else {
        lhs
    }
}

fn parse_postfix(input: &mut TokenStream) -> Node {
    let mut target = parse_atom(input);
    loop {
        if let Some(new_target) = try_parse_postfix_property(input, &target) {
            target = new_target;
            continue;
        }
        if input.peek() == Token::ParenthesesLeft {
            target = parse_postfix_call(input, target);
        }
        break;
    }
    target
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

    if input.peek() == Token::Dot {
        input.read(); // Dot
        needs_backtrack = true;

        if let Token::Identifier(value, _) = input.peek() {
            input.read(); // Identifier
            return Some(Node::PostfixProperty(PostfixProperty::new(
                target.to_owned(),
                value,
            )));
        }
    }

    if needs_backtrack {
        input.backtrack(&savepoint);
    }
    None
}

fn parse_postfix_call(input: &mut TokenStream, target: Node) -> Node {
    expect_to_read(input, Token::ParenthesesLeft);
    let mut arguments = vec![];
    if input.peek() != Token::ParenthesesRight {
        loop {
            let argument = parse_expression(input);
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
    Node::PostfixCall(PostfixCall::new(target, arguments))
}

fn parse_atom(input: &mut TokenStream) -> Node {
    let next = input.read();
    match next {
        Token::Identifier(_, _) => Node::Identifier(next.into()),
        Token::Integer(value) => Node::Integer(Integer { value }),
        _ => {
            panic_unexpected(next, None);
            unreachable!()
        }
    }
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

fn panic_unexpected_names(token: Token, expected_names: &str) {
    panic!(
        "Unexpected token: {:?} (expected {})",
        token, expected_names
    )
}

impl From<Token> for Identifier {
    fn from(token: Token) -> Identifier {
        match token {
            Token::Identifier(value, location) => {
                let mut identifier = Identifier::new(value);
                identifier.location = Some(location);
                identifier
            }
            _ => {
                panic_unexpected_names(token, "Identifier");
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::ast::nodes::{
        Block, Function, Identifier, Infix, Integer, Let, Node, PostfixCall, PostfixProperty,
        Program, Return,
    };

    use super::super::lexer::{Token, TokenStream};
    use super::super::location::Location;

    use super::{parse_block, parse_infix, parse_postfix, parse_program};

    fn input(input: &str) -> TokenStream {
        TokenStream::from_string(input.to_string())
    }

    fn parse_complete(program: &str) -> Vec<Node> {
        let mut token_stream = input(program);
        let node = parse_program(&mut token_stream);
        match node {
            Node::Program(program) => program.nodes,
            _ => panic!("Not a Program: {:?}", node),
        }
    }

    #[test]
    fn it_parses_program() {
        // Test handling of multiple newlines (terminals).
        assert_eq!(
            parse_program(&mut input(
                "
                1

                2

            "
            )),
            Node::Program(Program {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            }),
        );
    }

    #[test]
    fn it_parses_let() {
        let mut nodes = parse_complete("let a = 1");
        assert_eq!(
            nodes,
            vec![Node::Let(Let::new(
                Identifier::new("a"),
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
            })),))],
        );
        assert_eq!(
            parse_complete("{ return 1 }"),
            vec![Node::Block(Block {
                nodes: vec![Node::Return(Return::new(Some(Node::Integer(Integer {
                    value: 1
                })),)),],
            })],
        );
        assert_eq!(
            parse_complete("{ return 1; }"),
            vec![Node::Block(Block {
                nodes: vec![Node::Return(Return::new(Some(Node::Integer(Integer {
                    value: 1
                })),)),],
            })],
        );
    }

    #[test]
    fn it_parses_func() {
        assert_eq!(
            parse_complete("func foo() { 123 }"),
            vec![Node::Function(Function::new(
                "foo".to_string(),
                Block {
                    nodes: vec![Node::Integer(Integer { value: 123 })],
                },
            ))],
        );
    }

    #[test]
    fn it_parses_atom() {
        let mut nodes = parse_complete("/* */\n  foo");
        assert_eq!(nodes, vec![Node::Identifier(Identifier::new("foo"))]);
        let node = nodes.remove(0);
        match node {
            Node::Identifier(identifier) => {
                assert_eq!(identifier.location, Some(Location::new(8, 2, 3)));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_parses_block() {
        assert_eq!(
            parse_block(&mut input("{}")),
            Node::Block(Block { nodes: vec![] }),
        );
        assert_eq!(
            parse_block(&mut input("{ 1 }")),
            Node::Block(Block {
                nodes: vec![Node::Integer(Integer { value: 1 })],
            }),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; }")),
            Node::Block(Block {
                nodes: vec![Node::Integer(Integer { value: 1 })],
            }),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; 2 }")),
            Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            }),
        );
        assert_eq!(
            parse_block(&mut input("{ 1; 2; }")),
            Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            }),
        );
        assert_eq!(
            parse_block(&mut input("{\n  1\n  2;\n}")),
            Node::Block(Block {
                nodes: vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            }),
        );
    }

    #[test]
    fn it_parses_infix() {
        assert_eq!(
            parse_infix(&mut input("1 + 2")),
            Node::Infix(Infix::new(
                Node::Integer(Integer { value: 1 }),
                Token::Plus,
                Node::Integer(Integer { value: 2 }),
            )),
        );
        // Now with associativity!
        assert_eq!(
            parse_infix(&mut input("1 * 2 + 3 * 4")),
            Node::Infix(Infix::new(
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
            )),
        );
    }

    #[test]
    fn it_parses_postfix_property() {
        assert_eq!(
            parse_postfix(&mut input("foo.bar")),
            Node::PostfixProperty(PostfixProperty::new(
                Node::Identifier(Identifier::new("foo")),
                "bar".to_string(),
            ))
        );
    }

    #[test]
    fn it_parses_postfix_call() {
        assert_eq!(
            parse_postfix(&mut input("foo()")),
            Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo")),
                vec![],
            )),
        );
        assert_eq!(
            parse_postfix(&mut input("foo(1)")),
            Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo")),
                vec![Node::Integer(Integer { value: 1 }),],
            )),
        );
        assert_eq!(
            parse_postfix(&mut input("foo(1,)")),
            Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo")),
                vec![Node::Integer(Integer { value: 1 }),],
            )),
        );
        assert_eq!(
            parse_postfix(&mut input("foo(1, 2)")),
            Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo")),
                vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            )),
        );
        assert_eq!(
            parse_postfix(&mut input("foo(1, 2,)")),
            Node::PostfixCall(PostfixCall::new(
                Node::Identifier(Identifier::new("foo")),
                vec![
                    Node::Integer(Integer { value: 1 }),
                    Node::Integer(Integer { value: 2 }),
                ],
            )),
        );
    }
}
