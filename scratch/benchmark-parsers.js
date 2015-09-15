var ParserJison = require('../src/parser'),
    GrammarPegjs = require('../src/grammar.pegjs.js')

var AST   = require('../src/ast'),
    types = require('../src/types'),
    fs    = require('fs'),
    Benchmark = require('benchmark')

var parserJison = new ParserJison()

function parsePegjs (code) {
  return GrammarPegjs.parse(code, {file: '(unknown)'})
}
function parseJison (code) {
  return parserJison.parse(code)
}

var exampleCode = fs.readFileSync(__dirname+'/../examples/fibonacci.hb', 'utf8')

var suite = new Benchmark.Suite()

suite
var jisonBenchmark = new Benchmark('Jison', function () {
  parseJison(exampleCode)
})
var pegjsBenchmark = new Benchmark('PEG.js', function () {
  parsePegjs(exampleCode)
})

;[jisonBenchmark, pegjsBenchmark].forEach(function (benchmark) {
  benchmark.options.minSamples = 100
  var result = benchmark.run()
  console.log(result.toString())
})
