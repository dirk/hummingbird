var LLVM = require('../../../../llvm2')

var types = require('../../types')

// Manages native structures for Hummingbird type classes
function NativeObject (type) {
  this.type    = type
  this.defined = false
}
NativeObject.prototype.define = function (ctx) {
  if (this.defined) {
    console.error('Warning: NativeObject already defined')
    return
  }
  this.defined = true
  var name = this.type.name

  // Elements are a list of 3-tuples containing property index, name, and type
  var elements = []
  // Array of LLVM types to with which to construct our struct
  var structTypes = []

  var propKeys = Object.keys(this.type.properties)
  for (var i = 0; i < propKeys.length; i++) {
    var propName = propKeys[i],
        propType = this.type.properties[propName]
    // Add it to the elements
    elements.push([i, propName, propType])
    if (propType instanceof types.Function) {
      var nf = propType.getNativeFunction()
      // Type will be a pointer to the function
      var fnPtr = LLVM.Types.pointerType(nf.type.ptr)
      // Get the pointer to the LLVM type out of the NativeFunction
      structTypes.push(fnPtr)
    } else {
      throw new Error("Can't handle type of property: "+propType)
    }
  }
  // Convert the array of types into a pointer array for LLVM
  structTypes = new LLVM.RefTypes.TypeRefArray(structTypes)
  // Construct and save the type
  this.structType = LLVM.Library.LLVMStructType(structTypes, structTypes.length, false)
  // Also save the elements for future use
  this.elements   = elements
}
NativeObject.prototype.build = function (ctx, name) {
  name = (name ? name : '')
  return ctx.builder.buildAlloca(this.structType, name)
}
NativeObject.prototype.buildStructGEPForProperty = function (ctx, instanceValue, propertyName) {
  var el = null
  for (var i = 0; i < this.elements.length; i++) {
    if (this.elements[i][1] === propertyName) {
      el = this.elements[i]
    }
  }
  if (!el) {
    throw new Error("Property not found: "+propertyName)
  }
  var idx = el[0]
  return ctx.builder.buildStructGEP(instanceValue, idx, propertyName)
}

types.Object.prototype.setNativeObject = function (nf) {
  this.nativeObject = nf
}
types.Object.prototype.getNativeObject = function () {
  if (this.nativeObject) {
    return this.nativeObject
  }
  throw new Error('Native object not found for type: '+this.inspect())
}

module.exports = NativeObject

