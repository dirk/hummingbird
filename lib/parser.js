var fs = require('fs')

// Before loading the parser let's check to make sure it's up-to-date
var grammarFile       = __dirname+'/grammar.js',
    grammarSourceFile = __dirname+'/grammar.pegjs',
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

var grammar = require('./grammar'),
    AST     = require('./ast'),
    types   = require('./types')
    stderr  = process.stderr,
    _       = require('lodash')

var extend = require('util')._extend

var Parser = function () {
  this.file = '(unknown)'
}
Parser.prototype.parse = function (code) {
  var tree
  tree = grammar.parse(code, {file: this.file})
  return tree
}

module.exports = Parser
