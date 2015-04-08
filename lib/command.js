var stderr                 = process.stderr,
    fs                     = require('fs'),
    path                   = require('path'),
    inlineSourceMapComment = require('inline-source-map-comment')

var TypeSystem  = require('./typesystem').TypeSystem,
    Parser      = require('./parser'),
    reportError = require('./util').reportError

// Parse command-line arguments
var optimist = require('optimist')
var argv = optimist
    .usage('Usage: hb [command] [options]')
    .boolean('m')
    .alias('m', 'map')
    .boolean('v')
    // .demand(1)
    .argv

// Logging -------------------------------------------------------------------

var winston = require('winston'),
    logger  = new winston.Logger()
logger.add(winston.transports.Console, {
  colorize: true,
  level:    (argv.v ? 'info' : 'warn')
})

function showHelp () {
  var help = optimist.help().trim()
  help += "\n\n"
  help += "Commands:\n"
  help += "  run [file]                Compile and run a file\n"
  help += "  compile [options] [file]  Compile a file to JavaScript; writes to STDOUT\n"
  help += "  inspect [file]            Print the compiled AST of a file\n"
  help += "  help                      Show this help message\n\n"
  help += "Options:\n"
  help += "  -m, --map                 Include source maps in output\n"
  help += "  -v                        Verbose"
  console.error(help)
}

if (argv._.length === 0) {
  showHelp()
  process.exit(1)
}

// Utility function to read a file, parse it, walk it, and then return
// the finalized tree. On error it prints a nice error report and then
// calls `process.exit`.
function treeForFile (file) {
  var contents = null
  try {
    contents = fs.readFileSync(file).toString()
  } catch (err) {
    if (err.code === 'ENOENT') {
      logger.error("File not found: "+file)
      process.exit(1)
    }
    throw err
  }
  var name     = path.basename(file),
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

function fileFromArgs (args, idx) {
  var file = args[idx]
  if (!file) {
    logger.error("Missing file to inspect")
    process.exit(2)
  }
  return file
}

var commands = {
  inspect: function (args) {
    var file = fileFromArgs(args, 0),
        tree = treeForFile(file)
    // Print the resulting prettified AST
    tree.print()
    process.exit(0)
  },// inspect
  compile: function (args) {
    var file = fileFromArgs(args, 0),
        tree = treeForFile(file)
    // Load the JavaScript compile target and print the compiled source
    require('./targets/javascript')
    process.stdout.write(tree.compile())
    // Check whether we should also print the source-map
    var includeMap = argv.m
    if (includeMap) {
      process.stdout.write(inlineSourceMapComment(tree.sourceMap))
      process.stdout.write("\n")
    }
  },
  run: function (args) {
    var file = fileFromArgs(args, 0),
        tree = treeForFile(file)
    // Load the vm module and JavaScript target compiler
    var vm = require('vm')
    require('./targets/javascript')
    var compiledSource = tree.compile()
    // Run the compiled source in the VM
    vm.runInThisContext(compiledSource)
  },
  help: function (args) {
    showHelp()
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

