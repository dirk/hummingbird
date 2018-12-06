module bb.instructions;

import std.variant : Algebraic;

struct GetLocal { ubyte index; }
struct SetLocal { ubyte index; }

alias Instruction = Algebraic!(
  GetLocal,
  SetLocal,
);

alias UnitDeclarations = Algebraic!(
  // TODO: Class
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

  UnitDeclarations[] declarations;

  // TODO: Exports
}

// Not actually a function: these will be interpreted as the unit is loaded.
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
