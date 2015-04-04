
var AST        = require('../ast'),
    types      = require('../types'),
    repeat     = require('../util').repeat,
    sourcemap  = require('source-map'),
    SourceNode = sourcemap.SourceNode


// Context of the compiler as it's generating code ----------------------------

function Context () {
  // Keeping track of names that conflict and need to be munged/aliased.
  this.conflicts = {}
  this._indent = 0
  // Keep track of files that we've compiled
  this.filesCompiled = []
}
Context.prototype.incrementIndent = function () { this._indent += 2 }
Context.prototype.decrementIndent = function () { this._indent -= 2 }
Context.prototype.indent = function (additionalIndent) {
  if (additionalIndent === undefined) { additionalIndent = 0 }
  return repeat(' ', this._indent + additionalIndent)
}

function wrapContextIndent(func) {
  return function (context) {
    context.incrementIndent()
    var ret = func.call(this, context)
    context.decrementIndent()
    return ret
  }
}

function compileStatement (context, stmt, opts) {
  opts = (opts ? opts : {})
  if (stmt instanceof AST.Binary || stmt instanceof AST.Chain) {
    opts.statement = true
  }
  return stmt.compile(context, opts)
}

// Return a string as a SourceNode with position information copied from the
// given node.
function asSourceNode (node, ret) {
  var name = node.constructor.name
  if (!node._line)   { throw new Error('Missing line for '+name) }
  if (!node._column) { throw new Error('Missing column for '+name) }
  if (!node._file)   { throw new Error('Missing file for '+name) }
  return new SourceNode(node._line, node._column, node._file, ret)
}

// JS code-generating compiler ------------------------------------------------

AST.Root.prototype.compile = function () {
  // Compilation context
  var context = new Context()
  if (!this.file) {
    throw new Error('Missing File for Root')
  }
  var nodes = this.compileAsDependency(context)
  // Prepend our platform support stuff
  nodes.unshift("var requirejs = require('requirejs'),\n",
                "    define = requirejs.define;\n")
  // Require the main file to make it be the entry point
  // TODO: Refactor how we compute the unique names/IDs of each file/module
  var entryName = JSON.stringify(this.file.path)
  nodes.push("requirejs("+entryName+");\n")
  // And build the big output node
  var compiled = new SourceNode(1, 0, null, nodes).toStringWithSourceMap()
  this.sourceMap = compiled.map
  return compiled.code
}
AST.Root.prototype.compileAsDependency = function (context) {
  context.filesCompiled.push(this.tree)
  var depsToCompile = this.file.dependencies.filter(function (dep) {
    if (context.filesCompiled.indexOf(dep) !== -1) {
      return false
    }
    return true
  })
  // Set up each dependency as a source node
  var sourceNodes = depsToCompile.map(function (file) {
    var root = file.tree
    return root.compileAsDependency(context)
  })
  // Then add on our actual statements
  sourceNodes = sourceNodes.concat(this.compileStatements(context, true))
  return sourceNodes
}
// TODO: Cleanup this function! Way too much messy indent stuff.
AST.Root.prototype.compileStatements = function (context, isRoot) {
  var ret       = [],
      lastIndex = this.statements.length - 1
  // Setup the root options
  var opts = {imported: [], exported: []}
  
  // This block will be indented for the RequireJS definition
  var outerIndent = context.indent(),
      ind         = null
  context.incrementIndent()
  ind = context.indent()
  for (var i = 0; i < this.statements.length; i++) {
    //if (i !== lastIndex) {
      ret.push(ind)
    //}
    var stmt = this.statements[i]
    ret.push(compileStatement(context, stmt, opts))
  }

  // Build the "define(...)" for this module
  var name = JSON.stringify(this.file.path),
      def  = outerIndent+"define("+name+', '
  // Imported will be a set of File-name tuples for building the define(...)
  if (opts.imported.length > 0) {
    var depNames = [], depArgs = []
    opts.imported.forEach(function (tuple) {
      var file = tuple[0], name = tuple[1]
      // Use the file's full path as it's unique ID
      depNames.push(file.path)
      depArgs.push(name)
    })
    var names = depNames.map(function (n) { return JSON.stringify(n) }).join(', '),
        args  = depArgs.join(', ')
    // Update def with the complex definition
    def += '['+names+'], function('+args+") {\n"
  } else {
    def += "function () {\n"
  }
  // Prepend the define to our return body
  ret.unshift(def)

  // Add the exports to our return body
  if (opts.exported.length > 0) {
    var exp = ind+"return {\n"
    opts.exported.forEach(function (name) {
      exp += context.indent(2)+JSON.stringify(name)+': '+name+"\n"
    })
    exp += ind+"}\n"
    ret.push(exp)
  }

  // Outdent and close the define() function
  context.decrementIndent()
  ret.push(context.indent()+"});\n")
  return ret
}


