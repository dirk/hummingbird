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

var commands = {
  inspect: function (args) {
    var file = args[0]
    if (!file) {
      stderr.write("Missing file to inspect\n")
      return process.exit(2)
    }
    var contents = fs.readFileSync(file).toString(),
        name     = path.basename(file),
        parser   = new Parser()

    parser.file = name
    // Parse and type-check the file; catch and report any errors
    try {
      var tree       = parser.parse(contents),
          typesystem = new TypeSystem()
      typesystem.walk(tree)
      process.exit(0)
    } catch (err) {
      reportError(err)
      process.exit(1)
    }
  }
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

