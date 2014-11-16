
var grammar = require('./grammar'),
    AST     = require('./ast')

var extend = require('util')._extend

// Grammar extensions
extend(grammar.Parser, {
  Skip: {skip: true},
  Expression: {type: 'expression',
    transform: function () {
      if (this.binaryop) {
        return grammar.Parser.Binary.transform.apply(this)
      } else if(this.literal) {
        return grammar.Parser.Literal.transform.apply(this)
      } else {
        throw new Error("Don't know how to handle expression: "+this.textValue)
      }
    }
  },
  Group: {type: 'group',
    transform: function () {
      return new AST.Group(this.expr.transform())
    }
  },
  Literal: {type: 'literal',
    transform: function () {
      return new AST.Literal(parseInt(this.textValue, 10))
    }
  },
  Binary: {type: 'binary',
    transform: function () {
      // console.log(this.lexpr.transform())
      // console.log(this.expr.transform())
      return new AST.Binary(this.lexpr.transform(), this.binaryop.textValue, this.expr.transform())
    }
  },
  Integer: {type: 'integer',
    transform: function () {
      return this
    }
  },
  Assignment: {type: 'assignment',
    transform: function () {
      var declarator = this.elements[0].declarator,
          type = 'path'
      // Declarator will either be 'let' or 'var'
      if (declarator !== undefined) {
        type = declarator.textValue
      }
      var lvalue = this.lvalue, rvalue = this.rvalue.transform()
      if (type === 'let' || type === 'var') {
        var typifier = lvalue.typifier,
            typepath = false
        // Figure out the typepath string from the typifier
        if (typifier !== undefined && typifier.type !== undefined) {
          typepath = typifier.type.textValue
        }
        var Con = (type === 'let') ? AST.Let : AST.Var
        lvalue = new Con(lvalue.name.textValue, typepath)

      } else {
        // Path-based assignment
        lvalue = new AST.Path()
      }
      return new AST.Assignment(type, lvalue, rvalue)
    }
  },
  Root: {
    type: 'root',
    transform: function () {
      var headStatements = this.head,
          statements = this.tail.elements.map(function (element) {
            return element.statement
          })
      statements.unshift(headStatements)
      statements = statements.map(function (stmt) {
        return stmt.transform()
      })
      return new AST.Root(statements)
    }
  }
})

var Parser = function () {}
Parser.prototype.parse = function Parser_parse(code) {
  var parseTree = grammar.parse(code),
      ast = this.transform(parseTree)
  return ast
}

// Take a parse tree from the grammar and transform it into an AST.
Parser.prototype.transform = function (tree) {
  var newTree = tree.transform()
  return newTree
}

module.exports = Parser
