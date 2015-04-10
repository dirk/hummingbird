var LLVM           = require('../../../../llvm2'),
    types          = require('../../types'),
    errors         = require('../../errors'),
    NativeFunction = require('./native-function'),
    ICE            = errors.InternalCompilerError

var Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var stdCoreTypesStringConcat = null

function initialize (ctx) {
  stdCoreTypesStringConcat = NativeFunction.addExternalFunction(ctx, 'Mstd_Mcore_Mtypes_Mstring_Fconcat', Int8PtrType, [Int8PtrType, Int8PtrType])
}

function assertRexprType (rexprType, type) {
  if (rexprType instanceof type) { return }
  var e = type.name,
      g = rexprType.constructor.name
  throw new ICE('Invalid type of right side of binary op (expected: '+e+', got: '+g+')')
}

function getAdditionBuilder (lexprType, rexprType) {
  switch (lexprType.constructor) {
  case types.String:
    assertRexprType(rexprType, types.String)
    return function (ctx, lvalue, rvalue) {
      return ctx.builder.buildCall(stdCoreTypesStringConcat, [lvalue, rvalue], 'concat')
    }
  // TODO: Only compile Integers!
  case types.Integer:
    assertRexprType(rexprType, types.Integer)
    return function (ctx, lvalue, rvalue) {
      return ctx.builder.buildAdd(lvalue, rvalue, 'add')
    }
  default:
    var l = lexprType.inspect(),
        o = op,
        r = rexprType.inspect()
    throw new ICE('Binary op not found: '+l+' '+o+' '+r)
  }
}

function getBuilder (op, lexprType, rexprType) {
  switch (op) {
    case '+':
      return getAdditionBuilder(lexprType, rexprType)
    default:
      throw new ICE('Binary op builder not found: '+op)
  }
}

module.exports = {
  initialize: initialize,
  getBuilder: getBuilder
}

