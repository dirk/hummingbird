module vm.vm;

import std.stdio : writeln;
import std.variant : visit;

import target.bytecode.definitions;
import vm.frame;
import vm.value;

NativeCallTarget println = (Value[] args) {
  auto arg = args[0];
  if (arg is null) {
    writeln("null");
  } else if (arg.isInteger()) {
    writeln(arg.integerValue);
  } else {
    writeln(arg);
  }
  return null;
};

class VM {
  Frame[] stack;
  // The current execution frame at the top of the stack.
  Frame top;

  void runMain(Unit* unit) {
    auto mainFunc = unit.functions[0];
    pushFrame(new Frame(top, &mainFunc));
    top.locals ~= new Value(println);
    top.localsNames ~= "println";
    run();
  }

  // Returned to the run loop from the `dispatch` method. The loop then does
  // whatever the action tells it to do (advance to the next instruction,
  // branch, return, etc.).
  struct Action {
    enum : ubyte {
      ADVANCE,
      BRANCH,
      RETURN,
    }
    ubyte action;

    union Data {
      ubyte branchDestination;
      Value returnValue;

      this(ubyte branchDestination) {
        this.branchDestination = branchDestination;
      }

      this(Value returnValue) {
        this.returnValue = returnValue;
      }
    }
    Data data;

    pragma(inline):
    bool isAdvance() const {
      return action == ADVANCE;
    }

    pragma(inline):
    bool isBranch() const {
      return action == BRANCH;
    }

    pragma(inline):
    bool isReturn() const {
      return action == RETURN;
    }

    static Action advance() {
      return Action(ADVANCE, Data());
    }

    static Action branch(ubyte branchDestination) {
      return Action(BRANCH, Data(branchDestination));
    }

    static Action ret(Value returnValue) {
      return Action(RETURN, Data(returnValue));
    }
  }

  // Implements the main run-loop of the virtual machine.
  void run() {
    while (true) {
      auto instruction = top.current();
      auto action = dispatch(*instruction);
      if (action.isAdvance()) {
        top.advance();
      } else if (action.isBranch()) {
        top.branch(action.data.branchDestination);
      } else if (action.isReturn()) {
        if (stack.length > 1) {
          throw new Error("Cannot return dynamically");
        } else {
          // If we're returning from the top level.
          return;
        }
      }
    }
  }

  pragma(inline):
  Value readRegister(reg_t register) {
    if (register == 0) {
      throw new Error("Can't yet read null register");
    } else {
      return top.registers[register - 1];
    }
  }

  pragma(inline):
  void writeRegister(reg_t register, Value value) {
    if (register == 0) {
      // Null register.
      return;
    } else {
      top.registers[register - 1] = value;
    }
  }

  pragma(inline):
  Action dispatch(Instruction instruction) {
    return instruction.visit!(
      (GetLocal getLocal) {
        writeRegister(getLocal.lval, top.getLocal(getLocal.index));
        return Action.advance();
      },
      (GetLocalLexical getLocalLexical) {
        writeRegister(getLocalLexical.lval, top.getLocalLexical(getLocalLexical.name));
        return Action.advance();
      },
      (SetLocal setLocal) {
        top.setLocal(setLocal.index, readRegister(setLocal.rval));
        return Action.advance();
      },
      (SetLocalLexical setLocalLexical) {
        top.setLocalLexical(setLocalLexical.name, readRegister(setLocalLexical.rval));
        return Action.advance();
      },
      (MakeInteger makeInteger) {
        writeRegister(makeInteger.lval, new Value(makeInteger.value));
        return Action.advance();
      },
      (Branch branch) {
        return Action.branch(branch.id);
      },
      (Call call) {
        auto target = readRegister(call.target);
        auto arguments = new Value[call.arguments.length];
        foreach (index, argument; call.arguments) {
          arguments[index] = readRegister(argument);
        }
        Value lval;
        if (target.isDynamicFunction()) {
          throw new Error("Cannot call dynamically");
        } else if (target.isNativeFunction()) {
          auto callTarget = target.nativeFunctionValue.callTarget;
          lval = callTarget(arguments);
        } else {
          throw new Error("Cannot call non-function value");
        }
        writeRegister(call.lval, lval);
        return Action.advance();
      },
      (Return ret) {
        return Action.ret(readRegister(ret.rval));
      },
      (ReturnNull) {
        return Action.ret(null);
      },
    );
  }

  void pushFrame(Frame frame) {
    stack.length += 1;
    stack[$-1] = frame;
    top = frame;
  }
}
