
var inherits = require('util').inherits,
    inspect  = require('util').inspect,
    out      = process.stdout

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


var Let = function Let(name, typepath) {
  this.name = name.trim()
  this.typepath = typepath
}
inherits(Let, Node)
Let.prototype.print    = function () { _w(this.toString()+"\n") }
Let.prototype.toString = function () { return this.name }


// Quick and dirty clone of Let
var Var = Let.bind({})
inherits(Var, Node)
Var.prototype.print    = Let.prototype.print
Var.prototype.toString = Let.prototype.toString


var Expression = function () {}
inherits(Expression, Node)


var Group = function Group(expr) {
  this.expr = expr
}
inherits(Group, Expression)
Group.prototype.toString = function () { return '('+this.expr.toString()+')' }


var Binary = function (lexpr, op, rexpr) {
  this.lexpr = lexpr
  this.op    = op
  this.rexpr = rexpr
}
inherits(Binary, Expression)
Binary.prototype.toString = function () {
  return this.lexpr.toString()+' '+this.op+' '+this.rexpr.toString()
}
Binary.prototype.print = function () { out.write(this.toString()) }


var Literal = function Literal(value) {
  this.value = value
}
inherits(Literal, Node)
Literal.prototype.print = function () { out.write(this.toString()) }
Literal.prototype.toString = function () {
  return this.value.toString()
}


var Assignment = function (type, lvalue, rvalue) {
  this.type   = type
  this.lvalue = lvalue
  this.rvalue = rvalue
}
inherits(Assignment, Node)
Assignment.prototype.print = function () {
  var type = this.type ? (this.type+' ') : ''
  _w(type + this.lvalue.toString()+' = ')
  if (this.rvalue) {
    // _ind += INDENT
    this.rvalue.print()
    // _ind -= INDENT
  }
}



var Path = function () {}
inherits(Path, Node)


var Function = function (args, block) {
  this.args  = args
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


var Call = function (args) {
  this.args = args
}
inherits(Call, Node)
Call.prototype.toString = function () {
  return '('+this.args.map(function (arg) { return arg.toString() }).join(', ')+')'
}


var If = function (cond, block) {
  this.cond  = cond
  this.block = block
}
inherits(If, Node)
If.prototype.print = function () {
  var cond = this.cond.toString()
  _w("if "+cond+" ")
  this.block.print()
}


var For = function (init, cond, after, block) {
  this.init  = init // Initialization
  this.cond  = cond // Condition
  this.after = after // Afterthought
  this.block = block
}
inherits(For, Node)
For.prototype.print = function () {
  // var init  = this.init.toString()
  // var cond  = this.cond.toString()
  // var after = this.after.toString()
  _w("for ")
  var i = _ind
  _ind = 0;
  this.init.print();  out.write('; ')
  this.cond.print();  out.write('; ')
  this.after.print(); out.write(' ')
  _ind = i;
  this.block.print()
}


var Chain = function (name, tail) {
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


var Return = function (expr) {
  this.expr = expr
}
inherits(Return, Node)
Return.prototype.toString = function () {
  return 'return '+this.expr.toString()
}
Return.prototype.print = function () {
  _w(this.toString())
}


var Root = function (statements) {
  this.statements = statements
}
inherits(Root, Node)
Root.prototype.print = function() {
  _win("root {\n")
  this.statements.forEach(function (stmt) {
    stmt.print()
    out.write("\n")
  })
  _wout("}\n");
}


var Block = function (statements) {
  this.statements = statements
}
inherits(Block, Node)
Block.prototype.print = function() {
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

