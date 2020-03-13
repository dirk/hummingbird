use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::fmt;
use std::io::{BufWriter, Bytes, Result, Write};

use super::nodes::{self, *};
use super::typ::*;

macro_rules! iwrite {
    ($self: ident, $($arg:tt)*) => (
        $self.iwrite(format!($($arg)*))
    );
}

macro_rules! writeln {
    ($self: ident, $($arg:tt)*) => (
        $self.writeln(format!($($arg)*))
    );
}

pub struct PrinterOptions {
    pub print_pointers: bool,
}

impl Default for PrinterOptions {
    fn default() -> Self {
        Self {
            print_pointers: false,
        }
    }
}

pub struct Printer<O: Write> {
    output: RefCell<O>,
    indent: Cell<u8>,
    print_pointers: bool,
}

impl<O: Write> Printer<O> {
    pub fn new(output: O) -> Self {
        Self::new_with_options(output, PrinterOptions::default())
    }

    pub fn new_with_options(output: O, options: PrinterOptions) -> Self {
        Self {
            output: RefCell::new(output),
            indent: Cell::new(0),
            print_pointers: options.print_pointers,
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

    pub fn print_module(&self, module: &Module) -> Result<()> {
        self.writeln("Module {")?;
        self.indented(|this| {
            for statement in module.statements.iter() {
                use ModuleStatement::*;
                match statement {
                    Func(func) => self.print_func(func)?,
                }
            }
            Ok(())
        })?;
        self.writeln("}")?;
        Ok(())
    }

    fn print_func(&self, func: &nodes::Func) -> Result<()> {
        self.writeln("Func {")?;
        self.indented(|_| {
            writeln!(self, "name: {}", func.name)?;
            self.iwrite("arguments: [")?;
            if !func.arguments.is_empty() {
                self.write("\n")?;
                self.indented(|_| {
                    for argument in func.arguments.iter() {
                        iwrite!(self, "{}: ", argument.name)?;
                        self.write_type(&argument.typ, true)?;
                        self.write("\n")?;
                    }
                    Ok(())
                })?;
                self.writeln("]")?;
            } else {
                self.write("]\n")?;
            }
            // TODO: Body
            self.iwrite("typ: ")?;
            self.write_type(&func.typ, true)?;
            self.write("\n")?;
            self.writeln("body:")?;
            self.indented(|_| match &func.body {
                FuncBody::Block(block) => self.print_block(block),
            })?;
            Ok(())
        })?;
        self.writeln("}")
    }

    fn write_type(&self, typ: &Type, with_children: bool) -> Result<()> {
        self.write_recursive_type(typ, with_children, &mut HashSet::new())
    }

    // Write type recursively with guards against infinite recursion.
    fn write_recursive_type(
        &self,
        typ: &Type,
        with_children: bool,
        tracker: &mut HashSet<usize>,
    ) -> Result<()> {
        // True if we're writing a type we've written before.
        let recursive = !tracker.insert(typ.id());

        match typ {
            Type::Func(func) => {
                self.write(format!(
                    "{}#{}(",
                    func.name.clone().unwrap_or("".to_string()),
                    func.id
                ))?;
                let arguments = func.arguments.borrow();
                if !arguments.is_empty() {
                    if with_children {
                        for argument in arguments.iter() {
                            self.indented(|this1| {
                                this1.lnwrite("")?;
                                this1.write_recursive_type(argument, true, tracker)?;
                                this1.write(",")
                            })?;
                        }
                        self.lnwrite("): ")?;
                    } else {
                        self.write(format!("{}): ", arguments.len()))?;
                    }
                } else {
                    self.write("): ")?;
                }
                self.write_recursive_type(&func.retrn.borrow(), true, tracker)?;
                self.write_pointer(&**func)
            }
            Type::Object(object) => {
                self.write(format!("{}", object.class.name()))?;
                self.write_pointer(&**object)
            }
            Type::Generic(outer) => {
                let generic = outer.borrow();
                self.write(format!("${}", generic.id))?;
                if generic.has_constrains() && with_children && !recursive {
                    self.write("(")?;
                    self.indented(|this| this.write_constraints(&generic.constraints, tracker))?;
                    self.write(")")?;
                }
                self.write_pointer(&**outer)
            }
            Type::Tuple(tuple) => self.write(format!("({})", tuple.members.len())),
            Type::Variable(variable) => {
                let variable = &*variable.borrow();
                match variable {
                    Variable::Substitute { substitute, .. } => {
                        self.write("S(")?;
                        if recursive {
                            self.write("...")?;
                        } else {
                            self.write_type(&*substitute, with_children)?;
                        }
                        self.write(")")?;
                    }
                    Variable::Unbound { id, .. } => {
                        self.write(format!("U({})", id))?;
                    }
                    Variable::Generic { generic, .. } => {
                        self.write("G(")?;
                        self.write(format!("{}", generic.id))?;
                        if with_children && !recursive {
                            self.indented(|this| {
                                this.write_constraints(&generic.constraints, tracker)
                            })?;
                        }
                        self.write(")")?;
                    }
                }
                self.write_pointer(variable)
            }
            _ => unreachable!("Cannot print type: {:?}", typ),
        }
    }

    fn write_constraints(
        &self,
        constraints: &Vec<GenericConstraint>,
        tracker: &mut HashSet<usize>,
    ) -> Result<()> {
        if constraints.is_empty() {
            return Ok(());
        }
        self.lnwrite("where")?;
        self.indented(|this1| {
            for constraint in constraints {
                use GenericConstraint::*;
                match constraint {
                    Property(property) => {
                        this1.lnwrite(format!("{}: ", property.name))?;
                        this1.write_recursive_type(&property.typ, true, tracker)?;
                    }
                    Callable(callable) => {
                        if callable.arguments.is_empty() {
                            this1.lnwrite("(): ")?;
                        } else {
                            this1.lnwrite("(\n")?;
                            for argument in callable.arguments.iter() {
                                this1.indented(|this2| {
                                    this2.iwrite("")?;
                                    this2.write_recursive_type(argument, true, tracker)?;
                                    this2.write(",")
                                })?;
                            }
                            this1.lnwrite("): ")?;
                        }
                        this1.write_recursive_type(&callable.retrn, true, tracker)?;
                    }
                }
            }
            Ok(())
        })
    }

    fn print_var(&self, var: &Var, first: bool) -> Result<()> {
        let opener = format!("var {}: ", &var.name.name);
        if first {
            self.iwrite(opener)?;
        } else {
            self.lnwrite(opener)?;
        }
        self.write_type(&var.typ, true)?;
        if let Some(initializer) = &var.initializer {
            self.indented(|this| {
                this.write(" =")?;
                this.print_expression(initializer)
            })?;
        }
        Ok(())
    }

    fn print_block(&self, block: &Block) -> Result<()> {
        self.iwrite("Block {")?;
        if block.statements.is_empty() {
            return self.write("}\n");
        }
        self.write("\n")?;
        self.indented(|_| {
            let last_index = block.statements.len().checked_sub(1).unwrap_or(0);
            for statement in block.statements.iter() {
                use BlockStatement::*;
                match statement {
                    Expression(expression) => {
                        self.print_expression(expression)?;
                    }
                    Func(func) => self.print_func(func)?,
                    Var(var) => self.print_var(var, false)?,
                }
            }
            Ok(())
        })?;
        self.writeln("}")
    }

    fn print_expression(&self, expression: &Expression) -> Result<()> {
        use Expression::*;
        match expression {
            Closure(closure) => self.print_closure(closure),
            Identifier(identifier) => self.print_identifier(identifier),
            Infix(infix) => self.print_infix(infix),
            LiteralInt(literal) => self.print_literal_int(literal),
            PostfixCall(call) => self.print_postfix_call(call),
            PostfixProperty(property) => self.print_postfix_property(property),
        }
    }

    fn print_closure(&self, closure: &Closure) -> Result<()> {
        self.writeln("Closure {")?;
        self.indented(|_| {
            self.iwrite("arguments: [")?;
            if !closure.arguments.is_empty() {
                self.write("\n")?;
                self.indented(|_| {
                    for argument in closure.arguments.iter() {
                        self.iwrite(format!("{}: ", argument.name))?;
                        self.write_type(&argument.typ, true)?;
                        self.write("\n")?;
                    }
                    Ok(())
                })?;
                self.writeln("]")?;
            } else {
                self.write("]\n")?;
            }
            self.writeln("body:")?;
            self.indented(|_| match &*closure.body {
                ClosureBody::Block(block) => self.print_block(block),
                ClosureBody::Expression(expression) => self.print_expression(expression),
            })?;
            self.iwrite("typ: ")?;
            self.write_type(&closure.typ, false)?;
            self.write("\n")
        })?;
        self.writeln("}")
    }

    fn print_literal_int(&self, literal: &LiteralInt) -> Result<()> {
        self.writeln("LiteralInt {")?;
        self.indented(|_| {
            writeln!(self, "value: {}", literal.value)?;
            self.iwrite("typ: ")?;
            self.write_type(&literal.typ, false)?;
            self.write("\n")
        })?;
        self.writeln("}")
    }

    fn print_identifier(&self, identifier: &Identifier) -> Result<()> {
        self.writeln("Identifier {")?;
        self.indented(|_| {
            writeln!(self, "name: {}", identifier.name.name)?;
            self.iwrite("typ: ")?;
            self.write_type(&identifier.typ, true)?;
            self.write("\n")
        })?;
        self.writeln("}")
    }

    fn print_infix(&self, infix: &Infix) -> Result<()> {
        self.writeln("Infix {")?;
        self.indented(|_| {
            self.writeln("lhs:")?;
            self.indented(|_| self.print_expression(&infix.lhs))?;
            writeln!(self, "op: {}", infix.op.to_string())?;
            self.writeln("rhs:")?;
            self.indented(|_| self.print_expression(&infix.rhs))?;
            self.iwrite("typ: ")?;
            self.write_type(&infix.typ, false)?;
            self.write("\n")
        })?;
        self.writeln("}")
    }

    fn print_postfix_call(&self, call: &PostfixCall) -> Result<()> {
        self.writeln("PostfixCall {")?;
        self.indented(|_| {
            self.writeln("target:")?;
            self.indented(|_| self.print_expression(&call.target))?;
            self.iwrite("arguments: [")?;
            if !call.arguments.is_empty() {
                self.write("\n")?;
                self.indented(|_| {
                    for argument in call.arguments.iter() {
                        self.print_expression(argument)?;
                    }
                    Ok(())
                })?;
                self.writeln("]")?;
            } else {
                self.write("]\n")?;
            }
            self.iwrite("typ: ")?;
            self.write_type(&call.typ, true)?;
            self.write("\n")
        })?;
        self.writeln("}")
    }

    fn print_postfix_property(&self, property: &PostfixProperty) -> Result<()> {
        self.writeln("PostfixCall {")?;
        self.indented(|_| {
            self.writeln("target:")?;
            self.indented(|_| self.print_expression(&property.target))?;
            writeln!(self, "property: {}", property.property.name)?;
            self.iwrite("typ: ")?;
            self.write_type(&property.typ, true)?;
            self.write("\n")
        })?;
        self.writeln("}")
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

    /// Write indentation, a string, and then a newline.
    fn writeln<S: AsRef<str>>(&self, string: S) -> Result<()> {
        let indented = " ".repeat(self.indent.get() as usize);
        self.write_output(indented.as_bytes())
            .and_then(|_| self.write_output(string.as_ref().as_bytes()))
            .and_then(|_| self.write_output("\n".as_bytes()))
    }

    fn write_output(&self, bytes: &[u8]) -> Result<()> {
        let mut output = self.output.borrow_mut();
        output.write(bytes).map(|_| ())
    }

    fn write_pointer<P>(&self, ptr: *const P) -> Result<()> {
        if !self.print_pointers {
            return Ok(());
        }
        self.write(format!(" @ {:p}", ptr))
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
