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
    pushFrame(new Frame(top, 0, unit, &mainFunc));
    top.locals ~= new Value(println);
    top.localsNames ~= "println";
    run();
  }

  enum Action : ubyte {
    // Advance the instruction address in the current frame and then loop.
    ADVANCE,
    // Just loop.
    NO_ADVANCE,
    // The main function returned.
    EXIT,
  }

  // Implements the main run-loop of the virtual machine.
  void run() {
    while (true) {
      auto instruction = top.current();
      auto const action = dispatch(*instruction);
      if (action == Action.ADVANCE) {
        top.advance();
      } else if (action == Action.EXIT) {
        return;
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
        return Action.ADVANCE;
      },
      (GetLocalLexical getLocalLexical) {
        writeRegister(getLocalLexical.lval, top.getLocalLexical(getLocalLexical.name));
        return Action.ADVANCE;
      },
      (SetLocal setLocal) {
        top.setLocal(setLocal.index, readRegister(setLocal.rval));
        return Action.ADVANCE;
      },
      (SetLocalLexical setLocalLexical) {
        top.setLocalLexical(setLocalLexical.name, readRegister(setLocalLexical.rval));
        return Action.ADVANCE;
      },
      (MakeFunction makeFunction) {
        Function* callTarget = &top.unit.functions[makeFunction.id];
        writeRegister(makeFunction.lval, new Value(top.unit, callTarget, null));
        return Action.ADVANCE;
      },
      (MakeInteger makeInteger) {
        writeRegister(makeInteger.lval, new Value(makeInteger.value));
        return Action.ADVANCE;
      },
      (Branch branch) {
        top.branch(branch.id);
        return Action.NO_ADVANCE;
      },
      (Call call) {
        auto target = readRegister(call.target);
        auto arguments = new Value[call.arguments.length];
        foreach (index, argument; call.arguments) {
          arguments[index] = readRegister(argument);
        }
        if (target.isDynamicFunction()) {
          auto unit = target.dynamicFunctionValue.unit;
          auto callTarget = target.dynamicFunctionValue.callTarget;
          auto lexicalFrame = target.dynamicFunctionValue.lexicalFrame;
          auto frame = new Frame(top, call.lval, unit, callTarget);
          frame.lexicalParent = lexicalFrame;
          pushFrame(frame);
          return Action.NO_ADVANCE;
        } else if (target.isNativeFunction()) {
          auto callTarget = target.nativeFunctionValue.callTarget;
          auto lval = callTarget(arguments);
          writeRegister(call.lval, lval);
          return Action.ADVANCE;
        } else {
          throw new Error("Cannot call non-function value");
        }
      },
      (Return ret) {
        return doReturn(readRegister(ret.rval));
      },
      (ReturnNull _) {
        return doReturn(null);
      },
    );
  }

  pragma(inline):
  Action doReturn(Value value) {
    if (stack.length == 1) {
      // If we're at the top of the stack (ie. in the main function) then we
      // can't return anywhere, so just exit.
      return Action.EXIT;
    }
    auto poppedFrame = popFrame();
    writeRegister(poppedFrame.returnRegister, value);
    return Action.ADVANCE;
  }

  void pushFrame(Frame frame) {
    stack.length += 1;
    stack[$-1] = frame;
    top = frame;
  }

  Frame popFrame() {
    auto frame = stack[$-1];
    stack.length -= 1;
    top = stack[$-1];
    return frame;
  }
}
