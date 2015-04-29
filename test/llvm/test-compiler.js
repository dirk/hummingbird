var helper = require('./helper'),
    fs     = require('fs'),
    path   = require('path'),
    expect = require('expect.js')

var checkResult = helper.checkResult,
    spawnSync   = helper.spawnSync

describe('LLVM compiler', function () {
  var binFile = path.join(__dirname, 'a.out')

  describe('given a trivial program', function () {
    var source = "var a = \"1\"\n"+
                 "console.log(a)";
    it('should compile', function () {
      checkResult(spawnSync('bin/hbn', ['-', '-o', binFile], source))
    })
    it('should run', function () {
      console.log(binFile)
      console.log(fs.existsSync(binFile))

      fs.chmodSync(binFile, '755')
      var result = spawnSync(binFile)
      checkResult(result)
      var out = result.stdout.toString()
      expect(result.status).to.eql(0)
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
      checkResult(spawnSync('bin/hbn', ['-', '-o', binFile], source))
    })
    it('should run', function () {
      var result = spawnSync(binFile)
      checkResult(result)
      var out = result.stdout.toString()
      expect(out).to.eql("Hello world!\n")
    })
  })
})

