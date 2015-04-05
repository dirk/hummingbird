var fs            = require('fs'),
    path          = require('path'),
    child_process = require('child_process'),
    execSync      = child_process.execSync,
    TypeSystem    = require('./typesystem').TypeSystem,
    Parser        = require('./parser')

var optimist = require('optimist'),
    argv = optimist
    .usage('Usage: hbn [options] [entry]')
    .argv

function showHelp() {
  var help = optimist.help().trim()
  console.error(help)
}
if (argv._.length === 0) {
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

var outBase = 'out',
    bitFile = outBase+'.bc',
    objFile = outBase+'.o',
    binFile = outBase
tree.emitToFile(bitFile)
// process.exit(0)

// Compile using the LLVM bitcode-to-assembly/object compiler
execSync('llc -filetype=obj '+bitFile)
// Then compile the object file to a native binary
execSync('clang -o '+binFile+' '+objFile)

