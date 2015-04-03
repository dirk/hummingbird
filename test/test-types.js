// Pull in the helper to make sure we're tracking coverage
var helper = require('./helper'),
    expect = require('expect.js')

var types = require('../lib/types')

describe('Types', function () {
  var rootObject = new types.Object('fake')
  rootObject.supertype = null
  rootObject.isRoot    = true

  describe('given an Object', function () {
    var object = null
    it('should be constructed correctly', function () {
      object = new types.Object(rootObject)
      expect(object.supertype).to.eql(rootObject)
      expect(object.properties).to.eql({})
      expect(object.initializers).to.eql([])
    })
    it('should throw an error on missing property', function () {
      expect(function () {
        var type = object.getTypeOfProperty('missing')
      }).to.throwException()
    })
    var Number = new types.Number(rootObject)
    it('should create a property', function () {
      object.setTypeOfProperty('present', Number)
    })
    it('should get that property', function () {
      var type = object.getTypeOfProperty('present')
      expect(type).to.eql(Number)
    })
  })

  describe('given a Function', function () {
    var fn     = null,
        Number = new types.Number(rootObject),
        String = new types.String(rootObject)
    it('should be constructed correctly', function () {
      fn = new types.Function(rootObject, [Number], String)
      expect(fn.args.length).to.eql(1)
      expect(fn.args[0]).to.eql(Number)
      expect(fn.ret).to.eql(String)
    })
    it('should not equal non-Function type', function () {
      expect(fn.equals(Number)).to.be(false)
    })
    it('should not equal Function with different number of arguments', function () {
      var otherFn = new types.Function(rootObject, [], String)
      expect(fn.equals(otherFn)).to.be(false)
    })
    it('should not equal Function with different types of arguments', function () {
      var otherFn = new types.Function(rootObject, [String], String)
      expect(fn.equals(otherFn)).to.be(false)
    })
    it('should not equal Function with different return type', function () {
      var otherFn = new types.Function(rootObject, [Number], Number)
      expect(fn.equals(otherFn)).to.be(false)
    })
  })
})

