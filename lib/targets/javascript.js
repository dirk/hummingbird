
var AST    = require('../ast'),
    repeat = require('../util').repeat


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

// JS code-generating compiler ------------------------------------------------

AST.Root.prototype.compile = function () {
  // Compilation context
  var context = new Context()
  return this.statements.map(function compile_statements(stmt) {
    return stmt.compile(context)
  }).join('')
}

AST.Binary.prototype.compile = function (context, opts) {
  var lexpr = this.lexpr.compile(context)
  var rexpr = this.rexpr.compile(context)
  var ret   = lexpr+' '+this.op+' '+rexpr
  if (opts && opts.statement === true) { ret += ";\n" }
  return ret
}

AST.Literal.prototype.compile  = function (context) {
  return this.value.toString()
}

AST.Assignment.prototype.compile = function (context, opts) {
  var term = ";\n"
  if (opts && opts.omitTerminator === true) { term = '' }
  if (this.type === 'var' || this.type === 'let') {
    // TODO: Register name in context scope and check for conflicts.
    var lvalue = this.lvalue.name
    if (this.rvalue !== false) {
      var rvalue = this.rvalue.compile(context)
      return 'var '+lvalue+' '+this.op+' '+rvalue+term
    } else {
      return 'var '+lvalue+term
    }
  } else {
    // TODO: Handle more complex path assignments
    // throw new Error('Compilation of path-assignments not yet implemented')
    var lvalue = this.lvalue.name
    var rvalue = this.rvalue.compile(context)
    return lvalue+' '+this.op+' '+rvalue+term
  }
}

AST.Function.prototype.compile = function (context) {
  var args = this.args.map(function (arg) { return arg.name }),
      ret  = 'function ('+args.join(',')+") {\n",
      defs = ''
  // Build assignments for any default arguments
  for (var i = args.length - 1; i >= 0; i--) {
    var arg  = this.args[i],
        name = arg.name,
        def  = arg.def
    if (def) {
      var value = def.compile(context),
          ind   = context.indent(2)
      // Prepend the default to the body
      defs = ind+'if ('+name+' === undefined) { '+name+' = '+value+"; }\n"+defs
    }
  }// for args
  ret += defs
  ret += this.block.compile(context)
  ret += "}"
  return ret
}

AST.Call.prototype.compile = function (context) {
  var args = this.args.map(function (arg) {
    return arg.compile(context)
  })
  return '('+args.join(', ')+')'
}

AST.If.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = 'if ('+this.cond.compile()+") {\n"
  ret += this.block.compile(context)
  ret += ind+"}\n"
  return ret
}

AST.While.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = 'while ('
  ret += this.expr.compile(context, {omitTerminator: true})
  ret += ') {\n'
  ret += this.block.compile(context)
  ret += ind + "}\n"
  return ret
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

AST.Chain.prototype.compile = function (context) {
  var ret = this.name
  this.tail.forEach(function (item) {
    ret += item.compile(context)
  })
  return ret
}

AST.Return.prototype.compile  = function (context) {
  return 'return '+this.expr.compile(context)+";\n"
}

AST.Block.prototype.compile = wrapContextIndent(function (context) {
  var ret = []
  var ind = context.indent()
  this.statements.forEach(function (stmt) {
    var opts = {}
    if (stmt instanceof AST.Binary) {
      opts.statement = true
    }
    ret.push(ind+stmt.compile(context, opts))
  })
  return ret.join('')
})
