
// Full-system tests of Hummingbird parser and compiler.

var fs           = require('fs'),
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

  describe('given a string literal', function () {
    var source = "var a = \"Hello world.\"\nreturn a"
    it('should parse and produce the correct result', function () {
      var tree = parseAndWalk(source)
      expect(tree).to.be.ok()
      var result = runCompiledCode(tree)
      expect(result).to.eql('Hello world.')
    })
  })

  describe('given a class', function () {
    describe('with an invalid let-property', function () {
      var source = "var a = func () -> Integer { return 1 }\n"+
                   "class B { var c: () -> Integer = a }\n"
      it('should fail to parse', function () {
        expect(function () {
          parseAndWalk(source)
        }).to.throwException(/non-literal default for property/)
      })
    })

    describe('with a valid definition', function () {
      var source = "class A {\n"+
                   "  var b: Integer = 1\n"+
                   "  init () { this.b = this.b + 1 }\n"+
                   "  func c () -> Integer { return this.b + 1 }\n"+
                   "}\n"+
                   "var a = new A()\n"+
                   "return a.c()\n"
      var tree = null
      it('should parse', function () {
        tree = parseAndWalk(source)
        expect(tree).to.be.ok()
      })
      it('should product the correct result', function () {
        var result = runCompiledCode(tree)
        expect(result).to.eql(3)
      })
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
