use std::error;
use std::fmt::{Debug, Display, Error, Formatter};
use std::io::{self, Write};

use codespan::{Files, Span as CodespanSpan};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use termcolor::{ColorChoice, StandardStream};

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
    /// The source directly responsible for the error.
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

    pub fn print_debug(&self) -> io::Result<()> {
        let mut w = StandardStream::stdout(ColorChoice::Auto);
        writeln!(w, "{}", self)?;
        if let Some(stack) = &self.stack {
            for (index, debug_source) in stack.iter() {
                write!(w, "  {}: ", index)?;
                if let Some(function) = &debug_source.function {
                    write!(w, "{} at ", function)?;
                }
                write!(w, "{}", debug_source.module.name())?;
                if let Some(span) = &debug_source.span {
                    write!(w, ":{}", span.start.line)?;
                    if span.start.line != span.end.line {
                        write!(w, "-{}", span.end.line)?;
                    }
                }
                write!(w, "\n")?;
            }
        }
        if let Some(source) = &self.source {
            let mut files = Files::new();
            let name = source.module.name();
            let file = files.add(name, source.module.source());

            let label = self.kind.diagnostic_label();
            if let Some(source_span) = &source.span {
                let span = CodespanSpan::new(source_span.start.index, source_span.end.index);
                let diagnostic =
                    Diagnostic::new_error("".to_owned(), Label::new(file, span, label));
                codespan_reporting::term::emit(&mut w, &Default::default(), &files, &diagnostic)?;
            }
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
    function: Option<String>,
    span: Option<Span>,
}

impl DebugSource {
    pub fn new(module: LoadedModule, function: Option<String>, span: Option<Span>) -> Self {
        Self {
            module,
            function,
            span,
        }
    }
}
