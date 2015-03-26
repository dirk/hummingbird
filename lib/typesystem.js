
var inherits  = require('util').inherits,
    types     = require('./types'),
    AST       = require('./ast'),
    Scope     = require('./typesystem/scope'),
    TypeError = require('./typesystem/typeerror')

var inspect = require('util').inspect

function TypeSystem () {
  this.root = new Scope()
  this.root.isRoot = true
  this.setupIntrinsics()
}
TypeSystem.prototype.setupIntrinsics = function () {
  var rootObject = new types.Object('fake')
  rootObject.supertype = null
  rootObject.isRoot    = true
  var Number = new types.Number(rootObject)
  this.root.setLocal('Object',  new types.Object(rootObject))
  this.root.setLocal('String',  new types.String(rootObject))
  this.root.setLocal('Number',  Number)
  this.root.setLocal('Boolean', new types.Boolean(rootObject))
  // Alias Integer to Number
  this.root.setLocal('Integer', Number)
  // Expose rootObject to later functions
  this.rootObject = rootObject
}
TypeSystem.prototype.findByName = function (name) {
  if (typeof name !== 'string') {
    throw new Error('Non-string name for type lookup')
  }
  return this.root.getLocal(name)
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
    self.visitStatement(stmt, topLevelScope, rootNode)
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
    case AST.Chain:
      this.visitChain(node, scope)
      break
    case AST.Multi:
      this.visitMulti(node, scope)
      break
    case AST.Function:
      // Create the searcher in this parent node
      // TODO: Maybe just pass along the parent node rather than generating
      //       a whole new anonymous function every time we encounter a
      //       function statement?
      var searchInParent = function (cb) {
        var statements = parentNode.statements,
            found      = null
        // Call `cb` on each statement of the parent until it returns true
        for (var i = statements.length - 1; i >= 0; i--) {
          var stmt = statements[i],
              ret  = cb(stmt)
          if (ret === true) {
            found = stmt
            break
          }
        }
        return found
      }
      this.visitFunctionStatement(node, scope, searchInParent)
      break
    default:
      throw new TypeError("Don't know how to visit: "+node.constructor.name, node)
      break
  }
}

TypeSystem.prototype.visitFor = function (node, scope) {
  this.visitStatement(node.init, scope)

  // If there's a condition present then we need to visit the expression
  // and type-check what it resolves to
  if (node.cond) {
    this.visitExpression(node.cond, scope)
    var condType = node.cond.type
    if (!condType) {
      throw new TypeError('Missing type of `for` condition', node.cond)
    }
    // Check that the condition resolves to a boolean
    if (!condType.equals(this.findByName('Boolean'))) {
      throw new TypeError('Expected `for` condition to resolve to a Boolean', node.cond)
    }
  }

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

TypeSystem.prototype.resolveType = function (node, scope) {
  var self = this
  switch (node.constructor) {
    case AST.FunctionType:
      var args = node.args.map(function (arg) { return self.resolveType(arg, scope) }),
          ret  = this.resolveType(node.ret, scope)
      // Build the type and return it
      return new types.Function(this.rootObject, args, ret)
    case AST.NameType:
      // TODO: Improve the handling and look-ups of these; right now they're way too naive
      return this.findByName(node.name)
    default:
      throw new Error("Can't walk: "+node.constructor.name)
  }
}

TypeSystem.prototype.visitLet = function (node, scope) {
  var lvalueType = new types.Unknown()
  var name       = node.lvalue.name

  // If we have an explicit type then look it up
  if (node.lvalue.immediateType) {
    var immediateTypeNode = node.lvalue.immediateType
    // lvalueType = this.findByName(...)
    lvalueType = this.resolveType(immediateTypeNode, scope)
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
      scope.setLocal(name, lvalueType)
    }

  } else {
    // No rvalue present
    node.lvalue.type = lvalueType
    scope.setLocal(name, lvalueType)
  }
}
// Alias the var visitor to the let visitor
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
      // Sanity checks to make sure the name and when are not present
      if (node.name) {
        throw new TypeError('Function expression cannot have a `name`', node)
      }
      if (node.when) {
        throw new TypeError('Function expression cannot have a `when` condition', node)
      }
      // Then run the visitor
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
  // If we've already identified the type
  if (node.type) {
    return node.type
  } else if (node.typeName) {
    var type = this.findByName(node.typeName)
    node.type = type
    return type
  } else {
    throw new TypeError('Unknown literal type: '+node.typeName)
  }
}

var COMPARATOR_OPS = ['<']

