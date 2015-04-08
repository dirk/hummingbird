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
    .boolean('v')
    .argv

function getArg (name) {
  var value = argv[name]
  if (value === undefined) {
    throw new Error('Failed to find argument: '+name)
  }
  return value
}
var Opts = {
  verbose: getArg('v'),
  gc:      getArg('gc')
}

// Logging -------------------------------------------------------------------

var winston = require('winston'),
    logger  = new winston.Logger()
logger.add(winston.transports.Console, {
  colorize: true,
  level:    (Opts.verbose ? 'info' : 'warn')
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
  if (Opts.verbose) {
    // process.stdout.write(cmd+"\n")
    logger.info('exec:', cmd)
  }
  execSync(cmd)
}

// Compile using the LLVM bitcode-to-assembly/object compiler
execSync('llc -filetype=obj '+bitFile)

var gc = '-lgc'
if (!Opts.gc) { gc = '' }
// Then compile the object file to a native binary
execSync('clang '+gc+' -o '+binFile+' '+objFile)

