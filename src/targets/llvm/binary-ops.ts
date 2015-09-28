import errors = require('../../errors')
import types  = require('../../types')

import NativeFunction = require('./native-function')

var LLVM = require('./library'),
    ICE  = errors.InternalCompilerError

var Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var stringConcatFn: NativeFunction = null

export function initialize (ctx, rootScope) {
  var StringType         = rootScope.getLocal('String'),
      stdCoreTypesString = rootScope.get('std').getChild('core').getChild('types').getChild('string');

  stringConcatFn = stdCoreTypesString
    .getTypeOfProperty('concat')
    .getNativeFunction()
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
    return buildCallBuilder(stringConcatFn, 'concat')
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

export function getBuilder (op: string, lexprType: types.Type, rexprType: types.Type): Function {
  var ret = (function (): Function {
    switch (op) {
    case '+':
      return getAdditionBuilder(lexprType, rexprType)
    case '-':
      return getSubtractionBuilder(lexprType, rexprType)
    default:
      return null
    }
  })()

  if (ret) {
    return ret
  }

  var l = lexprType.inspect(),
      r = rexprType.inspect()
  throw new ICE('Binary op not found: '+l+' '+op+' '+r)
}

