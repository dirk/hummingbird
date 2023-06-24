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
        let character = *self.peek();
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        character
    }

    fn peek(&self) -> &char {
        match self.input.get(self.index) {
            Some(character) => character,
            None => &'\0',
        }
    }

    fn read_until(&mut self, target: &char) {
        while self.peek() != target {
            let character = self.read();
            assert!(character != '\0');
        }
    }

    fn read_while<T: Fn(&char) -> bool>(&mut self, test: T) -> (String, Location) {
        let mut string = String::new();
        loop {
            let character = self.peek();
            if test(character) {
                string.push(self.read());
            } else {
                break;
            }
        }
        (string, self.location())
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

pub struct TokenStream {
    input: StringStream,
}

impl TokenStream {
    fn new(input: StringStream) -> Self {
        Self { input }
    }

    pub fn read(&mut self) -> Token {
        self.consume_space();

        let location = self.input.location();
        let character = self.input.read();
        
        if alphabetical(character) || character == '_' {
            self.lex_word(location, character)

        } else if character == '/' {
            if *self.input.peek() == '/' {
                let (tail, end) = self
                    .input
                    .read_while(|character| *character != '\n' && *character != '\0');
                Token::CommentLine(character.to_string() + &tail, Span::new(location, end))
            } else {
                Token::Slash(location)
            }

        } else if character == '\r' {
            let location = self.input.location();
            assert_eq!(self.input.read(), '\n');
            Token::Newline(location)

        } else {
            match character {
                '\0' => Token::EOF(location),
                _ => unreachable!("Unrecognized character: {:?}", character),
            }
        }
    }

    fn lex_word(&mut self, start: Location, head: char) -> Token {
        let (tail, end) = self.input.read_while(
            |character| alphabetical(*character) || digit(*character) || *character == '_',
        );
        let name = head.to_string() + &tail;
        match name.as_str() {
            "func" => Token::Func(start),
            "import" => Token::Import(start),
            "let" => Token::Let(start),
            "struct" => Token::Struct(start),
            "var" => Token::Var(start),
            _ => Token::Word(Word { name, span: Span::new(start, end) }),
        }
    }

    fn consume_space(&mut self) {
        loop {
            let character = *self.input.peek();
            if character == ' ' || character == '\t' {
                self.input.read();
            } else {
                break;
            }
        }
    }
}

fn alphabetical(character: char) -> bool {
    (character >= 'a' && character <= 'z') || (character >= 'A' && character <= 'Z')
}

fn digit(character: char) -> bool {
    (character >= '0' && character <= '9')
}

/// Any identifier in the source code (eg. "Foo" in a type or "bar" in
/// an expression).
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Arrow(Location),
    BraceLeft(Location),
    BraceRight(Location),
    Comma(Location),
    CommentLine(String, Span),
    Dot(Location),
    EOF(Location),
    Equals(Location),
    Func(Location),
    Import(Location),
    Minus(Location),
    Newline(Location),
    Let(Location),
    ParenthesesLeft(Location),
    ParenthesesRight(Location),
    Plus(Location),
    Slash(Location),
    Star(Location),
    Struct(Location),
    Var(Location),
    Word(Word),
}

impl Token {
    pub fn is_eof(&self) -> bool {
        match self {
            Token::EOF(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Location, Span, StringStream, Token, TokenStream, Word};

    macro_rules! loc {
        ($index:literal, $line:literal, $column:literal) => { Location::new($index, $line, $column) };
    }

    fn parse(input: &str) -> Vec<Token> {
        let mut stream = TokenStream::new(StringStream::new(input));
        let mut tokens = Vec::new();
        loop {
            let token = stream.read();
            let eof = token.is_eof();
            tokens.push(token);
            if eof {
                break;
            }
        }
        tokens
    }

    #[test]
    fn test_parse_words() {
        assert_eq!(
            parse("func"),
            vec![
                Token::Func(loc!(0, 1, 1)),
                Token::EOF(loc!(4, 1, 5))
            ]
        );

        assert_eq!(
            parse("foo"),
            vec![
                Token::Word(Word {
                    name: "foo".to_string(),
                    span: Span::new(loc!(0, 1, 1), loc!(3, 1, 4)),
                }),
                Token::EOF(loc!(3, 1, 4)),
            ],
        );
        // Leading whitespace
        assert_eq!(
            parse(" bar"),
            vec![
                Token::Word(Word {
                    name: "bar".to_string(),
                    span: Span::new(loc!(1, 1, 2), loc!(4, 1, 5)),
                }),
                Token::EOF(loc!(4, 1, 5)),
            ],
        );
    }

    #[test]
    fn test_parse_comments() {
        assert_eq!(
            parse("foo // bar"),
            vec![
                Token::Word(Word {
                    name: "foo".to_string(),
                    span: Span::new(loc!(0, 1, 1), loc!(3, 1, 4)),
                }),
                Token::CommentLine(
                    "// bar".to_string(),
                    Span::new(loc!(4, 1, 5), loc!(10, 1, 11)),
                ),
                Token::EOF(loc!(10, 1, 11))
            ]
        );
    }

    #[test]
    fn test_parse_newlines() {
        // Including an \r to make sure it's safely ignored.
        assert_eq!(
            parse("foo\r\nbar"),
            vec![
                Token::Word(Word {
                    name: "foo".to_string(),
                    span: Span::new(loc!(0, 1, 1), loc!(3, 1, 4)),
                }),
                Token::Newline(loc!(4, 1, 5)),
                Token::Word(Word {
                    name: "bar".to_string(),
                    span: Span::new(loc!(5, 2, 1), loc!(8, 2, 4)),
                }),
                Token::EOF(loc!(8, 2, 4))
            ]
        );
    }
}
