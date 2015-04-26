
var inherits  = require('util').inherits,
    inspect   = require('util').inspect,
    // repeat = require('./util').repeat,
    out       = process.stdout

var types = require('./types')

// http://stackoverflow.com/a/5450113
function repeat(pattern: string, count: number): string {
  if (count < 1) { return '' }
  var result = ''
  while (count > 1) {
    if (count & 1) result += pattern;
    count >>= 1, pattern += pattern
  }
  return result + pattern
}

// TODO: Refactor all this crazy indentation stuff!
var INDENT = 2

var _ind = 0,
    _i   = function () { return repeat(' ', _ind) },
    _w   = function (s) { out.write(_i() + s) },
    _win = function (s) {
      // Indent and write
      _w(s); _ind += INDENT
    },
    _wout = function (s) { _ind -= INDENT; _w(s) },
    _include_types = true

// Nodes ----------------------------------------------------------------------

class _Node {
  _file:   string
  _line:   number
  _column: number

  print() { out.write(inspect(this)) }
  compile() {
    throw new Error('Compilation not yet implemented for node: '+this.constructor.name)
  }
  setPosition(file, line, column) {
    this._file   = file
    this._line   = line
    this._column = column
  }
}
// Node.prototype.setParsePosition = function (parser) {
//   this._file   = parser.file
//   this._line   = parser.line
//   this._column = parser.column
// }


class NameType extends _Node {
  name: string
  constructor(name: string) {
    super()
    this.name = name.trim()
  }
  toString(): string { return this.name }
}

class FunctionType extends _Node {
  args: any
  ret:  any
  constructor(args, ret) {
    super()
    this.args = args
    this.ret  = ret
  }
  toString(): string {
    var args = this.args.map(function (arg) { return arg.toString() }).join(', '),
        ret  = (this.ret ? this.ret.toString() : 'Void')
    return '('+args+') -> '+ret
  }
}


function Let (name, immediateType) {
  this.name          = name.trim()
  this.immediateType = immediateType
}
inherits(Let, _Node)
Let.prototype.print    = function () { _w(this.toString()+"\n") }
Let.prototype.toString = function () {
  var ret = this.name
  if (_include_types && this.immediateType) {
    ret += ': '+this.immediateType.toString()
  }
  return ret
}


// Quick and dirty clone of Let
function Var (name, immediateType) {
  this.name          = name.trim()
  this.immediateType = immediateType
}
inherits(Var, _Node)
Var.prototype.print    = Let.prototype.print
Var.prototype.toString = Let.prototype.toString


function Import (name, using) {
  this.name  = new String(name)
  this.using = using
  // .using can only be null or an Array
  if (this.using !== null) {
    assertPropertyIsInstanceOf(this, 'using', Array)
  }
  // Will be set to the File object when it's visited
  this.file = null
}
inherits(Import, _Node)
Import.prototype.print = function () {
  out.write(this.toString())
}
Import.prototype.toString = function () {
  return 'import '+this.name
}


function Export (name) {
  this.name = name
}
inherits(Export, _Node)
Export.prototype.print = function () {
  out.write(this.toString())
}
Export.prototype.toString = function () {
  return 'export '+this.name
}


function Class (name, block) {
  this.name       = name
  this.definition = block
  // Computed nodes from the definition
  this.initializers = this.definition.statements.filter(function (stmt) {
    return (stmt instanceof Init)
  })
}
inherits(Class, _Node)
Class.prototype.print = function () {
  out.write('class '+this.name+" ")
  this.definition.print()
}


var Expression = function () {}
inherits(Expression, _Node)


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
Binary.prototype.isBinaryStatement = function () {
  return (this.op === '+=')
}
Binary.prototype.print = function () { out.write(this.toString()) }
Binary.prototype.toString = function () {
  return this.lexpr.toString()+' '+this.op+' '+this.rexpr.toString()
}


var Literal = function Literal (value, typeName) {
  this.value    = value
  this.typeName = (typeName !== undefined) ? typeName : null
  this.type     = null
}
inherits(Literal, _Node)
Literal.prototype.print    = function () { out.write(this.toString()) }
Literal.prototype.toString = function () { return JSON.stringify(this.value) }


function Assignment (type, lvalue, op, rvalue) {
  this.type   = type
  this.lvalue = lvalue
  this.rvalue = rvalue
  // Possible values: '=', '+=', or null
  this.op     = op
  // Only allowed .op for lets/vars is a '='
  if ((this.type === 'let' || this.type === 'var') && this.op !== '=') {
    throw new Error('Invalid operator on '+this.type+" statement: '"+this.op+"'")
  }
}
inherits(Assignment, _Node)
Assignment.prototype.print = function () {
  var type = (this.type != 'path') ? (this.type+' ') : ''
  out.write(type + this.lvalue.toString())
  if (this.rvalue) {
    var op = (this.op === null) ? '?' : this.op.toString()
    out.write(' '+op+' ')
    // _ind += INDENT
    this.rvalue.print()
    // _ind -= INDENT
  }
}


