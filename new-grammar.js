var fs         = require('fs'),
    util       = require('util'),
    jison      = require('jison'),
    prettyjson = require('prettyjson')

function inspect(object) {
  // console.log(util.inspect(object, {depth: 10}))
  console.log(prettyjson.render(object, {}))
}

var bnf    = fs.readFileSync('src/grammar.jison', 'utf8'),
    parser = new jison.Parser(bnf)

var lines = [
  "a(b.c)[0].d ",
  "func e(f) { return f * 2 }",
  "g = func () {}",
  "let h = 1",
  "if true { }",
  "while false { }",
  "for 1; 2; 3 { }",
]

inspect(parser.parse(lines.join("\n")))
