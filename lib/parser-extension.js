
var AST   = require('./ast'),
    types = require('./types')

// `p` is an object set up by the grammar with all the handlers for
// matched grammar rules.
module.exports = function (p) {

  p.parseBlock = function (statements) {
    return new AST.Block(statements)
  }

  p.parseDeclaration = function (lvalue, rvalue) {
    var type = null
    if (lvalue instanceof AST.Let) {
      type = 'let'
    } else if (lvalue instanceof AST.Var) {
      type = 'var'
    }
    if (type === null) {
      throw new Error('Can\'t figure out type of declaration')
    }
    var op = '='
    return new AST.Assignment(type, lvalue, op, rvalue)
  }

  p.parseIf = function (cond, block) {
    return new AST.If(cond, block)
  }

  p.parseRoot = function (statements) {
    return new AST.Root(statements)
  }

  p.parseBinary = function (left, op, right) {
    return new AST.Binary(left, op, right)
  }

  p.parseInteger = function (integerString) {
    var type = new types.Number() // TODO: Look up from type-system
    return new AST.Literal(parseInt(integerString, 10), type)
  }

  p.parseLeftDeclaration = function (decl, name, typepath) {
    var Con = (decl === 'let') ? AST.Let : AST.Var
    return new Con(name, typepath)
  }

  p.parseFunction = function (args, returnType, block) {
    /*args = args.elements.filter(function (el) {
      return el.arg
    }).map(function (el) {
      var arg = el.arg
      var name = arg.name.textValue
      var type = null
      if (arg.typifier) {
        type = arg.typifier.type.textValue
      }
      return {name: name, type: type}
    })*/
    return new AST.Function(args, returnType, block)
  }

  p.parseFor = function (init, cond, after, block) {
    return new AST.For(init, cond, after, block)
  }

  p.parseWhile = function (cond, block) {
    return new AST.While(cond, block)
  }

  p.parseIf = function (cond, block) {
    return new AST.If(cond, block)
  }

  p.parseChain = function (name, tail) {
    return new AST.Chain(name, tail)
  }

  p.parseAssignment = function (path, op, expr) {
    return new AST.Assignment('path', path, op, expr)
  }

  p.parseReturn = function (expr) {
    return new AST.Return(expr)
  }

  p.parseCall = function (expr) {
    return new AST.Call(expr)
  }

  p.parsePath = function (name) {
    return new AST.Path(name)
  }

}
