use std::error;
use std::fmt::{Debug, Display, Error, Formatter};
use std::io;

use codespan::{Files, Span as CodespanSpan};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use termcolor::WriteColor;

use super::super::parser::Span;
use super::loader::LoadedModule;
use super::value::Value;
use super::vm::StackSnapshot;

#[derive(Debug)]
enum Kind {
    /// An error loading a file.
    /// (name, wrapped)
    LoadFile(String, Box<dyn error::Error>),
    /// (target, value)
    PropertyNotFound(Value, String),
    /// (name)
    UndefinedName(String),
}

impl Kind {
    /// Returns a label for the detail in the diagnostic.
    fn diagnostic_label(&self) -> String {
        use Kind::*;
        let label = match self {
            LoadFile(_, _) => "Import occurred here",
            PropertyNotFound(_, _) => "Missing property",
            UndefinedName(_) => "Undefined name",
        };
        label.to_owned()
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use Kind::*;
        match self {
            LoadFile(name, wrapped) => {
                write!(f, "LoadFileError: could not load `{}': {:?}", name, wrapped)
            }
            PropertyNotFound(target, value) => write!(
                f,
                "PropertyNotFoundError: `{}' not found on {:?}",
                value, target
            ),
            UndefinedName(name) => write!(f, "UndefinedNameError: `{}' not found", name),
        }
    }
}

pub struct VmError {
    kind: Kind,
    stack: Option<StackSnapshot>,
    source: Option<DebugSource>,
}

impl VmError {
    pub fn new_load_file(path: String, wrapped: Box<dyn error::Error>) -> Self {
        Self::new(Kind::LoadFile(path, wrapped))
    }

    pub fn new_property_not_found(target: Value, value: String) -> Self {
        Self::new(Kind::PropertyNotFound(target, value))
    }

    pub fn new_undefined_name(name: String) -> Self {
        Self::new(Kind::UndefinedName(name))
    }

    fn new(kind: Kind) -> Self {
        Self {
            kind,
            stack: None,
            source: None,
        }
    }

    /// Called by the VM to annotate the error with a stack trace.
    pub fn set_stack(&mut self, stack: StackSnapshot) {
        self.stack = Some(stack);
    }

    /// Called by the frame if it was able to find a source mapping for where
    /// the error occurred.
    pub fn set_source(&mut self, source: DebugSource) {
        self.source = Some(source);
    }

    pub fn print_debug<W: WriteColor>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "{}", self)?;
        if let Some(stack) = &self.stack {
            for (index, description) in stack.iter() {
                writeln!(w, "  {}: {}", index, description)?;
            }
        }
        if let Some(source) = &self.source {
            let mut files = Files::new();
            let name = source.module.name();
            let file = files.add(name, source.module.source());

            let label = self.kind.diagnostic_label();
            let span = CodespanSpan::new(source.span.start.index, source.span.end.index);
            let diagnostic = Diagnostic::new_error("".to_owned(), Label::new(file, span, label));
            codespan_reporting::term::emit(w, &Default::default(), &files, &diagnostic)?;
        }
        Ok(())
    }
}

impl Debug for VmError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "VmError {{ {:?} }}", self.kind)
    }
}

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.kind)
    }
}

impl error::Error for VmError {}

/// Describes in a module where an error occurred.
pub struct DebugSource {
    module: LoadedModule,
    span: Span,
}

impl DebugSource {
    pub fn new(module: LoadedModule, span: Span) -> Self {
        Self { module, span }
    }
}

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

#[derive(Debug)]
pub struct PropertyNotFoundError {
    target: Value,
    value: String,
}

impl PropertyNotFoundError {
    pub fn new(target: Value, value: String) -> Self {
        Self { target, value }
    }
}

impl Display for PropertyNotFoundError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(
            f,
            "PropertyNotFoundError: `{}' not found on {:?}",
            self.value, self.target
        )
    }
}

impl error::Error for PropertyNotFoundError {}
