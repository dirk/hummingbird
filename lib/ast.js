
var inherits = require('util').inherits,
    inspect  = require('util').inspect,
    out      = process.stdout

var types = require('./types')

// http://stackoverflow.com/a/5450113
var repeat = function (pattern, count) {
  if (count < 1) return '';
  var result = ''
  while (count > 1) {
    if (count & 1) result += pattern;
    count >>= 1, pattern += pattern
  }
  return result + pattern
}

var INDENT = 2

var _ind = 0,
    _i   = function () { return repeat(' ', _ind) },
    _w   = function (s) { out.write(_i() + s) },
    _win = function (s) {
      // Indent and write
      _w(s); _ind += INDENT
    },
    _wout = function (s) { _ind -= INDENT; _w(s) }


// Nodes ----------------------------------------------------------------------

var Node = function () {}
Node.prototype.print = function () { out.write(inspect(this)) }
Node.prototype.compile = function (context) {
  throw new Error('Compilation not yet implemented for node type: '+this.constructor.name)
}


function Let (name, typepath) {
  this.name = name.trim()
  this.typepath = typepath
}
inherits(Let, Node)
Let.prototype.print    = function () { _w(this.toString()+"\n") }
Let.prototype.toString = function () { return this.name }


// Quick and dirty clone of Let
function Var (name, typepath) {
  this.name = name.trim()
  this.typepath = typepath
}
inherits(Var, Node)
Var.prototype.print    = Let.prototype.print
Var.prototype.toString = Let.prototype.toString


var Expression = function () {}
inherits(Expression, Node)


function Group (expr) {
  this.expr = expr
}
inherits(Group, Expression)
Group.prototype.toString = function () { return '('+this.expr.toString()+')' }


function Binary (lexpr, op, rexpr) {
  this.lexpr = lexpr
  this.op    = op
  this.rexpr = rexpr
}
inherits(Binary, Expression)
Binary.prototype.print = function () { out.write(this.toString()) }
Binary.prototype.toString = function () {
  return this.lexpr.toString()+' '+this.op+' '+this.rexpr.toString()
}
Binary.prototype.compile = function (context) {
  var lexpr = this.lexpr.compile(context)
  var rexpr = this.rexpr.compile(context)
  return lexpr+' '+this.op+' '+rexpr
}


var Literal = function Literal (value, type) {
  this.value = value
  this.type  = (type !== undefined) ? type : null
}
inherits(Literal, Node)
Literal.prototype.print    = function () { out.write(this.toString()) }
Literal.prototype.toString = function () { return this.value.toString() }
Literal.prototype.compile  = function (context) {
  return this.value.toString()
}


function Assignment (type, lvalue, rvalue) {
  this.type   = type
  this.lvalue = lvalue
  this.rvalue = rvalue
}
inherits(Assignment, Node)
Assignment.prototype.print = function () {
  var type = (this.type != 'path') ? (this.type+' ') : ''
  _w(type + this.lvalue.toString()+' = ')
  if (this.rvalue) {
    // _ind += INDENT
    this.rvalue.print()
    // _ind -= INDENT
  }
}
Assignment.prototype.compile = function (context) {
  if (this.type === 'var' || this.type === 'let') {
    // TODO: Register name in context scope and check for conflicts.
    var lvalue = this.lvalue.name
    var rvalue = this.rvalue.compile(context)
    return 'var '+lvalue+' = '+rvalue+";\n"
  } else {
    // TODO: Handle more complex path assignments
    // throw new Error('Compilation of path-assignments not yet implemented')
    var lvalue = this.lvalue.name
    var rvalue = this.rvalue.compile(context)
    return lvalue+' = '+rvalue+";\n"
  }
}


function Path (name) { this.name = name }
inherits(Path, Node)
Path.prototype.toString = function () { return this.name }


function Function (args, ret, block) {
  this.args  = args
  this.ret   = ret
  this.block = block
}
inherits(Function, Node)
Function.prototype.print = function () {
  var args = this.args.map(function (arg) {
    var ret = arg.name
    if (arg.type) {
      ret += ': '+arg.type
    }
    return ret
  }).join(', ')
  out.write('func ('+args+') ')
  this.block.print()
}
Function.prototype.compile = function (context) {
  var args = this.args.map(function (arg) { return arg.name })
  var body = this.block.compile(context)
  var ret = 'function ('+args.join(',')+") {\n"
  ret += body
  ret += "}"
  return ret
}


