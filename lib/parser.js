
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
