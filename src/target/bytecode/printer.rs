use std::io::{Result, Write};

use super::layout::*;

pub struct Printer<O: Write> {
    output: O,
}

impl<O: Write> Printer<O> {
    pub fn new(output: O) -> Self {
        Self { output }
    }

    pub fn print_unit(&mut self, unit: &Unit) -> Result<()> {
        for function in unit.functions.iter() {
            self.print_function(function)?;
        }
        Ok(())
    }

    fn print_function(&mut self, function: &Function) -> Result<()> {
        writeln!(self.output, "{}() id({}) {{", function.name, function.id)?;
        writeln!(self.output, "  locals {{")?;
        for local in function.locals_names.iter() {
            writeln!(self.output, "    {}", local)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "  instructions {{")?;
        for (address, instruction) in function.instructions.iter().enumerate() {
            self.print_instruction(instruction, address)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "}}")
    }

    fn print_instruction(&mut self, instruction: &Instruction, address: usize) -> Result<()> {
        let formatted_instruction = match instruction {
            Instruction::GetLocal(lval, index) => format!("{} = GetLocal({})", reg(lval), index),
            Instruction::GetLocalLexical(lval, name) => {
                format!("{} = GetLocalLexical({})", reg(lval), name)
            }
            Instruction::SetLocal(index, rval) => format!("SetLocal({}, {})", index, reg(rval)),
            Instruction::MakeFunction(lval, id) => format!("MakeFunction({}, {})", reg(lval), id),
            Instruction::MakeInteger(lval, value) => {
                format!("{} = MakeInteger({})", reg(lval), value)
            }
            Instruction::Branch(destination) => format!("Branch({:04})", destination),
            Instruction::Call(lval, target, arguments) => format!(
                "{} = Call({}, [{}])",
                reg(lval),
                reg(target),
                arguments
                    .iter()
                    .map(|arg| reg(arg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instruction::Return(rval) => format!("Return({})", reg(rval)),
            Instruction::ReturnNull => "ReturnNull".to_string(),
            _ => "Unknown".to_string(),
        };
        writeln!(
            self.output,
            "      {:04} {}",
            address, formatted_instruction
        )
    }
}

// Format a `SharedValue` into a pretty string (eg. "$1").
fn reg(reg: &Reg) -> String {
    format!("r{}", reg)
}
