module ir.compiler;

import std.algorithm : map;
import std.algorithm.mutation : remove;
import std.algorithm.searching : canFind, countUntil;
import std.array : array;
import std.stdio : writeln;
import std.variant : visit;
import std.typecons : Tuple;

import ir = ir.builder;
import ir.builder :
  BasicBlockBuilder,
  FunctionBuilder,
  UnitBuilder,
  Value;
import target.bytecode.instructions;

class UnitCompiler {
  UnitBuilder builder;

  this(UnitBuilder builder) {
    this.builder = builder;
  }

  Unit compile() {
    auto functions = builder.functions
      .map!(func => new FunctionCompiler(func).compile())
      .array();

    return Unit(
      false,
      functions,
    );
  }
}


class RegisterAllocator {
  struct Register {
    bool live;
    Value value;
  }

  Register[] registers;

  // Used for sanity checks.
  Value[] allocated;

  ir.InstructionAddress[][Value] liveDependencies;

  reg_t allocate(Value value) {
    // Allocation can only happen once (due values being SSA-form).
    assert(!allocated.canFind(value));
    allocated ~= value;

    // If no one is going to read us then we can allocate to the null register.
    if (value.dependencies.length == 0) {
      return 0;
    }

    // All values start off with their dependencies being live.
    liveDependencies[value] = value.dependencies.dup();

    // Find the first available register.
    foreach (index, ref register; registers) {
      if (!register.live) {
        register.live = true;
        register.value = value;
        return offsetRegister(index);
      }
    }

    registers ~= Register(true, value);
    return offsetRegister(cast(int)registers.length - 1);
  }

  reg_t use(Value value, BasicBlockBuilder block, ulong instruction) {
    auto dependencyIndex = liveDependencies[value].countUntil!(address => (
      address.block == block &&
      address.instruction == instruction
    ));
    // We have to have found the dependency.
    assert(dependencyIndex > -1);
    // Remove the dependency now that we've used it.
    liveDependencies[value] = liveDependencies[value].remove(dependencyIndex);

    auto registerIndex = registers.countUntil!(register => (register.live && register.value == value));
    // And we have to have found it in the registers.
    assert(registerIndex > -1);

    // If this was the last use then we can free the register.
    if (liveDependencies[value].length == 0) {
      registers[registerIndex].live = false;
      registers[registerIndex].value = null;
    }

    return offsetRegister(registerIndex);
  }

  // Applies the right "algorithm" to properly offest to leave r0 free as it's
  // the null register.
  private reg_t offsetRegister(ulong registerIndex) {
    return cast(reg_t)(registerIndex + 1);
  }
}

struct BasicBlockFinder {
  ulong[BasicBlockBuilder] blockIndices;

  this(BasicBlockBuilder[] builders) {
    foreach (index, basicBlock; builders) {
      blockIndices[basicBlock] = index;
    }
  }

  ubyte find(BasicBlockBuilder builder) {
    return cast(ubyte)blockIndices[builder];
  }
}

class FunctionCompiler {
  FunctionBuilder builder;

  this(FunctionBuilder builder) {
    this.builder = builder;
  }

  Function compile() {
    auto basicBlockFinder = BasicBlockFinder(builder.basicBlocks);
    auto registerAllocator = new RegisterAllocator();

    BasicBlock[] basicBlocks;
    foreach (basicBlock; builder.basicBlocks) {
      auto compiler = new BasicBlockCompiler(
        basicBlock,
        &basicBlockFinder,
        registerAllocator,
      );
      basicBlocks ~= compiler.compile();
    }

    return Function(
      builder.name,
      basicBlocks,
      cast(ubyte)builder.locals.length,
      builder.locals,
    );
  }
}

class BasicBlockCompiler {
  BasicBlockBuilder builder;
  BasicBlockFinder* basicBlockFinder;
  RegisterAllocator registerAllocator;

  ulong currentInstruction;

  this(
    BasicBlockBuilder builder,
    BasicBlockFinder* basicBlockFinder,
    RegisterAllocator registerAllocator,
  ) {
    this.builder = builder;
    this.basicBlockFinder = basicBlockFinder;
    this.registerAllocator = registerAllocator;
  }

  BasicBlock compile() {
    auto id = basicBlockFinder.find(builder);

    Instruction[] instructions;
    foreach (index, instruction; builder.instructions) {
      currentInstruction = index;
      instructions ~= compileInstruction(instruction);
    }

    return BasicBlock(
      id,
      builder.name,
      instructions,
    );
  }

  Instruction compileInstruction(ir.Instruction instruction) {
    return instruction.visit!(
      (ir.GetLocal getLocal) => wrap(
        GetLocal(allocate(getLocal.lval), getLocal.index),
      ),
      (ir.SetLocal setLocal) => wrap(
        SetLocal(setLocal.index, use(setLocal.rval)),
      ),
      (ir.SetLocalLexical setLocalLexical) => wrap(
        SetLocalLexical(setLocalLexical.name, use(setLocalLexical.rval)),
      ),
      (ir.MakeInteger makeInteger) => wrap(
        MakeInteger(allocate(makeInteger.lval), makeInteger.value),
      ),
      (ir.Branch branch) => wrap(
        Branch(basicBlockFinder.find(branch.destination)),
      ),
      (ir.Call call) => wrap(
        Call(
          allocate(call.lval),
          use(call.target),
          call.arguments.map!(value => use(value)).array(),
        ),
      ),
    );
  }

  private Instruction wrap(T)(T instruction) {
    return Instruction(instruction);
  }

  private reg_t allocate(Value value) {
    return registerAllocator.allocate(value);
  }

  private reg_t use(Value value) {
    return registerAllocator.use(value, builder, currentInstruction);
  }
}
