use std::cell::Ref;
use std::io::{Result, Write};
use std::ops::Deref;

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
        writeln!(self.output, "{}() {{", function.name)?;
        writeln!(self.output, "  locals {{")?;
        for local in function.locals_names.iter() {
            writeln!(self.output, "    {}", local)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "  blocks {{")?;
        for basic_block in function.basic_blocks.iter() {
            self.print_basic_block(basic_block)?;
        }
        writeln!(self.output, "  }}")?;
        writeln!(self.output, "}}")
    }

    fn print_basic_block(&mut self, basic_block: &BasicBlock) -> Result<()> {
        writeln!(self.output, "    {}:", basic_block.name)?;
        for instruction in basic_block.instructions.iter() {
            self.print_instruction(&instruction)?;
        }
        Ok(())
    }

    fn print_instruction(&mut self, instruction: &Instruction) -> Result<()> {
        let formatted_instruction = match instruction {
            Instruction::GetLocal(lval, index) => format!("{} = GetLocal({})", reg(lval), index),
            Instruction::GetLocalLexical(lval, name) => {
                format!("{} = GetLocalLexical({})", reg(lval), name)
            }
            Instruction::SetLocal(index, rval) => format!("SetLocal({}, {})", index, reg(rval)),
            Instruction::MakeInteger(lval, value) => {
                format!("{} = MakeInteger({})", reg(lval), value)
            }
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
            _ => "Unknown".to_string(),
        };
        writeln!(self.output, "      {}", formatted_instruction)
    }
}

// Format a `SharedValue` into a pretty string (eg. "$1").
fn reg(reg: &Reg) -> String {
    format!("r{}", reg)
}
