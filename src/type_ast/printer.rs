use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::{BufWriter, Bytes, Result, Write};

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
        self.indented_steps(1, inner)
    }

    fn indented_steps<F: FnOnce(&Self) -> Result<()>>(&self, steps: u8, inner: F) -> Result<()> {
        let previous = self.indent.take();
        self.indent.set(previous + (2 * steps));
        let result = inner(self);
        self.indent.set(previous);
        result
    }

    pub fn print_module(&self, module: Module) -> Result<()> {
        for (index, statement) in module.statements.iter().enumerate() {
            use ModuleStatement::*;
            match statement {
                Func(func) => self.print_func(func, index == 0)?,
            }
        }
        Ok(())
    }

    fn print_func(&self, func: &nodes::Func, first: bool) -> Result<()> {
        let opener = format!("func {}(", func.name);
        if first {
            self.iwrite(opener)?;
        } else {
            self.lnwrite(opener)?;
        }
        if !func.arguments.is_empty() {
            self.write("\n")?;
            self.indented(|this| {
                for argument in func.arguments.iter() {
                    this.iwrite(format!("{}: ", argument.name))?;
                    this.write_type(&argument.typ, true)?;
                    this.write(",\n")?;
                }
                Ok(())
            })?;
        }
        self.iwrite("): ")?;
        let typ = match &func.typ {
            Type::Func(func) => &**func,
            other @ _ => unreachable!("Func node has non-Func type: {:?}", other),
        };
        self.write_type(&*typ.retrn, false)?;
        self.write(" ")?;
        match &func.body {
            FuncBody::Block(block) => self.print_block(block, false),
        }
    }

    fn write_type(&self, typ: &Type, with_constraints: bool) -> Result<()> {
        match typ {
            Type::Object(object) => self.write(format!("{}", object.class.name())),
            Type::Generic(generic) => {
                self.write(format!("${} @ {:p}", generic.id, *generic))?;
                if with_constraints {
                    self.indented(|this| this.write_constraints(&generic.constraints))?;
                }
                Ok(())
            }
            Type::Variable(variable) => {
                let variable = &*variable.borrow();
                match variable {
                    Variable::Substitute(substitution) => {
                        self.write("S(")?;
                        self.write_type(&*substitution, with_constraints)?;
                        self.write(format!(") @ {:p}", substitution))
                    }
                    Variable::Unbound { id } => self.write(format!("U({}) @ {:p}", id, variable)),
                    Variable::Generic(generic) => {
                        self.write("G(")?;
                        self.write(format!("{}", generic.id))?;
                        self.write(format!(") @ {:p}", generic))?;
                        if with_constraints {
                            self.indented(|this| this.write_constraints(&generic.constraints))?;
                        }
                        Ok(())
                    }
                }
            }
            _ => unreachable!("Cannot print type: {:?}", typ),
        }
    }

    fn write_constraints(&self, constraints: &Vec<GenericConstraint>) -> Result<()> {
        if constraints.is_empty() {
            return Ok(());
        }
        self.lnwrite("where")?;
        self.indented(|this| {
            for constraint in constraints {
                use GenericConstraint::*;
                match constraint {
                    Property(property) => {
                        this.lnwrite(format!("{}: ", property.name))?;
                        this.write_type(&property.typ, true)?;
                    }
                    other @ _ => unreachable!("Cannot write constraint: {:?}", other),
                }
            }
            Ok(())
        })
    }

    fn print_block(&self, block: &Block, initial_indent: bool) -> Result<()> {
        let opener = "{";
        if initial_indent {
            self.iwrite(opener)?;
        } else {
            self.write(opener)?;
        }

        self.indented(|this| {
            for (_, statement) in block.statements.iter().enumerate() {
                use BlockStatement::*;
                match statement {
                    Expression(expression) => {
                        self.print_expression(expression)?;
                    }
                    Func(func) => self.print_func(func, false)?,
                }
            }
            Ok(())
        })?;
        self.write("\n")?;
        self.iwrite("}\n")
    }

    fn print_expression(&self, expression: &Expression) -> Result<()> {
        use Expression::*;
        match expression {
            Identifier(identifier) => self.print_identifier(identifier),
            Infix(infix) => self.print_infix(infix),
            LiteralInt(literal) => self.lnwrite(format!("{}", literal.value)),
            PostfixProperty(property) => self.print_postfix_property(property, 0).map(|_| ()),
        }
    }

    fn print_identifier(&self, identifier: &Identifier) -> Result<()> {
        self.lnwrite(format!("Identifier({}): ", identifier.name.name))?;
        self.write_type(&identifier.typ, false)
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

    fn is_postfix(&self, expression: &Expression) -> bool {
        use Expression::*;
        match expression {
            PostfixProperty(_) => true,
            _ => false,
        }
    }

    fn print_postfix_property(&self, property: &PostfixProperty, current: u8) -> Result<u8> {
        let max = match &*property.target {
            // Links in the chain
            Expression::PostfixProperty(target) => {
                self.print_postfix_property(target, current + 1)?
            }
            // Tail of the chain
            other @ _ => {
                self.print_expression(other)?;
                current
            }
        };
        // Have a +1 so that we always indent at least one step.
        self.indented_steps(max - current + 1, |this| {
            this.lnwrite(format!("Property({}): ", property.property.name))?;
            this.write_type(&property.typ, false)
        })?;
        Ok(max)
    }

    /// Write a string.
    fn write<S: AsRef<str>>(&self, string: S) -> Result<()> {
        self.write_output(string.as_ref().as_bytes())
    }

    /// Write indentation and then a string.
    fn iwrite<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let indented = " ".repeat(self.indent.get() as usize);
        self.write_output(indented.as_bytes())
            .and_then(|_| self.write_output(string.as_ref().as_bytes()))
            .map(|_| ())
    }

    /// Write a newline, indentation, and then a string.
    fn lnwrite<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let indented = " ".repeat(self.indent.get() as usize);
        self.write_output("\n".as_bytes())
            .and_then(|_| self.write_output(indented.as_bytes()))
            .and_then(|_| self.write_output(string.as_ref().as_bytes()))
    }

    fn write_output(&self, bytes: &[u8]) -> Result<()> {
        let mut output = self.output.borrow_mut();
        output.write(bytes).map(|_| ())
    }

    // fn write_output(&self, bytes: &[u8]) -> Result<()> {
    //     let result = if let Some(buffer) = &mut *self.buffer.borrow_mut() {
    //         buffer.write(bytes)
    //     } else {
    //         let mut output = self.output.borrow_mut();
    //         output.write(bytes)
    //     };
    //     result.map(|_| ())
    // }
    //
    // fn buffer<F: FnOnce(&Self) -> Result<()>>(&self, inner: F) -> Result<Vec<u8>> {
    //     {
    //         let mut buffer = self.buffer.borrow_mut();
    //         *buffer = Some(vec![]);
    //     }
    //     inner(self)?;
    //     let buffer = self.buffer.replace(None);
    //     Ok(buffer.unwrap())
    // }
}
