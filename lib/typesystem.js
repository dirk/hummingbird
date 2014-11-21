

var types = require('./types')
var AST   = require('./ast')


function Scope (parent) {
  this.parent = (parent === undefined) ? null : parent
}



function TypeSystem () {
  this.cache = {}
  this.setupIntrinsics()
}
TypeSystem.prototype.setupIntrinsics = function () {
  this.cache['String'] = new types.String()
  this.cache['Number'] = new types.Number()
}

function assertInstanceOf(value, type, msg) {
  if (value instanceof type) { return; }
  throw new Error(msg)
}

TypeSystem.prototype.walk = function (rootNode) {
  assertInstanceOf(rootNode, AST.Root, "Node must be root")

  var self = this
  var topLevelScope = new Scope()
  rootNode.statements.forEach(function (stmt) {
    self.visitStatement(stmt, topLevelScope)
  })
}

TypeSystem.prototype.visitBlock = function (node, scope) {
  var self = this
  node.statements.forEach(function (stmt) {
    self.visitStatement(stmt, scope)
  })
}

TypeSystem.prototype.visitStatement = function (node, scope) {
  switch (node.constructor) {
    case AST.Assignment:
      if (node.lvalue instanceof AST.Let) {
        this.visitLet(node, scope)
        break
      }
    default:
      console.log("Don\'t know how to walk: "+node.constructor.name)
      // node.print()
      break
  }
}

TypeSystem.prototype.visitLet = function (node, scope) {
  // rvalue is an expression so let's determine its type first.
  var rvalueType = this.resolveExpression(node.rvalue, scope)
  
}

TypeSystem.prototype.resolveExpression = function (expr, scope) {
  // If we've already deduced the type of this then just return it
  if (expr.type) { return expr.type }

  this.visitExpression(expr, scope)
}

TypeSystem.prototype.visitExpression = function (node, scope) {
  switch (node.constructor) {
    case AST.Function:
      this.visitFunction(node, scope)
      break
    default:
      throw new Error("Can't walk: "+node.constructor.name)
  }
}

TypeSystem.prototype.visitFunction = function (node, parentScope) {
  if (node.type) { return node.type }

  var functionScope = new Scope(parentScope)

  // Begin by visiting our block
  this.visitBlock(node.block, functionScope)

  // Then we'll find all the `return`s and get their types
  var returns = []
}



module.exports = {TypeSystem: TypeSystem}
