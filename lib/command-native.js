var fs            = require('fs'),
    path          = require('path'),
    child_process = require('child_process'),
    TypeSystem    = require('./typesystem').TypeSystem,
    Parser        = require('./parser')

// Command-line arguments ----------------------------------------------------

var optimist = require('optimist'),
    argv = optimist
    // .usage('Usage: hbn [options] [entry]')
    .boolean('gc')
    .default('gc', true)
    .argv

function getArg (name) {
  var value = argv[name]
  if (value === undefined) {
    throw new Error('Failed to find argument: '+name)
  }
  return value
}
function hasArg (name) {
  return (process.argv.indexOf('-'+name) !== -1)
}
var Opts = {
  verbose:     hasArg('v'),
  veryVerbose: hasArg('vv'),
  gc:          getArg('gc')
}

// Logging -------------------------------------------------------------------

var logLevel = 'warn'
if (Opts.verbose)     { logLevel = 'info' }
if (Opts.veryVerbose) { logLevel = 'debug' }

var winston = require('winston'),
    logger  = new winston.Logger()
logger.add(winston.transports.Console, {
  colorize: true,
  level:    logLevel
})

function showHelp() {
  // var help = optimist.help().trim()+"\n"
  var help = "Usage: hbn [options] [entry]\n\n"
  help += "Options:\n"
  help += "  --no-gc  Don't link the GC"
  help += "  -v       Verbose"
  console.error(help)
}
if (argv._.length === 0 || argv._[0] === 'help') {
  showHelp()
  process.exit(0)
}

var entryFile = argv._[0]

var parser     = new Parser(),
    typesystem = new TypeSystem()

var source = fs.readFileSync(entryFile).toString()
var tree = parser.parse(source)
typesystem.walk(tree)

require('./targets/llvm')

// Compute the path of the source file without the .hb extension
var outBase = entryFile.replace(/\.hb$/i, '')
if (outBase === entryFile) {
  console.error("Couldn't figure out base output file path for input file: "+entryFile)
  process.exit(0)
}

// var outBase = 'out',
var bitFile = outBase+'.bc',
    objFile = outBase+'.o',
    binFile = 'a.out'
tree.emitToFile(bitFile, {logger: logger})
// process.exit(0)

function execSync (cmd) {
  var execSync = child_process.execSync
  logger.info('exec:', cmd)
  execSync(cmd)
}

// Compile using the LLVM bitcode-to-assembly/object compiler
execSync('llc -filetype=obj '+bitFile)

var gc = '-lgc'
if (!Opts.gc) { gc = '' }
// Then compile the object file to a native binary
execSync('clang -o '+binFile+' '+gc+' '+objFile)

