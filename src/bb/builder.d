module bb.builder;

import std.algorithm.mutation : move;
import std.algorithm.searching : canFind, countUntil;
import std.conv : to;
import std.stdio : writeln;
import std.typecons : Tuple;
import std.variant : Algebraic;

class UnitBuilder {
  string[] inlineLocals;

  FunctionBuilder[] functions;
  FunctionBuilder mainFunction;

  this() {
    this.functions ~= new FunctionBuilder("main");
    this.mainFunction = this.functions[$-1];
  }

  FunctionBuilder newFunction(string name) {
    return new FunctionBuilder(name);
  }

  string toPrettyString() {
    auto result = "unit:";
    foreach (func; functions) {
      result ~= "\n" ~ func.toPrettyString();
    }
    return result;
  }
}

class FunctionBuilder {
  string name;
  BasicBlockBuilder entry;
  BasicBlockBuilder current;
  BasicBlockBuilder[] basicBlocks;
  string[] locals;

  uint valueCounter = 1;

  this(string name) {
    this.name = name;
    this.basicBlocks ~= new BasicBlockBuilder("entry", this);
    this.entry = this.basicBlocks[$-1];
    this.current = this.basicBlocks[$-1];
  }

  ubyte getOrAddLocal(string local) {
    auto index = locals.countUntil(local);
    if (index > -1) {
      return cast(ubyte)index;
    }
    locals ~= local;
    return cast(ubyte)(locals.length - 1);
  }

  ubyte getLocal(string local) {
    auto index = locals.countUntil(local);
    if (index > -1) {
      return cast(ubyte)index;
    } else {
      throw new Error("Local not found: " ~ local);
    }
  }

  bool haveLocal(string local) {
    return locals.canFind(local);
  }

  BasicBlockBuilder newBlock() {
    auto name = "anonymous." ~ to!string(this.basicBlocks.length + 1);
    this.basicBlocks ~= new BasicBlockBuilder(name, this);
    this.current = this.basicBlocks[$-1];
    return this.current;
  }

  string toPrettyString() {
    auto result = name ~ "() {";
    foreach (basicBlock; basicBlocks) {
      result ~= "\n" ~ basicBlock.toPrettyString();
    }
    return result ~ "\n}";
  }

  Value nullValue() {
    return Value.NULL;
  }

  Value newValue() {
    auto value = new Value(valueCounter);
    valueCounter += 1;
    return value;
  }
}

alias InstructionAddress = Tuple!(BasicBlockBuilder, "block", int, "instruction");

class Value {
  private __gshared Value nullInstance;
  static Value NULL() {
    synchronized {
      if (nullInstance !is null) {
        nullInstance = new Value(0);
      }
    }
    return nullInstance;
  }

  uint id;
  // List of all the instructions that use this value.
  InstructionAddress[] dependencies;

  this(uint id) {
    this.id = id;
  }

  override string toString() const {
    return "$" ~ to!string(id);
  }

  // Call this to track an instruction that uses this value. This is critical
  // for fast register allocation.
  void usedBy(BasicBlockBuilder builder, int instruction) {
    auto dependency = InstructionAddress(builder, instruction);
    if (!dependencies.canFind(dependency)) {
      dependencies ~= dependency;
    }
  }
}

struct GetLocal {
  Value lval;
  ubyte index;

  string toString() const {
    return lval.toString() ~ " = GetLocal(" ~ to!string(index) ~ ")";
  }
}

struct SetLocal { ubyte index; Value rval; }

struct SetLocalLexical { string name; Value rval; }

struct MakeInteger {
  Value lval;
  long value;

  string toString() const {
    return lval.toString() ~ " = MakeInteger(" ~ to!string(value) ~ ")";
  }
}

struct Branch {
  BasicBlockBuilder destination;

  string toString() const {
    return "Branch(" ~ destination.name ~ ")";
  }
}

struct Call {
  Value lval;
  Value target;
  Value[] arguments;

  string toString() const {
    return lval.toString() ~ " = " ~ target.toString() ~ ".Call(" ~ to!string(arguments) ~ ")";
  }
}

alias Instruction = Algebraic!(
  GetLocal,
  SetLocal,
  SetLocalLexical,
  MakeInteger,
  Branch,
  Call,
);

class BasicBlockBuilder {
  FunctionBuilder parent;

  string name;
  Instruction[] instructions;

  this(string name, FunctionBuilder parent) {
    this.name = name;
    this.parent = parent;
  }

  Value buildGetLocal(ubyte index) {
    auto lval = parent.newValue();
    push(GetLocal(lval, index));
    return lval;
  }

  void buildSetLocal(ubyte index, Value rval) {
    pushAndTrack!SetLocal(index, rval);
  }

  void buildSetLocalLexical(string name, Value rval) {
    pushAndTrack!SetLocalLexical(name, rval);
  }

  Value buildMakeInteger(long value) {
    auto lval = parent.newValue();
    push(MakeInteger(lval, value));
    return lval;
  }

  void buildBranch(BasicBlockBuilder destination) {
    push(Branch(destination));
  }

  Value buildCall(Value target, Value[] arguments) {
    auto lval = parent.newValue();
    push(Call(lval, target, arguments));
    trackUse(target);
    trackUse(arguments);
    return lval;
  }

  // Must be called immediately after the instruction using the Value has been
  // pushed onto the instruction sequence.
  private void trackUse(T : Value)(T value) {
    auto index = (cast(int)instructions.length - 1);
    value.usedBy(this, index);
  }

  private void trackUse(T : Value[])(T values) {
    foreach (value; values) {
      trackUse(value);
    }
  }

  private void trackUse(T)(T) {
    return;
  }

  // All of the arguments will be scanned and automatically tracked as used.
  // Don't use this if the instruction being pushed "returns" an lval (if we
  // do we'll get a self-referential dependency).
  private void pushAndTrack(Type, Args...)(Args args) {
    instructions ~= Instruction(Type(args));
    static foreach (arg; args) {
      trackUse(arg);
    }
  }

  // You must manually call `trackUse` after calling this on any values which
  // are used by the instruction that was just pushed.
  private void push(Type)(Type instruction) {
    instructions ~= Instruction(instruction);
  }

  override string toString() const {
    return name;
  }

  string toPrettyString() {
    auto result = name ~ ":";
    foreach (instruction; instructions) {
      result ~= "\n  " ~ instruction.toString();
    }
    return result;
  }
}
