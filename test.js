
var fs     = require('fs'),
    Parser = require('./lib/parser'),
    parser = new Parser()

var code  = fs.readFileSync('./examples/fibonacci.hb').toString()
var input = "let a: Type = (2 + 3) + 4"

var tree = parser.parse(code)
// console.log(tree.statement.rvalue)
if (tree) tree.print();
