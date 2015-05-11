/// <reference path="../typescript/source-map-0.1.38.d.ts" />

import AST       = require('../ast')
import errors    = require('../errors')
import types     = require('../types')
import sourcemap = require('source-map')

type SourceNode = sourcemap.SourceNode

var repeat     = require('../util').repeat,
    SourceNode = sourcemap.SourceNode,
    TypeError  = errors.TypeError

// Context of the compiler as it's generating code ----------------------------

class Context {
  conflicts: any
  filesCompiled: any[]
  private _indent: number

  constructor() {
    // Keeping track of names that conflict and need to be munged/aliased.
    this.conflicts = {}
    this._indent = 0
    // Keep track of files that we've compiled
    this.filesCompiled = []
  }
  incrementIndent() { this._indent += 2 }
  decrementIndent() { this._indent -= 2 }
  indent(additionalIndent: number = 0): string {
    if (additionalIndent === undefined) { additionalIndent = 0 }
    return repeat(' ', this._indent + additionalIndent)
  }
}

function wrapContextIndent (context: Context, func: (Context) => void) {
  context.incrementIndent()
  var ret = func.call(this, context)
  context.decrementIndent()
  return ret
}
function wrapContextNoop (context: Context, func: (Context) => void) {
  return func.call(this, context)
}

interface CompileOptions {
  // If we're compiling a single file without imports and exports
  singleFile?: boolean
  // If it's being compiled as a statement
  statement?: boolean
  omitTerminator?: boolean
  // List of imported and exported
  imported?: any[]
  exported?: any[]
}
function getDefaultCompileOptions(): CompileOptions {
  return {singleFile: false, statement: false}
}

var InternalFile = '(internal)'
// Return a string as a SourceNode with position information copied from the
// given node.
function asSourceNode (node: AST.Node, ret: any[]): SourceNode {
  var name = node.constructor['name']
  if (node._file === InternalFile) {
    return new SourceNode(1, 0, node._file, ret)
  }
  if (node._line === undefined)   { throw new Error('Missing line for '+name) }
  if (node._column === undefined) { throw new Error('Missing column for '+name) }
  if (node._file === undefined)   { throw new Error('Missing file for '+name) }
  return new SourceNode(node._line, node._column, node._file, ret)
}

// JS code-generating compiler ------------------------------------------------

export class JSCompiler {
  context: Context

  compileRoot(root: AST.Root, opts) {
    opts = (opts ? opts : {})
    // Compilation context
    this.context = new Context()
    var nodes = null
    // Use the "single" flag to force it to just compile the tree without
    // including imports/exports
    if (root.file && !opts.single) {
      // Go through the dependency processing and such if it's hooked into the
      // compiler and files.
      nodes = this.compileRootAsDependency(root)
      // TODO: Refactor how we compute the unique names/IDs of each file/module

      // Prepend our platform support stuff
      nodes.unshift("var requirejs = require('requirejs'),\n",
                    "    define = requirejs.define;\n")
      // Require the main file to make it be the entry point
      var entryName = JSON.stringify(root.file.path)
      nodes.push("requirejs("+entryName+");\n")
    } else {
      // Just got a plain tree to compile
      nodes = this.compileRootStatements(root, {singleFile: true})
    }
    // And build the big output node
    var compiled = new SourceNode(1, 0, null, nodes).toStringWithSourceMap()
    root.sourceMap = compiled.map
    return compiled.code
  }// compileRoot

  compileRootAsDependency(root: AST.Root) {
    var self = this
    this.context.filesCompiled.push(root.file)
    var depsToCompile = root.file.dependencies.filter(function (dep) {
      if (self.context.filesCompiled.indexOf(dep) !== -1) {
        return false
      }
      return true
    })
    // Set up each dependency as a source node
    var sourceNodes = depsToCompile.map(function (file) {
      var root = file.tree
      return self.compileRootAsDependency(root)
    })
    // Then add on our actual statements
    sourceNodes = sourceNodes.concat(this.compileRootStatements(root))
    return sourceNodes
  }// compileRootAsDependency

