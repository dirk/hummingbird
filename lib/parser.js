
var grammar2 = require('./grammar2'),
    AST     = require('./ast'),
    types   = require('./types')
    stderr  = process.stderr,
    _       = require('lodash')

var extend = require('util')._extend

var Parser = function () {}
Parser.prototype.parse = function (code) {
  var tree
  try {
    tree = grammar2.parse(code)
  } catch(e) {
    stderr.write(e.name+': '+e.message+"\n")
    if (e.name == 'SyntaxError') {
      stderr.write('  Line '+e.line+' Column '+e.column+"\n")
    }
    return false
  }
  return tree
}

module.exports = Parser
