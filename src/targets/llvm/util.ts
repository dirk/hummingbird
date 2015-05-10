import errors = require('../../errors')
import types  = require('../../types')

var LLVM = require('./library'),
    ICE  = errors.InternalCompilerError

var Int64Type   = LLVM.Types.Int64Type,
    Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(Int8Type),
    IntNE       = LLVM.Library.LLVMIntNE

var TypeOf            = LLVM.Library.LLVMTypeOf,
    PrintTypeToString = LLVM.Library.LLVMPrintTypeToString

export function isLastInstructionTerminator (bb: Buffer) {
  var lastInstr = LLVM.Library.LLVMGetLastInstruction(bb)
  // Do nothing if this block is empty
  if (lastInstr.isNull()) {
    return false
  }
  // Get the opcode and check if it's a terminator
  var lastInstrOpcode = LLVM.Library.LLVMGetInstructionOpcode(lastInstr)
  return (LLVM.Library.TerminatorInstructions.indexOf(lastInstrOpcode) !== -1)
}

export function assertInstanceOf (value, type, message?) {
  if (!(value instanceof type)) {
    if (!message) {
      message = 'Incorrect type; expected '+type.name+', got '+value.constructor.name
    }
    throw new Error(message)
  }
}

export function compileTruthyTest (compiler, blockCtx, expr) {
  var value    = compiler.compileExpression(expr, blockCtx),
      instance = expr.type
  // Can only truthy-test instances
  assertInstanceOf(instance, types.Instance)
  type = instance.type
  switch (type.constructor) {
  case types.String:
    var nullStringPtr = LLVM.Library.LLVMConstNull(Int8PtrType)
    // Compare the string pointer to the NULL pointer
    return compiler.ctx.builder.buildICmp(IntNE, value, nullStringPtr, '')
  case types.Integer:
    var zeroInteger = LLVM.Library.LLVMConstInt(Int64Type, 0, true)
    return compiler.ctx.builder.buildICmp(IntNE, value, zeroInteger, '')
  case types.Boolean:
    // Pre-condition check to make sure we really have an `i1`
    var type       = TypeOf(value),
        typeString = PrintTypeToString(type)
    if (typeString !== 'i1') {
      throw new ICE('Cannot compile Boolean to truthy-testable value (expected: i1, have: '+typeString+')')
    }
    return value
  default:
    throw new ICE('Cannot compile to truthy-testable value: '+type.constructor.name)
  }
}

