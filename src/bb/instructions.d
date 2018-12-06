import std.variant : Algebraic;

struct GetLocal { ubyte index; }
struct SetLocal { ubyte index; }

alias Instruction = Algebraic!(
  GetLocal,
  SetLocal,
);

alias UnitFunction = Algebraic!(
  InlineFunction,
  Function,
);

// A single compiled file to be evaluated.
struct Unit {
  // A constant unit is one which only has declarations at its top-level. This
  // means it can be (re)loaded just by copying: no evaluation required.
  bool constant;

  // Number of local variables at the top-level scope of the unit.
  ubyte inlineLocals;
  string[] inlineLocalsNames;

  UnitFunction[] functions;
}

// Not actually a function: this is 
struct InlineFunction {
  BasicBlock[] basicBlocks;
}

struct Function {
  BasicBlock[] basicBlocks;
  ubyte locals;
  string[] localsNames;
}

struct BasicBlock {
  // It's index/identifier in the compilation unit.
  ubyte id;
  Instruction[] instructions;
}
