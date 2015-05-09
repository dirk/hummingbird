var LLVM           = require('./library'),
    types          = require('../../types'),
    errors         = require('../../errors'),
    NativeFunction = require('./native-function'),
    ICE            = errors.InternalCompilerError

var Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var stdCoreTypesStringConcatFn = null

function initialize (ctx, rootScope) {
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

function getAdditionBuilder (lt, rt) {
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

function getSubtractionBuilder (lt, rt) {
  switch (lt.constructor) {
  case types.Integer:
    assertRexprType(rt, types.Integer)
    return buildOpBuilder('buildSub', 'sub')
  }
}

function getBuilder (op, lexprType, rexprType) {
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

module.exports = {
  initialize: initialize,
  getBuilder: getBuilder
}

