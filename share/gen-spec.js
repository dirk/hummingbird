var fs = require('fs'),
    parser = require('../src/spec-parser'),
    parseSpecification = parser.parseSpecification

var specSource   = fs.readFileSync(__dirname+'/../doc/specification.md').toString(),
    runnerSource = fs.readFileSync(__dirname+'/../share/spec-runner.js').toString()

var specs = parseSpecification(specSource)

var specDir = __dirname+'/../test/spec'

// Clear out test/spec directory
function removeExisting () {
  var files = fs.readdirSync(specDir)
}
removeExisting()


// Now generate the specs
for (var i = specs.length - 1; i >= 0; i--) {
  var spec = specs[i]

  var js = spec.js+"\n",
      hb = spec.hb+"\n"
  fs.writeFileSync(specDir+'/source-'+spec.name+'.js', js)
  fs.writeFileSync(specDir+'/source-'+spec.name+'.hb', hb)

  var runner = runnerSource.replace(/NAME/g, spec.name)
  fs.writeFileSync(specDir+'/test-'+spec.name+'.js', runner)
  
  console.log('Generated tests for: '+spec.name)
}
