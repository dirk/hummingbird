var helper = require('./helper'),
    expect = require('expect.js')

var checkResult = helper.checkResult,
    spawnSync   = helper.spawnSync

describe('LLVM compiler', function () {
  describe('given a trivial program', function () {
    var source = "var a = \"1\"\n"+
                 "console.log(a)";
    it('should compile', function () {
      var result = spawnSync('bin/hbn', ['-'], source)
      checkResult(result)
    })
    it('should run', function () {
      var result = spawnSync('./a.out'),
          out    = result.stdout.toString()
      expect(result.status).to.eql(0)
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
      checkResult(spawnSync('bin/hbn', ['-'], source))
    })
    it('should run', function () {
      var result = spawnSync('./a.out'),
          out    = result.stdout.toString()
      expect(result.status).to.eql(0)
      expect(out).to.eql("Hello world!\n")
    })
  })
})

