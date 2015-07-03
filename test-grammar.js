var util       = require('util'),
    fs         = require('fs'),
    prettyjson = require('prettyjson'),
    inlineSourceMapComment = require('inline-source-map-comment')

require('source-map-support').install()

var grammar = require('./src/grammar')

// var source = "a(b)[c].d[e](f)"
var source = fs.readFileSync('examples/classes.hb').toString()

var tree = grammar.parse(source, {})

// console.log(prettyjson.render(tree, {}))

tree.dump()
