var LLVM  = require('../../../../llvm2'),
    types = require('../../types')

var VoidType    = LLVM.Types.VoidType,
    Int64Type   = LLVM.Types.Int64Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

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
    case types.Number:
      return Int64Type
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
    default:
      throw new Error("Can't compute native type for Hummingbird type: "+type.constructor.name)
  }
}

module.exports = {nativeTypeForType: nativeTypeForType}

