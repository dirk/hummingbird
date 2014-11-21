

var types = require('./types')
var AST   = require('./ast')


var TypeSystem = function () {
  this.cache = {}
  this.setupIntrinsics()
}
TypeSystem.prototype.setupIntrinsics = function () {
  this.cache['String'] = new types.String()
  this.cache['Number'] = new types.Number()
}

TypeSystem.prototype.walk = function (node) {
  var self = this
  switch (node.constructor) {
    case AST.Root:
      // All we need to do with the root is compute types for everything
      // inside of it.
      node.statements.forEach(function (stmt) {
        self.walk(stmt)
      })
      break
    default:
      console.log("Don\'t know how to walk:")
      node.print()
      console.log("\n")
      break
  }
}


module.exports = {TypeSystem: TypeSystem}
