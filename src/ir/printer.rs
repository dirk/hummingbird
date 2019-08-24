use std::cell::Ref;
use std::io::{Result, Write};

use super::layout::*;

pub struct Printer<O: Write> {
    output: O,
}

impl<O: Write> Printer<O> {
    pub fn new(output: O) -> Self {
        Self { output }
    }

    pub fn print_module(&mut self, unit: &Module) -> Result<()> {
        writeln!(self.output, "imports {{")?;
        for (name, import) in unit.imports.iter() {
            writeln!(self.output, "  {} <- {:?}", name, import)?;
        }
        writeln!(self.output, "}}")?;
        for function in unit.functions.iter() {
            self.print_function(function.borrow())?;
        }
        Ok(())
    }

    fn print_function(&mut self, function: Ref<Function>) -> Result<()> {
        writeln!(self.output, "{}() {{", function.name)?;
        writeln!(self.output, "  locals {{")?;
        for local in function.locals.iter() {
            writeln!(self.output, "    {}", local)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "  bindings {{")?;
        for binding in function.bindings.iter() {
            writeln!(self.output, "    {}", binding)?;
        }
        writeln!(self.output, "  }}")?;
        if function.parent_bindings {
            writeln!(self.output, "  parent_bindings")?;
        }
        writeln!(self.output, "  blocks {{")?;
        for basic_block in function.basic_blocks.iter() {
            let basic_block = basic_block.borrow();
            self.print_basic_block(basic_block)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "  values {{")?;
        for value in function.values.iter() {
            let formatted_value = id(value);
            let value = value.borrow();
            let formatted_dependents = if value.dependents.len() > 0 {
                format!(
                    " -> {}",
                    value
                        .dependents
                        .iter()
                        .map(|address| format!("{:04}", address))
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            } else {
                "".to_string()
            };
            writeln!(
                self.output,
                "    {}{}",
                formatted_value, formatted_dependents
            )?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "}}")
    }

    fn print_basic_block(&mut self, basic_block: Ref<BasicBlock>) -> Result<()> {
        writeln!(self.output, "    {}:", basic_block.name)?;
        for instruction in basic_block.instructions.iter() {
            self.print_instruction(&instruction)?;
        }
        Ok(())
    }

    fn print_instruction(&mut self, instruction: &(Address, Instruction)) -> Result<()> {
        let address = instruction.0;
        let instruction = &instruction.1;
        let formatted_instruction = match instruction {
            Instruction::Get(lval, slot) => format!("{} = Get({:?})", id(lval), slot),
            Instruction::Set(slot, rval) => format!("SetLocal({:?}, {})", slot, id(rval)),
            Instruction::MakeFunction(lval, function) => {
                format!("MakeFunction({}, {})", id(lval), function.borrow().name)
            }
            Instruction::MakeInteger(lval, value) => {
                format!("{} = MakeInteger({})", id(lval), value)
            }
            Instruction::Branch(block) => format!("Branch({})", block.borrow().name),
            Instruction::Call(lval, target, arguments) => format!(
                "{} = Call({}, [{}])",
                id(lval),
                id(target),
                arguments
                    .iter()
                    .map(|arg| id(arg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instruction::Return(rval) => format!("Return({})", id(rval),),
            Instruction::ReturnNull => "ReturnNull".to_string(),
        };
        writeln!(
            self.output,
            "      {:04} {}",
            address, formatted_instruction
        )
    }
}

// Format a `SharedValue` into a pretty string (eg. "$1").
fn id(value: &SharedValue) -> String {
    format!("${}", value.borrow().id)
}
