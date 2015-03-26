
function Scope (parent) {
  // Default to not being the root scope
  this.isRoot = false
  this.parent = (parent === undefined) ? null : parent
  this.locals = {}
}
Scope.prototype.get = function (name) {
  if (this.locals[name] !== undefined) {
    return this.locals[name]
  } else if (this.parent !== null) {
    return this.parent.get(name)
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
Scope.prototype.findScopeForName = function (name) {
  if (this.locals[name] !== undefined) {
    return this
  }
  if (this.parent) {
    return this.parent.findScopeForName(name)
  }
  return null
}

module.exports = Scope
