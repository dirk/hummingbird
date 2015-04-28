var expect        = require('expect.js'),
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

module.exports = {
  spawnSync:    spawnSync,
  checkResult:  checkResult
}

