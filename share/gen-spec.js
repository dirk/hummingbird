var fs = require('fs')

var openTagRegexp = /<spec name="([0-9A-Za-z-_]+)">/g

function extractJS (source) {
  return /``js\s([^(``)]+)```/gi.exec(source)[1]
}
function extractHB (source) {
  return /``hb\s([^(``)]+)```/gi.exec(source)[1]
}

function Spec (name, js, hb) {
  this.name = name
  this.js   = js
  this.hb   = hb
}

var specSource = fs.readFileSync(__dirname+'/../doc/specification.md').toString()

var specs = [],
    match = null

while ((match = openTagRegexp.exec(specSource)) != null) {
  var openingIndex = match.index,
      closingIndex = specSource.indexOf('</spec>', openingIndex)
  // String source of the spec
  var source = specSource.slice(openingIndex + match[0].length, closingIndex)
  source = source.trim()
  // Pull out the name of the spec
  var name = match[1]
  // Extract the two sources from the spec body
  var jsSource, hbSource;
  try {
    jsSource = extractJS(source)
  } catch(err) {
    throw new Error('Failed to find JavaScript source in '+name+' spec')
  }
  try {
    hbSource = extractHB(source)
  } catch(err) {
    throw new Error('Failed to find Hummingbird source in '+name+' spec')
  }
  specs.push(new Spec(name, jsSource.trim(), hbSource.trim()))
}

console.log(specs)

