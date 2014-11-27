
var fs     = require('fs'),
    Parser = require('./lib/parser'),
    parser = new Parser(),
    TypeSystem = require('./lib/typesystem').TypeSystem

var code  = fs.readFileSync('./examples/fibonacci.hb').toString()
var input = "let a: Type = (2 + 3) + 4"

var tree = parser.parse(code)
// if (tree) tree.print();

var ts = new TypeSystem()
ts.walk(tree)

// tree.print()

// Load the JavaScript compilation target
require('./lib/targets/javascript')

console.log(tree.compile())
