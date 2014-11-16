
var grammar = require('./grammar'),
    AST     = require('./ast')

var extend = require('util')._extend

// Grammar extensions
extend(grammar.Parser, {
  Skip: {skip: true},
  Assignment: {type: 'assignment',
    transform: function () {
      var declarator = this.elements[0].declarator,
          type = 'path'
      if (declarator !== undefined) {
        type = declarator.textValue
      }
      var lvalue = this.lvalue, rvalue
      if (type === 'let' || type === 'var') {
        var typifier = lvalue.typifier,
            typepath = false

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
