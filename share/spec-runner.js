var _       = require('lodash'),
    fs      = require('fs'),
    expect  = require('expect.js'),
    esprima = require('esprima'),
    parseAndWalk = require('../helper').parseAndWalk

describe('NAME spec', function () {
  var hbSource, jsSource
  it('should read Hummingbird file', function (done) {
    fs.readFile(__dirname+'/source-NAME.hb', function (err, data) {
      if (err) { return done(err) }
      hbSource = data.toString()
      done()
    })
  })
  it('should read JavaScript file', function (done) {
    fs.readFile(__dirname+'/source-NAME.js', function (err, data) {
      if (err) { return done(err) }
      jsSource = data.toString()
      done()
    })
  })

  var tree
  it('should parse', function () {
    tree = parseAndWalk(hbSource)
    expect(tree).to.be.ok()
  })
  // Pull in the JavaScript target
  require('../../lib/targets/javascript')

  var compiledSource
  it('should compile', function () {
    compiledSource = tree.compile()
    expect(compiledSource).to.be.ok()
  })

  it('should produce expected output', function () {
    var sourceTree   = esprima.parse(jsSource),
        compiledTree = esprima.parse(compiledSource)
    // Create a comparator function
    var matches = _.matches(sourceTree)
    // Compare the compile trees and make sure they're identical
    expect(matches(compiledTree)).to.be(true)
  })
})

