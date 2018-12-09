module vm.frame;

import target.bytecode.definitions :
  BasicBlock,
  Function,
  Instruction;
import vm.value;

class Frame {
  const Function* func;

  Frame stackParent;
  // The frame that should be used for lexical (ie. closure) variable setting
  // and getting.
  Frame lexicalParent;
  Value[] registers;
  Value[] locals;
  string[] localsNames;

  BasicBlock* block;
  ubyte instructionAddress = 0;

  this() {
    func = null;
  }

  this(Frame stackParent, Function* func) {
    this.stackParent = stackParent;
    this.func = func;
    registers.length = func.registers;
    locals.length = func.locals;
    localsNames = func.localsNames.dup();
    block = &func.basicBlocks[0];
  }

  Value getLocal(ubyte index) {
    return locals[index];
  }

  Value getLocalLexical(string name) {
    foreach (index, localName; localsNames) {
      if (name == localName) {
        return locals[index];
      }
    }
    if (lexicalParent !is null) {
      return lexicalParent.getLocalLexical(name);
    }
    throw new Error("Name not found: " ~ name);
  }

  void setLocal(ubyte index, Value value) {
    locals[index] = value;
  }

  void setLocalLexical(string name, Value value) {
    foreach (index, localName; localsNames) {
      if (name == localName) {
        locals[index] = value;
      }
    }
    if (lexicalParent !is null) {
      return lexicalParent.setLocalLexical(name, value);
    }
    throw new Error("Name not found: " ~ name);
  }

  Instruction* current() {
    return &block.instructions[instructionAddress];
  }

  void advance() {
    instructionAddress += 1;
  }

  void branch(ubyte blockIndex) {
    // The function defining should remain constant, but the compiler isn't
    // smart enough to figure out that this is a non-mutating access.
    block = &(cast(Function*)func).basicBlocks[blockIndex];
    instructionAddress = 0;
  }

  bool root() {
    return (stackParent is null);
  }
}
