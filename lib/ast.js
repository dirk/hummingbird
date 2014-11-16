
var inherits = require('util').inherits,
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

var _ind = 0,
    _i   = function () { return repeat(' ', _ind) },
    _w   = function (s) { out.write(_i() + s) },
    _win = function (s) {
      // Indent and write
      _w(s); _ind += 2
    },
    _wout = function (s) { _ind -= 2; _w(s) }


// Nodes ----------------------------------------------------------------------

var Node = function () {}
Node.prototype.print = function () { }


var Let = function Let(name, typepath) {
  this.name = name.trim()
  this.typepath = typepath
}
inherits(Let, Node)
Let.prototype.print    = function () { _w(this.toString()) }
Let.prototype.toString = function () { return this.name }


// Quick and dirty clone of Let
var Var = Let.bind({})
inherits(Var, Node)
Var.prototype.print    = Let.prototype.print
Var.prototype.toString = Let.prototype.toString


var Assignment = function (type, lvalue, rvalue) {
  this.type   = type
  this.lvalue = lvalue
  this.rvalue = rvalue
}
inherits(Assignment, Node)
Assignment.prototype.print = function () {
  var out = ''
  if (this.type) { out += this.type+' ' }
  out += this.lvalue.toString()
  _w(out+"\n")
}


var Path = function () {}
inherits(Path, Node)


var Root = function (statements) {
  this.statements = statements
}
inherits(Root, Node)
Root.prototype.print = function() {
  _win("root {\n")
  this.statements.forEach(function (stmt) {
    stmt.print()
  })
  _wout("}\n");
}


module.exports = {
  Node: Node,
  Let: Let,
  Var: Var,
  Path: Path,
  Root: Root,
  Assignment: Assignment
}

