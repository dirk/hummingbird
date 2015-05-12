import util   = require('util')
import errors = require('./errors')
import AST    = require('./ast')

var inherits  = util.inherits,
    inspect   = util.inspect,
    TypeError = errors.TypeError

var SUPERTYPE_NONE = (new Date()).getTime()
type NoneType = number

// Base class for the type of any expression in Hummingbird.
export class Type {
  name:       string
  supertype:  Type
  intrinsic:  boolean
  primitive:  boolean
  isRoot:     boolean
  properties: any

  constructor(supertype: Type|NoneType, isRoot = false) {
    if (!supertype) {
      throw new TypeError('Missing supertype')
    }
    if (typeof supertype === 'number') {
      if (supertype === SUPERTYPE_NONE) {
        supertype = null
      } else {
        throw new TypeError('Invalid supertype')
      }
    }
    // Whether or not the type is intrinsic to the language
    this.intrinsic  = false
    this.name       = this.constructor['name']
    // Set the supertype; coercing it since we know it will be a Type or null
    this.supertype  = <Type>supertype
    // Maps property names (strings) to Types
    this.properties = {}
    // Whether or not the type is a root type in the type hierarchy
    this.isRoot     = isRoot ? true : false
  }
  getTypeOfProperty(name: string, fromNode = null): Type {
    if (!this.properties.hasOwnProperty(name)) {
      throw new TypeError('Property not found on '+this.name+': '+name, fromNode)
    }
    return this.properties[name]
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
export class Instance {
  type: Type

  constructor(type: Type) {
    this.type = type
  }
  getTypeOfProperty(name: string, fromNode = null) {
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


type TypeFlags = string

export var Flags = {
  ReadOnly: 'r'
}

// Types ----------------------------------------------------------------------

export class Object extends Type {
  propertiesFlags: any
  initializers:    any[]

  static createRootObject() {
    return new Object(SUPERTYPE_NONE)
  }

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
  getFlagsOfProperty(name: string): TypeFlags {
    var flags = this.propertiesFlags[name]
    return (flags ? flags : null)
  }
  setFlagsOfProperty(name: string, flags: TypeFlags): void {
    this.propertiesFlags[name] = flags
  }
  // Checks if a given property has a certain flag set
  hasPropertyFlag(name: string, flag: TypeFlags) {
    var flags = this.getFlagsOfProperty(name)
    return (flags && flags.indexOf(flag) !== -1)
  }
  addInitializer(initFunction: Function) {
    if (!(initFunction instanceof Function)) {
      throw new TypeError('Initializer must be a Function')
    }
    this.initializers.push(initFunction)
  }
  inspect() { return this.name }
}


// Modules have no supertype
export class Module extends Object {
  parent: Module

  constructor(name: string = null) {
    super(SUPERTYPE_NONE)
    this.intrinsic = true
    this.name      = name
    // Parent module (if present)
    this.parent    = null
  }
  setParent(parent: Module) {
    if (!(parent instanceof Module)) {
      throw new TypeError('Expected parent to be a Module')
    }
    this.parent = parent
  }
  addChild(child: Module) {
    if (!(child instanceof Module)) {
      throw new TypeError('Expected child to be a Module')
    }
    var childName = child.name
    this.setTypeOfProperty(childName, child)
  }
  getChild(name: string): Module {
    var child = this.getTypeOfProperty(name)
    if (child instanceof Module) {
      return child
    } else {
      throw new TypeError('Unexpected non-Module child: '+name)
    }
  }
  inspect() { return '.'+this.name }
}


export class Any extends Type {
  constructor() {
    super(SUPERTYPE_NONE)
    this.intrinsic = true
  }
  // Any always equals another type
  equals(other) { return true }
}

export class Void extends Type {
  constructor() {
    super(SUPERTYPE_NONE)
    this.intrinsic = true
  }
  equals(other) {
    // There should never be more than 1 instance of Void
    return this === other
  }
}


export class String extends Type {
  constructor(supertype) {
    super(supertype)
    this.intrinsic = true
    this.primitive = true
  }
  inspect() { return 'String' }
  equals(other) {
    // Check that they're both strings
    return (other.constructor === String)
  }
}


export class Integer extends Type {
  constructor(supertype) {
    super(supertype)
    this.intrinsic = true
    this.primitive = true
  }
  inspect() { return 'Integer' }
  equals(other) {
    return (other.constructor === Integer)
  }
}


export class Boolean extends Type {
  constructor(supertype) {
    super(supertype)
    this.intrinsic = true
    this.primitive = true
  }
  inspect() { return 'Boolean' }
  equals(other) {
    return (other.constructor === Boolean)
  }
}


export class Unknown extends Type {
  known: any

  constructor() {
    super(SUPERTYPE_NONE)
    this.intrinsic = true
    this.known     = null
  }
  inspect() { return 'Unknown' }
}


// Utility function used by the Function type
function isTrue (value) {
  return value === true
}


export class Function extends Type {
  args: Type[]
  ret:  Type
  isInstanceMethod: boolean
  // If this is an intrinsic instance method shimming for a module method,
  // this points to that module method to call in place of this shim.
  shimFor: any

  constructor(supertype, args?: Type[], ret?: Type) {
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

export class Multi extends Type {
  functionNodes:    AST.Function[]
  args:             Type[]
  ret:              Type
  isInstanceMethod: boolean = false

  constructor(supertype, args, ret) {
    super(supertype)
    this.args = args
    this.ret  = ret
    assertPresent(this, 'args')
    assertPresent(this, 'ret')
    // Set up an array to point to all the function implementations to which
    // we'll multiple dispatch
    this.functionNodes = []
  }
  addFunctionNode(functionNode: AST.Function) {
    this.functionNodes.push(functionNode)
  }
}

/*
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
*/

