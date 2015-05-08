var helper = require('./helper'),
    fs     = require('fs'),
    path   = require('path'),
    expect = require('expect.js'),
    child_process = require('child_process')

var checkResult = helper.checkResult,
    spawnSync   = helper.spawnSync,
    runSync     = helper.runSync

describe('LLVM compiler', function () {
  var BinFile = path.join(__dirname, 'a.out')

  // Need to go through the spawn system since we want to pass it STDIN
  function buildSource (source) {
    var result = runSync('bin/hbn - -o '+BinFile, source),
        output = result.toString().trim()
    if (output.length) {
      console.error(output)
    }
    return result
  }
  function runBinary (otherBin) {
    var bin = (otherBin ? otherBin : BinFile)
    return runSync(bin)
  }

  function testCompileAndRun (source, expectedResult) {
    it('should compile', function () {
      buildSource(source)
    })
    it('should run', function () {
      var result = runBinary().toString()
      if (expectedResult) {
        expect(result.trim()).to.eql(expectedResult)
      }
    })
  }

  describe('given a trivial program', function () {
    var source = "var a = \"1\"\n"+
                 "console.log(a)";
    it('should compile', function () {
      buildSource(source)
    })
    it('should run', function () {
      fs.chmodSync(BinFile, '755')
      var result = runBinary(),
          out    = result.toString()
      expect(out.trim()).to.eql('1')
    })
  })

  describe('given a class', function () {
    var source = "class A {\n"+
                 "  var b: String\n"+
                 "  init () { this.b = \"Hello world!\" }\n"+
                 "  func c () { return this.b }\n"+
                 "}\n"+
                 "var a = new A()\n"+
                 "console.log(a.c())"
    testCompileAndRun(source, "Hello world!")
  })


  describe('given strings to concatenate', function () {
    var source = "var a = func (n: String) { return \"a\" + n }\n"+
                 "var b = func () { return \"b\" }\n"+
                 "console.log(a(b()))"
    testCompileAndRun(source, "ab")
  })

  describe('given a class with a default', function () {
    var source = "class A {\n"+
                 "  var b: String = \"c\"\n"+
                 "  init () { }\n"+
                 "}\n"+
                 "var a = new A()\n"+
                 "console.log(a.b)"
    testCompileAndRun(source, "c")
  })
})

