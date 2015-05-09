var LLVM   = require('./library'),
    types  = require('../../types'),
    Errors = require('../../errors'),
    ICE    = Errors.InternalCompilerError

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
  // If it's external (ie. don't load)
  this.external = false
  // Whether or not this function has been defined
  this.defined = false
  // The function to call to define the function (if it hasn't been defined)
  this.definer = null
}
NativeFunction.prototype.computeType = function () {
  var args  = this.args.map(nativeTypeForType)
  if (this.ret instanceof types.Instance) {
    throw new ICE('Expected return type to be plain type; got instance')
  }
  var ret = null
  // Bit of a hack, but we need to wrap the return as an pointer
  // (ie. instance), however if the return is Void then we just
  // use the plain Void type.
  if (this.ret instanceof types.Void) {
    ret = nativeTypeForType(this.ret)
  } else {
    ret = nativeTypeForType(new types.Instance(this.ret))
  }
  this.type = new LLVM.FunctionType(ret, args, false)
}
NativeFunction.prototype.defineExternal = function () {
  if (this.defined) {
    throw new ICE('Cannot redefine external function')
  }
  this.external = true
  this.definer = function (ctx) {
    if (!ctx) {
      throw new ICE('Missing context for defining external function')
    }
    this.computeType()
    this.fn = NativeFunction.addExternalFunction(ctx, this.name, this.ret, this.args)
    this.defined = true
  }
}
NativeFunction.prototype.defineBody = function (ctx, cb) {
  if (!this.type) {
    this.computeType()
  }
  this.fn = ctx.module.addFunction(this.name, this.type)
  this.defined = true
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
NativeFunction.prototype.getPtr = function (ctx) {
  if (!this.defined) {
    if (!(this.definer instanceof Function)) {
      throw new ICE('Missing definer for not-yet-defined NativeFunction')
    }
    this.definer(ctx)
  }
  return this.fn.ptr
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

function computeType (ret, args, varArgs) {
  var args = args.map(nativeTypeForType),
      ret  = nativeTypeForType(ret)
  varArgs  = (varArgs ? true : false)
  return new LLVM.FunctionType(ret, args, varArgs)
}

NativeFunction.addExternalFunction = function (ctx, name, ret, args, varArgs) {
  var type = computeType(ret, args, varArgs)
  // Add the linkage to the module
  var externalFn = ctx.module.addFunction(name, type)
  return externalFn
}

module.exports = NativeFunction

