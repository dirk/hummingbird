module target.bytecode.definitions;

import std.variant : Algebraic;

alias reg_t = ubyte;

struct GetLocal { reg_t lval; ubyte index; }
struct SetLocal { ubyte index; reg_t rval; }
struct SetLocalLexical { string name; reg_t rval; }
struct MakeInteger { reg_t lval; long value; }
struct Branch { ubyte id; }
struct Call { reg_t lval; reg_t target; reg_t[] arguments; }

alias Instruction = Algebraic!(
  GetLocal,
  SetLocal,
  SetLocalLexical,
  MakeInteger,
  Branch,
  Call,
);

// A single compiled file to be evaluated.
struct Unit {
  // A constant unit is one which only has declarations at its top-level. This
  // means it can be (re)loaded just by copying: no evaluation required.
  bool constant;

  // The "main" function to evaluate the unit must be the first function.
  Function[] functions;

  // TODO: Exports
}

struct Function {
  string name;
  BasicBlock[] basicBlocks;
  ubyte locals;
  string[] localsNames;
}

struct BasicBlock {
  // It's index/identifier in the compilation unit.
  ubyte id;
  // A human-friendly name of the block.
  string name;
  Instruction[] instructions;
}
