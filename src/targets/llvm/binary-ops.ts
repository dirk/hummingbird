import errors = require('../../errors')
import types  = require('../../types')

import NativeFunction = require('./native-function')

var LLVM = require('./library'),
    ICE  = errors.InternalCompilerError

var Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var stdCoreTypesStringConcatFn = null

export function initialize (ctx, rootScope) {
  var StringType = rootScope.getLocal('String')
  stdCoreTypesStringConcatFn = new NativeFunction('Mstd_Mcore_Mtypes_Mstring_Fconcat', [StringType, StringType], StringType)
  stdCoreTypesStringConcatFn.defineExternal(ctx)
  // stdCoreTypesStringConcat = NativeFunction.addExternalFunction(ctx, 'Mstd_Mcore_Mtypes_Mstring_Fconcat', Int8PtrType, [Int8PtrType, Int8PtrType])
}

function assertRexprType (rexprType, type) {
  if (rexprType instanceof type) { return }
  var e = type.name,
      g = rexprType.constructor.name
  throw new ICE('Invalid type of right side of binary op (expected: '+e+', got: '+g+')')
}

function buildCallBuilder (fn, name) {
  return function (ctx, lvalue, rvalue) {
    return ctx.builder.buildCall(fn.getPtr(ctx), [lvalue, rvalue], name)
  }
}
function buildOpBuilder (opName, name) {
  return function (ctx, lvalue, rvalue) {
    return ctx.builder[opName](lvalue, rvalue, name)
  }
}

function getAdditionBuilder (lt: types.Type, rt: types.Type) {
  switch (lt.constructor) {
  case types.String:
    assertRexprType(rt, types.String)
    return buildCallBuilder(stdCoreTypesStringConcatFn, 'concat')
  // TODO: Only compile Integers!
  case types.Integer:
    assertRexprType(rt, types.Integer)
    return buildOpBuilder('buildAdd', 'add')
  }
}

function getSubtractionBuilder (lt: types.Type, rt: types.Type) {
  switch (lt.constructor) {
  case types.Integer:
    assertRexprType(rt, types.Integer)
    return buildOpBuilder('buildSub', 'sub')
  }
}

export function getBuilder (op: string, lexprType: types.Type, rexprType: types.Type) {
  var ret = (function () {
    switch (op) {
    case '+':
      return getAdditionBuilder(lexprType, rexprType)
    case '-':
      return getSubtractionBuilder(lexprType, rexprType)
    // default:
    //   throw new ICE('Binary op builder not found: '+op)
    }
  })()
  // If a builder was found then return it
  if (ret) {
    return ret
  }
  // Otherwise throw a compilation error
  var l = lexprType.inspect(),
      r = rexprType.inspect()
  throw new ICE('Binary op not found: '+l+' '+op+' '+r)
}

