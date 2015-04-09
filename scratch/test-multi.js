
var fs          = require('fs'),
    Parser      = require('../src/parser'),
    parser      = new Parser(),
    TypeSystem  = require('../src/typesystem').TypeSystem,
    reportError = require('../src/util').reportError

var input = fs.readFileSync(__dirname + '/../examples/multi.hb').toString()

try {
  parser.file = 'input.hb'
  var tree = parser.parse(input)

  var ts = new TypeSystem()
  ts.walk(tree)
} catch (err) {
  reportError(err)
  process.exit()
}

require('../src/targets/javascript')

console.log(tree.compile())

