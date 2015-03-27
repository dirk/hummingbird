
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

describe('System', function () {
  describe('given a for loop program', function () {
    var tree = null
    it('should parse', function () {
      tree = parseAndWalk(programs.forLoop)
      expect(tree).to.be.ok()
    })
    it('should produce the right result when executed', function () {
      var result = runCompiledCode(tree)
      expect(result).to.eql(10)
    })
  })
  xdescribe('given a while-true program', function () {
    var tree = null
    it('should parse', function () {
      tree = parseAndWalk(programs.whileTrue)
      expect(tree).to.be.ok()
    })
    it('should produce the correct result when executed', function () {
      var result = runCompiledCode(tree)
      expect(result).to.eql(5)
    })
  })
})
>>>>>>> Switch to Mocha
