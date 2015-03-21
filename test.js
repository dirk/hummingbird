
var fs          = require('fs'),
    Parser      = require('./lib/parser'),
    parser      = new Parser(),
    grammar2    = require('./lib/grammar2'),
    TypeSystem  = require('./lib/typesystem').TypeSystem,
    reportError = require('./lib/util').reportError

var code  = fs.readFileSync('./examples/fibonacci.hb').toString()
// var input = "let a = (2 + 3) + 4"
// var input = "var foo: (String) -> Number = func (a: Integer) -> Integer {\nreturn 1\n}\nfoo(1)\n"
// var input = "var f = func (a: Number = 2) -> Number { return a }"
var input = "for var i = 0; i < 2; i += 1 { }"

try {
  parser.file = 'input.hb'
  var tree = parser.parse(input)

  var ts = new TypeSystem()
  ts.walk(tree)
} catch (err) {
  reportError(err)
  process.exit()
}

// var inspect = require('util').inspect
// console.log(inspect(tree, {depth: null}))

// tree.print()

// Load the JavaScript compilation target
require('./lib/targets/javascript')

console.log(tree.compile())

