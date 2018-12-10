module vm.value;

import target.bytecode.definitions : Function, Unit;
import vm.frame : Frame;

struct DynamicFunction {
  Unit* unit;
  Function* callTarget;
  // The lexical scope in which this function was declared.
  Frame lexicalFrame;
}

alias NativeCallTarget = Value function(Value[]);

struct NativeFunction {
  NativeCallTarget callTarget;
}

class Value {
  enum Type {
    DYNAMIC_FUNCTION,
    NATIVE_FUNCTION,
    INTEGER,
  }
  Type type;

  union {
    long integerValue;
    DynamicFunction dynamicFunctionValue;
    NativeFunction nativeFunctionValue;
  }

  this(long value) {
    type = Type.INTEGER;
    integerValue = value;
  }

  this(Unit* unit, Function* callTarget, Frame lexicalFrame) {
    type = Type.DYNAMIC_FUNCTION;
    dynamicFunctionValue = DynamicFunction(unit, callTarget, lexicalFrame);
  }

  this(NativeCallTarget callTarget) {
    type = Type.NATIVE_FUNCTION;
    nativeFunctionValue = NativeFunction(callTarget);
  }

  bool isInteger() {
    return (type == Type.INTEGER);
  }

  pragma(inline):
  bool isNativeFunction() {
    return (type == Type.NATIVE_FUNCTION);
  }

  pragma(inline):
  bool isDynamicFunction() {
    return (type == Type.DYNAMIC_FUNCTION);
  }
}
