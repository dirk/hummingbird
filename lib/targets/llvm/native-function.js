var LLVM  = require('../../../../llvm2'),
    types = require('../../types')

var VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

// Computes the equivalent native type for a given Hummingbird type
function nativeTypeForType (type) {
  switch (type.constructor) {
    case types.Void:
      return VoidType
    case types.String:
      return Int8PtrType
    default:
      throw new Error("Can't compute native type for Hummingbird type: "+type.constructor.name)
  }
}

function NativeFunction (name, args, ret) {
  this.name = name
  this.args = args
  this.ret  = ret
  // LLVM variable set during definition/compilation
  this.type = null
  this.fn   = null
}
NativeFunction.prototype.computeType = function () {
  var args = this.args.map(nativeTypeForType)
  var ret  = nativeTypeForType(this.ret)
  this.type = new LLVM.FunctionType(ret, args, false)
}
NativeFunction.prototype.defineBody = function (ctx, cb) {
  this.computeType()
  this.fn = ctx.module.addFunction(this.name, this.type)
  // Setup the entry block and position the builder in it
  var entry = this.fn.appendBasicBlock('entry')
  ctx.builder.positionAtEnd(entry)
  // Call the callback with the builder and entry block
  cb.call(this, entry)
}

types.Function.prototype.setNativeFunction = function (nf) {
  this.nativeFunction = nf
}
types.Function.prototype.getNativeFunction = function () {
  if (this.nativeFunction) {
    return this.nativeFunction
  }
  throw new Error('Native function not found for type: '+this.inspect())
}

module.exports = NativeFunction

