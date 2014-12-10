
var vows       = require('vows'),
    expect     = require('expect.js'),
    Parser     = require('../lib/parser'),
    parser     = new Parser(),
    types      = require('../lib/types'),
    TypeSystem = require('../lib/typesystem').TypeSystem,
    AST        = require('../lib/ast')

var parseAndWalk = function (source) {
  var tree = parser.parse(source)
  var typeSystem = new TypeSystem()
  typeSystem.walk(tree)
  return tree
}

vows.describe('Hummingbird').addBatch({
  'Parser': {
    'given a var declaration': {
      'with an implicit type': {
        topic: 'var foo = 1',
        'should parse': function (topic) {
          var tree = parseAndWalk(topic)
          expect(tree.statements.length).to.eql(1)
          var decl = tree.statements[0]
          // Check that it parsed as a var-assignment with a type of Number
          expect(decl).to.be.an(AST.Assignment)
          expect(decl.type).to.eql('var')
          expect(decl.lvalue.type).to.be.a(types.Number)
        }
      },
      'with a valid explicit type': {
        topic: 'var foo: Integer = 1',
        'should parse': function (topic) {
          parseAndWalk(topic)
        }
      },
      'with an invalid explicit type': {
        topic: 'var foo: Integer = func () -> Integer { return 1 }',
        'should fail to parse': function (topic) {
          var tree = false
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
        }
      }
    }
  }
}).export(module)
