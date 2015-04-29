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
    var result = runSync('bin/hbn - -v -o '+BinFile, source)
    console.log(result.toString().trim())
    return result
  }
  function runBinary (otherBin) {
    var bin = (otherBin ? otherBin : BinFile)
    return runSync(bin)
  }

  describe('given a trivial program', function () {
    var source = "var a = \"1\"\n"+
                 "console.log(a)";
    it('should compile', function () {
      buildSource(source)
    })
    it('should run', function () {
      console.log(runSync('ls -l '+__dirname).toString().trim())
      // console.log(binFile)
      // console.log(fs.existsSync(binFile))
      // var result = child_process.execSync(binFile)
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
    it('should compile', function () {
      buildSource(source)
    })
    it('should run', function () {
      var result = runBinary(),
          out    = result.toString()
      // var out = result.stdout.toString()
      expect(out).to.eql("Hello world!\n")
    })
  })

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

  describe('given strings to concatenate', function () {
    var source = "var a = func (n: String) { return \"a\" + n }\n"+
                 "var b = func () { return \"b\" }\n"+
                 "console.log(a(b()))"
    testCompileAndRun(source, "ab")
  })
})

