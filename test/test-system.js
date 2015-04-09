
// Full-system tests of Hummingbird parser and compiler.

var fs           = require('fs'),
    expect       = require('expect.js'),
    AST          = require('../src/ast'),
    types        = require('../src/types'),
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

  describe('given a multi definition', function () {
    describe('without a return type', function () {
      it('should fail to parse', function () {
        var source = "multi a (b: Integer, c: Integer)\n"
        expect(function () {
          parseAndWalk(source)
        }).to.throwException(/Missing multi return type/)
      })
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
                   "  func c (d: Integer) -> Integer { return this.b + d }\n"+
                   "}\n"+
                   "var a = new A()\n"
      it('should parse', function () {
        var tree = parseAndWalk(source)
        expect(tree).to.be.ok()
      })
      it('should produce an expected result', function () {
        var extended = source+"return a.c(1)\n",
            tree     = parseAndWalk(extended),
            result   = runCompiledCode(tree)
        expect(result).to.eql(3)
      })
      it('should fail on parsing mismatched argument lengths', function () {
        var extended = source+"a.c(1, 2)\n"
        expect(function () {
          parseAndWalk(extended)
        }).to.throwException(/Wrong number of arguments/)
      })
      it('should fail on parsing mismatched argument types', function () {
        var extended = source+"a.c(\"1\")\n"
        expect(function () {
          parseAndWalk(extended)
        }).to.throwException(/Argument mismatch/)
      })
    })

    describe('with a let-property in its definition', function () {
      var source = "class A {\n"+
                   "  let b: Integer = 0\n"+
                   "  init () { }\n"+
                   "}\n"+
                   "var a = new A()\n"
      it('should parse the basic definition', function () {
        var tree = parseAndWalk(source)
        expect(tree).to.be.ok()
      })
      it('should error when trying to write to that property', function () {
        var extendedSource = source+"a.b = 1"
        expect(function () {
          parseAndWalk(extendedSource)
        }).to.throwException(/assign to read-only property/)
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
    describe('with more than one inferred return type', function () {
      var source = "var a = func () {\n"+
                   "  if 1 { return 1 } else { return \"1\" }\n"+
                   "}\n"
      it('should fail to parse', function () {
        expect(function () {
          parseAndWalk(source)
        }).to.throwException(/Too many return types/)
      })
    })
    describe('with mismatched return types', function () {
      var source = "var a = func () -> String { return 1 }\n"
      it('should fail to parse', function () {
        expect(function () {
          parseAndWalk(source)
        }).to.throwException(/Type returned by function does not match declared return type/)
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
