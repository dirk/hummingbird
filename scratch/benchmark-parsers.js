var ParserJison = require('../src/parser'),
    GrammarPegjs = require('../src/grammar.pegjs.js')

var AST         = require('../src/ast'),
    types       = require('../src/types'),
    reportError = require('../src/util').reportError,
    fs          = require('fs'),
    Benchmark   = require('benchmark')

var parserJison = new ParserJison()

function parsePegjs (code) {
  return GrammarPegjs.parse(code, {file: '(unknown)'})
}
function parseJison (code) {
  return parserJison.parse(code)
}

var exampleCode = fs.readFileSync(__dirname+'/../examples/all-2.hb', 'utf8')
try {
  parseJison(exampleCode)
} catch (err) {
  reportError(err)
  process.exit(0)
}

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
