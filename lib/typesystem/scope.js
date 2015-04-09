var inherits = require('util').inherits

var _super = function (self) {
  return self.constructor.super_
}

var scopeIDCounter = 1

var Flags = {
  Constant: 'c'
}

function Scope (parent) {
  this.id = (scopeIDCounter++)
  // Default to not being the root scope
  this.isRoot = false
  // Default to not being a closing scope
  this.isClosing = false
  this.parent = (parent === undefined) ? null : parent
  // Actual map of names (string) to types (Type)
  this.locals = {}
  // Storing flags for read-only locals and such
  this.localsFlags = {}
}
Scope.Flags = Flags
Scope.prototype.get = function (name, cb) {
  if (this.locals[name] !== undefined) {
    var type = this.locals[name]
    // Call the callback with ourselves first
    if (cb) { cb(this, type) }
    // Then return the type that was found
    return type
  } else if (this.parent !== null) {
    return this.parent.get(name, cb)
  } else {
    throw new TypeError('Not found: '+name)
  }
}
Scope.prototype.getLocal = function (name) {
  if (this.locals[name] !== undefined) {
    return this.locals[name]
  }
  throw new TypeError('Local not found: '+name)
}
Scope.prototype.setLocal = function (name, type) {
  if (this.locals[name] !== undefined) {
    throw new TypeError("Can't redefine local: "+name)
  }
  this.locals[name] = type
}
Scope.prototype.getFlagsForLocal = function (name) {
  var flags = this.localsFlags[name]
  if (flags === undefined) { return '' }
  return flags
}
Scope.prototype.setFlagsForLocal = function (name, flags) {
  // Ensure the local exists
  if (!this.locals[name]) { throw new TypeError('Local not found: '+name) }
  // Then set the flags
  this.localsFlags[name] = flags
}
Scope.prototype.localHasFlag = function (name, flag) {
  // NOTE: Right now this doesn't check that a local exists
  var flags = this.getFlagsForLocal(name)
  if (!flags) { return false }
  if (flags.indexOf(flag) !== -1) { return true }
  return false
}
Scope.prototype.findScopeForName = function (name) {
  if (this.locals[name] !== undefined) {
    return this
  }
  if (this.parent) {
    return this.parent.findScopeForName(name)
  }
  return null
}

function ClosingScope (parent) {
  _super(this).call(this, parent)
  this.isClosing = true
}
inherits(ClosingScope, Scope)
ClosingScope.prototype.get = function (name, cb) {
  var type = null
  if (this.locals[name] !== undefined) {
    type = this.locals[name]
    // Call callback is present, then return the type
    if (cb) { cb(this, type) }
    return type
  }
  this.parent.get(name, function (scope, _type) {
    if (!scope.isRoot) {
      return
    }
    type = _type
    // Call the callback with the finding scope
    if (cb) { cb(scope, type) }
  })
  if (!type) {
    throw new TypeError('Not found in this closure: '+name)
  }
  return type
}

module.exports = {
  Scope:        Scope,
  ClosingScope: ClosingScope
}