function Path (name, path) {
  this.name = name
  this.path = path
}
inherits(Path, _Node)
Path.prototype.toString = function () {
  var ret = this.name
  this.path.forEach(function (item) {
    ret += item.toString()
  })
  return ret
}


function assertHasProperty (obj, prop) {
  var val = obj[prop]
  if (val !== undefined) { return }
  throw new Error("Object missing property '"+prop+"'")
}


function assertPropertyIsInstanceOf (recv, prop, type) {
  if (recv[prop] instanceof type) { return }
  throw new Error('Expected '+prop+' to be an instance of '+type.name)
}
function assertPropertyIsTypeOf (recv, prop, type) {
  if (typeof recv[prop] === type) { return }
  throw new Error('Expected '+prop+' to be a type of '+type)
}


// Compiler sanity check to make sure all the args have the correct properties
function assertSaneArgs (args) {
  for (var i = args.length - 1; i >= 0; i--) {
    var arg = args[i]
    assertHasProperty(arg, 'name')
    assertHasProperty(arg, 'type')
    // assertHasProperty(arg, 'def')
    var def = arg.def
    if (def && !(def instanceof Literal)) {
      throw new Error('Expected default to be an AST.Literal')
    }
  }// for
}// assertSaneArgs


class _Function extends _Node {
  args:  any
  ret:   any
  block: any
  // Statement properties
  name: any = null
  when: any = null
  // Computed type (set by typesystem)
  type: any = null
  // Parent `multi` type (if this is present the Function will not
  // not codegen itself and instead defer to the Multi's codegen)
  parentMultiType: any = null
  // This will be set by type-system visitor later
  scope: any = null

  constructor(args, ret, block) {
    super()
    this.args  = args
    this.ret   = ret
    this.block = block
    // Run some compiler checks
    assertPropertyIsInstanceOf(this, 'args', Array)
    assertSaneArgs(this.args)
  }
  print(): void {
    var args = this.args.map(function (arg) {
      var ret = arg.name
      if (arg.type) {
        ret += ': '+arg.type
      }
      return ret
    }).join(', ')
    out.write('func ('+args+') ')
    var instance = this.type
    if (this.ret) {
      out.write('-> '+this.ret+' ')
    } else {
      // If we computed an inferred return type for the type
      out.write('-i> '+instance.type.ret.inspect()+' ')
    }
    this.block.print()
  }
  setParentMultiType(multi): void {
    this.parentMultiType = multi
  }
  isChildOfMulti(): boolean {
    return this.parentMultiType ? true : false
  }
}


function Multi (name, args, ret) {
  this.name = name
  this.args = args
  this.ret  = ret
}
inherits(Multi, _Node)
Multi.prototype.print = function () {
  var args = this.args.map(function (arg) {
    return arg.name+(arg.type ? (': '+arg.type) : '')
  }).join(', ')
  out.write('multi '+this.name+'('+args+")\n")
}


function Init (args, block) {
  this.args  = args
  this.block = block
  assertSaneArgs(this.args)
}
inherits(Init, _Node)
Init.prototype.print = function () {
  var args = this.args.map(function (arg) { return arg.name+': '+arg.type.toString() }).join(', ')
  out.write('init ('+args+') ')
  this.block.print()
}


function New (name, args) {
  this.name = name
  this.args = args
  // Corresponding initializer Function for the class type it's initializing
  this.initializer = null
}
inherits(New, _Node)
New.prototype.setInitializer = function (init) {
  this.initializer = init
  assertPropertyIsInstanceOf(this, 'initializer', types.Function)
}
New.prototype.getInitializer = function () {
  return this.initializer
}
New.prototype.toString = function () {
  var args = this.args.map(function(arg) { return arg.toString() }).join(', ')
  return 'new '+this.name+'('+args+')'
}
New.prototype.print = function () { out.write(this.toString()) }


function Identifier (name) {
  this.name   = name
  this.parent = null
}
inherits(Identifier, _Node)
Identifier.prototype.toString = function () { return this.name }
Identifier.prototype.print    = function () { out.write(this.toString()) }


