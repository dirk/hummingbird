var expect        = require('expect.js'),
    child_process = require('child_process')

function spawnSync (cmd, args, input) {
  var opts = {
    stdio: 'pipe'
  }
  if (input !== undefined) {
    opts.input = input
  }
  if (args === undefined) { args = [] }
  return child_process.spawnSync(cmd, args, opts)
}

function checkResult (result) {
  // Print out the output streams if the status wasn't what we expected
  if (result.status !== 0) {
    console.log((result.stdout ? result.stdout : 'Missing STDOUT').toString())
  }
  var err = (result.stderr ? result.stderr : 'Missing STDERR').toString().trim()
  if (err.length > 0) {
    console.error(err)
  }
  expect(result.status).to.eql(0)
}

module.exports = {
  spawnSync:    spawnSync,
  checkResult:  checkResult
}

