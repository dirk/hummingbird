var __extends = this.__extends || function (d, b) {
    for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p];
    function __() { this.constructor = d; }
    __.prototype = b.prototype;
    d.prototype = new __();
};
/// <reference path="typescript/node-0.12.0.d.ts" />
var util = require('util');
var errors = require('./errors');
var inherits = util.inherits, inspect = util.inspect, TypeError = errors.TypeError, out = process.stdout;
var types = require('./types');
// http://stackoverflow.com/a/5450113
function repeat(pattern, count) {
    if (count < 1) {
        return '';
    }
    var result = '';
    while (count > 1) {
        if (count & 1)
            result += pattern;
        count >>= 1, pattern += pattern;
    }
    return result + pattern;
}
// TODO: Refactor all this crazy indentation stuff!
var INDENT = 2;
var _ind = 0, _i = function () { return repeat(' ', _ind); }, _w = function (s) { out.write(_i() + s); }, _win = function (s) {
    // Indent and write
    _w(s);
    _ind += INDENT;
}, _wout = function (s) { _ind -= INDENT; _w(s); }, _include_types = true;
// Nodes ----------------------------------------------------------------------
var _Node = (function () {
    function _Node() {
        this.isLastStatement = false;
    }
    _Node.prototype.print = function () { out.write(inspect(this)); };
    _Node.prototype.compile = function () {
        throw new Error('Compilation not yet implemented for node: ' + this.constructor['name']);
    };
    _Node.prototype.setPosition = function (file, line, column) {
        this._file = file;
        this._line = line;
        this._column = column;
    };
    return _Node;
})();
exports._Node = _Node;
// Node.prototype.setParsePosition = function (parser) {
//   this._file   = parser.file
//   this._line   = parser.line
//   this._column = parser.column
// }
var NameType = (function (_super) {
    __extends(NameType, _super);
    function NameType(name) {
        _super.call(this);
        this.name = name.trim();
    }
    NameType.prototype.toString = function () { return this.name; };
    return NameType;
})(_Node);
exports.NameType = NameType;
var FunctionType = (function (_super) {
    __extends(FunctionType, _super);
    function FunctionType(args, ret) {
        _super.call(this);
        this.args = args;
        this.ret = ret;
    }
    FunctionType.prototype.toString = function () {
        var args = this.args.map(function (arg) { return arg.toString(); }).join(', '), ret = (this.ret ? this.ret.toString() : 'Void');
        return '(' + args + ') -> ' + ret;
    };
    return FunctionType;
})(_Node);
exports.FunctionType = FunctionType;
var Let = (function (_super) {
    __extends(Let, _super);
    function Let(name, immediateType) {
        _super.call(this);
        this.name = name.trim();
        this.immediateType = immediateType;
    }
    Let.prototype.print = function () { _w(this.toString() + "\n"); };
    Let.prototype.toString = function () {
        var ret = this.name;
        if (_include_types && this.immediateType) {
            ret += ': ' + this.immediateType.toString();
        }
        return ret;
    };
    return Let;
})(_Node);
exports.Let = Let;
// Quick and dirty clone of Let
var Var = (function (_super) {
    __extends(Var, _super);
    function Var() {
        _super.apply(this, arguments);
    }
    return Var;
})(Let);
exports.Var = Var;
var Import = (function (_super) {
    __extends(Import, _super);
    function Import(name, using) {
        _super.call(this);
        this.name = new String(name);
        this.using = using;
        // .using can only be null or an Array
        if (this.using !== null) {
            assertPropertyIsInstanceOf(this, 'using', Array);
        }
        // Will be set to the File object when it's visited
        this.file = null;
    }
    Import.prototype.print = function () { out.write(this.toString()); };
    Import.prototype.toString = function () { return 'import ' + this.name; };
    return Import;
})(_Node);
exports.Import = Import;
var Export = (function (_super) {
    __extends(Export, _super);
    function Export(name) {
        _super.call(this);
        this.type = null;
        this.name = name;
    }
    Export.prototype.print = function () { out.write(this.toString()); };
    Export.prototype.toString = function () { return 'export ' + this.name; };
    return Export;
})(_Node);
exports.Export = Export;
var Class = (function (_super) {
    __extends(Class, _super);
    function Class(name, block) {
        _super.call(this);
        this.name = name;
        this.definition = block;
        // Computed nodes from the definition
        this.initializers = this.definition.statements.filter(function (stmt) {
            return (stmt instanceof Init);
        });
    }
    Class.prototype.print = function () {
        out.write('export class ' + this.name + " ");
        this.definition.print();
    };
    return Class;
})(_Node);
exports.Class = Class;
var Expression = (function (_super) {
    __extends(Expression, _super);
    function Expression() {
        _super.apply(this, arguments);
    }
    return Expression;
})(_Node);
exports.Expression = Expression;
var Group = (function (_super) {
    __extends(Group, _super);
    function Group(expr) {
        _super.call(this);
        this.expr = expr;
    }
    Group.prototype.toString = function () { return '(' + this.expr.toString() + ')'; };
    return Group;
})(_Node);
exports.Group = Group;
var Binary = (function (_super) {
    __extends(Binary, _super);
    function Binary(lexpr, op, rexpr) {
        _super.call(this);
        this.lexpr = lexpr;
        this.op = op;
        this.rexpr = rexpr;
    }
    Binary.prototype.isBinaryStatement = function () { return (this.op === '+='); };
    Binary.prototype.print = function () { out.write(this.toString()); };
    Binary.prototype.toString = function () {
        return this.lexpr.toString() + ' ' + this.op + ' ' + this.rexpr.toString();
    };
    return Binary;
})(_Node);
exports.Binary = Binary;
var Literal = (function (_super) {
    __extends(Literal, _super);
    function Literal(value, typeName) {
        _super.call(this);
        this.value = value;
        this.typeName = (typeName !== undefined) ? typeName : null;
        this.type = null;
    }
    Literal.prototype.print = function () { out.write(this.toString()); };
    Literal.prototype.toString = function () { return JSON.stringify(this.value); };
    return Literal;
})(_Node);
exports.Literal = Literal;
var Assignment = (function (_super) {
    __extends(Assignment, _super);
    function Assignment(type, lvalue, op, rvalue) {
        _super.call(this);
        this.type = type;
        this.lvalue = lvalue;
        this.rvalue = rvalue;
        // Possible values: '=', '+=', or null
        this.op = op;
        // Only allowed .op for lets/vars is a '='
        if ((this.type === 'let' || this.type === 'var') && this.op !== '=') {
            throw new Error('Invalid operator on ' + this.type + " statement: '" + this.op + "'");
        }
    }
    Assignment.prototype.print = function () {
        var type = (this.type != 'path') ? (this.type + ' ') : '';
        out.write(type + this.lvalue.toString());
        if (this.rvalue) {
            var op = (this.op === null) ? '?' : this.op.toString();
            out.write(' ' + op + ' ');
            // _ind += INDENT
            this.rvalue.print();
        }
    };
    return Assignment;
})(_Node);
exports.Assignment = Assignment;
var Path = (function (_super) {
    __extends(Path, _super);
    function Path(name, path) {
        _super.call(this);
        this.name = name;
        this.path = path;
    }
    Path.prototype.toString = function () {
        var ret = this.name;
        this.path.forEach(function (item) {
            ret += item.toString();
        });
        return ret;
    };
    return Path;
})(_Node);
exports.Path = Path;
function assertHasProperty(obj, prop) {
    var val = obj[prop];
    if (val !== undefined) {
        return;
    }
    throw new Error("Object missing property '" + prop + "'");
}
function assertPropertyIsInstanceOf(recv, prop, type) {
    if (recv[prop] instanceof type) {
        return;
    }
    throw new Error('Expected ' + prop + ' to be an instance of ' + type.name);
}
function assertPropertyIsTypeOf(recv, prop, type) {
    if (typeof recv[prop] === type) {
        return;
    }
    throw new Error('Expected ' + prop + ' to be a type of ' + type);
}
// Compiler sanity check to make sure all the args have the correct properties
function assertSaneArgs(args) {
    for (var i = args.length - 1; i >= 0; i--) {
        var arg = args[i];
        assertHasProperty(arg, 'name');
        assertHasProperty(arg, 'type');
        // assertHasProperty(arg, 'def')
        var def = arg.def;
        if (def && !(def instanceof Literal)) {
            throw new Error('Expected default to be an AST.Literal');
        }
    } // for
} // assertSaneArgs
var Function = (function (_super) {
    __extends(Function, _super);
    function Function(args, ret, block) {
        _super.call(this);
        // Statement properties
        this.name = null;
        this.when = null;
        // Computed type (set by typesystem)
        this.type = null;
        // Parent `multi` type (if this is present the Function will not
        // not codegen itself and instead defer to the Multi's codegen)
        this.parentMultiType = null;
        // This will be set by type-system visitor later
        this.scope = null;
        this.args = args;
        this.ret = ret;
        this.block = block;
        // Run some compiler checks
        assertPropertyIsInstanceOf(this, 'args', Array);
        assertSaneArgs(this.args);
    }
    Function.prototype.print = function () {
        var args = this.args.map(function (arg) {
            var ret = arg.name;
            if (arg.type) {
                ret += ': ' + arg.type;
            }
            return ret;
        }).join(', ');
        out.write('func (' + args + ') ');
        var instance = this.type;
        if (this.ret) {
            out.write('-> ' + this.ret + ' ');
        }
        else {
            // If we computed an inferred return type for the type
            out.write('-i> ' + instance.type.ret.inspect() + ' ');
        }
        this.block.print();
    };
    Function.prototype.setParentMultiType = function (multi) {
        this.parentMultiType = multi;
    };
    Function.prototype.isChildOfMulti = function () {
        return this.parentMultiType ? true : false;
    };
    return Function;
})(_Node);
exports.Function = Function;
var Multi = (function (_super) {
    __extends(Multi, _super);
    function Multi(name, args, ret) {
        _super.call(this);
        this.name = name;
        this.args = args;
        this.ret = ret;
    }
    Multi.prototype.print = function () {
        var args = this.args.map(function (arg) {
            return arg.name + (arg.type ? (': ' + arg.type) : '');
        }).join(', ');
        out.write('multi ' + this.name + '(' + args + ")\n");
    };
    return Multi;
})(_Node);
exports.Multi = Multi;
var Init = (function (_super) {
    __extends(Init, _super);
    function Init(args, block) {
        _super.call(this);
        this.args = args;
        this.block = block;
        assertSaneArgs(this.args);
    }
    Init.prototype.print = function () {
        var args = this.args.map(function (arg) { return arg.name + ': ' + arg.type.toString(); }).join(', ');
        out.write('init (' + args + ') ');
        this.block.print();
    };
    return Init;
})(_Node);
exports.Init = Init;
var New = (function (_super) {
    __extends(New, _super);
    function New(name, args) {
        _super.call(this);
        this.name = name;
        this.args = args;
        // Corresponding initializer Function for the export class type it's initializing
        this.initializer = null;
    }
    New.prototype.setInitializer = function (init) {
        this.initializer = init;
        assertPropertyIsInstanceOf(this, 'initializer', types.Function);
    };
    New.prototype.getInitializer = function () {
        return this.initializer;
    };
    New.prototype.toString = function () {
        var args = this.args.map(function (arg) { return arg.toString(); }).join(', ');
        return 'new ' + this.name + '(' + args + ')';
    };
    New.prototype.print = function () { out.write(this.toString()); };
    return New;
})(_Node);
exports.New = New;
var Identifier = (function (_super) {
    __extends(Identifier, _super);
    function Identifier(name) {
        _super.call(this);
        this.name = name;
        this.parent = null;
    }
    Identifier.prototype.print = function () { out.write(this.toString()); };
    Identifier.prototype.toString = function () { return this.name; };
    return Identifier;
})(_Node);
exports.Identifier = Identifier;
var Call = (function (_super) {
    __extends(Call, _super);
    function Call(base, callArgs) {
        _super.call(this);
        this.base = base;
        this.args = callArgs;
        this.parent = null;
        assertPropertyIsInstanceOf(this, 'base', _Node);
        assertPropertyIsInstanceOf(this, 'args', Array);
    }
    Call.prototype.toString = function () {
        var args = '(' + this.args.map(function (arg) { return arg.toString(); }).join(', ') + ')';
        return this.base + args;
    };
    Call.prototype.print = function () {
        out.write(this.toString());
    };
    return Call;
})(_Node);
exports.Call = Call;
var Property = (function (_super) {
    __extends(Property, _super);
    function Property(base, property) {
        _super.call(this);
        this.base = base;
        this.property = property;
        this.parent = null;
        assertPropertyIsInstanceOf(this, 'base', _Node);
        assertPropertyIsInstanceOf(this, 'property', _Node);
    }
    Property.prototype.toString = function () {
        return this.base + '.' + this.property.toString();
    };
    Property.prototype.print = function () { out.write(this.toString()); };
    return Property;
})(_Node);
exports.Property = Property;
var If = (function (_super) {
    __extends(If, _super);
    function If(cond, block, elseIfs, elseBlock) {
        _super.call(this);
        this.cond = cond;
        this.block = block;
        this.elseIfs = elseIfs ? elseIfs : null;
        this.elseBlock = elseBlock ? elseBlock : null;
    }
    If.prototype.print = function () {
        var cond = this.cond.toString();
        out.write("if " + cond + " ");
        this.block.print();
        if (this.elseIfs) {
            for (var i = 0; i < this.elseIfs.length; i++) {
                var ei = this.elseIfs[i];
                cond = ei.cond.toString();
                out.write(" else if " + cond + " ");
                ei.block.print();
            }
        }
        if (this.elseBlock) {
            out.write(" else ");
            this.elseBlock.print();
        }
    };
    return If;
})(_Node);
exports.If = If;
var While = (function (_super) {
    __extends(While, _super);
    function While(expr, block) {
        _super.call(this);
        this.expr = expr; // Loop expression
        this.block = block;
    }
    While.prototype.print = function () {
        out.write("while " + this.expr.toString() + " ");
        this.block.print();
    };
    return While;
})(_Node);
exports.While = While;
var For = (function (_super) {
    __extends(For, _super);
    function For(init, cond, after, block) {
        _super.call(this);
        this.init = init; // Initialization
        this.cond = cond; // Condition
        this.after = after; // Afterthought
        this.block = block;
    }
    For.prototype.print = function () {
        out.write("for ");
        // Don't indent while we're writing out these statements
        var i = _ind;
        _ind = 0;
        this.init.print();
        out.write('; ');
        this.cond.print();
        out.write('; ');
        this.after.print();
        out.write(' ');
        // Restore indent and print the block
        _ind = i;
        this.block.print();
    };
    return For;
})(_Node);
exports.For = For;
/*
export class Chain extends _Node {
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
var Return = (function (_super) {
    __extends(Return, _super);
    function Return(expr) {
        _super.call(this);
        this.expr = expr;
    }
    Return.prototype.print = function () { out.write(this.toString()); };
    Return.prototype.toString = function () {
        if (this.expr) {
            return 'return ' + this.expr.toString();
        }
        return 'return';
    };
    return Return;
})(_Node);
exports.Return = Return;
var Root = (function (_super) {
    __extends(Root, _super);
    function Root(statements) {
        _super.call(this);
        this.includeTypes = false;
        this.statements = statements;
        this.sourceMap = null;
        this.scope = null;
        // Lists of import and export nodes; the nodes add themselves during
        // type-system walking
        this.imports = [];
        this.exports = [];
    }
    Root.prototype.print = function () {
        _include_types = this.includeTypes;
        _win("root {\n");
        this.statements.forEach(function (stmt) {
            _w('');
            stmt.print();
            out.write("\n");
        });
        _wout("}\n");
    };
    Root.prototype.getRootScope = function () {
        var rootScope = this.scope.parent;
        if (!rootScope || !rootScope.isRoot) {
            throw new TypeError('Missing root scope', this);
        }
        return rootScope;
    };
    return Root;
})(_Node);
exports.Root = Root;
var Block = (function (_super) {
    __extends(Block, _super);
    function Block(statements) {
        _super.call(this);
        this.statements = statements;
        this.scope = null;
        // Set the `isLastStatement` property on the last statement
        var lastStatement = statements[statements.length - 1];
        if (lastStatement) {
            lastStatement.isLastStatement = true;
        }
    }
    Block.prototype.print = function () {
        out.write("{\n");
        _ind += INDENT;
        this.statements.forEach(function (stmt) {
            _w('');
            stmt.print();
            out.write("\n");
        });
        _ind -= INDENT;
        _w('}');
        // out.write(repeat(' ', _ind - INDENT) + '}')
    };
    return Block;
})(_Node);
exports.Block = Block;
/*
var mod = {
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
  // Chain: Chain,
  Return: Return,
  Call: Call,
  Property: Property
}
export = mod
*/