function Call(base, callArgs) {
  this.base   = base
  this.args   = callArgs
  this.parent = null
  assertPropertyIsInstanceOf(this, 'base', _Node)
  assertPropertyIsInstanceOf(this, 'args', Array)
}
inherits(Call, _Node)
Call.prototype.toString = function () {
  var args ='('+this.args.map(function (arg) { return arg.toString() }).join(', ')+')'
  return this.base+args
}
Call.prototype.print = function () {
  out.write(this.toString())
}

function Property (base, property) {
  this.base     = base
  this.property = property
  this.parent   = null
  assertPropertyIsInstanceOf(this, 'base', _Node)
  assertPropertyIsInstanceOf(this, 'property', _Node)
}
inherits(Property, _Node)
Property.prototype.toString = function () {
  return this.base+'.'+this.property.toString()
}
Property.prototype.print = function () { out.write(this.toString()) }


function If (cond, block, elseIfs, elseBlock) {
  this.cond      = cond
  this.block     = block
  this.elseIfs   = elseIfs ? elseIfs : null
  this.elseBlock = elseBlock ? elseBlock : null
}
inherits(If, _Node)
If.prototype.print = function () {
  var cond = this.cond.toString()
  out.write("if "+cond+" ")
  this.block.print()
  if (this.elseIfs) {
    for (var i = 0; i < this.elseIfs.length; i++) {
      var ei = this.elseIfs[i]
      cond = ei.cond.toString()
      out.write(" else if "+cond+" ")
      ei.block.print()
    }
  }
  if (this.elseBlock) {
    out.write(" else ")
    this.elseBlock.print()
  }
}


function While (expr, block) {
  this.expr  = expr // Loop expression
  this.block = block
}
inherits(While, _Node)
While.prototype.print = function () {
  out.write("while "+this.expr.toString()+" ")
  this.block.print()
}


function For (init, cond, after, block) {
  this.init  = init // Initialization
  this.cond  = cond // Condition
  this.after = after // Afterthought
  this.block = block
}
inherits(For, _Node)
For.prototype.print = function () {
  out.write("for ")
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


function Chain (name, tail) {
  this.name = name
  this.tail = tail
  // Added by the typesystem
  this.headType = null
  this.type     = null
}
inherits(Chain, _Node)
Chain.prototype.toString = function () {
  var base = this.name
  this.tail.forEach(function (expr) {
    base += expr.toString()
  })
  return base
}
Chain.prototype.print = function () { out.write(this.toString()) }


function Return (expr) {
  this.expr = expr
}
inherits(Return, _Node)
Return.prototype.print    = function () { out.write(this.toString()) }
Return.prototype.toString = function () {
  if (this.expr) {
    return 'return '+this.expr.toString()
  }
  return 'return'
}


function Root (statements) {
  this.statements = statements
  this.sourceMap  = null
  this.scope      = null
  // Lists of import and export nodes; the nodes add themselves during
  // type-system walking
  this.imports = []
  this.exports = []
}
inherits(Root, _Node)
Root.prototype.print = function (includeTypes) {
  if (includeTypes !== undefined) {
    _include_types = includeTypes
  }
  _win("root {\n")
  this.statements.forEach(function (stmt) {
    _w('')
    stmt.print()
    out.write("\n")
  })
  _wout("}\n");
}
Root.prototype.getRootScope = function () {
  var rootScope = this.scope.parent
  if (!rootScope || !rootScope.isRoot) {
    throw new TypeError('Missing root scope', this)
  }
  return rootScope
}


function Block (statements) {
  this.statements = statements
  this.scope      = null
  // Set the `isLastStatement` property on the last statement
  var lastStatement = statements[statements.length - 1]
  if (lastStatement) {
    lastStatement.isLastStatement = true
  }
}
inherits(Block, _Node)
Block.prototype.print = function () {
  out.write("{\n")
  _ind += INDENT
  this.statements.forEach(function (stmt) {
    _w('')
    stmt.print()
    out.write("\n")
  })
  _ind -= INDENT
  _w('}')
  // out.write(repeat(' ', _ind - INDENT) + '}')
}


module.exports = {
  Node: _Node,
  NameType: NameType,
  FunctionType: FunctionType,
  Import: Import,
  Export: Export,
  Class: Class,
  Init: Init,
  New: New,
  Let: Let,
  Var: Var,
  Path: Path,
  Root: Root,
  Assignment: Assignment,
  Expression: Expression,
  Binary: Binary,
  Literal: Literal,
  Group: Group,
  Function: _Function,
  Multi: Multi,
  Block: Block,
  If: If,
  While: While,
  For: For,
  Identifier: Identifier,
  Chain: Chain,
  Return: Return,
  Call: Call,
  Property: Property
}
