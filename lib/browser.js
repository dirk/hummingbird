// This is the target file for our browserification. It presents a unified
// external API to browser clients.

var TypeSystem = require('./typesystem').TypeSystem,
    Parser     = require('./parser'),
    AST        = require('./ast'),
    types      = require('./types')

require('./targets/javascript')

function parseAndWalk (code) {
  var parser     = new Parser(),
      typesystem = new TypeSystem(),
      tree       = parser.parse(code)
  typesystem.walk(tree)
  return tree
}

module.exports = {
  TypeSystem:   TypeSystem,
  Parser:       Parser,
  AST:          AST,
  types:        types,

  parseAndWalk: parseAndWalk
}

