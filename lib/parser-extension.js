
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

  p.parseClass = function (name, block) {
    var klass = new AST.Class(name, block)
    klass.setParsePosition(this)
    return klass
  }

  p.parseInit = function (args, block) {
    var init = new AST.Init(args, block)
    init.setParsePosition(this)
    return init
  }

  p.parseIf = function (cond, block, elseIfs, elseBlock) {
    var i = new AST.If(cond, block, elseIfs, elseBlock)
    i.setParsePosition(this)
    return i
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
    // var type = new types.Number() // TODO: Look up from type-system
    var typeName = 'Number'
    return new AST.Literal(parseInt(integerString, 10), typeName)
  }
  p.parseString = function (string) {
    var typeName = 'String'
    return new AST.Literal(string, typeName)
  }

  p.parseLeftDeclaration = function (decl, name, typepath) {
    var Con = (decl === 'let') ? AST.Let : AST.Var,
        decl = new Con(name, typepath)
    decl.setParsePosition(this)
    return decl
  }

  p.parseFunction = function (name, args, returnType, whenCond, block) {
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
    var func = new AST.Function(args, returnType, block)
    // Set statement properties (they're null for non-statement functions)
    func.name = name
    func.when = whenCond
    func.setParsePosition(this)
    return func
  }

  p.parseNew = function (name, args) {
    var n = new AST.New(name, args)
    n.setParsePosition(this)
    return n
  }

  p.parseFor = function (init, cond, after, block) {
    return new AST.For(init, cond, after, block)
  }

  p.parseWhile = function (cond, block) {
    return new AST.While(cond, block)
  }

  p.parseChain = function (name, tail) {
    var chain = new AST.Chain(name, tail)
    chain.setParsePosition(this)
    return chain
  }

  p.parseAssignment = function (path, op, expr) {
    var assignment = new AST.Assignment('path', path, op, expr)
    assignment.setParsePosition(this)
    return assignment
  }

  p.parseReturn = function (expr) {
    return new AST.Return(expr)
  }

  p.parseCall = function (expr) {
    var call = new AST.Call(expr)
    call.setParsePosition(this)
    return call
  }

  p.parseProperty = function (name) {
    var property = new AST.Property(name)
    property.setParsePosition(this)
    return property
  }

  p.parsePath = function (name, path) {
    return new AST.Path(name, path)
  }

  p.parseFunctionType = function (args, ret) {
    // Turn null args into a proper empty array of arguments
    if (args === null) { args = [] }
    return new AST.FunctionType(args, ret)
  }

  p.parseNameType = function (name) {
    return new AST.NameType(name)
  }

  p.parseMutli = function (name, args, ret) {
    var multi = new AST.Multi(name, args, ret)
    multi.setParsePosition(this)
    return multi
  }

}