TypeSystem.prototype.visitBinary = function (node, scope) {
  var lexprType = this.resolveExpression(node.lexpr, scope)
  var rexprType = this.resolveExpression(node.rexpr, scope)

  if (lexprType.equals(rexprType)) {
    // Naive type assignment based off left side; this is refined below
    node.type = lexprType
  } else {
    console.log(node.rexpr)
    throw new TypeError('Unequal types in binary operation: '+lexprType.inspect()+' </> '+rexprType.inspect())
  }
  // TODO: Check adder, comparator, etc. interfaces of the left and right
  var op = node.op
  if (COMPARATOR_OPS.indexOf(op) !== -1) {
    node.type = this.findByName('Boolean')
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
  var type = new types.Function(this.rootObject)

  if (node.ret) {
    type.ret = this.resolveType(node.ret)
  }

  // If we have a callback for the immediate (not-yet-fully resolved type)
  // then call it now.
  if (immediate !== undefined) {
    immediate(type)
  }

  var functionScope = new Scope(parentScope)
  // Save this new scope on the node object for later use
  node.scope = functionScope

  // Build up the args to go into the type definition
  var typeArgs = []
  node.args.forEach(function (arg) {
    // Deprecated simplistic type lookup:
    //   var argType = self.findByName(arg.type)
    var argType = self.resolveType(arg.type)
    // Setup a local in the function's scope for the argument
    functionScope.setLocal(arg.name, argType)
    // Add it to the type's args
    typeArgs.push(argType)
  })
  type.args = typeArgs

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
  throw new TypeError('Inferred return types not supported yet', node)

  // Then we'll find all the `return`s and get their types
  var returns = []
}


TypeSystem.prototype.visitFunctionStatement = function (node, scope, searchInParent) {
  // First run the generic function visitor
  this.visitFunction(node, scope)
  // Type-system checks
  if (typeof node.name !== 'string') {
    throw new TypeError('Non-string name for function statement', node)
  }
  assertInstanceOf(node.scope, Scope, "Missing function's scope")
  // Now do statement-level visiting
  if (node.when) {
    this.visitExpression(node.when, node.scope)
  }
  var name = node.name
  // Now look up the parent `multi` in the containing block
  var multiNode = searchInParent(function (stmt) {
    if (stmt.constructor === AST.Multi && stmt.name === name) {
      return true
    }
    return false
  })
  if (!multiNode) {
    throw new TypeError('Failed to find associated multi statement')
  }
  var multiType = multiNode.type
  // Add this implementation to its list of functions and set the parent of
  // the function so that it knows not to codegen itself
  multiType.addFunctionNode(node)
  node.setParentMultiType(multiNode.type)
}


// Resolve an Unknown type to a known one (sort of a second pass) or throw
// an error if it's still unknown
var know = function (node, type) {
  if (type instanceof types.Unknown) {
    if (type.known === null) {
      throw new TypeError('Unknown type')
    }
    return type.known
  }
  return type
}

TypeSystem.prototype.visitChain = function (node, scope) {
  var type = know(node, scope.get(node.name))
  node.tail.forEach(function (item) {
    if (item instanceof AST.Call) {
      assertInstanceOf(type, types.Function, 'Trying to call non-Function')
      var typeArgs = type.args,
          itemArgs = item.args
      // Check to make sure we're getting as many arguments as we expected
      if (typeArgs.length !== itemArgs.length) {
        var t = typeArgs.length, i = itemArgs.length
        throw new TypeError('Wrong number of arguments: expected '+t+', got '+i)
      }
      // Then type-check each individual arguments
      for (var i = itemArgs.length - 1; i >= 0; i--) {
        var itemArgNode = itemArgs[i],
            itemArgType = itemArgNode.type,
            typeArg     = typeArgs[i]
        if (!itemArgType.equals(typeArg)) {
          var message  = 'Argument mismatch at argument index '+i,
              got      = itemArgType.inspect(),
              expected = typeArg.inspect()
          message += "\n  expected "+expected+', got '+got
          throw new TypeError(message, item)
        }
      }
      // Replace current type with type that's going to be returned
      var returnType = type.ret
      type = returnType
    } else {
      throw new TypeError('Cannot handle Chain item of type: '+item.constructor.name)
    }
  })
  node.type = type
}


TypeSystem.prototype.visitMulti = function (node, scope) {
  var self = this
  // Construct a new array of name-type args
  var args = node.args.map(function (arg) {
    var name = arg.name,
        type = self.resolveType(arg.type)
    return {name: name, type: type}
  })
  if (!node.ret) {
    throw new TypeError('Missing multi return type', node)
  }
  var ret = this.resolveType(node.ret)
  // Construct Multi type with the arguments and return types
  var multi = new types.Multi(this.rootObject, args, ret)
  node.type = multi
  // Add multi to the scope
  scope.setLocal(node.name, multi)
}


module.exports = {TypeSystem: TypeSystem}

