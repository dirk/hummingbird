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
        writeln!(self.output, "  bindings {{")?;
        for local in function.bindings.iter() {
            writeln!(self.output, "    {}", local)?;
        }
        writeln!(self.output, "  }}")?;
        if function.parent_bindings {
            writeln!(self.output, "  parent_bindings")?;
        }
        writeln!(self.output, "  instructions {{")?;
        for (address, instruction) in function.instructions.iter().enumerate() {
            self.print_instruction(instruction, address)?;
        }
        writeln!(self.output, "  }}")?;
        // writeln!(self.output, "  source_mappings {{")?;
        // for (address, span) in function.source_mappings.iter() {
        //     writeln!(self.output, "    {:04} {:?}", address, span)?;
        // }
        // writeln!(self.output, "  }}")?;
        writeln!(self.output, "}}")
    }

    fn print_instruction(&mut self, instruction: &Instruction, address: usize) -> Result<()> {
        let formatted_instruction = match instruction {
            Instruction::GetLocal(lval, index) => format!("{} = GetLocal({})", reg(lval), index),
            Instruction::GetLexical(lval, name) => format!("{} = GetLexical({})", reg(lval), name),
            Instruction::GetStatic(lval, name) => format!("{} = GetStatic({})", reg(lval), name),
            Instruction::SetLocal(index, rval) => format!("SetLocal({}, {})", index, reg(rval)),
            Instruction::SetLexical(name, rval) => format!("SetLexical({}, {})", name, reg(rval)),
            Instruction::SetStatic(name, rval) => format!("SetStatic({}, {})", name, reg(rval)),
            Instruction::MakeFunction(lval, id) => format!("MakeFunction({}, {})", reg(lval), id),
            Instruction::MakeInteger(lval, value) => {
                format!("{} = MakeInteger({})", reg(lval), value)
            }
            Instruction::MakeString(lval, value) => {
                format!("{} = MakeString({:?})", reg(lval), value)
            }
            Instruction::MakeSymbol(lval, symbol) => {
                format!("{} = MakeSymbol({:?})", reg(lval), symbol)
            }
            Instruction::OpAdd(lval, lhs, rhs) => {
                format!("{} = OpAdd({}, {})", reg(lval), reg(lhs), reg(rhs))
            }
            Instruction::OpLessThan(lval, lhs, rhs) => {
                format!("{} = OpLessThan({}, {})", reg(lval), reg(lhs), reg(rhs))
            }
            Instruction::OpProperty(lval, target, value) => {
                format!("{} = OpProperty({}, {})", reg(lval), reg(target), value)
            }
            Instruction::Branch(destination) => format!("Branch({:04})", destination),
            Instruction::BranchIf(destination, condition) => {
                format!("BranchIf({:04}, {})", destination, reg(condition))
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
            Instruction::Return(rval) => format!("Return({})", reg(rval)),
            Instruction::ReturnNull => "ReturnNull".to_string(),
            Instruction::Export(name, rval) => format!("Export({}, {})", name, reg(rval)),
            Instruction::Import(alias, name) => format!("Import(Static({}), {:?})", alias, name),
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
