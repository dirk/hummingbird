
var expect       = require('expect.js'),
    types        = require('../src/types'),
    AST          = require('../src/ast'),
    parseAndWalk = require('./helper').parseAndWalk

describe('Parser', function () {
  describe('given a var declaration', function () {
    it('should parse an implicit type', function () {
      var topic = 'var foo = 1',
          tree  = parseAndWalk(topic)
      expect(tree.statements.length).to.eql(1)
      var decl = tree.statements[0]
      // Check that it parsed as a var-assignment with a type of Number
      expect(decl).to.be.an(AST.Assignment)
      expect(decl.type).to.eql('var')
      expect(decl.lvalue.type).to.be.a(types.Instance)
      expect(decl.lvalue.type.type).to.be.a(types.Number)
    })

    it('should parse an explicit type', function () {
      var topic = 'var foo: Integer = 1'
      parseAndWalk(topic)
    })

    it('should fail on an invalid explicit type', function () {
      var topic = 'var foo: Integer = func () -> Integer { return 1 }',
          tree  = false
      try {
        tree = parseAndWalk(topic)
        // tree.print()
      } catch (err) {
        // Caught an error successfully
        expect(err.name).to.eql('TypeError')
        expect(err.message).to.contain('Unequal types in declaration')
        return
      }
      expect(tree).to.be(false)
    })
  })
})
