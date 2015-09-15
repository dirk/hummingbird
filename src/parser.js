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

function annotateNode (node, token) {
  node._line   = token.first_line
  node._column = token.first_column
  node._file   = JisonParser.yy.file
  return node
}
function getRestOfArguments (args, length) {
  var rest = Array.prototype.slice.call(args, 2)
  
  if (rest.length !== length) {
    throw new Error('Expected '+length+' additional arguments, got '+rest.length)
  }
  return rest
}

JisonParser.yy = {
  AST: AST,
  binary: function (token, lexpr, op, rexpr) {
    return JisonParser.yy.node3('Binary', token, lexpr, op, rexpr)
  },
  extendIf: function (_if, _else_ifs, _else) {
    if (_else_ifs) { _if.elseIfs = _else_ifs }
    if (_else)     { _if.elseBlock = _else }
    return _if
  },
  node1: function (klass, token) {
    var rest = getRestOfArguments(arguments, 1)
    return annotateNode(new AST[klass](rest[0]), token)
  },
  node2: function (klass, token) {
    var rest = getRestOfArguments(arguments, 2)
    return annotateNode(new AST[klass](rest[0], rest[1]), token)
  },
  node3: function (klass, token) {
    var rest = getRestOfArguments(arguments, 3)
    return annotateNode(new AST[klass](rest[0], rest[1], rest[2]), token)
  },
  node4: function (klass, token) {
    var rest = getRestOfArguments(arguments, 4)
    return annotateNode(new AST[klass](rest[0], rest[1], rest[2], rest[3]), token)
  },

  // Set by a HBParser instance
  file: '(unknown)',
}

var HBParser = function () {
  this.file = '(unknown)'
}
HBParser.prototype.parse = function (code) {
  this.setupParser()
  return JisonParser.parse(code)
}
HBParser.prototype.setupParser = function () {
  JisonParser.yy.file = this.file
}

module.exports = HBParser
