
function Spec (name, js, hb) {
  this.name = name
  this.js   = js
  this.hb   = hb
}

function extractJS (source) {
  return /``js\s([^(``)]+)```/gi.exec(source)[1]
}
function extractHB (source) {
  return /``hb\s([^(``)]+)```/gi.exec(source)[1]
}

function parseSpecification (source) {
  var openTagRegexp = new RegExp('<spec name="([0-9A-Za-z-_]+)">', 'g')
  var closingTag = '</spec>'

  var specs = [],
      match = null

  while ((match = openTagRegexp.exec(source)) != null) {
    var openingIndex = match.index,
        closingIndex = source.indexOf(closingTag, openingIndex)
    // String source of the spec
    var spec = source.slice(openingIndex + match[0].length, closingIndex)
    spec = spec.trim()
    // Pull out the name of the spec
    var name = match[1]
    // Extract the two sources from the spec body
    var jsSource, hbSource;
    try {
      jsSource = extractJS(spec)
    } catch(err) {
      throw new Error('Failed to find JavaScript source in '+name+' spec')
    }
    try {
      hbSource = extractHB(spec)
    } catch(err) {
      throw new Error('Failed to find Hummingbird source in '+name+' spec')
    }
    specs.push(new Spec(name, jsSource.trim(), hbSource.trim()))
  }
  return specs
}// parseSpecification

module.exports = {
  parseSpecification: parseSpecification,
  Spec: Spec
}
