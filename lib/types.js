
var inherits = require('util').inherits,
    inspect  = require('util').inspect

var _super = function (self) {
  return self.constructor.super_
}

// Base class for the type of any expression in Hummingbird.
function Type () {
  // Whether or not the type is a root type of the language
  this.intrinsic = false
}
Type.prototype.equals = function (other) { return false }
Type.prototype.toString = function () {
  if (this.inspect) { return this.inspect() }
  return this.constructor.name
}

function String () {
  _super(this).apply(this)
  this.intrinsic = true
}
inherits(String, Type)
String.prototype.toString = function () { return 'String' }
String.prototype.inspect  = String.prototype.toString
String.prototype.equals   = function (other) {
  // Check that they're both strings
  return (other.constructor === String)
}


function Number () {
  _super(this).apply(this)
  this.intrinsic = true
}
inherits(Number, Type)
Number.prototype.inspect = function () { return 'Number' }
Number.prototype.equals = function (other) {
  return (other.constructor === Number)
}


function Unknown () {
  _super(this).apply(this)
  this.intrinsic = true
  this.known     = null
}
inherits(Unknown, Type)
Unknown.prototype.inspect = function () { return 'Unknown' }


// Utility function used by the Function type
function isTrue (value) {
  return value === true
}


function Function (args, ret) {
  _super(this).apply(this)
  this.intrinsic = true
  // Types of arguments and return
  this.args = (args === undefined) ? [] : args
  this.ret  = (ret === undefined) ? null : ret
}
inherits(Function, Type)
Function.prototype.inspect = function () {
  var ret = (this.ret ? this.ret.inspect() : 'Void')
  var args = ''
  if (this.args.length > 0) {
    args = this.args.map(function (arg) { return arg.inspect() }).join(', ')
  }
  return 'Function('+args+') -> '+ret
}
Function.prototype.equals = function (other) {
  // If they're not both functions then they're definitely not equal
  if (this.prototype !== other.prototype) {
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
  // Finally compare types of their returns
  return this.ret.equals(other.ret)
}

module.exports = {
  Type: Type,
  String: String,
  Number: Number,
  Unknown: Unknown,
  Function: Function
}
