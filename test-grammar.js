var util       = require('util'),
    prettyjson = require('prettyjson')

var grammar = require('./src/grammar')

var source = "a(b)[c].d[e](f)"

var tree = grammar.parse(source, {})

console.log(prettyjson.render(tree, {}))
