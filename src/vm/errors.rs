use std::error;
use std::fmt::{Display, Error, Formatter};

use super::vm::StackSnapshot;

#[derive(Debug)]
pub struct AnnotatedError {
    wrapped: Box<dyn error::Error>,
    stack: StackSnapshot,
}

impl AnnotatedError {
    pub fn new(wrapped: Box<dyn error::Error>, stack: StackSnapshot) -> Self {
        Self { wrapped, stack }
    }

    pub fn get_wrapped(&self) -> &Box<dyn error::Error> {
        &self.wrapped
    }
}

impl Display for AnnotatedError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.wrapped)?;
        for (index, description) in self.stack.iter() {
            write!(f, "\n  {}: {}", index, description)?;
        }
        Ok(())
    }
}

impl error::Error for AnnotatedError {}

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
