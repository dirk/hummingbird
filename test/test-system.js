
// Full-system tests of Hummingbird parser and compiler.

var fs           = require('fs'),
    vows         = require('vows'),
    expect       = require('expect.js'),
    AST          = require('../lib/ast'),
    types        = require('../lib/types'),
    parseAndWalk = require('./helper').parseAndWalk

var programs = {
  forLoop: fs.readFileSync(__dirname+'/system/for-loop.hb').toString(),
  whileTrue: fs.readFileSync(__dirname+'/system/while-true.hb').toString()
}

function runCompiledCode (tree) {
  // Wrap the compiled code in an immediately-called function
  return eval("(function(){\n"+tree.compile()+"\n})()")
}

vows.describe('Hummingbird').addBatch({
  'System': {
    'given a for loop program': {
      topic: parseAndWalk(programs.forLoop),
      'should parse': function (tree) {
        expect(tree).to.be.ok()
      },
      'when executed': {
        topic: function (tree) { return runCompiledCode(tree) },
        'should produce the right result': function (result) {
          expect(result).to.eql(10)
        }
      }
    },
    'given a while-true program': {
      topic: parseAndWalk(programs.whileTrue),
      'should parse': function (tree) {
        expect(tree).to.be.ok()
      },
      'when executed': {
        topic: function (tree) { return runCompiledCode(tree) },
        'should produce the right result': function (result) {
          expect(result).to.eql(5)
        }
      }
    }
  }
}).export(module)
