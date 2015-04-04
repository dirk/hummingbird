var fs   = require('fs'),
    path = require('path')

var TypeSystem = require('./typesystem').TypeSystem,
    Parser     = require('./parser')

function File (path, sourceCode, compiler) {
  this.path     = path
  this.code     = sourceCode
  this.compiler = compiler ? compiler : null
  this.tree     = null
  // Imported and exported bindings of the file
  this.imports = {}
  this.exports = {}
}
File.prototype.compile = function () {
  if (this.tree) {
    throw new Error('File already compiled')
  }
  this.compiler.parseFile(this)
  // Now that we've parsed the tree get the compiler to walk it
  this.compiler.walkFile(this)
  return this.tree
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
  file.compile()
  return file
}
Compiler.prototype.parseFile = function (file) {
  this.parser.file = file.path
  var tree = this.parser.parse(file.code)
  file.tree = tree
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

