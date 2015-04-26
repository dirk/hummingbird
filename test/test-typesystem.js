// Pull in the helper to make sure we're tracking coverage
var helper = require('./helper'),
    expect = require('expect.js')

var AST          = require('../src/ast'),
    types        = require('../src/types'),
    TypeSystem   = require('../src/typesystem').TypeSystem,
    scope        = require('../src/typesystem/scope'),
    Scope        = scope.Scope,
    ClosingScope = scope.ClosingScope

describe('Type-system', function () {
  var rootObject = new types.Object('fake')
  rootObject.supertype = null
  rootObject.isRoot    = true

  it('should fail finding with non-string name', function () {
    var typesystem = new TypeSystem()
    expect(function () {
      var ret = typesystem.findByName(null)
    }).to.throwException()
  })

  describe('given a class definition function node', function () {
    it('should fail when function name is missing', function () {
      var typesystem = new TypeSystem(),
          node       = new AST._Function([], null, null)
      expect(function () {
        typesystem.visitClassFunction(node, null, null)
      }).to.throwException(/Missing function name/)
    })
  })

  describe('given a return statement', function () {
    var typesystem = new TypeSystem()
    it('should fail when expression is undefined', function () {
      var node = new AST.Return(undefined)
      expect(function () {
        typesystem.visitReturn(node, null)
      }).to.throwException(/undefined expression in Return/)
    })
    it('should have Void type when expression is null', function () {
      var node  = new AST.Return(null),
          scope = new Scope()
      typesystem.visitReturn(node, scope)
      var typeInstance = node.type
      expect(typeInstance).to.be.a(types.Instance)
      var type = typeInstance.type
      expect(type).to.be.a(types.Void)
    })
  })
})

describe('Scope', function () {
  var scope  = new Scope(),
      Integer = new types.Integer('fake')
  it('should set a variable', function () {
    scope.setLocal('Integer', Integer)
  })
  it('should fail when re-setting a variable', function () {
    expect(function () {
      scope.setLocal('Integer', Integer)
    }).to.throwException(/Can't redefine local/)
  })
  it('should fail when finding a missing local', function () {
    expect(function () {
      scope.getLocal('Missing')
    }).to.throwException(/Local not found/)
  })
  it('should fail when finding a missing variable', function () {
    expect(function () {
      scope.get('Missing')
    }).to.throwException(/Not found/)
  })
})

