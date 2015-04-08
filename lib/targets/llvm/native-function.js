var LLVM  = require('../../../../llvm2'),
    types = require('../../types')

var VoidType    = LLVM.Types.VoidType,
    Int64Type   = LLVM.Types.Int64Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var nativeTypes       = require('./native-types'),
    nativeTypeForType = nativeTypes.nativeTypeForType

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
  // Get the previous entry so we can restore it
  var builderPtr    = ctx.builder.ptr,
      previousEntry = LLVM.Library.LLVMGetInsertBlock(builderPtr)
  // Setup the entry block and position the builder in it
  var entry = this.fn.appendBasicBlock('entry')
  ctx.builder.positionAtEnd(entry)
  // Call the callback with the builder and entry block
  cb.call(this, entry)
  // Restore the previous entry point
  ctx.builder.positionAtEnd(previousEntry)
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
types.Function.prototype.hasNativeFunction = function () {
  return (this.nativeFunction ? true : false)
}

module.exports = NativeFunction

