/// <reference path="typescript/node-0.12.0.d.ts" />
import util   = require('util')
import errors = require('./errors')

var inherits  = util.inherits,
    inspect   = util.inspect,
    TypeError = errors.TypeError,
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

export class Node {
  _file:           string
  _line:           number
  _column:         number
  isLastStatement: boolean = false

  print() { out.write(inspect(this)) }
  compile(...rest) {
    throw new Error('Compilation not yet implemented for node: '+this.constructor['name'])
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


export class NameType extends Node {
  name: string
  constructor(name: string) {
    super()
    this.name = name.trim()
  }
  toString(): string { return this.name }
}

export class FunctionType extends Node {
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


export class Let extends Node {
  name:          string
  immediateType: any

  constructor(name, immediateType) {
    super()
    this.name          = name.trim()
    this.immediateType = immediateType
  }
  print(): void { _w(this.toString()+"\n") }
  toString(): string {
    var ret = this.name
    if (_include_types && this.immediateType) {
      ret += ': '+this.immediateType.toString()
    }
    return ret
  }
}


// Quick and dirty clone of Let
export class Var extends Let {}

export class Import extends Node {
  name:  String
  using: any[]
  file:  any
  constructor(name, using) {
    super()
    this.name  = new String(name)
    this.using = using
    // .using can only be null or an Array
    if (this.using !== null) {
      assertPropertyIsInstanceOf(this, 'using', Array)
    }
    // Will be set to the File object when it's visited
    this.file = null
  }
  print() { out.write(this.toString()) }
  toString() { return 'import '+this.name }
}


export class Export extends Node {
  name: string
  type: any = null

  constructor(name) {
    super()
    this.name = name
  }
  print() { out.write(this.toString()) }
  toString(): string { return 'export '+this.name }
}


export class Class extends Node {
  name:         any
  definition:   any
  initializers: any[]
  // Computed by the typesystem
  type:         any

  constructor(name, block) {
    super()
    this.name       = name
    this.definition = block
    // Computed nodes from the definition
    this.initializers = this.definition.statements.filter(function (stmt) {
      return (stmt instanceof Init)
    })
  }
  print() {
    out.write('export class '+this.name+" ")
    this.definition.print()
  }
}


export class Expression extends Node {}


export class Group extends Node {
  expr: any
  constructor(expr) {
    super()
    this.expr = expr
  }
  toString() { return '('+this.expr.toString()+')' }
}


export class Binary extends Node {
  lexpr: any
  op:    string
  rexpr: any
  type:  any

  constructor(lexpr, op, rexpr) {
    super()
    this.lexpr = lexpr
    this.op    = op
    this.rexpr = rexpr
  }
  isBinaryStatement(): boolean { return (this.op === '+=') }

  print(): void { out.write(this.toString()) }
  toString(): string {
    return this.lexpr.toString()+' '+this.op+' '+this.rexpr.toString()
  }
}


export class Literal extends Node {
  value:    any
  typeName: string
  type:     any

  constructor(value, typeName) {
    super()
    this.value    = value
    this.typeName = (typeName !== undefined) ? typeName : null
    this.type     = null
  }
  print(): void { out.write(this.toString()) }
  toString(): string { return JSON.stringify(this.value) }
}


export class Assignment extends Node {
  type:   any
  lvalue: any
  op:     string
  rvalue: any

  constructor(type, lvalue, op, rvalue) {
    super()
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
  print() {
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
}


export class Path extends Node {
  name: any
  path: any

  constructor(name, path) {
    super()
    this.name = name
    this.path = path
  }
  toString() {
    var ret = this.name
    this.path.forEach(function (item) {
      ret += item.toString()
    })
    return ret
  }
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


interface FunctionArgument {
  name:  string
  type?: Node
  def?:  Node
}

export class Function extends Node {
  args:  FunctionArgument[]
  ret:   any
  block: Block
  // Statement properties
  name: string = null
  when: Node  = null
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


export class Multi extends Node {
  name: any
  args: any
  ret:  any
  type: any

  constructor(name, args, ret) {
    super()
    this.name = name
    this.args = args
    this.ret  = ret
  }
  print() {
    var args = this.args.map(function (arg) {
      return arg.name+(arg.type ? (': '+arg.type) : '')
    }).join(', ')
    out.write('multi '+this.name+'('+args+")\n")
  }
}


export class Init extends Node {
  args:  any
  block: Block
  constructor(args, block) {
    super()
    this.args  = args
    this.block = block
    assertSaneArgs(this.args)
  }
  print() {
    var args = this.args.map(function (arg) { return arg.name+': '+arg.type.toString() }).join(', ')
    out.write('init ('+args+') ')
    this.block.print()
  }
}


export class New extends Node {
  name:            any
  args:            any
  initializer:     Function
  // Type of the class that's going to be constructed
  constructorType: any
  // Type of the object once it's been constructed
  type:            any

  constructor(name, args) {
    super()
    this.name = name
    this.args = args
    // Corresponding initializer Function for the export class type it's initializing
    this.initializer = null
  }
  setInitializer(init) {
    this.initializer = init
    assertPropertyIsInstanceOf(this, 'initializer', types.Function)
  }
  getInitializer() {
    return this.initializer
  }
  toString() {
    var args = this.args.map(function(arg) { return arg.toString() }).join(', ')
    return 'new '+this.name+'('+args+')'
  }
  print() { out.write(this.toString()) }
}


export class Identifier extends Node {
  name:   any
  parent: any
  type:   any

  constructor(name) {
    super()
    this.name   = name
    this.parent = null
  }
  print(): void { out.write(this.toString()) }
  toString(): string { return this.name }
}


export class Call extends Node {
  base:     any
  args:     any
  parent:   any
  type:     any
  baseType: any

  constructor(base, callArgs) {
    super()
    this.base   = base
    this.args   = callArgs
    this.parent = null
    assertPropertyIsInstanceOf(this, 'base', Node)
    assertPropertyIsInstanceOf(this, 'args', Array)
  }
  toString() {
    var args ='('+this.args.map(function (arg) { return arg.toString() }).join(', ')+')'
    return this.base+args
  }
  print() {
    out.write(this.toString())
  }
}


export class Property extends Node {
  base:     any
  property: any
  parent:   any
  type:     any
  baseType: any
  
  constructor(base, property) {
    super()
    this.base     = base
    this.property = property
    this.parent   = null
    assertPropertyIsInstanceOf(this, 'base', Node)
    assertPropertyIsInstanceOf(this, 'property', Node)
  }
  toString() {
    return this.base+'.'+this.property.toString()
  }
  print() { out.write(this.toString()) }
}


export class If extends Node {
  cond:      any
  block:     Block
  elseIfs:   If[]
  elseBlock: Block

  constructor(cond, block, elseIfs, elseBlock) {
    super()
    this.cond      = cond
    this.block     = block
    this.elseIfs   = elseIfs ? elseIfs : null
    this.elseBlock = elseBlock ? elseBlock : null
  }
  print() {
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
}


export class While extends Node {
  expr:  any
  block: any

  constructor(expr, block) {
    super()
    this.expr  = expr // Loop expression
    this.block = block
  }
  print() {
    out.write("while "+this.expr.toString()+" ")
    this.block.print()
  }
}


export class For extends Node {
  init:  any
  cond:  any
  after: any
  block: Block

  constructor(init, cond, after, block) {
    super()
    this.init  = init // Initialization
    this.cond  = cond // Condition
    this.after = after // Afterthought
    this.block = block
  }
  print() {
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
}


/*
export class Chain extends Node {
  name:     any
  tail:     any
  headType: any
  type:     any
  
  constructor(name, tail) {
    super()
    this.name = name
    this.tail = tail
    // Added by the typesystem
    this.headType = null
    this.type     = null
  }
  toString() {
    var base = this.name
    this.tail.forEach(function (expr) {
      base += expr.toString()
    })
    return base
  }
  print() { out.write(this.toString()) }
}
*/


export class Return extends Node {
  expr: any 
  type: any
  
  constructor(expr) {
    super()
    this.expr = expr
  }
  print() { out.write(this.toString()) }
  toString() {
    if (this.expr) {
      return 'return '+this.expr.toString()
    }
    return 'return'
  }
}


export class Root extends Node {
  statements:   any[]
  scope:        any
  sourceMap:    any
  file:         any
  imports:      any[]
  exports:      any[]
  includeTypes: boolean = false
  
  constructor(statements) {
    super()
    this.statements = statements
    this.sourceMap  = null
    this.scope      = null
    // Lists of import and export nodes; the nodes add themselves during
    // type-system walking
    this.imports = []
    this.exports = []
  }
  print() {
    _include_types = this.includeTypes
    _win("root {\n")
    this.statements.forEach(function (stmt) {
      _w('')
      stmt.print()
      out.write("\n")
    })
    _wout("}\n");
  }
  getRootScope() {
    var rootScope = this.scope.parent
    if (!rootScope || !rootScope.isRoot) {
      throw new TypeError('Missing root scope', this)
    }
    return rootScope
  }
}


export class Block extends Node {
  statements: any[]
  scope:      any
  returnType: any

  constructor(statements) {
    super()
    this.statements = statements
    this.scope      = null
    // Set the `isLastStatement` property on the last statement
    var lastStatement = statements[statements.length - 1]
    if (lastStatement) {
      lastStatement.isLastStatement = true
    }
  }
  print() {
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
}

/*
var mod = {
  Node: Node,
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
  // Chain: Chain,
  Return: Return,
  Call: Call,
  Property: Property
}
export = mod
*/

