use std::collections::HashSet;

pub type Reg = u8;

#[derive(Clone, Debug)]
pub enum Instruction {
    GetLocal(Reg, u8),
    GetLexical(Reg, String),
    GetStatic(Reg, String),
    SetLocal(u8, Reg),
    SetLexical(String, Reg),
    SetStatic(String, Reg),
    MakeFunction(Reg, u16),
    MakeInteger(Reg, i64),
    MakeString(Reg, String),
    OpAdd(Reg, Reg, Reg),         // $1 = $2 + $3
    OpLessThan(Reg, Reg, Reg),    // $1 = $2 < $3
    OpProperty(Reg, Reg, String), // $1 = $2.$3
    Branch(u8),
    BranchIf(u8, Reg),
    Call(Reg, Reg, Vec<Reg>), // $1 = $2($3[])
    Return(Reg),
    ReturnNull,
    // Export the val in $2 as the name $1.
    Export(String, Reg),
    // The first string is the alias for the import to be assigned to in the
    // module's static scope. The second string is the name/path of the file.
    Import(String, String), // static($1) = import($2)
}

#[derive(Clone, Debug)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub id: u16,
    pub name: String,
    pub registers: u8,
    pub instructions: Vec<Instruction>,
    pub locals: u8,
    pub locals_names: Vec<String>,
    pub bindings: HashSet<String>,
    pub parent_bindings: bool,
}
