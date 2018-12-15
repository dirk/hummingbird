#[derive(Clone, Debug, PartialEq)]
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
}
