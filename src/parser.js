if (!process.browser) {
  var fs = require('fs')

  // Before loading the parser let's check to make sure it's up-to-date
  var grammarFile       = __dirname+'/grammar.jison.js',
      grammarSourceFile = __dirname+'/grammar.jison',
      grammarStat       = null,
      grammarSourceStat = null

  // Make sure the grammar has been generated
  try {
    grammarStat = fs.statSync(grammarFile)
  } catch (err) {
    if (err.code === 'ENOENT') {
      process.stderr.write("Missing generated parser file, please run `npm run grammar` to generate it.\n")
      process.exit(1)
    }
    // Don't recognize this error, rethrow
    throw err
  }
  // Now check to make sure that it's up-to-date
  grammarSourceStat = fs.statSync(grammarSourceFile)
  if (grammarSourceStat.mtime > grammarStat.mtime) {
    process.stderr.write("Parser file is out of date, please do `npm run grammar` to re-generate it.\n")
    process.exit(1)
  }
}//if !process.browser

var JisonParser = require('./grammar.jison.js').parser,
    AST         = require('./ast'),
    types       = require('./types')
    stderr      = process.stderr,
    _           = require('lodash')

var extend = require('util')._extend

JisonParser.yy = {
  AST: AST,
  binary: function (token, lexpr, op, rexpr) {
    return JisonParser.yy.node('Binary', token, lexpr, op, rexpr)
  },
  file: '(unknown)',
  node: function () {
    throw 'Must be overridden'
  }
}

var HBParser = function () {
  this.file = '(unknown)'
}
HBParser.prototype.parse = function (code) {
  this.setupParser()
  var tree = JisonParser.parse(code)
  return tree
}

HBParser.prototype.setupParser = function () {
  var parser = this

  JisonParser.yy.node = function () {

    var klass = AST[arguments[0]],
        token = arguments[1],
        rest  = Array.prototype.slice.call(arguments, 2)

    if (!klass) {
      throw 'AST class not found: '+arguments[0]
    }
    if (!token.first_line) {
      throw 'Expected token as second argument'
    }

    var n = null
    switch (rest.length) {
      case 0:
        n = eval('new klass()')
        break
      case 1:
        n = eval('new klass(rest[0])')
        break
      case 2:
        n = eval('new klass(rest[0], rest[1])')
        break
      case 3:
        n = eval('new klass(rest[0], rest[1], rest[2])')
        break
      case 4:
        n = eval('new klass(rest[0], rest[1], rest[2], rest[3])')
        break
      default:
        throw 'Cannot handle node constructor with '+rest.length+' arguments'
    }

    // { first_line: 2, last_line: 2, first_column: 0, last_column: 5 }
    n._line   = token.first_line
    n._column = token.first_column
    n._file   = parser.file
    return n
  }

}// HBParser.prototype.setupParser

module.exports = HBParser
