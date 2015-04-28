var stderr                 = process.stderr,
    fs                     = require('fs'),
    path                   = require('path'),
    inlineSourceMapComment = require('inline-source-map-comment')

// Make source maps work with TypeScript source
require('source-map-support').install()

var TypeSystem  = require('./typesystem').TypeSystem,
    Parser      = require('./parser'),
    reportError = require('./util').reportError,
    Compiler    = require('./compiler')

// Parse command-line arguments
var optimist = require('optimist')
var argv = optimist
    .usage('Usage: hb [command] [options]')
    .boolean('m')
    .alias('m', 'map')
    .boolean('v')
    .boolean('s')
    .alias('s', 'single')
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
  help += "  -s, --single              Don't include imports (uses RequireJS)\n"
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


var compileOpts = {
  isEntry: true
}
var targetOpts = {}

function compileFile (args) {
  var filePath      = fileFromArgs(args, 0),
      // Get the directory of the file for the import-path
      fileDirectory = path.dirname(filePath)
  try {
    var compiler = new Compiler()
    compiler.importPath.push(fileDirectory)
    var file = compiler.compile(filePath, compileOpts)
  } catch (err) {
    reportError(err)
    process.exit(1)
  }
  return file
}

var commands = {
  inspect: function (args) {
    var file = compileFile(args)
    file.tree.print()
    process.exit(0)
  },// inspect
  compile: function (args) {
    var file = compileFile(args)
    // If there's no imports and exports then set it to single mode
    if (file.tree.imports.length === 0) {
      targetOpts.single = true
    }
    // Load the JavaScript compile target and print the compiled source
    var javascript = require('./targets/javascript'),
        compiler   = new javascript.JSCompiler(),
        compiled   = compiler.compileRoot(file.tree, targetOpts)
    process.stdout.write(compiled)
    // Check whether we should also print the source-map
    var includeMap = argv.map
    if (includeMap) {
      process.stdout.write(inlineSourceMapComment(file.tree.sourceMap))
      process.stdout.write("\n")
    }
  },
  run: function (args) {
    var file = compileFile(args)
    // Load the JavaScript compile target
    require('./targets/javascript')
    // Load the vm module and JavaScript target compiler
    var vm = require('vm')
    // Compile the whole file into a bundle to run
    var compiledSource = file.tree.compile()
    // Expose "require(...)" to the script
    global.require = require;
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
  // Check if there was a "--single" option to force single-file target
  // compilation mode
  if (argv.single === true) {
    targetOpts.single = true
  }
  command(otherArgs)
}

module.exports = {run: run}