function importExportPreCheck (opts) {
  if (!opts.imported || !opts.exported) {
    throw new Error('Missing import and/or exported lists for statement')
  }
}

AST.Import.prototype.compile = function (context, opts) {
  importExportPreCheck(opts)
  opts.imported.push([this.file, this.name.value])
  // We need to output something; so let's just leave a nice little comment
  // about what's going on as it compiled this node.
  return '// '+this.toString()+"\n"
}


AST.Export.prototype.compile = function (context, opts) {
  importExportPreCheck(opts)
  opts.exported.push(this.name)
  return '// '+this.toString()+"\n"
}


AST.Binary.prototype.compile = function (context, opts) {
  var lexpr = this.lexpr.compile(context)
  var rexpr = this.rexpr.compile(context)
  var ret   = [lexpr, ' '+this.op+' ', rexpr]
  if (opts && opts.statement === true) { ret.push(";\n") }
  return asSourceNode(this, ret)
}

AST.Literal.prototype.compile = function (context) {
  if (this.typeName === 'String') {
    return JSON.stringify(this.value)
  }
  return this.value.toString()
}

AST.Assignment.prototype.compile = function (context, opts) {
  var term = ";\n",
      ret  = null
  if (opts && opts.omitTerminator === true) { term = '' }
  if (this.type === 'var' || this.type === 'let') {
    // TODO: Register name in context scope and check for conflicts.
    var lvalue = this.lvalue.name
    if (this.rvalue !== false) {
      var rvalue = this.rvalue.compile(context)
      ret = ['var ', lvalue, ' '+this.op+' ', rvalue, term]
    } else {
      ret = ['var ', lvalue, term]
    }
  } else {
    // TODO: Handle more complex path assignments
    // throw new Error('Compilation of path-assignments not yet implemented')
    var lvalue = this.lvalue.name
    this.lvalue.path.forEach(function (item) {
      lvalue += item.compile(context)
    })
    var rvalue = this.rvalue.compile(context)
    ret = [lvalue, ' '+this.op+' ', rvalue, term]
  }
  return asSourceNode(this, ret)
}

AST.Function.prototype.compile = function (context) {
  // Skip compilation for functions that are children of multi types
  if (this.isChildOfMulti()) { return "" }

  var args = this.args.map(function (arg) { return arg.name }),
      ret  = ['function (', args.join(', '), ") {\n"],
      defs = []
  // Build assignments for any default arguments
  for (var i = args.length - 1; i >= 0; i--) {
    var arg  = this.args[i],
        name = arg.name,
        def  = arg.def
    if (def) {
      var value = def.compile(context),
          ind   = context.indent(2)
      // Prepend the default to the body
      defs = [ind, 'if (', name, ' === undefined) { ', name, ' = ', value, "; }\n"].concat(defs)
    }
  }// for args
  ret = ret.concat(defs)
  ret.push(this.block.compile(context))
  ret.push(context.indent()+"}")
  // Name property indicates it's a function statement
  if (this.name) {
    ret.push("\n")
  }
  return asSourceNode(this, ret)
}

function interpose (arr, sep) {
  var newArr  = [],
      len     = arr.length,
      lastIdx = len - 1
  for (var i = 0; i < len; i++) {
    newArr.push(arr[i])
    if (i !== lastIdx) {
      newArr.push(sep)
    }
  }
  return newArr
}
Array.prototype.extend = function (other) {
  var len = other.length
  for (var i = 0; i < len; i++) {
    this.push(other[i])
  }
}

