
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

module.exports = {repeat: repeat}
