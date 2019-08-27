use super::location::{Location, Span};

#[derive(Clone)]
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

    fn location(&self) -> Location {
        Location::new(self.index as u32, self.line, self.column)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Arrow,
    BraceLeft,
    BraceRight,
    Comma,
    Dot,
    EOF,
    Equals,
    Export,
    Identifier(String, Span),
    Import,
    Integer(i64),
    LeftAngle,
    Let(Location),
    Minus,
    ParenthesesLeft,
    ParenthesesRight,
    Plus,
    Return,
    Star,
    String(String),
    Terminal(char),
    Var(Location),
    While(Location),
}

impl Token {
    pub fn location(&self) -> Option<Location> {
        match self {
            Token::Let(location) | Token::Var(location) => Some(location.clone()),
            Token::Identifier(_, location) => Some(location.start.clone()),
            _ => None,
        }
    }
}

impl Token {
    pub fn newline(self) -> bool {
        match self {
            Token::Terminal(character) => character == '\n',
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct TokenStream {
    input: StringStream,
    peeking: bool,
    next_token: Option<Token>,
}

impl TokenStream {
    pub fn from_string(input: String) -> Self {
        Self::new(StringStream::new(input.as_str()))
    }

    fn new(input: StringStream) -> Self {
        TokenStream {
            input: input,
            peeking: false,
            next_token: None,
        }
    }

    pub fn backtrack(&mut self, other: &TokenStream) {
        self.input = other.input.clone();
        self.peeking = other.peeking;
        self.next_token = other.next_token.clone();
    }

    pub fn peek(&mut self) -> Token {
        if !self.peeking {
            self.peeking = true;
            self.next_token = Some(self.lex());
        }
        self.next_token.clone().unwrap()
    }

    /// Peeks, and if the peeked token is the same type as the expected then
    /// read that type and return the read (not expected) token.
    pub fn read_if(&mut self, expected: Token) -> Option<Token> {
        use std::mem::discriminant;

        let got = self.peek();
        if discriminant(&expected) == discriminant(&got) {
            Some(self.read())
        } else {
            None
        }
    }

    pub fn read(&mut self) -> Token {
        let token = self.peek();
        self.peeking = false;
        return token;
    }

    fn lex(&mut self) -> Token {
        self.consume_space_and_comments();

        loop {
            let location = self.input.location();
            let character = self.input.peek();

            if identifier_head(character) {
                return self.lex_identifier(location);
            } else if numeric_head(character) {
                return self.lex_arrow_minus_or_numeric();
            } else if string_head(character) {
                return self.lex_string();
            } else {
                self.input.read();
                match character {
                    '\0' => return Token::EOF,
                    '{' => return Token::BraceLeft,
                    '}' => return Token::BraceRight,
                    '(' => return Token::ParenthesesLeft,
                    ')' => return Token::ParenthesesRight,
                    ',' => return Token::Comma,
                    '.' => return Token::Dot,
                    '=' => return Token::Equals,
                    '<' => return Token::LeftAngle,
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

    fn lex_identifier(&mut self, start: Location) -> Token {
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
            "export" => Token::Export,
            "let" => Token::Let(start),
            "import" => Token::Import,
            "return" => Token::Return,
            "var" => Token::Var(start),
            "while" => Token::While(start),
            _ => Token::Identifier(identifier_string, Span::new(start, self.input.location())),
        }
    }

    fn lex_string(&mut self) -> Token {
        let opening = self.input.read();
        assert_eq!(opening, '"');
        let mut characters = vec![];
        loop {
            let character = self.input.read();
            if character != '"' {
                characters.push(character);
            } else {
                break;
            }
        }
        let string: String = characters.into_iter().collect();
        Token::String(string)
    }

    fn lex_arrow_minus_or_numeric(&mut self) -> Token {
        let first_character = self.input.read();
        if first_character == '-' && self.input.peek() == '>' {
            self.input.read();
            return Token::Arrow;
        }
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
        Token::Integer(number_string.parse().unwrap())
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

fn string_head(character: char) -> bool {
    character == '"'
}

fn alphabetical(character: char) -> bool {
    (character >= 'a' && character <= 'z') || (character >= 'A' && character <= 'Z')
}

fn digit(character: char) -> bool {
    (character >= '0' && character <= '9')
}

#[cfg(test)]
mod tests {
    use super::{Location, Span, StringStream, Token, TokenStream};

    fn make_token_stream(input: &str) -> TokenStream {
        let string_stream = StringStream::new(input);
        TokenStream::new(string_stream)
    }

    fn parse(input: &str) -> Vec<Token> {
        let mut token_stream = make_token_stream(input);
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
            vec![
                Token::Identifier(
                    "foo".to_string(),
                    Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                ),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn it_parses_keywords() {
        assert_eq!(
            parse("let"),
            vec![Token::Let(Location::new(0, 1, 1)), Token::EOF],
        );
        assert_eq!(parse("return"), vec![Token::Return, Token::EOF]);
        assert_eq!(
            parse("var"),
            vec![Token::Var(Location::new(0, 1, 1)), Token::EOF],
        );
    }

    #[test]
    fn it_parses_arrows() {
        assert_eq!(
            parse("foo() -> {}"),
            vec![
                Token::Identifier(
                    "foo".to_string(),
                    Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                ),
                Token::ParenthesesLeft,
                Token::ParenthesesRight,
                Token::Arrow,
                Token::BraceLeft,
                Token::BraceRight,
                Token::EOF,
            ],
        );
    }

    #[test]
    fn it_parses_comments_and_terminals() {
        assert_eq!(
            parse("foo /* Comment */ bar"),
            vec![
                Token::Identifier(
                    "foo".to_string(),
                    Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                ),
                Token::Identifier(
                    "bar".to_string(),
                    Span::new(Location::new(18, 1, 19), Location::new(21, 1, 22)),
                ),
                Token::EOF,
            ]
        );

        assert_eq!(
            parse("foo // Comment \n bar"),
            vec![
                Token::Identifier(
                    "foo".to_string(),
                    Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                ),
                Token::Terminal('\n'),
                Token::Identifier(
                    "bar".to_string(),
                    Span::new(Location::new(17, 2, 2), Location::new(20, 2, 5)),
                ),
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
                Token::Identifier(
                    "foo".to_string(),
                    Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                ),
                Token::Terminal('\n'),
                Token::Identifier(
                    "bar".to_string(),
                    Span::new(Location::new(100, 4, 13), Location::new(103, 4, 16)),
                ),
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
            vec![Token::Minus, Token::Integer(1), Token::EOF],
        );
        assert_eq!(
            parse("- -2"),
            vec![Token::Minus, Token::Integer(-2), Token::EOF],
        );
        assert_eq!(
            parse("1+2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::EOF,
            ],
        );
        assert_eq!(
            parse("1 + 2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::EOF,
            ],
        );
        assert_eq!(
            parse("1 + -2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(-2),
                Token::EOF,
            ],
        );
    }

    #[test]
    fn it_parses_strings() {
        assert_eq!(
            parse("\"\""),
            vec![Token::String("".to_string()), Token::EOF]
        );
        assert_eq!(
            parse("\"abc123\""),
            vec![Token::String("abc123".to_string()), Token::EOF]
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

    #[test]
    fn test_read_if() {
        let input = "a";
        // Check that `read` works correctly.
        let actual = Token::Identifier(
            "a".to_string(),
            Span::new(Location::new(0, 1, 1), Location::new(1, 1, 2)),
        );
        assert_eq!(make_token_stream(input).read(), actual,);
        // Now check that the contents of the expected token don't equal the
        // actual token.
        let expected = Token::Identifier(
            "".to_string(),
            Span::new(Location::unknown(), Location::unknown()),
        );
        assert_ne!(make_token_stream(input).read(), expected);
        // But `read_if` should still return the actual token since they're
        // both `Identifier`s.
        assert_eq!(make_token_stream("a").read_if(expected), Some(actual));
        // And finally if we look for something that isn't an identifier it
        // should return `None`.
        assert_eq!(make_token_stream("a").read_if(Token::ParenthesesLeft), None);
    }
}
