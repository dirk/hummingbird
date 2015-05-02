var expect        = require('expect.js'),
    child_process = require('child_process')

function spawnSync (cmd, args, input) {
  var opts = {
    stdio: 'pipe',
    env:   process.env
  }
  if (input !== undefined) {
    opts.input = input
  }
  if (args === undefined) { args = [] }
  return child_process.spawnSync(cmd, args, opts)
}

function checkResult (result) {
  // Print out the output streams if the status wasn't what we expected
  if (result.error) {
    throw result.error
  }
  var err = (result.stderr ? result.stderr : 'Missing STDERR').toString().trim()
  if (err.length > 0) {
    console.error(err)
  }
  // if (result.status !== 0) {
  //   console.log((result.stdout ? result.stdout : 'Missing STDOUT').toString())
  // }
  expect(result.status).to.eql(0)
  return result
}

function runSync (cmd, input) {
  var opts = {}
  if (input !== undefined) {
    opts.input = input
  }
  try {
    return child_process.execSync(cmd, opts)
  } catch (err) {
    if (err.stdout) {
      console.error(err.stdout.toString())
    }
    if (err.stderr) {
      console.error(err.stderr.toString())
    }
    throw err
  }
}

module.exports = {
  runSync:      runSync,
  spawnSync:    spawnSync,
  checkResult:  checkResult
}

