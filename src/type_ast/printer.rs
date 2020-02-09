use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::{Result, Write};

use super::nodes::{self, *};
use super::typ::*;

macro_rules! iwrite {
    ($self: ident, $($arg:tt)*) => (
        $self.writeln(format_args!($($arg)*))
    );
}

macro_rules! lnwrite {
    ($self: ident, $($arg:tt)*) => (
        $self.writeln(format_args!($($arg)*))
    );
}

pub struct Printer<O: Write> {
    output: RefCell<O>,
    indent: Cell<u8>,
}

impl<O: Write> Printer<O> {
    pub fn new(output: O) -> Self {
        Self {
            output: RefCell::new(output),
            indent: Cell::new(0),
        }
    }

    fn indented<F: FnOnce(&Self) -> Result<()>>(&self, inner: F) -> Result<()> {
        let previous = self.indent.take();
        self.indent.set(previous + 2);
        let result = inner(self);
        self.indent.set(previous);
        result
    }

    pub fn print_module(&self, module: Module) -> Result<()> {
        for statement in module.statements.iter() {
            use ModuleStatement::*;
            match statement {
                Func(func) => self.print_func(func)?,
            }
        }
        Ok(())
    }

    fn print_func(&self, func: &nodes::Func) -> Result<()> {
        self.iwrite(format!("func {}(", func.name))?;
        if !func.arguments.is_empty() {
            self.write("\n")?;
            self.indented(|this| {
                for argument in func.arguments.iter() {
                    this.iwrite(format!("{}: ", argument.name))?;
                    this.write_type(&argument.typ)?;
                    this.write(",\n")?;
                }
                Ok(())
            })?;
        }
        self.write(") ")?;
        match &func.body {
            FuncBody::Block(block) => self.print_block(block, false),
        }
    }

    fn write_type(&self, typ: &Type) -> Result<()> {
        match typ {
            Type::Object(object) => self.write(format!("{}", object.class.name())),
            Type::Generic(generic) => self.write(format!("${} @ {:p}", generic.id, *generic)),
            Type::Variable(variable) => {
                let variable = &*variable.borrow();
                match variable {
                    Variable::Substitute(substitution) => {
                        self.write("S(")?;
                        self.write_type(&*substitution)?;
                        self.write(format!(") @ {:p}", substitution))
                    }
                    Variable::Unbound { id } => self.write(format!("U({}) @ {:p}", id, variable)),
                    Variable::Generic(generic) => {
                        self.write("G(")?;
                        self.write(format!("{}", generic.id))?;
                        self.write(format!(") @ {:p}", generic))
                    }
                }
            }
            _ => unreachable!("Cannot print type: {:?}", typ),
        }
    }

    fn print_block(&self, block: &Block, initial_indent: bool) -> Result<()> {
        let opener = "{";
        if initial_indent {
            self.iwrite(opener)?;
        } else {
            self.write(opener)?;
        }

        self.indented(|this| {
            for statement in block.statements.iter() {
                this.print_block_statement(statement)?;
            }
            Ok(())
        })?;
        self.write("\n")?;
        self.iwrite("}\n")
    }

    fn print_block_statement(&self, statement: &BlockStatement) -> Result<()> {
        use BlockStatement::*;
        match statement {
            Expression(expression) => {
                self.print_expression(expression)?;
            }
            Func(func) => self.print_func(func)?,
        }
        Ok(())
    }

    fn print_expression(&self, expression: &Expression) -> Result<()> {
        use Expression::*;
        match expression {
            Identifier(identifier) => self.print_identifier(identifier),
            Infix(infix) => self.print_infix(infix),
            LiteralInt(literal) => self.lnwrite(format!("{}", literal.value)),
        }
    }

    fn print_identifier(&self, identifier: &Identifier) -> Result<()> {
        self.lnwrite(format!("Identifier({}): ", identifier.name.name))?;
        self.write_type(&identifier.typ)
    }

    fn print_infix(&self, infix: &Infix) -> Result<()> {
        self.lnwrite("Infix(")?;
        self.indented(|this1| {
            this1.lnwrite("lhs:")?;
            this1.indented(|this2| this2.print_expression(&infix.lhs))?;
            this1.lnwrite(format!("op: {}", infix.op.to_string()))?;
            this1.lnwrite(format!("rhs:"))?;
            this1.indented(|this2| this2.print_expression(&infix.rhs))
        })?;
        self.write(")")
    }

    /// Write a string.
    fn write<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let mut output = self.output.borrow_mut();
        output.write(string.as_ref().as_bytes()).map(|_| ())
    }

    /// Write indentation and then a string.
    fn iwrite<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let indented = " ".repeat(self.indent.get() as usize);
        let mut output = self.output.borrow_mut();
        output
            .write(indented.as_bytes())
            .and_then(|_| output.write(string.as_ref().as_bytes()))
            .map(|_| ())
    }

    /// Write a newline, indentation, and then a string.
    fn lnwrite<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let indented = " ".repeat(self.indent.get() as usize);
        let mut output = self.output.borrow_mut();
        output
            .write("\n".as_bytes())
            .and_then(|_| output.write(indented.as_bytes()))
            .and_then(|_| output.write(string.as_ref().as_bytes()))
            .map(|_| ())
    }
}