AST.Multi.prototype.compile = function (context) {
  // console.log(this)
  var args = this.args.map(function (arg) { return arg.name }),
      joinedArgs = args.join(', '),// joinedArgs = interpose(args, ', ')
      name = this.name
  // Build function definition
  var ret = ['function ', name, ' (', joinedArgs, ") {\n"]
  context.incrementIndent()

  // Figure out the branches for the dispatcher
  var def = [], cond = []
  for (var i = this.type.functionNodes.length - 1; i >= 0; i--) {
    var fn = this.type.functionNodes[i]
    fn.childName = name+'_'+(i+1)
    if (fn.when) {
      cond.push(fn)
    } else {
      // If there's no `when` condition then it's a default
      def.push(fn)
    }
  }
  if (def.length > 1) {
    var n = def.length
    throw new TypeError('Multi has more than 1 default branch (has '+n+')', this)
  }
  // De-array default to just the node or null
  def = (def.length === 0) ? null : def[0]

  // Build the dispatcher
  var ind = context.indent()
  ret.push(ind+"switch (false) {\n")
  cond.forEach(function (condFunction) {
    var childName = condFunction.childName
    ret.push(ind+"case !(", condFunction.when.compile(context), "):\n")
    ret.push(context.indent(2)+"return "+childName+"(", joinedArgs, ");\n")
  })
  ret.push(ind+"default:\n")
  var defInd = context.indent(2)
  if (def) {
    ret.push(defInd+"return "+def.childName+"(", joinedArgs, ");\n")
  } else {
    ret.push(defInd+"throw new Error('Fell through to default branch');\n")
  }
  ret.push(ind+"}\n")

  // Build all of the implementation functions
  ind = context.indent()
  var i = 1
  this.type.functionNodes.forEach(function (functionNode) {
    var functionName = functionNode.childName,
        functionArgs = functionNode.args.map(function (arg) { return arg.name })
    ret.push(ind+'function '+functionName+" ("+functionArgs.join(', ')+") {\n")
    ret.push(functionNode.block.compile(context))
    ret.push(ind+"}\n")

    i += 1
  })

  context.decrementIndent()
  ret.push(context.indent()+"}\n")
  return asSourceNode(this, ret)
}


AST.Class.prototype.compile = function (context) {
  var klass = this.type,
      name  = this.name
  // Generate the simple class function
  // TODO: Multiple-dispatch initializers
  var ret = ["function "+name+" () {\n"]

  // Compile the initializer preamble
  ret = ret.concat(this.compilePreamble(context))

  // Compile the initializers
  var initializers = []
  for (var i = 0; i < this.definition.statements.length; i++) {
    var node = this.definition.statements[i]
    if (node instanceof AST.Init) {
      initializers.push(node)
    }
  }
  // TODO: Add fast codepath for just one initializer
  if (initializers.length > 0) {
    ret.push(this.compileInitializers(context, initializers))
  }

  ret.push(context.indent()+"}\n")
  // Now add all the instance methods
  this.definition.statements.forEach(function (node) {
    if (!(node instanceof AST.Function)) { return }
    var methodName = node.name
    ret.push(name+".prototype."+methodName+" = ", node.compile(context))
  })
  return asSourceNode(this, ret)
}
AST.Class.prototype.compilePreamble = function (context) {
  var letProperties = [],
      varProperties = []
  this.definition.statements.forEach(function (node) {
    if (!(node instanceof AST.Assignment)) { return }
    if (node.type === 'var') {
      varProperties.push(node)
    }
    if (node.type === 'let') {
      letProperties.push(node)
    }
  })
  var ind = context.indent(2),
      ret = []
  // By default we'll do the same handling for both properties' defaults
  var properties = letProperties.concat(varProperties)
  properties.forEach(function (prop) {
    // Skip properties with no default
    if (!prop.rvalue) { return }
    ret.push(ind+'this.'+prop.lvalue.name+' = ', prop.rvalue.compile(context), ";\n")
  })
  return ret
}
AST.Class.prototype.compileInitializers = function (context, initializers) {
  context.incrementIndent()
  var ind = context.indent()
  // Compute the length branches
  var branches = {}
  initializers.forEach(function (init) {
    var argsLength = init.args.length
    if (branches[argsLength]) {
      throw new Error('Multiple initializers taking '+argLength+' arguments')
    }
    branches[argsLength] = init
  })
  // Build the dispatcher
  var ret = [ind+"switch (arguments.length) {\n"]
  var branchLengths = Object.keys(branches)
  branchLengths.forEach(function (branchLength) {
    ret.push(ind+'  case '+branchLength+":\n")
    // Build the arguments for the branch
    var args = []
    for (var i = 0; i < branchLength; i++) {
      args.push('arguments['+i+']')
    }
    if (args.length > 0) {
      args = ', '+args.join(', ')
    } else { args = '' }
    ret.push(ind+'    init'+branchLength+".call(this"+args+"); break;\n")
  })
  ret.push(ind+"  default:\n")
  ret.push(ind+"    throw new Error('No initializer found');\n")
  ret.push(ind+"}\n")
  // Build the branches
  branchLengths.forEach(function (branchLength) {
    var branch = branches[branchLength]
    var args = branch.args.map(function (arg) {
      return arg.name
    }).join(', ')
    ret.push(ind+'function init'+branchLength+' ('+args+") {\n")
    ret.push(branch.block.compile(context))
    ret.push(ind+"}\n")
  })
  context.decrementIndent()
  return ret
}

