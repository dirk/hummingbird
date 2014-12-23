
var Parser     = require('../lib/parser'),
    parser     = new Parser(),
    types      = require('../lib/types'),
    TypeSystem = require('../lib/typesystem').TypeSystem

// Load the JavaScript compilation target
require(__dirname+'/../lib/targets/javascript')

var parseAndWalk = function (source) {
  var tree = parser.parse(source)
  var typeSystem = new TypeSystem()
  typeSystem.walk(tree)
  return tree
}

module.exports = {
  parser: parser,
  parseAndWalk: parseAndWalk
}
