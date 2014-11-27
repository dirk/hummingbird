
var AST    = require('../ast'),
    repeat = require('../util').repeat

function Context () {
  // Keeping track of names that conflict and need to be munged/aliased.
  this.conflicts = {}
  this._indent = 0
}
Context.prototype.incrementIndent = function () { this._indent += 2 }
Context.prototype.decrementIndent = function () { this._indent -= 2 }
Context.prototype.indent = function () {
  return repeat(' ', this._indent)
}

function wrapContextIndent(func) {
  return function (context) {
    context.incrementIndent()
    var ret = func.call(this, context)
    context.decrementIndent()
    return ret
  }
}


AST.Root.prototype.compile = function () {
  // Compilation context
  var context = new Context()
  return this.statements.map(function compile_statements(stmt) {
    return stmt.compile(context)
  }).join('')
}

AST.Binary.prototype.compile = function (context) {
  var lexpr = this.lexpr.compile(context)
  var rexpr = this.rexpr.compile(context)
  return lexpr+' '+this.op+' '+rexpr
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
    var rvalue = this.rvalue.compile(context)
    return 'var '+lvalue+' = '+rvalue+term
  } else {
    // TODO: Handle more complex path assignments
    // throw new Error('Compilation of path-assignments not yet implemented')
    var lvalue = this.lvalue.name
    var rvalue = this.rvalue.compile(context)
    return lvalue+' = '+rvalue+term
  }
}

AST.Function.prototype.compile = function (context) {
  var args = this.args.map(function (arg) { return arg.name })
  var body = this.block.compile(context)
  var ret = 'function ('+args.join(',')+") {\n"
  ret += body
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
  var ind = context.indent()
  var ret = 'for ('
  ret += this.init.compile(context, {omitTerminator: true})+'; '
  ret += this.cond.compile(context)+'; '
  ret += this.after.compile(context)+") {\n"
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
    ret.push(ind+stmt.compile(context))
  })
  return ret.join('')
})
