pub type Reg = u8;

#[derive(Debug)]
pub enum Instruction {
    GetLocal(Reg, u8),
    GetLocalLexical(Reg, String),
    SetLocal(u8, Reg),
    SetLocalLexical(String, Reg),
    MakeFunction(Reg, u16),
    MakeInteger(Reg, i64),
    Branch(u8),
    Call(Reg, Reg, Vec<Reg>), // $1 = $2($3[])
    Return(Reg),
    ReturnNull,
}

#[derive(Debug)]
pub struct Unit {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub id: u16,
    pub name: String,
    pub registers: u8,
    pub basic_blocks: Vec<BasicBlock>,
    pub locals: u8,
    pub locals_names: Vec<String>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: u8,
    pub name: String,
    pub instructions: Vec<Instruction>,
}
