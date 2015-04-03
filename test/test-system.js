
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

  describe('given boolean literals', function () {
    var source = "var a = true\nvar b = false"
    it('should parse and walk', function () {
      var tree = parseAndWalk(source)
      expect(tree.statements.length).to.eql(2)
      var truthyNode = tree.statements[0],
          falseyNode = tree.statements[1]
      expect(truthyNode.rvalue.value).to.eql('true')
      expect(falseyNode.rvalue.value).to.eql('false')
      // Check the node's computed types
      function checkBooleanNodeType (node) {
        var instanceType = node.lvalue.type
        expect(instanceType).to.be.a(types.Instance)
        var type = instanceType.type
        expect(type).to.be.a(types.Boolean)
      }
      checkBooleanNodeType(truthyNode)
      checkBooleanNodeType(falseyNode)
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

    describe('with an init declaration', function () {
      function checkTree (tree) {
        expect(tree).to.be.ok()
        expect(tree.statements.length).to.eql(1)
        var klass = tree.statements[0]
        expect(klass.name).to.eql('A')
        expect(klass.type.getTypeOfProperty('b')).to.be.ok()
        expect(klass.type.initializers.length).to.eql(1)
      }
      it('should parse the first formulation', function () {
        var source = "class A { var b: Integer; init() { } }"
        checkTree(parseAndWalk(source))
      })
      it('should parse the second formulation', function () {
        var source = "class A { var b: Integer; init () { } }"
        checkTree(parseAndWalk(source))
      })
    })
  })

  describe('given an else-if', function () {
    var preamble = "var a = 1\n"
    function checkTree (tree) {
      expect(tree).to.be.ok()
      var root = tree
      expect(root.statements.length).to.eql(2)
      // The if condition is second
      var i = root.statements[1]
      expect(i).to.be.an(AST.If)
      expect(i.elseIfs).to.be.ok()
      expect(i.elseIfs.length).to.eql(1)
      expect(i.elseBlock).to.be(null)
      // Check the single else-if
      var ei = i.elseIfs[0]
      expect(ei).to.be.an(AST.If)
      expect(ei.elseIfs).to.be(null)
      expect(ei.elseBlock).to.be(null)
    }
    it('should parse the first formulation', function () {
      var source = preamble+
                   "if a { }\n"+
                   "else if a { }\n"
      var tree = parseAndWalk(source)
      checkTree(tree)
    })
    it('should parse the second formulation', function () {
      var source = preamble+
                   "if a {\n"+
                   "} else if a { }\n"
      expect(parseAndWalk(source)).to.be.ok()
    })
    it('should parse the third formulation', function () {
      var source = preamble+
                   "if a { } else\n"+
                   "if a { }\n"
      expect(parseAndWalk(source)).to.be.ok()
    })
  })

  describe('given a function', function () {
    function testParsingAndResult (source, expectedResult) {
      var tree = null
      it('should parse', function () {
        tree = parseAndWalk(source)
        expect(tree).to.be.ok()
      })
      it('should produced the expected result', function () {
        var result = runCompiledCode(tree)
        expect(result).to.eql(expectedResult)
      })
    }
    describe('with an inferred return', function () {
      var source = "var a = func () { return 1 }\n"+
                   "return a()"
      testParsingAndResult(source, 1)
    })
    describe('with an explicit return', function () {
      var source = "var a = func () -> Integer { return 2 }\n"+
                   "return a()"
      testParsingAndResult(source, 2)
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
