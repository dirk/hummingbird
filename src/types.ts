import errors = require('./errors')

var inherits  = require('util').inherits,
    inspect   = require('util').inspect,
    TypeError = errors.TypeError

var _super = function (self) {
  return self.constructor.super_
}

// Base class for the type of any expression in Hummingbird.
class Type {
  name:       string
  supertype:  Type
  intrinsic:  boolean
  isRoot:     boolean
  properties: any

  constructor(supertype, isRoot = false) {
    if (!supertype) {
      throw new TypeError('Missing supertype')
    }
    // Whether or not the type is intrinsic to the language
    this.intrinsic  = false
    this.name       = this.constructor['name']
    this.supertype  = supertype
    // Maps property names (strings) to Types
    this.properties = {}
    // Whether or not the type is a root type in the type hierarchy
    this.isRoot     = isRoot ? true : false
  }
  getTypeOfProperty(name: string, fromNode = null): Type {
    var type = this.properties[name]
    if (type) { return type }
    throw new TypeError('Property not found on '+this.name+': '+name, fromNode)
  }
  setTypeOfProperty(name: string, type: Type): void {
    // TODO: Check that it's not overwriting properties
    this.properties[name] = type
  }
  equals(other: Type) { return false }
  toString(): string {
    if (this.inspect) { return this.inspect() }
    return this.constructor['name']
  }
  inspect(): string { return this.constructor['name'] }
}


// Instance of a type (ie. all variables are Instances)
class Instance {
  type: Type

  constructor(type) {
    this.type = type
  }
  getTypeOfProperty(name, fromNode) {
    return this.type.getTypeOfProperty(name, fromNode)
  }
  equals(other) {
    if (other.constructor !== Instance) { return false }
    return this.type.equals(other.type)
  }
  inspect() {
    return "'"+this.type.inspect()
  }
}


var Flags = {
  ReadOnly: 'r'
}

// Types ----------------------------------------------------------------------

class Object extends Type {
  name:            string
  intrinsic:       boolean
  primitive:       boolean
  propertiesFlags: any
  initializers:    any[]

  constructor(supertype) {
    super(supertype)
    this.intrinsic = true
    this.primitive = false
    // Flags about properties; maps name (string) to optional flags (string)
    //   r = read-only
    this.propertiesFlags = {}
    // List of initializers (Function) for the type
    this.initializers = []
  }
  getFlagsOfProperty(name) {
    var flags = this.propertiesFlags[name]
    return (flags ? flags : null)
  }
  setFlagsOfProperty(name, flags) {
    this.propertiesFlags[name] = flags
  }
  // Checks if a given property has a certain flag set
  hasPropertyFlag(name, flag) {
    var flags = this.getFlagsOfProperty(name)
    return (flags && flags.indexOf(flag) !== -1)
  }
  addInitializer(initFunction) {
    if (!(initFunction instanceof Function)) {
      throw new TypeError('Initializer must be a Function')
    }
    this.initializers.push(initFunction)
  }
  inspect() { return this.name }
}


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
Module.prototype.getChild = function (name) {
  return this.getTypeOfProperty(name)
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


class Function extends Type {
  args: Type[]
  ret:  Type
  isInstanceMethod: boolean
  // If this is an intrinsic instance method shimming for a module method,
  // this points to that module method to call in place of this shim.
  shimFor: any

  constructor(supertype, args, ret) {
    super(supertype)
    this.intrinsic        = true
    this.isInstanceMethod = false
    this.shimFor          = null
    // Types of arguments and return
    this.args = (args === undefined) ? [] : args
    this.ret  = (ret === undefined) ? null : ret
  }
  // Returns true if this function's arguments match the given array of
  // other arguments (`otherArgs`).
  argsMatch(otherArgs: Type[]): boolean {
    var args = this.args
    if (args.length !== otherArgs.length) {
      return false
    }
    var matching = new Array(args.length)
    for (var i = args.length - 1; i >= 0; i--) {
      var arg      = args[i],
          otherArg = otherArgs[i]
      matching[i] = arg.equals(otherArg)
    }
    if (!matching.every(isTrue)) { return false }
    return true
  }
  inspect() {
    var ret  = this.ret.inspect(),
        args = ''
    if (this.args.length > 0) {
      args = this.args.map(function (arg) { return arg.inspect() }).join(', ')
    }
    return 'Function('+args+') -> '+ret
  }
  equals(other) {
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
