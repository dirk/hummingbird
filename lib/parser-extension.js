
var AST   = require('./ast'),
    types = require('./types')

// `p` is an object set up by the grammar with all the handlers for
// matched grammar rules.
module.exports = function (p) {

  p.parseImport = function (name) {
    return new AST.Import(name)
  }
  p.parseExport = function (name) {
    return new AST.Export(name)
  }

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

  p.parseClass = function (name, block) {
    return new AST.Class(name, block)
  }

  p.parseInit = function (args, block) {
    return new AST.Init(args, block)
  }

  p.parseIf = function (cond, block, elseIfs, elseBlock) {
    return new AST.If(cond, block, elseIfs, elseBlock)
  }

  p.parseRoot = function (statements) {
    return new AST.Root(statements)
  }

  p.parseBinary = function (left, op, right) {
    return new AST.Binary(left, op, right)
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
    var Con = (decl === 'let') ? AST.Let : AST.Var
    return new Con(name, typepath)
  }

  p.parseFunction = function (name, args, returnType, whenCond, block) {
    var func = new AST.Function(args, returnType, block)
    // Set statement properties (they're null for non-statement functions)
    func.name = name
    func.when = whenCond
    return func
  }

  p.parseNew = function (name, args) {
    return new AST.New(name, args)
  }

  p.parseFor = function (init, cond, after, block) {
    return new AST.For(init, cond, after, block)
  }

  p.parseWhile = function (cond, block) {
    return new AST.While(cond, block)
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

  p.parseProperty = function (name) {
    return new AST.Property(name)
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
    return new AST.Multi(name, args, ret)
  }

}
