
var inherits = require('util').inherits,
    inspect  = require('util').inspect

var _super = function (self) {
  return self.constructor.super_
}

// Base class for the type of any expression in Hummingbird.
function Type (supertype, isRoot) {
  if (!supertype) {
    throw new TypeError('Missing supertype')
  }
  // Whether or not the type is intrinsic to the language
  this.intrinsic = false
  this.name      = this.constructor.name
  this.supertype = supertype
  // Whether or not the type is a root type in the type hierarchy
  this.isRoot    = isRoot ? true : false
}
Type.prototype.equals = function (other) { return false }
Type.prototype.toString = function () {
  if (this.inspect) { return this.inspect() }
  return this.constructor.name
}
Type.prototype.inspect = function () { return this.constructor.name }


// Instance of a type (ie. all variables are Instances)
function Instance (type) {
  this.type = type
}
Instance.prototype.getTypeOfProperty = function (name, fromNode) {
  return this.type.getTypeOfProperty(name, fromNode)
}
Instance.prototype.equals = function (other) {
  if (other.constructor !== Instance) { return false }
  return this.type.equals(other.type)
}
Instance.prototype.inspect = function () {
  return "'"+this.type.inspect()
}


var Flags = {
  ReadOnly: 'r'
}

// Types ----------------------------------------------------------------------

function Object (supertype) {
  Object.super_.call(this, supertype)
  this.intrinsic = true
  this.primitive = false
  // Maps property names (strings) to Types
  this.properties = {}
  // Flags about properties; maps name (string) to optional flags (string)
  //   r = read-only
  this.propertiesFlags = {}
  // List of initializers (Function) for the type
  this.initializers = []
}
inherits(Object, Type)
Object.prototype.getTypeOfProperty = function (name, fromNode) {
  var type = this.properties[name]
  if (type) { return type }
  throw new TypeError('Property not found on '+this.name+': '+name, fromNode)
}
Object.prototype.setTypeOfProperty = function (name, type) {
  // TODO: Check that it's not overwriting properties
  this.properties[name] = type
}
Object.prototype.getFlagsOfProperty = function (name) {
  var flags = this.propertiesFlags[name]
  return (flags ? flags : null)
}
Object.prototype.setFlagsOfProperty = function (name, flags) {
  this.propertiesFlags[name] = flags
}
// Checks if a given property has a certain flag set
Object.prototype.hasPropertyFlag = function (name, flag) {
  var flags = this.getFlagsOfProperty(name)
  return (flags && flags.indexOf(flag) !== -1)
}
Object.prototype.addInitializer = function (initFunction) {
  this.initializers.push(initFunction)
}
Object.prototype.inspect = function () { return this.name }


// Modules have no supertype
function Module (name) {
  _super(this).call(this, 'fake')
  this.intrinsic = true
  this.supertype = null
  this.name      = (name ? name : null)
  // Parent module (if present)
  this.parent    = null
}
inherits(Module, Object)
Module.prototype.setParent = function (parent) {
  if (!(parent instanceof Module)) {
    throw new TypeError('Expected parent to be a Module')
  }
  this.parent = parent
}
Module.prototype.addChild = function (child) {
  var childName = child.name
  this.setTypeOfProperty(childName, child)
}
Module.prototype.inspect = function () { return '.'+this.name }


function Any () {
  _super(this).call(this, 'fake')
  this.intrinsic = true
  this.supertype = null
}
inherits(Any, Type)
// Any always equals another type
Any.prototype.equals = function (other) { return true }


function Void () {
  _super(this).call(this, 'fake')
  this.intrinsic = true
  this.supertype = null
}
inherits(Void, Type)
Void.prototype.equals = function (other) {
  // There should never be more than 1 instance of Void
  return this === other
}


function String (supertype) {
  _super(this).call(this, supertype)
  this.intrinsic = true
  this.primitive = true
}
inherits(String, Type)
String.prototype.toString = function () { return 'String' }
String.prototype.inspect  = String.prototype.toString
String.prototype.equals   = function (other) {
  // Check that they're both strings
  return (other.constructor === String)
}


function Integer (supertype) {
  _super(this).call(this, supertype)
  this.intrinsic = true
  this.primitive = true
}
inherits(Integer, Type)
Integer.prototype.inspect = function () { return 'Integer' }
Integer.prototype.equals = function (other) {
  return (other.constructor === Integer)
}


function Boolean (supertype) {
  _super(this).call(this, supertype)
  this.intrinsic = true
  this.primitive = true
}
inherits(Boolean, Type)
Boolean.prototype.inspect = function () { return 'Boolean' }
Boolean.prototype.equals = function (other) {
  return (other.constructor === Boolean)
}


function Unknown () {
  _super(this).call(this, 'fake')
  this.intrinsic = true
  this.supertype = null
  this.known     = null
}
inherits(Unknown, Type)
Unknown.prototype.inspect = function () { return 'Unknown' }


// Utility function used by the Function type
function isTrue (value) {
  return value === true
}


function Function (supertype, args, ret) {
  _super(this).call(this, supertype)
  this.intrinsic        = true
  this.isInstanceMethod = false
  // Types of arguments and return
  this.args = (args === undefined) ? [] : args
  this.ret  = (ret === undefined) ? null : ret
}
inherits(Function, Type)
Function.prototype.inspect = function () {
  var ret  = this.ret.inspect(),
      args = ''
  if (this.args.length > 0) {
    args = this.args.map(function (arg) { return arg.inspect() }).join(', ')
  }
  return 'Function('+args+') -> '+ret
}
Function.prototype.equals = function (other) {
  // If they're not both functions then they're definitely not equal
  if (this.constructor !== other.constructor) {
    return false
  }
  // Args must be the same length
  if (this.args.length !== other.args.length) {
    return false
  }
  var args = new Array(this.args.length)
  // Compare types of their arguments
  for (var i = this.args.length - 1; i >= 0; i--) {
    var ta = this.args[i], oa = other.args[i]
    args[i] = ta.equals(oa)
  }
  if (!args.every(isTrue)) { return false }
  // Simple null return checks (should be able to deprecate this soon)
  if (this.ret === null || other.ret === null) { return this.ret === other.ret }
  // Finally compare types of their returns
  return this.ret.equals(other.ret)
}


function assertPresent (recv, prop) {
  var val = recv[prop]
  if (val) { return }
  throw new Error('Missing property '+prop)
}

function Multi (supertype, args, ret) {
  _super(this).call(this, supertype)
  this.args = args
  this.ret  = ret
  assertPresent(this, 'args')
  assertPresent(this, 'ret')
  // Set up an array to point to all the function implementations to which
  // we'll multiple dispatch
  this.functionNodes = []
}
inherits(Multi, Type)
Multi.prototype.addFunctionNode = function (functionNode) {
  this.functionNodes.push(functionNode)
}


module.exports = {
  Flags: Flags,
  Type: Type,
  Instance: Instance,
  Any: Any,
  Void: Void,
  Object: Object,
  Module: Module,
  String: String,
  Integer: Integer,
  Boolean: Boolean,
  Unknown: Unknown,
  Function: Function,
  Multi: Multi
}
