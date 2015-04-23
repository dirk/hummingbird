var LLVM  = require('../../../../llvm2'),
    types = require('../../types')

var VoidType    = LLVM.Types.VoidType,
    Int64Type   = LLVM.Types.Int64Type,
    Int8Type    = LLVM.Types.Int8Type,
    Int8PtrType = LLVM.Types.pointerType(Int8Type),
    Int1Type    = LLVM.Types.Int1Type

// Computes the equivalent native type for a given Hummingbird type
function nativeTypeForType (type) {
  switch (type.constructor) {
    case Buffer:
      // Already a pointer to a native type!
      return type
    case types.Void:
      return VoidType
    case types.String:
      return Int8PtrType
    case types.Integer:
      return Int64Type
    case types.Boolean:
      return Int1Type
    case types.Function:
      var nativeFunction = type.getNativeFunction()
      return nativeFunction.type.ptr
      // Just going to return a simple pointer for functions
      // return Int8PtrType
    case types.Object:
      var nativeObject = type.getNativeObject()
      return nativeObject.structType
    case types.Instance:
      var unboxed = type.type,
          unboxedNativeType = nativeTypeForType(unboxed)
      // Don't pointerify primitives
      if (unboxed.primitive === true) {
        return unboxedNativeType
      }
      return LLVM.Types.pointerType(unboxedNativeType)
    /* case types.Module:
      return VoidType */
    default:
      throw new Error("Can't compute native type for Hummingbird type: "+type.constructor.name)
  }
}

// Add native names for some types
types.Module.prototype.getNativeName = function () {
  var ret = 'M'+this.name
  if (this.parent) {
    ret = this.parent.getNativeName()+'_'+ret
  }
  return ret
}
types.String.prototype.getNativePrefix   = function () { return 'S' }
types.Function.prototype.getNativePrefix = function () { return 'F' }

module.exports = {nativeTypeForType: nativeTypeForType}

