
var inherits = require('util').inherits,
    types    = require('./types'),
    AST      = require('./ast')


function TypeError (message, origin) {
  Error.apply(this)
  this.name = 'TypeError'
  this.message = message
  this.origin = (origin !== undefined) ? origin : null
}
inherits(TypeError, Error)


function Scope (parent) {
  this.parent = (parent === undefined) ? null : parent
  this.locals = {}
}
Scope.prototype.get = function (name) {
  if (this.locals[name] !== undefined) {
    return this.locals[name]
  } else if (this.parent !== null) {
    return this.parent.get(name)
  } else {
    throw new TypeError('Unknown variable: '+name)
  }
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

function TypeSystem () {
  this.cache = {}
  this.setupIntrinsics()
}
TypeSystem.prototype.setupIntrinsics = function () {
  this.cache['String'] = new types.String()
  this.cache['Number'] = new types.Number()
  // Alias Integer to Number
  this.cache['Integer'] = new types.Number()
}
TypeSystem.prototype.findByName = function (name) {
  var type = this.cache[name]
  if (type === undefined) {
    throw new TypeError('Type not found: '+name)
  }
  return type
}

function assertInstanceOf(value, type, msg) {
  if (value instanceof type) { return; }
  throw new Error(msg)
}


// AST typing -----------------------------------------------------------------

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
    self.visitStatement(stmt, scope, node)
  })
}

TypeSystem.prototype.visitStatement = function (node, scope, parentNode) {
  switch (node.constructor) {
    case AST.Assignment:
      if (node.lvalue instanceof AST.Let) {
        this.visitLet(node, scope)
      } else if (node.lvalue instanceof AST.Var) {
        this.visitVar(node, scope)
      } else if (node.lvalue instanceof AST.Path) {
        this.visitPath(node, scope)
      } else {
        throw new TypeError('Cannot visit Assignment with: '+node.lvalue+' ('+node.lvalue.constructor.name+')')
      }
      break
    case AST.If:
      this.visitIf(node, scope)
      break
    case AST.While:
      this.visitWhile(node, scope)
      break
    case AST.For:
      this.visitFor(node, scope)
      break
    case AST.Return:
      this.visitReturn(node, scope, parentNode)
      break
    case AST.Binary:
      if (node.isBinaryStatement()) {
        this.visitBinary(node, scope)
      } else {
        throw new TypeError('Cannot visit non-statement binary: '+node.op)
      }
      break
    default:
      throw new TypeError("Don't know how to visit: "+node.constructor.name)
      // node.print()
      break
  }
}

TypeSystem.prototype.visitFor = function (node, scope) {
  // TODO: Check that the condition (`node.cond`) resolves to a
  //       boolean-checkable type.

  this.visitStatement(node.init, scope)
  this.visitExpression(node.cond, scope)
  this.visitStatement(node.after, scope)

  var blockScope = new Scope(scope)
  this.visitBlock(node.block, blockScope)
}

TypeSystem.prototype.visitIf = function (node, scope) {
  assertInstanceOf(node.block, AST.Block, 'Expected Block in If statement')

  this.visitExpression(node.cond, scope)

  var blockScope = new Scope(scope)
  this.visitBlock(node.block, blockScope)
}

TypeSystem.prototype.visitWhile = function (node, scope) {
  assertInstanceOf(node.block, AST.Block, 'Expected Block in While statement')

  this.visitExpression(node.expr, scope)

  var blockScope = new Scope(scope)
  this.visitBlock(node.block, blockScope)
}

TypeSystem.prototype.visitReturn = function (node, scope, parentNode) {
  if (node.expr === null || node.expr === undefined) {
    throw new TypeError('Cannot handle empty Return')
  }
  var expr = node.expr
  var exprType = this.resolveExpression(expr, scope)
  node.type = exprType
  if (parentNode) {
    assertInstanceOf(parentNode, AST.Block, 'Expected Block as parent of Return')
    if (parentNode.returnType) {
      throw new TypeError('Block already has returned')
    }
    parentNode.returnType = exprType
  }
}

TypeSystem.prototype.visitPath = function (node, scope) {
  var path = node.lvalue
  var foundScope = scope.findScopeForName(path.name)
  if (foundScope === null) {
    throw new TypeError('Failed to find '+path.name)
  }
  var lvalueType = foundScope.get(path.name)
  var rvalueType = this.resolveExpression(node.rvalue, scope)

  if (!lvalueType.equals(rvalueType)) {
    throw new TypeError('Unequal types in assignment: '+lvalueType.inspect()+' </> '+rvalueType.inspect())
  }
}

