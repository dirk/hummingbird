
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
      }
    }
  }
}).export(module)
