type Reg = u8;

enum Instruction {
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

struct Unit {
    functions: Vec<Function>,
}

struct Function {
    id: u16,
    name: String,
    registers: u8,
    basic_blocks: Vec<BasicBlock>,
    locals: u8,
    locals_names: Vec<String>,
}

struct BasicBlock {
    id: u8,
    name: String,
    instructions: Vec<Instruction>,
}
