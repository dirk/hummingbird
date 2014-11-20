

var inherits = require('util').inherits

var _super = function (self) {
  return self.constructor.super_
}

// Base class for the type of any expression in Hummingbird.
var Type = function () {
  // Whether or not the type is a root type of the language
  this.intrinsic = false
}

var String = function () {
  _super(this).apply(this)
  this.intrinsic = true
}
inherits(String, Type)
String.prototype.toString = function () { return 'String' }


var Number = function () {
  _super(this).apply(this)
  this.intrinsic = true
}
inherits(Number, Type)


module.exports = {
  Type: Type,
  String: String
}