AST.New.prototype.compile = function (context) {
  var name = this.name,
      args = this.args.map(function (arg) {
        return arg.compile(context)
      })
  // Interpose commas between the args
  args = interpose(args, ', ')
  var ret = ["(new "+name+"(", args, "))"]
  return asSourceNode(this, ret)
}


AST.Call.prototype.compile = function (context) {
  var args = this.args.map(function (arg) {
    return arg.compile(context)
  })
  var ret = ['(']
  var length = args.length, lastIndex = args.length - 1
  for (var i = 0; i < length; i++) {
    ret.push(args[i])
    if (i !== lastIndex) {
      ret.push(', ')
    }
  }
  ret.push(')')
  return asSourceNode(this, ret)
}

AST.Property.prototype.compile = function (context) {
  return asSourceNode(this, '.'+this.name)
}

AST.If.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = ['if (', this.cond.compile(), ") {\n"]
  ret.push(this.block.compile(context))
  ret.push(ind+"}")
  if (this.elseIfs) {
    for (var i = 0; i < this.elseIfs.length; i++) {
      var ei = this.elseIfs[i]
      ret.push(' else if (', ei.cond.compile(context), ") {\n")
      ret.push(ei.block.compile(context))
      ret.push(ind+"}")
    }
  }
  ret.push("\n")
  return asSourceNode(this, ret)
}

AST.While.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = [
    'while (',
      this.expr.compile(context, {omitTerminator: true}),
    ') {\n',
      this.block.compile(context),
    ind + "}\n"
  ]
  return asSourceNode(this, ret)
}

AST.For.prototype.compile = function (context) {
  var cond = this.cond
  var ind  = context.indent()
  var ret  = ['for (']
  ret.push(this.init.compile(context, {omitTerminator: true}), '; ')
  ret.push((cond ? cond.compile(context) : ''), '; ')
  ret.push(this.after.compile(context, {omitTerminator: true}), ") {\n")
  ret.push(this.block.compile(context))
  ret.push(ind+"}\n")
  return asSourceNode(this, ret)
}

AST.Chain.prototype.compile = function (context, opts) {
  var ret = [this.name]
  this.tail.forEach(function (item) {
    ret.push(item.compile(context))
  })
  if (opts && opts.statement === true) { ret.push(";\n") }
  return asSourceNode(this, ret)
}

AST.Return.prototype.compile  = function (context) {
  var ret = "return;\n"
  if (this.expr) {
    ret = ['return ', this.expr.compile(context), ";\n"]
  }
  return asSourceNode(this, ret)
}

AST.Block.prototype.compile = wrapContextIndent(function (context) {
  var ret = []
  var ind = context.indent()
  this.statements.forEach(function (stmt) {
    ret.push(ind, compileStatement(context, stmt))
  })
  return asSourceNode(this, ret)
})