  // TODO: Cleanup this function! Way too much messy indent stuff.
  // TODO: If no imports or exports detected and it's the root then just skip
  //       the RequireJS stuff altogether
  compileRootStatements(root: AST.Root, compileOpts?: CompileOptions) {
    if (!compileOpts) { compileOpts = getDefaultCompileOptions() }
    var isSingle: boolean = compileOpts.singleFile,
        self              = this
    // Set up the return buffer
    var ret = []
    // TODO: Just use `root.imports` and `root.exports` for handling imports
    //       and exports rather than rescanning for them
    // Setup the root options
    var opts = {imported: [], exported: [], singleFile: isSingle},
        ind  = null

    // Set the wrapping function based on whether or not to indent
    var wrap = (isSingle ? wrapContextNoop : wrapContextIndent)
    // Call the wrapper and compile our statements
    wrap(this.context, function () {
      ind = self.context.indent()
      for (var i = 0; i < root.statements.length; i++) {
        ret.push(ind)
        var stmt = root.statements[i]
        ret.push(self.compileStatement(stmt, opts))
      }
    })

    if (!isSingle) {
      this.wrapRootInDefine(root, opts, ret)
    }
    return ret
  }// compileRootStatements

  wrapRootInDefine(root: AST.Root, opts: CompileOptions, ret: any[]) {
    // Build the "define(...)" for this module
    var self = this,
        name = JSON.stringify(root.file.path),
        ind  = this.context.indent(2),
        def  = this.context.indent()+"define("+name+', '

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
        exp += self.context.indent(4)+JSON.stringify(name)+': '+name+"\n"
      })
      exp += ind+"}\n"
      ret.push(exp)
    }

    // Close the define() function
    ret.push(this.context.indent()+"});\n")
  }// wrapRootInDefine

  compileStatement(stmt: AST.Node, opts?: CompileOptions) {
    opts = (opts ? opts : getDefaultCompileOptions())
    // if (stmt instanceof AST.Binary || stmt instanceof AST.Chain) {
    if (stmt instanceof AST.Binary) {
      opts.statement = true
    }
    switch (stmt.constructor) {
    case AST.Class:
      return this.compileClass(<AST.Class>stmt, opts)
    case AST.Assignment:
      return this.compileAssignment(<AST.Assignment>stmt, opts)
    case AST.Return:
      return this.compileReturn(<AST.Return>stmt, opts)
    case AST.Export:
      return this.compileExport(<AST.Export>stmt, opts)
    case AST.Import:
      return this.compileImport(<AST.Import>stmt, opts)
    case AST.If:
      return this.compileIf(<AST.If>stmt, opts)
    case AST.While:
      return this.compileWhile(<AST.While>stmt, opts)
    case AST.For:
      return this.compileFor(<AST.For>stmt, opts)
    case AST.Multi:
      return this.compileMulti(<AST.Multi>stmt, opts)
    default:
      var ExpressionStatements: any[] = [
        AST.Function,
        AST.New,
        AST.Property
      ];
      if (ExpressionStatements.indexOf(stmt.constructor) !== -1) {
        return this.compileExpression(stmt, opts)
      }
      throw new TypeError('Cannot compile statement: '+stmt.constructor['name'])
    }
  }// compileStatement
  
  compileExpression(expr: AST.Node, opts?: CompileOptions) {
    opts = (opts ? opts : getDefaultCompileOptions())
    switch (expr.constructor) {
    case AST.Literal:
      return this.compileLiteral(<AST.Literal>expr, opts)
    case AST.Function:
      return this.compileFunction(<AST.Function>expr, opts)
    case AST.Property:
      return this.compileProperty(<AST.Property>expr, opts)
    case AST.Call:
      return this.compileCall(<AST.Call>expr, opts)
    case AST.Identifier:
      return this.compileIdentifier(<AST.Identifier>expr, opts)
    case AST.New:
      return this.compileNew(<AST.New>expr, opts)
    case AST.Binary:
      return this.compileBinary(<AST.Binary>expr, opts)
    default:
      throw new TypeError('Cannot compile expression: '+expr.constructor['name'])
    }
  }// compileExpression

  compileIf(stmt: AST.If, opts: CompileOptions) {
    var ind = this.context.indent()
    var ret = ['if (', this.compileExpression(stmt.cond, opts), ") {\n"]
    ret.push(this.compileBlock(stmt.block))
    ret.push(ind+"}")
    if (stmt.elseIfs) {
      for (var i = 0; i < stmt.elseIfs.length; i++) {
        var ei = stmt.elseIfs[i]
        ret.push(' else if (',this.compileExpression(ei.cond, opts), ") {\n")
        ret.push(this.compileBlock, opts)
        ret.push(ind+"}")
      }
    }
    ret.push("\n")
    return asSourceNode(stmt, ret)
  }

  compileWhile(stmt: AST.While, opts: CompileOptions) {
    var ind = this.context.indent()
    var ret = [
      'while (',
        this.compileExpression(stmt.expr, {omitTerminator: true}),
      ') {\n',
        this.compileBlock(stmt.block),
      ind + "}\n"
    ]
    return asSourceNode(stmt, ret)
  }

  compileFor(stmt: AST.For, opts: CompileOptions) {
    var cond = stmt.cond
    var ind  = this.context.indent()
    var ret  = ['for (']
    ret.push(this.compileStatement(stmt.init, {omitTerminator: true}), '; ')
    ret.push((cond ? this.compileExpression(stmt.cond) : ''), '; ')
    ret.push(this.compileStatement(stmt.after, {omitTerminator: true}), ") {\n")
    ret.push(this.compileBlock(stmt.block))
    ret.push(ind+"}\n")
    return asSourceNode(stmt, ret)
  }

  importExportPreCheck(opts: CompileOptions) {
    if (!opts.imported || !opts.exported) {
      throw new Error('Missing import and/or exported lists for statement')
    }
  }

  compileExport(exp: AST.Export, opts: CompileOptions) {
    this.importExportPreCheck(opts)
    opts.exported.push(exp.name)
    return asSourceNode(exp, ['// '+exp.toString()+"\n"])
  }// compileExport

  compileImport(imp: AST.Import, opts: CompileOptions) {
    this.importExportPreCheck(opts)
    opts.imported.push([imp.file, imp.name])
    // We need to output something; so let's just leave a nice little comment
    // about what's going on as it compiled this node.
    return asSourceNode(imp, ['// '+imp.toString()+"\n"])
  }// compileImport

  compileReturn(ret: AST.Return, opts: CompileOptions) {
    var body: any[] = ["return;\n"]
    if (ret.expr) {
      var returnValue = this.compileExpression(ret.expr, {omitTerminator: true})
      body = ['return ', returnValue, ";\n"]
    }
    return asSourceNode(ret, body)
  }

  compileBinary(bin: AST.Binary, opts: CompileOptions) {
    var lexpr: SourceNode = this.compileExpression(bin.lexpr, {omitTerminator: true})
    var rexpr: SourceNode = this.compileExpression(bin.rexpr, {omitTerminator: true})
    var ret = [lexpr, ' '+bin.op+' ', rexpr]
    if (opts && opts.statement === true) { ret.push(";\n") }
    return asSourceNode(bin, ret)
  }

  compileProperty(prop: AST.Property, opts: CompileOptions) {
    var term = ";\n"
    if (opts && opts.omitTerminator === true) { term = '' }

    var property = prop.property
    if (typeof property !== 'string') {
      property = this.compileExpression(prop.property, {omitTerminator: true})
    }
    var base = this.compileIdentifier(prop.base, {omitTerminator: true})
    var ret = [base, '.' , property, term]
    return asSourceNode(prop, ret)
  }// compileProperty

  compileCall(call: AST.Call, opts: CompileOptions) {
    var self = this
    var args = call.args.map(function (arg) {
      return self.compileExpression(arg, {omitTerminator: true})
    })
    var ret = [this.compileExpression(call.base), '(']
    var length = args.length, lastIndex = args.length - 1
    for (var i = 0; i < length; i++) {
      ret.push(args[i])
      if (i !== lastIndex) {
        ret.push(', ')
      }
    }
    ret.push(')')
    if (!opts || opts.omitTerminator !== true) { ret.push(";\n") }
    return asSourceNode(call, ret)
  }// compileCall

  compileNew(n: AST.New, opts: CompileOptions) {
    var self = this,
        name = n.name,
        args = n.args.map(function (arg) {
          return self.compileExpression(arg, opts)
        })
    // Interpose commas between the args
    args = interpose(args, ', ')
    var ret = ["(new "+name+"(", args, "))"]
    return asSourceNode(n, ret)
  }// compileNew

  compileFunction(func: AST.Function, opts: CompileOptions) {
    // Skip compilation for functions that are children of multi types
    if (func.isChildOfMulti()) { return asSourceNode(func, [""]) }

    var args = func.args.map(function (arg) { return arg.name }),
        ret  = ['function ', (func.name ? func.name : ''), ' (', args.join(', '), ") {\n"],
        defs = []
    // Build assignments for any default arguments
    for (var i = args.length - 1; i >= 0; i--) {
      var arg  = func.args[i],
          name = arg.name,
          def  = arg.def
      if (def) {
        var value = this.compileExpression(def, opts),
            ind   = this.context.indent(2)
        // Prepend the default to the body
        defs = [ind, 'if (', name, ' === undefined) { ', name, ' = ', value, "; }\n"].concat(defs)
      }
    }// for args
    ret = ret.concat(defs)
    ret.push(this.compileBlock(func.block))
    ret.push(this.context.indent()+"}")
    // Name property indicates it's a function statement
    if (func.name) {
      ret.push("\n")
    }
    return asSourceNode(func, ret)
  }// compileFunction

  compileAssignment(assg: AST.Assignment, opts: CompileOptions) {
    var self = this,
        term = ";\n",
        ret  = null
    if (opts && opts.omitTerminator === true) { term = '' }

    if (assg.type === 'var' || assg.type === 'let') {
      // TODO: Register name in context scope and check for conflicts.
      var lvalue = assg.lvalue.name
      if (assg.rvalue !== false) {
        var rvalue = this.compileExpression(assg.rvalue, {omitTerminator: true})
        ret = ['var ', lvalue, ' '+assg.op+' ', rvalue, term]
      } else {
        ret = ['var ', lvalue, term]
      }
    } else {
      // TODO: Handle more complex path assignments
      // throw new Error('Compilation of path-assignments not yet implemented')
      var lvalue = assg.lvalue.name
      assg.lvalue.path.forEach(function (item) {
        lvalue += '.'+self.compileIdentifier(item, opts)
      })
      var rvalue = this.compileExpression(assg.rvalue, {omitTerminator: true})
      ret = [lvalue, ' '+assg.op+' ', rvalue, term]
    }
    return asSourceNode(assg, ret)
  }// compileAssignment

  compileIdentifier(id: AST.Identifier, opts: CompileOptions) {
    return asSourceNode(id, id.name)
  }

  compileLiteral(literal: AST.Literal, opts: CompileOptions) {
    if (literal.typeName === 'String') {
      return JSON.stringify(literal.value)
    }
    return literal.value.toString()
  }
  
  compileClass(klass: AST.Class, opts: CompileOptions) {
    var self = this,
        name = klass.name
    // Generate the simple class function
    // TODO: Multiple-dispatch initializers
    var ret: any[] = ["function "+name+" () {\n"]

    // Compile the initializer preamble
    ret = ret.concat(this.compileClassPreamble(klass))

    // Compile the initializers
    var initializers = []
    for (var i = 0; i < klass.definition.statements.length; i++) {
      var node = klass.definition.statements[i]
      if (node instanceof AST.Init) {
        initializers.push(node)
      }
    }
    // TODO: Add fast codepath for just one initializer
    if (initializers.length > 0) {
      ret.push(this.compileClassInitializers(initializers))
    }

    ret.push(this.context.indent()+"}\n")
    // Now add all the instance methods
    klass.definition.statements.forEach(function (node) {
      if ([AST.Function, AST.Multi].indexOf(node.constructor) === -1) { return }
      var methodName       = node.name,
          methodDefinition = self.compileStatement(node, opts)
      if (methodDefinition.children.length > 0) {
        ret.push(name+".prototype."+methodName+" = ", methodDefinition)
      }
    })
    return asSourceNode(klass, ret)
  }// compileClass

  compileClassPreamble(klass: AST.Class) {
    var letProperties = [],
        varProperties = []
    klass.definition.statements.forEach(function (node) {
      if (!(node instanceof AST.Assignment)) { return }
      if (node.type === 'var') {
        varProperties.push(node)
      }
      if (node.type === 'let') {
        letProperties.push(node)
      }
    })
    var self = this,
        ind  = this.context.indent(2),
        ret  = []
    // By default we'll do the same handling for both properties' defaults
    var properties = letProperties.concat(varProperties)
    properties.forEach(function (prop) {
      // Skip properties with no default
      if (!prop.rvalue) { return }
      var rvalue = self.compileExpression(prop.rvalue)
      ret.push(ind+'this.'+prop.lvalue.name+' = ', rvalue, ";\n")
    })
    return ret
  }// compileClassPreamble

  compileClassInitializers(initializers: AST.Init[]) {
    this.context.incrementIndent()
    var self = this,
        ind  = this.context.indent()
    // Compute the length branches
    var branches = {}
    initializers.forEach(function (init) {
      var argsLength = init.args.length
      if (branches[argsLength]) {
        throw new Error('Multiple initializers taking '+argsLength+' arguments')
      }
      branches[argsLength] = init
    })
    // Build the dispatcher
    var ret = [ind+"switch (arguments.length) {\n"]
    var branchLengths = Object.keys(branches)
    branchLengths.forEach(function (branchLength) {
      ret.push(ind+'  case '+branchLength+":\n")
      // Build the arguments for the branch
      var args   = [],
          length = parseInt(branchLength, 10)
      for (var i = 0; i < length; i++) {
        args.push('arguments['+i+']')
      }
      var argsString = ''
      if (args.length > 0) {
        argsString = ', '+args.join(', ')
      }
      ret.push(ind+'    init'+branchLength+".call(this"+argsString+"); break;\n")
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
      ret.push(self.compileBlock(branch.block))
      ret.push(ind+"}\n")
    })
    this.context.decrementIndent()
    return ret
  }// compileClassInitializers

  compileBlock(block: AST.Block) {
    var self = this
    return wrapContextIndent(this.context, function () {
      var ret = [],
          ind = self.context.indent()
      block.statements.forEach(function (stmt) {
        ret.push(ind, self.compileStatement(stmt))
      })
      return asSourceNode(block, ret)
    })
  }// compileBlock

  compileMulti(multi: AST.Multi, opts: CompileOptions) {
    var self       = this,
        args       = multi.args.map(function (arg) { return arg.name }),
        joinedArgs = args.join(', '),// joinedArgs = interpose(args, ', ')
        name       = (!multi.type.isInstanceMethod ? multi.name+' ' : '')
    // Build function definition
    var ret = ['function ', name, '(', joinedArgs, ") {\n"]
    this.context.incrementIndent()

    // Figure out the branches for the dispatcher
    var definitions    = [],
        cond           = [],
        branchBaseName = multi.name
    for (var i = multi.type.functionNodes.length - 1; i >= 0; i--) {
      var fn = multi.type.functionNodes[i]
      fn.childName = branchBaseName+'_'+(i+1)
      if (fn.when) {
        cond.push(fn)
      } else {
        // If there's no `when` condition then it's a default
        definitions.push(fn)
      }
    }
    if (definitions.length > 1) {
      var n = definitions.length
      throw new TypeError('Multi has more than 1 default branch (has '+n+')', this)
    }
    // De-array default to just the node or null
    var def = (definitions.length === 0) ? null : definitions[0]

    // Build the dispatcher
    var ind = this.context.indent()
    ret.push(ind+"switch (false) {\n")
    cond.forEach(function (condFunction) {
      var childName = condFunction.childName,
          caseExpr  = self.compileExpression(condFunction.when)
      ret.push(ind+"case !(", caseExpr, "):\n")
      ret.push(self.context.indent(2)+"return "+childName+"(", joinedArgs, ");\n")
    })
    ret.push(ind+"default:\n")
    var defInd = this.context.indent(2)
    if (def) {
      ret.push(defInd+"return "+def.childName+"(", joinedArgs, ");\n")
    } else {
      ret.push(defInd+"throw new Error('Fell through to default branch');\n")
    }
    ret.push(ind+"}\n")

    // Build all of the implementation functions
    ind = this.context.indent()
    var i = 1
    multi.type.functionNodes.forEach(function (functionNode) {
      var functionName = functionNode.childName,
          functionArgs = functionNode.args.map(function (arg) { return arg.name })
      ret.push(ind+'function '+functionName+" ("+functionArgs.join(', ')+") {\n")
      ret.push(self.compileBlock(functionNode.block))
      ret.push(ind+"}\n")

      i += 1
    })

    this.context.decrementIndent()
    ret.push(this.context.indent()+"}\n")
    return asSourceNode(multi, ret)
  }// compileMulti

}// JSCompiler


function interpose (arr: any[], sep: string) {
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

interface Array<T> {
  extend: (other: Array<any>) => void;
}
Array.prototype['extend'] = function (other) {
  var len = other.length
  for (var i = 0; i < len; i++) {
    this.push(other[i])
  }
}

