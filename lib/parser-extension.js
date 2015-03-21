
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
    var op   = '=',
        node = new AST.Assignment(type, lvalue, op, rvalue)
    // Set the line and offset from our parse state
    node.setParsePosition(this)
    return node
  }

  p.parseIf = function (cond, block) {
    return new AST.If(cond, block)
  }

  p.parseRoot = function (statements) {
    return new AST.Root(statements)
  }

  p.parseBinary = function (left, op, right) {
    var binary = new AST.Binary(left, op, right)
    binary.setParsePosition(this)
    return binary
  }

  p.parseInteger = function (integerString) {
    var type = new types.Number() // TODO: Look up from type-system
    return new AST.Literal(parseInt(integerString, 10), type)
  }

  p.parseLeftDeclaration = function (decl, name, typepath) {
    var Con = (decl === 'let') ? AST.Let : AST.Var,
        decl = new Con(name, typepath)
    decl.setParsePosition(this)
    return decl
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
    var chain = new AST.Chain(name, tail)
    chain.setParsePosition(this)
    return chain
  }

  p.parseAssignment = function (path, op, expr) {
    return new AST.Assignment('path', path, op, expr)
  }

  p.parseReturn = function (expr) {
    return new AST.Return(expr)
  }

  p.parseCall = function (expr) {
    var call = new AST.Call(expr)
    call.setParsePosition(this)
    return call
  }

  p.parsePath = function (name) {
    return new AST.Path(name)
  }

  p.parseFunctionType = function (args, ret) {
    // Turn null args into a proper empty array of arguments
    if (args === null) { args = [] }
    return new AST.FunctionType(args, ret)
  }

  p.parseNameType = function (name) {
    return new AST.NameType(name)
  }

}
