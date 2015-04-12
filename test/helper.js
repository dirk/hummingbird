// Initialize blanket test coverage library; every test suite that we want
// to have code coverage must require this helper file to get it!
require('blanket')({
  "pattern": [""],
  "data-cover-never": [
    "src/ast",
    "src/grammar",
    "src/parser",
    "src/util",
    "node_modules",
    "test"
  ]
})

var types      = require('../src/types'),
    Parser     = require('../src/parser'),
    TypeSystem = require('../src/typesystem').TypeSystem,
    parser     = new Parser()

// Load the JavaScript compilation target
require(__dirname+'/../src/targets/javascript')

var parseAndWalk = function (source, filename) {
  parser.file = filename ? filename : 'unknown'
  var tree       = parser.parse(source),
      typeSystem = new TypeSystem()
  typeSystem.walk(tree)
  return tree
}

module.exports = {
  parser:       parser,
  parseAndWalk: parseAndWalk
}
