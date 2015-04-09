var fs   = require('fs'),
    path = require('path')

var TypeSystem = require('./typesystem').TypeSystem,
    Parser     = require('./parser'),
    AST        = require('./ast')

function File (path, sourceCode, compiler) {
  this.path     = path
  this.code     = sourceCode
  this.compiler = compiler ? compiler : null
  this.tree     = null
  // Imported and exported bindings of the file
  this.imports = {}
  this.exports = {}
  // Dependencies (list of Files this File depends upon)
  this.dependencies = []
}

// Manages the entire process of compiling a file (the "entry") and
// generating a target output (object file for linking or JavaScript
// file for distribution/running).
function Compiler () {
  this.importPath = []
  this.entryFile  = null
  this.typeSystem = new TypeSystem()
  this.parser     = new Parser()
}
Compiler.prototype.compile = function (filePath, opts) {
  if (!opts) { opts = {} }
  var source = fs.readFileSync(filePath).toString()
  // Create the File object to manage compilation
  var file = new File(filePath, source, this)
  if (opts.isEntry) {
    this.entryFile = file
  }
  // Tell the file to compile itself
  if (file.tree) {
    throw new Error('File already compiled')
  }
  this.parseFile(file)
  // Now that we've parsed the tree get the compiler to walk it
  this.walkFile(file)
  return file
}
Compiler.prototype.parseFile = function (file) {
  this.parser.file = file.path
  var tree = this.parser.parse(file.code)
  if (!(tree instanceof AST.Root)) {
    throw new Error('Unexpected non-Root node from parser')
  }
  // Set the circular reference between the file and tree root
  file.tree = tree
  tree.file = file
  return tree
}
Compiler.prototype.walkFile = function (file) {
  this.typeSystem.walk(file.tree, file, this)
}

Compiler.prototype.importFileByName = function (fileName) {
  // console.log('importFileByName: '+fileName)
  for (var i = 0; i < this.importPath.length; i++) {
    var p = this.importPath[i]
    var filePath = path.join(p, fileName+'.hb')
    if (fs.existsSync(filePath)) {
      var file = this.compile(filePath)
      return file
    }// if
  }// for
  throw new Error('File not found: '+fileName)
}

module.exports = Compiler
