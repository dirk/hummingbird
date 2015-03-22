var stderr = process.stderr,
    fs     = require('fs'),
    path   = require('path')

var TypeSystem  = require('./typesystem').TypeSystem,
    Parser      = require('./parser'),
    reportError = require('./util').reportError

// Parse command-line arguments
var argv = require('optimist')
    .usage('Usage: hb [command] [options]')
    .demand(1)
    .argv

// Utility function to read a file, parse it, walk it, and then return
// the finalized tree. On error it prints a nice error report and then
// calls `process.exit`.
function treeForFile (file) {
  var contents = fs.readFileSync(file).toString(),
      name     = path.basename(file),
      parser   = new Parser()

  parser.file = name
  // Parse and type-check the file; catch and report any errors
  try {
    var tree       = parser.parse(contents),
        typesystem = new TypeSystem()
    typesystem.walk(tree)
  } catch (err) {
    reportError(err)
    process.exit(1)
  }
  return tree
}// treeForFile

function parseAndWalk (file, code) {
  var parser     = new Parser(),
      typesystem = new TypeSystem()
  parser.file = file
  var tree = parser.parse(contents)
  typesystem.walk(tree)
  return tree
}

var commands = {
  inspect: function (args) {
    var file = args[0]
    if (!file) {
      stderr.write("Missing file to inspect\n")
      return process.exit(2)
    }
    var tree = treeForFile(args[0])
    // Print the resulting prettified AST
    tree.print()
    process.exit(0)
  },// inspect
}

function run () {
  var commandArg = argv._[0],
      otherArgs  = argv._.slice(1),
      command    = commands[commandArg]

  if (!command) {
    return stderr.write("Unrecognized command '"+commandArg+"'\n")
  }
  command(otherArgs)
}

module.exports = {run: run}

