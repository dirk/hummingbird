module target.bytecode.printer;

import std.algorithm.iteration : map;
import std.array : join;
import std.format : format;
import std.stdio : writeln, writefln;
import std.variant : visit;

import target.bytecode.definitions;

class UnitPrinter {
  static void print(immutable Unit unit) {
    foreach (func; unit.functions) {
      FunctionPrinter.print(func);
    }
  }
}

class FunctionPrinter {
  static void print(immutable Function func) {
    writefln!"%s() id(%d) registers(%d) {"(func.name, func.id, func.registers);
    foreach (basicBlock; func.basicBlocks) {
      BasicBlockPrinter.print(basicBlock);
    }
    writeln("}");
  }
}

class BasicBlockPrinter {
  static void print(immutable BasicBlock basicBlock) {
    writeln("  " ~ format!"%s(@%d)"(basicBlock.name, basicBlock.id) ~ ":");
    foreach (instruction; basicBlock.instructions) {
      InstructionPrinter.print(instruction);
    }
  }
}

class InstructionPrinter {
  static void print(Instruction instruction) {
    string value = instruction.visit!(
      (GetLocal getLocal) => format!"r%d = GetLocal #%d"(getLocal.lval, getLocal.index),
      (GetLocalLexical getLocalLexical) => format!"r%d = GetLocalLexical \"%s\""(
        getLocalLexical.lval,
        getLocalLexical.name,
      ),
      (SetLocal setLocal) => format!"SetLocal #%d r%d"(setLocal.index, setLocal.rval),
      (SetLocalLexical setLocalLexical) => format!"SetLocalLexical \"%s\" r%d"(
        setLocalLexical.name,
        setLocalLexical.rval,
      ),
      (MakeFunction makeFunction) => format!"r%d = MakeFunction id(%d)"(makeFunction.lval, makeFunction.id),
      (MakeInteger makeInteger) => format!"r%d = MakeInteger %d"(makeInteger.lval, makeInteger.value),
      (Branch branch) => format!"Branch @%d"(branch.id),
      (Call call) => format!"r%d = Call r%d [%s]"(
        call.lval,
        call.target,
        call.arguments.map!(argument => format!"r%d"(argument)).join(" "),
      ),
      (Return ret) => format!"Return r%d"(ret.rval),
      (ReturnNull) => "ReturnNull",
    );
    writeln("    " ~ value);
  }
}
