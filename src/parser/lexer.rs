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
        let substr = &self.input[self.index..(target.len())];
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
    EOF,
    Identifier(String),
    Integer(i64),
    Let,
    Func,
    Var,
    Return,
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
                return self.lex_numeric();
            } else {
                match character {
                    '\0' => return Token::EOF,
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

    fn lex_numeric(&mut self) -> Token {
        let mut number = vec![self.input.read()];
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
    digit(character)
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
}
