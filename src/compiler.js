var fs   = require('fs'),
    path = require('path')

var types      = require('./types'),
    TypeSystem = require('./typesystem').TypeSystem,
    Parser     = require('./parser'),
    AST        = require('./ast')

function File (path, sourceCode, compiler) {
  this.path     = path
  this.code     = sourceCode
  this.compiler = compiler ? compiler : null
  this.tree     = null
  // For the main/entry file this will be null, otherwise it will be a
  // types.Module for the module this file represents.
  this.module   = null
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
  if (opts.module) {
    if (opts.isEntry) {
      throw new Error('File cannot be both an entry point and a module')
    }
    file.module = opts.module
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
  var foundFilePath   = null,
      foundImportPath = null
  // console.log('importFileByName: '+fileName)
  for (var i = 0; i < this.importPath.length; i++) {
    var p = this.importPath[i]
    var filePath = path.join(p, fileName+'.hb')
    if (fs.existsSync(filePath)) {
      foundFilePath   = filePath
      foundImportPath = p
      break
    }// if
  }// for
  if (!foundFilePath) {
    throw new Error('File not found: '+fileName)
  }
  var relativeName = path.relative(foundImportPath, foundFilePath)
  // TODO: Actually do some handling with the relative name. Also use the import
  //       path and all that jazz.
  var moduleName = relativeName.replace(/\.hb$/i, '')
  if (/[^A-Za-z0-9_-]/.test(moduleName)) {
    throw new Error('Cannot handle module name: '+moduleName)
  }
  var mod  = new types.Module(moduleName),
      file = this.compile(foundFilePath, {module: mod})
  return file
}

module.exports = Compiler

