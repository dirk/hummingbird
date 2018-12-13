struct StringStream {
    input: Vec<char>,
    index: usize,
    line: i32,
    column: i32,
}

impl StringStream {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn read(&mut self) -> char {
        let character = self
            .input
            .get(self.index)
            .map(|value| *value)
            .unwrap_or('\0');
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        character
    }

    fn peek(&self) -> char {
        self.input
            .get(self.index)
            .map(|value| *value)
            .unwrap_or('\0')
    }

    fn read_until(&mut self, target: char) {
        while self.peek() != target {
            let character = self.read();
            assert!(character != '\0');
        }
    }

    fn read_if_match(&mut self, target: &[char]) -> bool {
        // Bounds-check first!
        if (self.index + target.len()) > self.input.len() {
            return false;
        }
        let substr = &self.input[self.index..(self.index + target.len())];
        if substr != target {
            return false;
        }
        // Consume if they match.
        for _ in target {
            self.read();
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    BraceLeft,
    BraceRight,
    EOF,
    Identifier(String),
    Integer(i64),
    Let,
    Func,
    Minus,
    ParenthesesLeft,
    ParenthesesRight,
    Plus,
    Return,
    Star,
    Terminal(char),
    Var,
}

struct TokenStream {
    input: StringStream,
    peeking: bool,
    next_token: Option<Token>,
}

impl TokenStream {
    pub fn new(input: StringStream) -> Self {
        TokenStream {
            input: input,
            peeking: false,
            next_token: None,
        }
    }

    pub fn peek(&mut self) -> Token {
        if !self.peeking {
            self.peeking = true;
            self.next_token = Some(self.lex());
        }
        self.next_token.clone().unwrap()
    }

    pub fn read(&mut self) -> Token {
        let token = self.peek();
        self.peeking = false;
        return token;
    }

    fn lex(&mut self) -> Token {
        self.consume_space_and_comments();

        loop {
            let character = self.input.peek();

            if identifier_head(character) {
                return self.lex_identifier();
            } else if numeric_head(character) {
                return self.lex_numeric_or_minus();
            } else {
                self.input.read();
                match character {
                    '\0' => return Token::EOF,
                    '{' => return Token::BraceLeft,
                    '}' => return Token::BraceRight,
                    '(' => return Token::ParenthesesLeft,
                    ')' => return Token::ParenthesesRight,
                    '+' => return Token::Plus,
                    '*' => return Token::Star,
                    ';' => return Token::Terminal(character),
                    '\n' => {
                        self.consume_more_newline_terminals();
                        return Token::Terminal(character);
                    }
                    _ => panic!("Unexpected character: {}", character),
                }
            }
        }
    }

    fn lex_identifier(&mut self) -> Token {
        let mut identifier = vec![self.input.read()];
        loop {
            let character = self.input.peek();
            if identifier_tail(character) {
                self.input.read();
                identifier.push(character);
            } else {
                break;
            }
        }
        let identifier_string: String = identifier.into_iter().collect();
        match identifier_string.as_str() {
            "let" => Token::Let,
            "func" => Token::Func,
            "var" => Token::Var,
            "return" => Token::Return,
            _ => Token::Identifier(identifier_string),
        }
    }

    fn lex_numeric_or_minus(&mut self) -> Token {
        let first_character = self.input.read();
        if first_character == '-' && !digit(self.input.peek()) {
            return Token::Minus;
        }
        let mut number = vec![first_character];
        loop {
            let character = self.input.peek();
            if digit(character) {
                self.input.read();
                number.push(character);
            } else {
                break;
            }
        }
        let number_string: String = number.into_iter().collect();
        return Token::Integer(number_string.parse().unwrap());
    }

    fn consume_more_newline_terminals(&mut self) {
        loop {
            self.consume_space_and_comments();
            if self.input.peek() == '\n' {
                self.input.read();
                continue;
            } else {
                break;
            }
        }
    }

    fn consume_space_and_comments(&mut self) {
        loop {
            let character = self.input.peek();
            if character == ' ' || character == '\t' {
                self.input.read();
            } else if self.input.read_if_match(&['/', '/']) {
                self.input.read_until('\n');
            } else if self.input.read_if_match(&['/', '*']) {
                loop {
                    if self.input.read_if_match(&['*', '/']) {
                        break;
                    } else {
                        self.input.read();
                    }
                }
            } else {
                // Not any kind of comment.
                break;
            }
        }
    }
}

fn identifier_head(character: char) -> bool {
    alphabetical(character)
}

fn identifier_tail(character: char) -> bool {
    alphabetical(character)
}

fn numeric_head(character: char) -> bool {
    digit(character) || (character == '-')
}

fn alphabetical(character: char) -> bool {
    (character >= 'a' && character <= 'z') || (character >= 'A' && character <= 'Z')
}

fn digit(character: char) -> bool {
    (character >= '0' && character <= '9')
}

#[cfg(test)]
mod tests {
    use super::{StringStream, Token, TokenStream};

    fn parse(input: &str) -> Vec<Token> {
        let string_stream = StringStream::new(input);
        let mut token_stream = TokenStream::new(string_stream);
        let mut tokens = vec![];
        loop {
            let token = token_stream.read();
            tokens.push(token.clone());
            if token == Token::EOF {
                break;
            }
        }
        return tokens;
    }

    #[test]
    fn it_parses_identifier() {
        assert_eq!(
            parse("foo"),
            vec![Token::Identifier("foo".to_string()), Token::EOF,]
        );
    }

    #[test]
    fn it_parses_keywords() {
        assert_eq!(parse("func"), vec![Token::Func, Token::EOF,]);
        assert_eq!(parse("let"), vec![Token::Let, Token::EOF,]);
        assert_eq!(parse("return"), vec![Token::Return, Token::EOF,]);
        assert_eq!(parse("var"), vec![Token::Var, Token::EOF,]);
    }

    #[test]
    fn it_parse_comments_and_terminals() {
        assert_eq!(
            parse("foo /* Comment */ bar"),
            vec![
                Token::Identifier("foo".to_string()),
                Token::Identifier("bar".to_string()),
                Token::EOF,
            ]
        );

        assert_eq!(
            parse("foo // Comment \n bar"),
            vec![
                Token::Identifier("foo".to_string()),
                Token::Terminal('\n'),
                Token::Identifier("bar".to_string()),
                Token::EOF,
            ]
        );

        assert_eq!(
            parse(
                "foo
            // Comment about the call
            // Another comment about the call
            bar()"
            ),
            vec![
                Token::Identifier("foo".to_string()),
                Token::Terminal('\n'),
                Token::Identifier("bar".to_string()),
                Token::ParenthesesLeft,
                Token::ParenthesesRight,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn it_parses_integers() {
        assert_eq!(parse("1"), vec![Token::Integer(1), Token::EOF]);
        assert_eq!(parse("-1"), vec![Token::Integer(-1), Token::EOF]);
        assert_eq!(
            parse("- 1"),
            vec![Token::Minus, Token::Integer(1), Token::EOF]
        );
        assert_eq!(
            parse("1+2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::EOF,
            ]
        );
        assert_eq!(
            parse("1 + 2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn it_parses_blocks() {
        assert_eq!(
            parse("{ 1; }"),
            vec![
                Token::BraceLeft,
                Token::Integer(1),
                Token::Terminal(';'),
                Token::BraceRight,
                Token::EOF,
            ]
        );
    }
}