
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

function compileStatement (context, stmt) {
  var opts = {}
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
  var context  = new Context()
  var compiled = new SourceNode(1, 0, null, this.statements.map(function (stmt) {
    return compileStatement(context, stmt)
  })).toStringWithSourceMap()
  this.sourceMap = compiled.map
  return compiled.code
}

AST.Binary.prototype.compile = function (context, opts) {
  var lexpr = this.lexpr.compile(context)
  var rexpr = this.rexpr.compile(context)
  var ret   = lexpr+' '+this.op+' '+rexpr
  if (opts && opts.statement === true) { ret += ";\n" }
  return ret
}

AST.Literal.prototype.compile  = function (context) {
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
      ret = ['var ', lvalue, ' ', this.op, ' ', rvalue, term]
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
    ret = [lvalue, ' ', this.op, ' ', rvalue, term]
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
  ret.push("}")
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
      name = this.name
  // Build function definition
  var ret = 'function '+name+' ('+args.join(', ')+") {\n"
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
  ret += ind+"switch (false) {\n"
  cond.forEach(function (condFunction) {
    var childName = condFunction.childName
    ret += ind+"case !("+condFunction.when.compile(context)+"):\n"
    ret += context.indent(2)+"return "+childName+"("+args.join(", ")+");\n"
  })
  ret += ind+"default:\n"
  var defInd = context.indent(2)
  if (def) {
    ret += defInd+"return "+def.childName+"("+args.join(", ")+");\n"
  } else {
    ret += defInd+"throw new Error('Fell through to default branch');\n"
  }
  ret += ind+"}\n"

  // Build all of the implementation functions
  ind = context.indent()
  var i = 1
  this.type.functionNodes.forEach(function (functionNode) {
    var functionName = functionNode.childName,
        functionArgs = functionNode.args.map(function (arg) { return arg.name })
    ret += ind+'function '+functionName+" ("+functionArgs.join(', ')+") {\n"
    ret += functionNode.block.compile(context)
    ret += ind+"}\n"
    
    i += 1
  })

  context.decrementIndent()
  ret += context.indent()+"}\n"
  return asSourceNode(this, ret)
}


AST.Class.prototype.compile = function (context) {
  var klass = this.type,
      name  = this.name
  // Generate the simple class function
  // TODO: Multiple-dispatch initializers
  var ret = "function "+name+" () {\n"
  
  // Compile the initializer preamble
  ret += this.compilePreamble(context)

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
    ret += this.compileInitializers(context, initializers)
  }

  ret += context.indent()+"}\n",
  // Now add all the instance methods
  this.definition.statements.forEach(function (node) {
    if (!(node instanceof AST.Function)) { return }
    var methodName = node.name
    ret += name+".prototype."+methodName+" = "+node.compile(context)
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
      ret = ''
  // By default we'll do the same handling for both properties' defaults
  var properties = letProperties.concat(varProperties)
  properties.forEach(function (prop) {
    // Skip properties with no default
    if (!prop.rvalue) { return }
    ret += ind+'this.'+prop.lvalue.name+' = '+prop.rvalue.compile(context)+";\n"
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
  var ret = ind+"switch (arguments.length) {\n"
  var branchLengths = Object.keys(branches)
  branchLengths.forEach(function (branchLength) {
    ret += ind+'  case '+branchLength+":\n"
    // Build the arguments for the branch
    var args = []
    for (var i = 0; i < branchLength; i++) {
      args.push('arguments['+i+']')
    }
    if (args.length > 0) {
      args = ', '+args.join(', ')
    } else { args = '' }
    ret += ind+'    init'+branchLength+".call(this"+args+"); break;\n"
  })
  ret += ind+"  default:\n"
  ret += ind+"    throw new Error('No initializer found');\n"
  ret += ind+"}\n"
  // Build the branches
  branchLengths.forEach(function (branchLength) {
    var branch = branches[branchLength]
    var args = branch.args.map(function (arg) {
      return arg.name
    }).join(', ')
    ret += ind+'function init'+branchLength+' ('+args+") {\n"
    ret += branch.block.compile(context) 
    ret += ind+"}\n"
  }) 
  context.decrementIndent()
  return ret
}

AST.New.prototype.compile = function (context) {
  var name = this.name,
      args = this.args.map(function (arg) {
        return arg.compile(context)
      }).join(', ')
  var ret = "(new "+name+"("+args+"))"
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
  var ret  = 'for ('
  ret += this.init.compile(context, {omitTerminator: true})+'; '
  ret += (cond ? cond.compile(context) : '')+'; '
  ret += this.after.compile(context, {omitTerminator: true})+") {\n"
  ret += this.block.compile(context)
  ret += ind+"}\n"
  return ret
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
    ret = 'return '+this.expr.compile(context)+";\n"
  }
  return asSourceNode(this, ret)
}

AST.Block.prototype.compile = wrapContextIndent(function (context) {
  var ret = []
  var ind = context.indent()
  this.statements.forEach(function (stmt) {
    ret.push(ind+compileStatement(context, stmt))
  })
  return ret.join('')
})

