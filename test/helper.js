
var types      = require('../lib/types'),
    Parser     = require('../lib/parser'),
    TypeSystem = require('../lib/typesystem').TypeSystem,
    parser     = new Parser()

// Load the JavaScript compilation target
require(__dirname+'/../lib/targets/javascript')

var parseAndWalk = function (source) {
  var tree       = parser.parse(source),
      typeSystem = new TypeSystem()
  typeSystem.walk(tree)
  return tree
}

module.exports = {
  parser:       parser,
  parseAndWalk: parseAndWalk
}
