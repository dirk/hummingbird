
var fs     = require('fs'),
    Parser = require('./lib/parser'),
    parser = new Parser(),
    grammar2 = require('./lib/grammar2'),
    TypeSystem = require('./lib/typesystem').TypeSystem

var code  = fs.readFileSync('./examples/fibonacci.hb').toString()
var input = "let a = (2 + 3) + 4"

try {
  var tree = grammar2.parse(code)  
} catch (e) {
  var stdout = process.stdout
  stdout.write(e.name+': '+e.message+"\n")
  if (e.name == 'SyntaxError') {
    stdout.write('  Line '+e.line+' Column '+e.column+"\n")
  }
  process.exit()
}

// var inspect = require('util').inspect
// console.log(inspect(tree, {depth: null}))
// tree.print()

var ts = new TypeSystem()
ts.walk(tree)

// tree.print()

// Load the JavaScript compilation target
require('./lib/targets/javascript')

console.log(tree.compile())
