#[derive(Clone, Debug)]
pub struct Location {
    pub index: u32,
    pub line: i32,
    pub column: i32,
}

impl Location {
    pub fn new(index: u32, line: i32, column: i32) -> Self {
        Self {
            index,
            line,
            column,
        }
    }

    pub fn unknown() -> Self {
        Self {
            index: 0,
            line: -1,
            column: -1,
        }
    }

    pub fn is_unknown(&self) -> bool {
        self.index == 0 && self.line == -1 && self.column == -1
    }

    /// Return a location move forward by one "character". Useful for making
    /// closing delimiters inclusive.
    pub fn plus_one(&self) -> Self {
        Self::new(self.index + 1, self.line, self.column + 1)
    }
}

#[cfg(test)]
impl PartialEq for Location {
    /// Unknown equals anything and anything equals unknown. This makes life
    /// easier in testing.
    fn eq(&self, other: &Self) -> bool {
        if self.is_unknown() || other.is_unknown() {
            true
        } else {
            (self.index == other.index && self.line == other.line && self.column == other.column)
        }
    }
}

#[cfg(not(test))]
impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        (self.index == other.index && self.line == other.line && self.column == other.column)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    // Inclusive
    pub start: Location,
    // Exclusive
    pub end: Location,
}

impl Span {
    pub fn new(start: Location, end: Location) -> Self {
        Self { start, end }
    }

    pub fn unknown() -> Self {
        Self {
            start: Location::unknown(),
            end: Location::unknown(),
        }
    }
}
