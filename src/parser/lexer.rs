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
        if let Some(character) = self.input.get(self.index) {
            character
        } else {
            &'\0'
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
