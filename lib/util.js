
// http://stackoverflow.com/a/5450113
function repeat (pattern, count) {
  if (count < 1) return '';
  var result = ''
  while (count > 1) {
    if (count & 1) result += pattern;
    count >>= 1, pattern += pattern
  }
  return result + pattern
}

var stderr = process.stderr

function reportError (err) {
  stderr.write(err.name+': '+err.message+"\n")
  if (err.name == 'SyntaxError') {
    stderr.write('  Line '+err.line+' Column '+err.column+"\n")
  } else if (err.name == 'TypeError') {
    // If there's position info for us to use
    if (err.origin && err.origin._file && err.origin._line) {
      var file = err.origin._file,
          line = err.origin._line
      stderr.write('  '+file+':'+line+"\n")
    }
    // Otherwise if there's a backtrace to report
    if (err.stack) {
      stderr.write(err.stack)
    }
  }
}

module.exports = {
  repeat:      repeat,
  reportError: reportError
}
