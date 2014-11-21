
var grammar = require('./grammar'),
    AST     = require('./ast'),
    types   = require('./types')
    stderr  = process.stderr,
    _       = require('lodash')

var extend = require('util')._extend

function transform_statement_list (el) {
  var headStatements = el.head,
      statements = el.tail.elements.map(function (element) {
        return element.statement
      })
  statements.unshift(headStatements)
  statements = statements.map(function (stmt) {
    return stmt.transform()
  }).filter(function (stmt) {
    return stmt !== null
  })
  return statements
}

// Grammar extensions
extend(grammar.Parser, {
  Skip: {skip: true},
  Expression: {type: 'expression',
    transform: function () {
      if (this.binaryop) {
        return grammar.Parser.Binary.transform.apply(this)
      } else if(this.literal) {
        return grammar.Parser.Literal.transform.apply(this)
      } else if(this.args && this.block) {
        return grammar.Parser.Function.transform.apply(this)
      } else if(this.name) {
        return grammar.Parser.Chain.transform.apply(this)
      } else {
        var defaultProps = ['textValue', 'offset', 'elements', 'transform', 'type', '_'],
            keys = _.keys(this),
            ownProps = keys.filter(function (key) {
              return defaultProps.indexOf(key) === -1
            })
        throw new Error("Don't know how to handle expression signature: "+ownProps.join(', '))
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
      // TODO: Handle non-number values
      var type = new types.Number() // TODO: Look up from type-system
      return new AST.Literal(parseInt(this.textValue, 10), type)
    }
  },
  Function: {type: 'function',
    transform: function () {
      var block = this.block.transform()
      var args  = this.args.elements.filter(function (el) {
        return el.arg
      }).map(function (el) {
        var arg = el.arg
        var name = arg.name.textValue
        var type = null
        if (arg.typifier) {
          type = arg.typifier.type.textValue
        }
        return {name: name, type: type}
      })
      var ret = null
      if (this.ret) {
        var type = this.ret.type.textValue
        ret = type
      }
      return new AST.Function(args, ret, block)
    }
  },
  If: {type: 'if',
    transform: function () {
      // Expression for the condition
      var condition = this.condition.transform()
      var block     = this.block.transform()
      return new AST.If(condition, block)
    }
  },
  Return: {type: 'return',
    transform: function () {
      var elements = this.elements
      var expr = null
      if (elements.length == 2) {
        expr = elements[1].expr.transform()
      }
      return new AST.Return(expr)
    }
  },
  For: {type: 'for',
    transform: function () {
      var init  = this.init.transform()
      var cond  = this.cond.transform()
      var after = this.after.transform()
      var block = this.block.transform()
      return new AST.For(init, cond, after, block)
    }
  },
  Call: {type: 'call',
    transform: function () {
      var args = []
      if (this.args.head) {
        args.push(this.args.head.transform())
        this.args.tail.elements.forEach(function (arg) {
          args.push(arg.expr.transform())
        })
      }
      return new AST.Call(args)
    }
  },
  Chain: {type: 'chain',
    transform: function () {
      // TODO: Implement the tail of properties, calls, and indexer
      var name = this.name.textValue
      var tail = this.tail.elements.map(function (el) {
        return el.transform()
      })
      return new AST.Chain(name, tail)
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
        var tail = this.lvalue.tail.elements
        if (tail.length) {
          throw new Error('Cannot handle tails in paths yet')
        }
        var name = this.lvalue.name.textValue
        lvalue = new AST.Path(name)
      }
      return new AST.Assignment(type, lvalue, rvalue)
    }
  },
  Comment: {type: 'comment', transform: function () { return null }},
  Block: {type: 'block',
    transform: function () {
      var statements = transform_statement_list(this.statement_list)
      return new AST.Block(statements)
    }
  },
  Root: {
    type: 'root',
    transform: function () {
      var statements = transform_statement_list(this)
      return new AST.Root(statements)
    }
  }
})

var Parser = function () {}
Parser.prototype.parse = function (code) {
  var parseTree = grammar.parse(code), ast = null
  try {
    ast = this.transform(parseTree)
  } catch(err) {
    stderr.write("Transform error:\n")
    stderr.write(err.stack)
    stderr.write("\n")
    return false
  }
  return ast
}

// Take a parse tree from the grammar and transform it into an AST.
Parser.prototype.transform = function (tree) {
  var newTree = tree.transform()
  return newTree
}

module.exports = Parser
