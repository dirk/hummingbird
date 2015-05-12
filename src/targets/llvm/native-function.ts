
import types  = require('../../types')
import Errors = require('../../errors')

var LLVM        = require('./library'),
    nativeTypes = require('./native-types')

var ICE               = Errors.InternalCompilerError,
    VoidType          = LLVM.Types.VoidType,
    Int64Type         = LLVM.Types.Int64Type,
    Int8PtrType       = LLVM.Types.pointerType(LLVM.Types.Int8Type),
    nativeTypeForType = nativeTypes.nativeTypeForType

class NativeFunction {
  name: string
  args: types.Type[]
  ret:  types.Type
  // LLVM variable set during definition/compilation
  type: any = null
  fn:   any = null
  // If it's external (ie. don't load)
  external: boolean = false
  // Whether or not this function has been defined
  defined: boolean = false
  // The function to call to define the function (if it hasn't been defined)
  definer: any = null

  constructor(name, args, ret) {
    this.name = name
    this.args = args
    this.ret  = ret
  }

  computeType() {
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

  defineExternal() {
    var self = this
    if (this.defined) {
      throw new ICE('Cannot redefine external function')
    }
    this.external = true
    this.definer = function (ctx) {
      if (!ctx) {
        throw new ICE('Missing context for defining external function')
      }
      self.computeType()
      self.fn = NativeFunction.addExternalFunction(ctx, self.name, self.ret, self.args)
      self.defined = true
    }
  }
  defineBody(ctx, cb: (entry: Buffer) => void) {
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
  getPtr(ctx): Buffer {
    if (!this.defined) {
      if (!(this.definer instanceof Function)) {
        throw new ICE('Missing definer for not-yet-defined NativeFunction')
      }
      this.definer(ctx)
    }
    return this.fn.ptr
  }

  static addExternalFunction(ctx, name, ret, args, varArgs: boolean = false) {
    var type = computeType(ret, args, varArgs)
    // Add the linkage to the module
    var externalFn = ctx.module.addFunction(name, type)
    return externalFn
  }
}// NativeFunction

types.Function.prototype['setNativeFunction'] = function (nf) {
  this.nativeFunction = nf
}
types.Function.prototype['getNativeFunction'] = function () {
  if (this.nativeFunction) {
    return this.nativeFunction
  }
  throw new Error('Native function not found for type: '+this.inspect())
}
types.Function.prototype['hasNativeFunction'] = function () {
  return (this.nativeFunction ? true : false)
}

function computeType (retType: types.Type, argTypes: types.Type[], varArgs: boolean) {
  var args        = <Buffer[]>argTypes.map(nativeTypeForType),
      ret: Buffer = nativeTypeForType(retType)
  varArgs = (varArgs ? true : false)
  return new LLVM.FunctionType(ret, args, varArgs)
}

export = NativeFunction

