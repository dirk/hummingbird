module bb.builder;

import std.algorithm.mutation : move;
import std.algorithm.searching : canFind, countUntil;
import std.conv : to;
import std.variant : Algebraic;

class UnitBuilder {
  string[] inlineLocals;

  FunctionBuilder[] functions;
  FunctionBuilder* mainFunction;

  this() {
    this.functions ~= new FunctionBuilder("main");
    this.mainFunction = &this.functions[$-1];
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
  BasicBlockBuilder* entry;
  BasicBlockBuilder* current;
  BasicBlockBuilder[] basicBlocks;
  string[] locals;

  uint valueCounter = 1;

  this(string name) {
    this.name = name;
    this.basicBlocks ~= new BasicBlockBuilder("entry", this);
    this.entry = &this.basicBlocks[$-1];
    this.current = &this.basicBlocks[$-1];
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

  BasicBlockBuilder* newBlock() {
    auto name = "anonymous." ~ to!string(this.basicBlocks.length + 1);
    this.basicBlocks ~= new BasicBlockBuilder(name, this);
    this.current = &this.basicBlocks[$-1];
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
    return Value(0);
  }

  Value newValue() {
    auto value = Value(valueCounter);
    valueCounter += 1;
    return value;
  }
}

struct Value {
  uint id;

  string toString() {
    return "$" ~ to!string(id);
  }
}

struct GetLocal {
  Value lval;
  ubyte index;

  string toString() {
    return lval.toString() ~ " = GetLocal(" ~ to!string(index) ~ ")";
  }
}

struct SetLocal { ubyte index; Value rval; }

struct SetLocalLexical { string name; Value rval; }

struct MakeInteger {
  Value lval;
  long value;

  string toString() {
    return lval.toString() ~ " = MakeInteger(" ~ to!string(value) ~ ")";
  }
}

struct Branch {
  BasicBlockBuilder* destination;

  string toString() {
    return "Branch(" ~ destination.name ~ ")";
  }
}

struct Call {
  Value lval;
  Value target;
  Value[] arguments;

  string toString() {
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
    push(SetLocal(index, rval));
  }

  void buildSetLocalLexical(string name, Value rval) {
    push(SetLocalLexical(name, rval));
  }

  Value buildMakeInteger(long value) {
    auto lval = parent.newValue();
    push(MakeInteger(lval, value));
    return lval;
  }

  void buildBranch(BasicBlockBuilder* destination) {
    push(Branch(destination));
  }

  Value buildCall(Value target, Value[] arguments) {
    auto lval = parent.newValue();
    push(Call(lval, target, arguments));
    return lval;
  }

  private void push(T)(T instruction) {
    instructions ~= Instruction(instruction);
  }

  override string toString() {
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
