var fs         = require('fs'),
    util       = require('util'),
    jison      = require('jison'),
    prettyjson = require('prettyjson'),
    AST        = require('./src/ast')

require('source-map-support').install()

function inspect(object) {
  // console.log(util.inspect(object, {depth: 10}))
  console.log(prettyjson.render(object, {}))
}

var bnf    = fs.readFileSync('src/grammar.jison', 'utf8'),
    parser = new jison.Parser(bnf)

parser.yy = {
  AST: AST,
  binary: function (lexpr, op, rexpr) {
    return new AST.Binary(lexpr, op, rexpr)
  }
}

/*
var lines = [
  "a(b.c)[0].d ",
  "func e(f: G) { return f * 2 }",
  "g = func () {}",
  "let h = 1",
  "if true { }",
  "while false { }",
  "for 1; 2; 3 { }",
]
*/

var file = fs.readFileSync('examples/fibonacci.hb', 'utf8')


var root = parser.parse(file)
root.dump()
