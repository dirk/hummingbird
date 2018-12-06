module bb.instructions;

import std.variant : Algebraic;

alias reg_t = ubyte;

struct GetLocal { reg_t lval; ubyte index; }
struct SetLocal { ubyte index; reg_t rval; }

alias Instruction = Algebraic!(
  GetLocal,
  SetLocal,
);

// A single compiled file to be evaluated.
struct Unit {
  // A constant unit is one which only has declarations at its top-level. This
  // means it can be (re)loaded just by copying: no evaluation required.
  bool constant;

  Function[] functions;
  // The "main" (like in C) function to evaluate the unit.
  Function* mainFunction;

  // TODO: Exports
}

struct Function {
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