TypeSystem.prototype.visitLet = function (node, scope) {
  var lvalueType = new types.Unknown()
  var name       = node.lvalue.name

  // If we have an explicit type then look it up
  if (node.lvalue.typepath) {
    var typepath = node.lvalue.typepath
    lvalueType = this.findByName(typepath)
  }

  // Create a scope inside the Let statement for recursive calls
  var letScope = new Scope(scope)
  letScope.setLocal(name, lvalueType)

  if (node.rvalue) {
    // rvalue is an expression so let's determine its type first.
    var rvalueType = this.resolveExpression(node.rvalue, letScope, function (immediateType) {
      if (lvalueType instanceof types.Unknown) {
        // If the lvalue is unknown then annotate it with the resolved type
        lvalueType.known = immediateType
      }
    })
    if (lvalueType instanceof types.Unknown) {
      // If the lvalue was inferred then update on the lvalue
      node.lvalue.type = rvalueType
      scope.setLocal(name, rvalueType)
    } else {
      // If the lvalue type is explicit then make sure they match up
      if (!lvalueType.equals(rvalueType)) {
        var message = 'Unequal types in declaration: '+lvalueType.inspect()+' </> '+rvalueType.inspect()
        throw new TypeError(message, node)
      }
    }

  } else {
    // No rvalue present
    node.lvalue.type = lvalueType
    scope.setLocal(name, lvalueType)
  }
}
TypeSystem.prototype.visitVar = TypeSystem.prototype.visitLet


TypeSystem.prototype.resolveExpression = function (expr, scope, immediate) {
  // If we've already deduced the type of this then just return it
  if (expr.type) { return expr.type }

  this.visitExpression(expr, scope, immediate)

  if (expr.type === null || expr.type === undefined) {
    throw new TypeError('Failed to resolve type')
  }
  return expr.type
}

TypeSystem.prototype.visitExpression = function (node, scope, immediate) {
  switch (node.constructor) {
    case AST.Function:
      this.visitFunction(node, scope, immediate)
      break
    case AST.Binary:
      this.visitBinary(node, scope)
      break
    case AST.Chain:
      this.visitChain(node, scope)
      break
    case AST.Literal:
      this.visitLiteral(node, scope)
      break
    default:
      throw new Error("Can't walk: "+node.constructor.name)
  }
}

TypeSystem.prototype.visitLiteral = function (node, scope) {
  if (node.type) {
    return// pass
  } else {
    throw new TypeError('Unknown literal type: '+node.type)
  }
}

TypeSystem.prototype.visitBinary = function (node, scope) {
  var lexprType = this.resolveExpression(node.lexpr, scope)
  var rexprType = this.resolveExpression(node.rexpr, scope)

  if (lexprType.equals(rexprType)) {
    node.type = lexprType
  } else {
    throw new TypeError('Unequal types in binary operation: '+lexprType.inspect()+' </> '+rexprType.inspect())
  }
}

function getAllReturnTypes (block) {
  var returnTypes = []
  if (block.returnType) { returnTypes.push(block.returnType) }

  block.statements.forEach(function (stmt) {
    switch (stmt.constructor) {
      case AST.If:
      case AST.While:
      case AST.For:
        var subblockTypes = getAllReturnTypes(stmt.block)
        returnTypes = returnTypes.concat(subblockTypes)
    }
  })
  return returnTypes
}

TypeSystem.prototype.visitFunction = function (node, parentScope, immediate) {
  if (node.type) { return node.type }
  var self = this
  var type = new types.Function()

  if (node.ret) {
    type.ret = this.findByName(node.ret)
  }

  // If we have a callback for the immediate (not-yet-fully resolved type)
  // then call it now.
  if (immediate !== undefined) {
    immediate(type)
  }

  var functionScope = new Scope(parentScope)

  node.args.forEach(function (arg) {
    var argType = self.findByName(arg.type)
    functionScope.setLocal(arg.name, argType)
  })

  // Begin by visiting our block
  this.visitBlock(node.block, functionScope)

  if (type.ret) {
    // Get all possible return types of this function (recursively collects
    // returning child blocks).
    var returnTypes = getAllReturnTypes(node.block)
    returnTypes.forEach(function (returnType) {
      if (!type.ret.equals(returnType)) {
        throw new TypeError('Type returned by function does not match declared return type')
      }
    })
    node.type = type
    return
  }
  throw new Error('Inferred return types not supported yet')

  // Then we'll find all the `return`s and get their types
  var returns = []
}

var know = function (type) {
  if (type instanceof types.Unknown) {
    if (type.known === null) {
      throw new TypeError('Unknown type')
    }
    return type.known
  }
  return type
}

TypeSystem.prototype.visitChain = function (node, scope) {
  var type = know(scope.get(node.name))
  node.tail.forEach(function (item) {
    if (item instanceof AST.Call) {
      assertInstanceOf(type, types.Function, 'Trying to call non-Function')
      // TODO: Type-check arguments
      // console.log(item.constructor.name+': '+item.toString())
      // Replace current type with type that's going to be returned
      var returnType = type.ret
      type = returnType
    } else {
      throw new TypeError('Cannot handle Chain item of type: '+item.constructor.name)
    }
  })
  node.type = type
}


module.exports = {TypeSystem: TypeSystem}
