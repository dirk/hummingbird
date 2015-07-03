var util       = require('util'),
    prettyjson = require('prettyjson'),
    inlineSourceMapComment = require('inline-source-map-comment')

require('source-map-support').install()

var grammar = require('./src/grammar')

var source = "a(b)[c].d[e](f)"

var tree = grammar.parse(source, {})

// console.log(prettyjson.render(tree, {}))

tree.print()
