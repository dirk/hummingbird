use std::error;
use std::fmt::{Display, Error, Formatter};

#[derive(Debug)]
pub struct UndefinedNameError {
    name: String,
}

impl UndefinedNameError {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Display for UndefinedNameError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "UndefinedNameError: `{}' not found", self.name)
    }
}

impl error::Error for UndefinedNameError {}