function Call(args) {
  this.args = args
}
inherits(Call, Node)
Call.prototype.toString = function () {
  return '('+this.args.map(function (arg) { return arg.toString() }).join(', ')+')'
}
Call.prototype.compile = function (context) {
  var args = this.args.map(function (arg) {
    return arg.compile(context)
  })
  return '('+args.join(', ')+')'
}


function If (cond, block) {
  this.cond  = cond
  this.block = block
}
inherits(If, Node)
If.prototype.print = function () {
  var cond = this.cond.toString()
  _w("if "+cond+" ")
  this.block.print()
}
If.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = 'if ('+this.cond.compile()+") {\n"
  ret += this.block.compile(context)
  ret += ind+"}\n"
  return ret
}


function For (init, cond, after, block) {
  this.init  = init // Initialization
  this.cond  = cond // Condition
  this.after = after // Afterthought
  this.block = block
}
inherits(For, Node)
For.prototype.print = function () {
  _w("for ")
  // Don't indent while we're writing out these statements
  var i = _ind
  _ind = 0;
  this.init.print();  out.write('; ')
  this.cond.print();  out.write('; ')
  this.after.print(); out.write(' ')
  // Restore indent and print the block
  _ind = i;
  this.block.print()
}
For.prototype.compile = function (context) {
  var ind = context.indent()
  var ret = 'for ('
  ret += this.init.compile(context).trim()+'; '
  ret += this.cond.compile(context)+' ;'
  ret += this.after.compile(context)+" {\n"
  ret += this.block.compile(context)
  ret += ind+"}\n"
  return ret
}


function Chain (name, tail) {
  this.name = name
  this.tail = tail
}
inherits(Chain, Node)
Chain.prototype.toString = function () {
  var base = this.name
  this.tail.forEach(function (expr) {
    base += expr.toString()
  })
  return base
}
Chain.prototype.print = function () { out.write(this.toString()) }
Chain.prototype.compile = function (context) {
  var ret = this.name
  this.tail.forEach(function (item) {
    ret += item.compile(context)
  })
  return ret
}


function Return (expr) {
  this.expr = expr
}
inherits(Return, Node)
Return.prototype.print    = function () { _w(this.toString()) }
Return.prototype.toString = function () { return 'return '+this.expr.toString() }
Return.prototype.compile  = function (context) {
  return 'return '+this.expr.compile(context)+";\n"
}


var Root = function (statements) {
  this.statements = statements
}
inherits(Root, Node)
Root.prototype.print = function () {
  _win("root {\n")
  this.statements.forEach(function (stmt) {
    stmt.print()
    out.write("\n")
  })
  _wout("}\n");
}
Root.prototype.compile = function () {
  // Compilation context
  var context = new Context()
  return this.statements.map(function compile_statements(stmt) {
    return stmt.compile(context)
  }).join('')
}


function Block (statements) {
  this.statements = statements
}
inherits(Block, Node)
Block.prototype.print = function () {
  out.write("{\n")
  _ind += INDENT
  this.statements.forEach(function (stmt) {
    stmt.print()
    out.write("\n")
  })
  _ind -= INDENT
  _w('}')
  // out.write(repeat(' ', _ind - INDENT) + '}')
}
Block.prototype.compile = wrapContextIndent(function (context) {
  var ret = []
  var ind = context.indent()
  this.statements.forEach(function (stmt) {
    ret.push(ind+stmt.compile(context))
  })
  return ret.join('')
})


// JS code-generating compiler ------------------------------------------------

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

module.exports = {
  Node: Node,
  Let: Let,
  Var: Var,
  Path: Path,
  Root: Root,
  Assignment: Assignment,
  Expression: Expression,
  Binary: Binary,
  Literal: Literal,
  Group: Group,
  Function: Function,
  Block: Block,
  If: If,
  For: For,
  Chain: Chain,
  Return: Return,
  Call: Call
}

