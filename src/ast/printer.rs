use std::fmt;
use std::io::{Result, Write};

use super::nodes::*;

pub struct Printer<O: Write> {
    output: O,
    indent: u8,
}

macro_rules! writeln {
    ($self: ident, $($arg:tt)*) => (
        $self.writeln(format_args!($($arg)*))
    );
}

impl<O: Write> Printer<O> {
    pub fn new(output: O) -> Self {
        Self { output, indent: 0 }
    }

    fn indented<F: FnOnce(&mut Self) -> Result<()>>(&mut self, inner: F) -> Result<()> {
        self.indent += 2;
        let result = inner(self);
        self.indent -= 2;
        result
    }

    pub fn print_root(&mut self, root: Root) -> Result<()> {
        if let Some(bindings) = &root.bindings {
            writeln!(self, "bindings [")?;
            self.indented(|printer| {
                for capture in bindings.iter() {
                    writeln!(printer, "{}", capture)?;
                }
                Ok(())
            })?;
            writeln!(self, "]")?;
        }
        if let Some(parent_bindings) = &root.parent_bindings {
            writeln!(self, "parent_bindings [")?;
            self.indented(|printer| {
                for capture in parent_bindings.iter() {
                    writeln!(printer, "{}", capture)?;
                }
                Ok(())
            })?;
            writeln!(self, "]")?;
        }
        for node in root.nodes {
            self.print_node(node)?
        }
        Ok(())
    }

    fn print_node(&mut self, node: Node) -> Result<()> {
        match node {
            Node::Assignment(assignment) => self.print_assignment(assignment),
            Node::Block(block) => self.print_block(block),
            Node::Function(function) => self.print_function(function),
            Node::Identifier(identifier) => self.print_identifier(identifier),
            Node::Integer(integer) => self.print_integer(integer),
            Node::PostfixCall(postfix_call) => self.print_postfix_call(postfix_call),
            Node::Return(ret) => self.print_return(ret),
            Node::Var(var) => self.print_var(var),
            _ => writeln!(self, "{:?}", node),
        }
    }

    fn print_assignment(&mut self, assignment: Assignment) -> Result<()> {
        writeln!(self, "Assignment(")?;
        self.indented(|printer| {
            printer.print_node(*assignment.lhs)?;
            printer.print_node(*assignment.rhs)
        })?;
        writeln!(self, ")")
    }

    fn print_block(&mut self, block: Block) -> Result<()> {
        writeln!(self, "Block(")?;
        self.indented(|printer| {
            for node in block.nodes {
                printer.print_node(node)?;
            }
            Ok(())
        })?;
        writeln!(self, ")")
    }

    fn print_function(&mut self, function: Function) -> Result<()> {
        writeln!(
            self,
            "Function({}",
            function.name.clone().unwrap_or("".to_string()),
        )?;
        if let Some(bindings) = function.get_bindings() {
            self.indented(|printer| {
                writeln!(printer, "bindings [")?;
                printer.indented(|bindings_printer| {
                    for capture in bindings.iter() {
                        writeln!(bindings_printer, "{}", capture)?;
                    }
                    Ok(())
                })?;
                writeln!(printer, "]")
            })?;
        }
        if let Some(parent_bindings) = function.get_parent_bindings() {
            self.indented(|printer| {
                writeln!(printer, "parent_bindings [")?;
                printer.indented(|parent_bindings_printer| {
                    for capture in parent_bindings.iter() {
                        writeln!(parent_bindings_printer, "{}", capture)?;
                    }
                    Ok(())
                })?;
                writeln!(printer, "]")
            })?;
        }
        self.indented(|printer| printer.print_node(*function.body))?;
        writeln!(self, ")")
    }

    fn print_identifier(&mut self, identifier: Identifier) -> Result<()> {
        writeln!(self, "Identifier({})", identifier.value)
    }

    fn print_integer(&mut self, integer: Integer) -> Result<()> {
        writeln!(self, "Integer({})", integer.value)
    }

    fn print_postfix_call(&mut self, postfix_call: PostfixCall) -> Result<()> {
        writeln!(self, "PostfixCall(")?;
        self.indented(|printer| {
            printer.print_node(*postfix_call.clone().target)?;
            if postfix_call.arguments.is_empty() {
                return writeln!(printer, "[]");
            }
            writeln!(printer, "[")?;
            printer.indented(|arguments_printer| {
                for argument in postfix_call.arguments {
                    arguments_printer.print_node(argument)?;
                }
                Ok(())
            })?;
            writeln!(printer, "]")
        })?;
        writeln!(self, ")")
    }

    fn print_return(&mut self, ret: Return) -> Result<()> {
        if let Some(rhs) = ret.rhs {
            writeln!(self, "Return(")?;
            self.indented(|printer| printer.print_node(*rhs))?;
            writeln!(self, ")")
        } else {
            writeln!(self, "Return()")
        }
    }

    fn print_var(&mut self, var: Var) -> Result<()> {
        if let Some(rhs) = var.rhs {
            writeln!(self, "Var({}", var.lhs.value)?;
            self.indented(|printer| printer.print_node(*rhs))?;
            writeln!(self, ")")
        } else {
            writeln!(self, "Var({})", var.lhs.value)
        }
    }

    fn writeln(&mut self, args: fmt::Arguments) -> Result<()> {
        let indented = " ".repeat(self.indent as usize);
        self.output
            .write(indented.as_bytes())
            .and_then(|_| self.output.write_fmt(args))
            .and_then(|_| self.output.write("\n".as_bytes()))
            .map(|_| ())
    }
}
