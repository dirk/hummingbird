// Pull in the helper to make sure we're tracking coverage
var helper = require('./helper'),
    expect = require('expect.js')

var types = require('../src/types')

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
    var Integer = new types.Integer(rootObject)
    it('should create a property', function () {
      object.setTypeOfProperty('present', Integer)
    })
    it('should get that property', function () {
      var type = object.getTypeOfProperty('present')
      expect(type).to.eql(Integer)
    })
  })

  describe('given a Function', function () {
    var fn     = null,
        Integer = new types.Integer(rootObject),
        String = new types.String(rootObject)
    it('should be constructed correctly', function () {
      fn = new types.Function(rootObject, [Integer], String)
      expect(fn.args.length).to.eql(1)
      expect(fn.args[0]).to.eql(Integer)
      expect(fn.ret).to.eql(String)
    })
    it('should not equal non-Function type', function () {
      expect(fn.equals(Integer)).to.be(false)
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
      var otherFn = new types.Function(rootObject, [Integer], Integer)
      expect(fn.equals(otherFn)).to.be(false)
    })
  })
})

