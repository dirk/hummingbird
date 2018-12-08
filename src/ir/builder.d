module ir.builder;

import std.algorithm : map;
import std.algorithm.mutation : move;
import std.algorithm.searching : canFind, countUntil;
import std.array : join;
import std.conv : to;
import std.format : format;
import std.stdio : writeln;
import std.string : leftJustify;
import std.typecons : Tuple;
import std.variant : Algebraic;

class UnitBuilder {
  string[] inlineLocals;

  FunctionBuilder[] functions;
  FunctionBuilder mainFunction;

  this() {
    this.mainFunction = new FunctionBuilder("main");
    this.functions = [this.mainFunction];
  }

  FunctionBuilder newFunction(string name) {
    this.functions ~= new FunctionBuilder(name);
    return this.functions[$-1];
  }

  string toPrettyString() {
    auto result = "";
    foreach (index, func; functions) {
      if (index > 0) {
        result ~= "\n";
      }
      result ~= func.toPrettyString();
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

  Value[] values;
  uint instructionCounter = 1;

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

  // TODO: Define printer in a separate module.
  string toPrettyString() {
    auto result = name ~ "() {";
    result ~= "\n  blocks {";
    foreach (basicBlock; basicBlocks) {
      result ~= "\n" ~ basicBlock.toPrettyString();
    }
    result ~= "\n  }";
    if (values.length > 0) {
      result ~= "\n  values {";
      foreach (value; values) {
        if (value.dependencies.length > 0) {
          auto dependencies = value.dependencies
            .map!(dependency => format!"%04d"(dependency))
            .join(" ");
          result ~= format!"\n    %s -> %s"(
            value.toString().leftJustify(5),
            dependencies,
          );
        } else {
          result ~= "\n    " ~ value.toString();
        }
      }
      result ~= "\n  }";
    }
    return result ~ "\n}";
  }

  Value nullValue() {
    return Value.NULL;
  }

  Value newValue() {
    auto id = cast(uint)(values.length + 1);
    values ~= new Value(id);
    return values[$-1];
  }

  Address nextAddress() {
    auto address = instructionCounter;
    instructionCounter += 1;
    return address;
  }
}

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
  Address[] dependencies;

  this(uint id) {
    this.id = id;
  }

  override string toString() const {
    return "$" ~ to!string(id);
  }

  bool isNull() const {
    return id == 0;
  }

  // Call this to track an instruction that uses this value. This is critical
  // for fast register allocation.
  void usedBy(Address address) {
    if (isNull()) {
      throw new Error("Null Value cannot be used");
    }
    if (!dependencies.canFind(address)) {
      dependencies ~= address;
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

alias Address = uint;

struct AddressedInstruction {
  Address address;
  Instruction instruction;

  string toString() {
    return format!"%04d %s"(
      address,
      instruction.toString(),
    );
  }
}

class BasicBlockBuilder {
  FunctionBuilder parent;

  string name;
  AddressedInstruction[] instructions;

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
    auto addressedInstruction = instructions[$-1];
    value.usedBy(addressedInstruction.address);
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
    push(Type(args));
    static foreach (arg; args) {
      trackUse(arg);
    }
  }

  // You must manually call `trackUse` after calling this on any values which
  // are used by the instruction that was just pushed.
  private void push(Type)(Type instruction) {
    auto address = parent.nextAddress();
    instructions ~= AddressedInstruction(address, Instruction(instruction));
  }

  override string toString() const {
    return name;
  }

  string toPrettyString() {
    auto result = "    " ~ name ~ ":";
    foreach (instruction; instructions) {
      result ~= "\n      " ~ instruction.toString();
    }
    return result;
  }
}
