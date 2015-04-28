var fs = require('fs'),
    expect = require('expect.js'),
    child_process = require('child_process')

function spawnSync (cmd, args, input) {
  var opts = {}
  if (input !== undefined) {
    opts.input = input
  }
  if (args === undefined) { args = [] }
  return child_process.spawnSync(cmd, args, opts)
}

function checkResult (result) {
  // Print out the output streams if the status wasn't what we expected
  if (result.status !== 0) {
    console.log(result.stderr.toString())
    console.log(result.stdout.toString())
  }
  expect(result.status).to.eql(0)
  expect(result.stderr.length).to.eql(0)
}

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
})

