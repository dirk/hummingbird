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

    fn read_while<T: Fn(char) -> bool>(&mut self, test: T) -> (String, Location) {
        let mut characters = vec![];
        loop {
            let character = self.peek();
            if test(character) {
                characters.push(self.read());
            } else {
                break;
            }
        }
        let string: String = characters.into_iter().collect();
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

#[derive(Clone, Debug, PartialEq)]
pub struct LiteralInt {
    pub value: i64,
    pub span: Span,
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
    LiteralInt(LiteralInt),
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
    pub fn location(&self) -> Location {
        use Token::*;
        match self {
            CommentLine(_, span) => span.start.clone(),
            LiteralInt(literal) => literal.span.start.clone(),
            Word(word) => word.span.start.clone(),
            BraceLeft(location)
            | BraceRight(location)
            | Comma(location)
            | Dot(location)
            | EOF(location)
            | Equals(location)
            | Func(location)
            | Import(location)
            | Minus(location)
            | Newline(location)
            | Let(location)
            | ParenthesesLeft(location)
            | ParenthesesRight(location)
            | Plus(location)
            | Slash(location)
            | Star(location)
            | Struct(location)
            | Var(location) => location.clone(),
        }
    }

    pub fn to_string(&self) -> String {
        use Token::*;
        match self {
            Plus(_) => "+",
            Star(_) => "*",
            _ => unreachable!("Cannot stringify: {:?}", self),
        }
        .to_string()
    }

    pub fn same_variant_as(&self, other: &Token) -> bool {
        use Token::*;
        match (self, other) {
            (Plus(_), Plus(_)) => true,
            (Slash(_), Slash(_)) => true,
            (Star(_), Star(_)) => true,
            _ => false,
        }
    }

    pub fn is_brace_right(&self) -> bool {
        match self {
            Token::BraceRight(_) => true,
            _ => false,
        }
    }

    pub fn is_eof(&self) -> bool {
        match self {
            Token::EOF(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct TokenStream {
    input: StringStream,
    peek: Option<Token>,
}

impl TokenStream {
    pub fn from_string(input: String) -> Self {
        Self::new(StringStream::new(input.as_str()))
    }

    fn new(input: StringStream) -> Self {
        TokenStream { input, peek: None }
    }

    pub fn peek(&mut self) -> Token {
        if self.peek.is_none() {
            self.peek = Some(self.next());
        }
        self.peek.clone().unwrap()
    }

    pub fn read(&mut self) -> Token {
        match self.peek.clone() {
            Some(token) => {
                self.peek = None;
                token
            }
            None => self.next(),
        }
    }

    fn next(&mut self) -> Token {
        self.consume_space();

        let location = self.input.location();
        let character = self.input.read();
        if word_head(character) {
            self.lex_word(location, character)
        } else if numeric_head(character) {
            self.lex_arrow_minus_or_numeric(location, character)
        } else if character == '/' {
            self.lex_slash_or_line_comment(location)
        } else if character == '\r' {
            let next_location = self.input.location();
            let next = self.input.read();
            assert_eq!(next, '\n');
            Token::Newline(next_location)
        } else {
            match character {
                '{' => Token::BraceLeft(location),
                '}' => Token::BraceRight(location),
                ',' => Token::Comma(location),
                '.' => Token::Dot(location),
                '=' => Token::Equals(location),
                '(' => Token::ParenthesesLeft(location),
                ')' => Token::ParenthesesRight(location),
                '+' => Token::Plus(location),
                '\n' => Token::Newline(location),
                '/' => Token::Slash(location),
                '*' => Token::Star(location),
                '\0' => Token::EOF(location),
                _ => unreachable!("Unrecognized character: {:?}", character),
            }
        }
    }

    fn lex_word(&mut self, start: Location, head: char) -> Token {
        let (tail, end) = self.input.read_while(word_tail);
        let name = head.to_string() + &tail;
        match name.as_str() {
            "func" => Token::Func(start),
            "import" => Token::Import(start),
            "struct" => Token::Struct(start),
            "var" => Token::Var(start),
            _ => Token::Word(Word {
                name,
                span: Span::new(start, end),
            }),
        }
    }

    fn lex_arrow_minus_or_numeric(&mut self, start: Location, head: char) -> Token {
        // if first_character == '-' && self.input.peek() == '>' {
        //     self.input.read();
        //     return Token::Arrow;
        // }
        if head == '-' && !digit(self.input.peek()) {
            return Token::Minus(start);
        }
        let (tail, end) = self.input.read_while(digit);
        let number_string = head.to_string() + &tail;
        Token::LiteralInt(LiteralInt {
            value: number_string.parse().unwrap(),
            span: Span::new(start, end),
        })
    }

    fn lex_slash_or_line_comment(&mut self, start: Location) -> Token {
        let next = self.input.peek();
        if next == '/' {
            let (tail, end) = self
                .input
                .read_while(|character| character != '\n' && character != '\0');
            Token::CommentLine(format!("/{}", tail), Span::new(start, end))
        } else {
            Token::Slash(start)
        }
    }

    fn consume_space(&mut self) {
        loop {
            let character = self.input.peek();
            if character == ' ' || character == '\t' {
                self.input.read();
                continue;
            }
            // Not any kind of inline whitespace.
            break;
        }
    }
}

fn word_head(character: char) -> bool {
    alphabetical(character) || character == '_'
}

fn word_tail(character: char) -> bool {
    alphabetical(character) || digit(character) || character == '_'
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
    use super::{Location, Span, StringStream, Token, TokenStream, Word};

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
            if let Token::EOF(_) = token {
                break;
            }
        }
        return tokens;
    }

    #[test]
    fn test_parse_words() {
        assert_eq!(
            parse("func"),
            vec![
                Token::Func(Location::new(0, 1, 1)),
                Token::EOF(Location::new(4, 1, 5))
            ]
        );
        assert_eq!(
            parse("struct"),
            vec![
                Token::Struct(Location::new(0, 1, 1)),
                Token::EOF(Location::new(6, 1, 7))
            ]
        );
        assert_eq!(
            parse("foo"),
            vec![
                Token::Word(Word {
                    name: "foo".to_string(),
                    span: Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                }),
                Token::EOF(Location::new(3, 1, 4)),
            ]
        );
    }

    #[test]
    fn test_parse_comments() {
        assert_eq!(
            parse("foo // bar"),
            vec![
                Token::Word(Word {
                    name: "foo".to_string(),
                    span: Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                }),
                Token::CommentLine(
                    "// bar".to_string(),
                    Span::new(Location::new(4, 1, 5), Location::new(10, 1, 11)),
                ),
                Token::EOF(Location::new(10, 1, 11))
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
                    span: Span::new(Location::new(0, 1, 1), Location::new(3, 1, 4)),
                }),
                Token::Newline(Location::new(4, 1, 5)),
                Token::Word(Word {
                    name: "bar".to_string(),
                    span: Span::new(Location::new(5, 2, 1), Location::new(8, 2, 4)),
                }),
                Token::EOF(Location::new(8, 2, 4))
            ]
        );
    }
}
