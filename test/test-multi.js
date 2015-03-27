
var fs          = require('fs'),
    Parser      = require('../lib/parser'),
    parser      = new Parser(),
    grammar2    = require('../lib/grammar2'),
    TypeSystem  = require('../lib/typesystem').TypeSystem,
    reportError = require('../lib/util').reportError

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

require('../lib/targets/javascript')

console.log(tree.compile())

