

var inherits = require('util').inherits

var _super = function (self) {
  return self.constructor.super_
}

// Base class for the type of any expression in Hummingbird.
function Type () {
  // Whether or not the type is a root type of the language
  this.intrinsic = false
}
Type.prototype.equals = function (other) { return false }

function String () {
  _super(this).apply(this)
  this.intrinsic = true
}
inherits(String, Type)
String.prototype.toString = function () { return 'String' }


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


function Function (args, ret) {
  _super(this).apply(this)
  this.intrinsic = true
  // Types of arguments and return
  this.args = (args === undefined) ? [] : args
  this.ret  = (ret === undefined) ? null : ret
}
inherits(Function, Type)
Function.prototype.inspect = function () {
  return 'Function() -> ' + (this.ret ? this.ret.inspect() : 'Void')
}

module.exports = {
  Type: Type,
  String: String,
  Number: Number,
  Unknown: Unknown,
  Function: Function
}
