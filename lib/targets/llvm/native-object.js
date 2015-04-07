var LLVM = require('../../../../llvm2')

var types             = require('../../types'),
    nativeTypes       = require('./native-types'),
    nativeTypeForType = nativeTypes.nativeTypeForType,
    NativeFunction    = require('./native-function'),
    errors            = require('../../errors'),
    ICE               = errors.InternalCompilerError

// Manages native structures for Hummingbird type classes
function NativeObject (type) {
  this.type         = type
  this.defined      = false
  // Instances of LLVM.Function for each initializer function for the class
  this.initializers = []
  // Internal name will be computed when `define()` is called
  this.internalName = null
}
NativeObject.prototype.addInitializer = function (initFn) {
  if (!(initFn instanceof NativeFunction)) {
    throw new Error('Initializer must be a NativeFunction')
  }
  this.initializers.push(initFn)
}
NativeObject.prototype.define = function (ctx) {
  if (this.defined) {
    console.error('Warning: NativeObject already defined')
    return
  }
  this.defined = true
  var name          = this.type.name,
      globalContext = LLVM.Library.LLVMGetGlobalContext()

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
      structTypes.push(nativeTypeForType(propType))
    }
    // throw new Error("Can't handle type of property: "+propType)
  }
  // Convert the array of types into a pointer array for LLVM
  structTypes = new LLVM.RefTypes.TypeRefArray(structTypes)
  // Construct and save the type
  this.internalName = 'T'+name
  this.structType = LLVM.Library.LLVMStructCreateNamed(globalContext, this.internalName)
  LLVM.Library.LLVMStructSetBody(this.structType, structTypes, structTypes.length, false)
  // Also save the elements for future use
  this.elements = elements
}

NativeObject.prototype.build = function (ctx, name) {
  // TODO: Multiple dispatch initializers and all that jazz
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

