use std::cell::{Cell, RefCell};
use std::collections::HashSet;
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

    pub fn print_module(&self, module: Module) -> Result<()> {
        for (index, statement) in module.statements.iter().enumerate() {
            use ModuleStatement::*;
            match statement {
                Func(func) => self.print_func(func, index == 0)?,
            }
        }
        self.lnwrite("")?;
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
        let retrn = match &func.typ {
            Type::Func(func) => func.retrn.borrow(),
            other @ _ => unreachable!("Func node has non-Func type: {:?}", other),
        };
        self.write_type(&retrn, false)?;
        self.write(" ")?;
        match &func.body {
            FuncBody::Block(block) => self.print_block(block, false),
        }
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
            },
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
                    Var(var) => self.print_var(var, false)?,
                }
            }
            Ok(())
        })?;
        self.write("\n")?;
        self.iwrite("}")
    }

    fn print_expression(&self, expression: &Expression) -> Result<()> {
        use Expression::*;
        match expression {
            Closure(closure) => self.print_closure(closure),
            Identifier(identifier) => self.print_identifier(identifier),
            Infix(infix) => self.print_infix(infix),
            LiteralInt(literal) => self.lnwrite(format!("{}", literal.value)),
            PostfixCall(call) => self.print_postfix_call(call, 0).map(|_| ()),
            PostfixProperty(property) => self.print_postfix_property(property, 0).map(|_| ()),
        }
    }

    fn print_closure(&self, closure: &Closure) -> Result<()> {
        self.lnwrite("Closure(")?;
        if !closure.arguments.is_empty() {
            self.write("\n")?;
            self.indented(|this| {
                for argument in closure.arguments.iter() {
                    this.iwrite(format!("{}: ", argument.name))?;
                    this.write_type(&argument.typ, true)?;
                    this.write(",\n")?;
                }
                Ok(())
            })?;
        }
        self.iwrite("): ")?;
        let retrn = match &closure.typ {
            Type::Func(func) => func.retrn.borrow(),
            other @ _ => unreachable!("Closure node has non-Func type: {:?}", other),
        };
        self.write_type(&retrn, false)?;
        self.write(" ")?;
        match &*closure.body {
            ClosureBody::Block(block) => self.print_block(block, false),
            ClosureBody::Expression(expression) => self.indented(|this| {
                this.write("(")?;
                this.print_expression(expression)?;
                this.write(")")
            }),
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

    fn print_postfix_call(&self, call: &PostfixCall, current: u8) -> Result<u8> {
        let max = self.print_postfix_target(&*call.target, current)?;
        // Have a +1 so that we always indent at least one step.
        self.indented_steps(max - current + 1, |this1| {
            this1.lnwrite(format!("Call("))?;
            if !call.arguments.is_empty() {
                for argument in call.arguments.iter() {
                    this1.indented(|this2| {
                        this2.print_expression(argument)?;
                        this2.write(",")
                    })?;
                }
            }
            this1.write("): ")?;
            this1.write_type(&call.typ, false)
        })?;
        Ok(max)
    }

    fn print_postfix_property(&self, property: &PostfixProperty, current: u8) -> Result<u8> {
        let max = self.print_postfix_target(&*property.target, current)?;
        self.indented_steps(max - current + 1, |this| {
            this.lnwrite(format!("Property({}): ", property.property.name))?;
            this.write_type(&property.typ, false)
        })?;
        Ok(max)
    }

    fn print_postfix_target(&self, target: &Expression, current: u8) -> Result<u8> {
        match target {
            // Links in the chain
            Expression::PostfixCall(target) => self.print_postfix_call(target, current + 1),
            Expression::PostfixProperty(target) => self.print_postfix_property(target, current + 1),
            // Tail of the chain
            other @ _ => {
                self.print_expression(other)?;
                Ok(current)
            }
        }
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
