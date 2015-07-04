import AST    = require('../ast')
import errors = require('../errors')
import scope  = require('../typesystem/scope')
import types  = require('../types')

import BinaryOps      = require('./llvm/binary-ops')
import Builtins       = require('./llvm/builtins')
import nativeTypes    = require('./llvm/native-types')
import NativeFunction = require('./llvm/native-function')
import slots          = require('./llvm/slots')
import util           = require('./llvm/util')

var _            = require('lodash'),
    Scope        = scope.Scope,
    ClosingScope = scope.ClosingScope,
    LLVM         = require('./llvm/library'),
    TypeError    = errors.TypeError,
    ICE          = errors.InternalCompilerError,
    // Target information and setup
    target       = require('./llvm/target'),
    NativeObject = require('./llvm/native-object')

var isLastInstructionTerminator = util.isLastInstructionTerminator,
    compileTruthyTest           = util.compileTruthyTest,
    assertInstanceOf            = util.assertInstanceOf

const nativeTypeForType = nativeTypes.nativeTypeForType
const ObjectPrefix = types.Object.prototype['getNativePrefix']()

// Unbox the slots module
var Slots         = slots.Slots,
    ConstantSlots = slots.ConstantSlots,
    GlobalSlots   = slots.GlobalSlots

var Int1Type    = LLVM.Types.Int1Type,
    Int8Type    = LLVM.Types.Int8Type,
    Int32Type   = LLVM.Types.Int32Type,
    Int64Type   = LLVM.Types.Int64Type,
    VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var Int32Zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

// LLVM library functions we'll be using
var TypeOf            = LLVM.Library.LLVMTypeOf,
    GetTypeKind       = LLVM.Library.LLVMGetTypeKind,
    DumpType          = function (ty) { LLVM.Library.LLVMDumpType(ty); console.log('') },
    PrintTypeToString = LLVM.Library.LLVMPrintTypeToString,
    GetParam          = LLVM.Library.LLVMGetParam,
    PointerTypeKind   = GetTypeKind(Int8PtrType),
    ConstInt          = LLVM.Library.LLVMConstInt


interface GlobalsMap {
  [index: string]: Buffer
}

class Context {
  // Externally linked functions
  extern:       any = {}
  // Globals
  globals:      GlobalsMap = {}
  builder:      any
  module:       any
  isMain:       boolean
  targetModule: any
  dumpModule:   boolean
  logger:       any
  globalSlots:  any
  slotsMap:     any
  outputs:      any
  castValuePointers: boolean
  noCastValuePointers: boolean

  addGlobal(name: string, type: Buffer): Buffer {
    assertInstanceOf(type, Buffer)
    var global = LLVM.Library.LLVMAddGlobal(this.module.ptr, type, name)
    this.globals[name] = global
    return global
  }
  getGlobal(name): Buffer {
    if (!this.hasGlobal(name)) { throw new Error('Global definition not found: '+name) }
    return this.globals[name]
  }
  hasGlobal(name): boolean {
    return (this.globals[name] ? true : false)
  }
  buildGlobalLoad(name): Buffer {
    var global = this.globals[name],
        // ptr = this.builder.buildGEP(global, [Int32Zero], name),
        val    = this.builder.buildLoad(global, name)
    return val
  }
}


class BlockContext {
  ctx: Context
  fn: any
  block: any
  slots: any

  constructor(ctx: Context, parentFn, block, slots) {
    this.ctx   = ctx
    this.fn    = parentFn
    this.block = block
    this.slots = slots
  }
}

interface ExprContext {
  type?:  any
  value?: Buffer
  path?:  any[]

  parentType?:  any
  parentValue?: Buffer
}

class BasicLogger {
  info = console.log
}


function unboxInstanceType (instance: any, expectedType?) {
  assertInstanceOf(instance, types.Instance)
  var type = instance.type
  if (expectedType !== undefined) {
    assertInstanceOf(type, expectedType)
  }
  return type
}

var StaticPassManager = null
function getStaticPassManager () {
  if (StaticPassManager) { return StaticPassManager }

  // Setup the pass manager and optimization passes
  var pm = LLVM.Library.LLVMCreatePassManager()
  // Global value numbering
  LLVM.Library.LLVMAddGVNPass(pm)
  // Promote by-reference arguments to scalar
  LLVM.Library.LLVMAddArgumentPromotionPass(pm)
  // Convert aggregate stack allocations into SSA registers
  LLVM.Library.LLVMAddScalarReplAggregatesPass(pm)
  // Convert load-stores on stack allocated slots to phi nodes
  LLVM.Library.LLVMAddPromoteMemoryToRegisterPass(pm)
  // Remove local-block-redundant stores
  LLVM.Library.LLVMAddDeadStoreEliminationPass(pm)
  // Merge duplicate global constants
  LLVM.Library.LLVMAddConstantMergePass(pm)
  // Turns induction variables (ie. in loops) into simpler forms for easier
  // later analysis
  LLVM.Library.LLVMAddIndVarSimplifyPass(pm)
  // Simple dead code elimination and basic-block merging
  //   http://llvm.org/docs/Passes.html#simplifycfg-simplify-the-cfg
  LLVM.Library.LLVMAddCFGSimplificationPass(pm)
  // Save the pass manager and return it
  StaticPassManager = pm
  return pm
}

export class LLVMCompiler {
  ctx: Context

  emitToFile(root: AST.Root, opts: any) {
    if (opts.module) {
      assertInstanceOf(opts.module, types.Module)
    }
    var ctx = new Context()
    ctx.isMain       = (opts.module ? false : true)
    ctx.targetModule = (opts.module ? opts.module : null)
    ctx.dumpModule   = (opts.dump ? true : false)
    ctx.module       = new LLVM.Module(ctx.targetModule ? ctx.targetModule.getNativeName() : 'main')
    ctx.builder      = new LLVM.Builder()
    // Slots for global values (eg. builtins)
    ctx.globalSlots  = new GlobalSlots()
    // Maps sets of Slots to their associated Scope by the scope's ID
    ctx.slotsMap     = {}
    // Configuration options
    ctx.castValuePointers   = false
    ctx.noCastValuePointers = !ctx.castValuePointers
    ctx.logger              = (opts.logger ? opts.logger : (new BasicLogger()))
    // List of output bitcode files built
    ctx.outputs = (opts.outputs ? opts.outputs : [])
    // Add ourselves to that list
    var outFile = bitcodeFileForSourceFile(root.file.path)
    ctx.outputs.push(outFile)

    var mainType = new LLVM.FunctionType(Int32Type, [], false),
        mainFunc = null
    if (ctx.isMain) {
      // Set up the main function
      mainFunc = ctx.module.addFunction('main', mainType)
      // Also setup information about our compilation target
      target.initializeTarget(ctx)
    } else {
      var initName = ctx.targetModule.getNativeName()+'_init'
      mainFunc = ctx.module.addFunction(initName, mainType)
    }
    var mainEntry = mainFunc.appendBasicBlock('entry')

    // Add the root scope to the slots map
    var rootScope = root.scope.parent
    if (!rootScope.isRoot) { throw new Error("Couldn't find root scope") }
    ctx.slotsMap[rootScope.id] = ctx.globalSlots

    // Add the builtins
    Builtins.compile(ctx, mainEntry, root)
    BinaryOps.initialize(ctx, rootScope)

    // Setup the entry into the function
    ctx.builder.positionAtEnd(mainEntry)
    if (ctx.isMain === true) {
      // Initialize the GC
      ctx.builder.buildCall(ctx.extern.GC_init, [], '')
    }

    // Compile ourselves in the main entry function
    this.ctx = ctx
    this.buildMain(root, mainFunc, mainEntry)

    // Run the optimization passes
    LLVM.Library.LLVMRunPassManager(getStaticPassManager(), ctx.module.ptr)

    if (ctx.dumpModule) {
      ctx.module.dump()
    }

    // Verify the module to make sure that nothing's amiss before we hand it
    // of to the bitcode compiler
    var errPtr = LLVM.Library.newCStringPtr()
    LLVM.Library.LLVMVerifyModule(ctx.module.ptr, LLVM.Library.LLVMAbortProcessAction, errPtr)

    ctx.module.writeBitcodeToFile(outFile)
    ctx.logger.info('Wrote bitcode to file: '+outFile)
  }

  buildMain(root: AST.Root, mainFunc, mainEntry) {
    // Compile ourselves now that setup is done
    this.compileBlock(root, mainFunc)
    this.ctx.builder.buildRet(Int32Zero)
  }

  // AST.Node.prototype.compileToValue = function () {
  //   throw new Error('Compilation to value not yet implemented for: '+this.constructor.name)
  // }

  compileBlock(block: AST.Block|AST.Root, parentFn: Buffer, preStatementsCb?) {
    var ctx = this.ctx
    // Bunch of pre-conditions to make sure we got sane arguments
    if (!(block.statements instanceof Array)) {
      throw new Error('Missing statements in block')
    }
    if (!(block.scope instanceof Scope)) {
      throw new Error('Missing block\'s scope')
    }

    // Set up slots for each local
    var slots = new Slots()
    Object.keys(block.scope.locals).forEach(function (name) {
      // var isConstant = block.scope.localHasFlag(name, Scope.Flags.Constant)
      // if (isConstant) { ... }

      var slotType  = null,
          localType = block.scope.getLocal(name)

      // Not actually going to allocate slots for Modules
      if (localType instanceof types.Module) {
        return
      }
      // In a fair amount of cases the type of a complex value (ie. function
      // or object instance of a type) may not yet have its type built
      // by this early slot-allocation stage. If that's the case then we enqueue
      // slot allocation to be retried when the types are available.
      try {
        slotType = nativeTypeForType(localType)
      } catch (err) {
        // FIXME: This is pretty brittle (and slow); let's figure out a faster
        //        way to do these checks.
        if (/^Native (object|function) not found for type/.test(err.message)) {
          slots.enqueueAlloc(name, localType)
          ctx.logger.debug('Enqueued slot allocation for: '+name+' (has type: '+localType.inspect()+')')
          return
        }
        throw err
      }
      slots.buildAlloc(ctx, name, slotType)
      ctx.logger.debug('Added slot allocation for: '+name+' (has type: '+localType.inspect()+')')
    })
    // Add the slots to the map of slots in the context
    ctx.slotsMap[block.scope.id] = slots
    // Set up a new context just for this block
    var blockCtx = new BlockContext(ctx, parentFn, block, slots)
    // If a callback to run before statements are compiled is provided then
    // call that callback with the new block-context and the slots.
    if (preStatementsCb) {
      preStatementsCb(blockCtx, slots)
    }
    // Compile all the statements
    var statements = block.statements
    for (var i = 0; i < statements.length; i++) {
      var stmt: AST.Node = statements[i]
      this.compileStatement(stmt, blockCtx)
    }// for
  }// compileBlock

  compileStatement(stmt: AST.Node, blockCtx: BlockContext) {
    switch (stmt.constructor) {
    case AST.Import:
      return this.compileImport(<AST.Import>stmt, blockCtx)
    case AST.Function:
      return this.compileFunction(<AST.Function>stmt, blockCtx)
    case AST.Export:
      return this.compileExport(<AST.Export>stmt, blockCtx)
    case AST.Assignment:
      return this.compileAssignment(<AST.Assignment>stmt, blockCtx)
    case AST.Function:
      return this.compileFunction(<AST.Function>stmt, blockCtx)
    case AST.Class:
      return this.compileClass(<AST.Class>stmt, blockCtx)
    case AST.Return:
      return this.compileReturn(<AST.Return>stmt, blockCtx)
    case AST.If:
      return this.compileIf(<AST.If>stmt, blockCtx)
    default:
      var ExpressionStatements: any[] = [
        AST.Call,
        AST.Identifier,
        AST.New,
        AST.Property
      ];
      if (ExpressionStatements.indexOf(stmt.constructor) !== -1) {
        return this.compileExpression(stmt, blockCtx)
      }
      throw new ICE('Cannot compile statement: '+stmt.constructor['name'])
    }
  }

  compileExpression(expr: AST.Node, blockCtx: BlockContext, exprCtx?: ExprContext) {
    switch (expr.constructor) {
    case AST.Property:
      return this.compileProperty(<AST.Property>expr, blockCtx, exprCtx)
    case AST.Call:
      return this.compileCall(<AST.Call>expr, blockCtx, exprCtx)
    case AST.Identifier:
      return this.compileIdentifier(<AST.Identifier>expr, blockCtx, exprCtx)
    case AST.Literal:
      return this.compileLiteral(<AST.Literal>expr, blockCtx, exprCtx)
    case AST.Function:
      return this.compileFunctionAsExpression(<AST.Function>expr, blockCtx)
    case AST.New:
      return this.compileNew(<AST.New>expr, blockCtx)
    case AST.Binary:
      return this.compileBinary(<AST.Binary>expr, blockCtx, exprCtx)
    default:
        throw new ICE('Cannot compile expression: '+expr.constructor['name'])
    }
  }

  compileImport(imp: AST.Import, blockCtx: BlockContext) {
    var nativeName = imp.file.module.getNativeName(),
        outFile    = bitcodeFileForSourceFile(imp.file.path)
    // Actually output the file
    var compiler = new LLVMCompiler()
    compiler.emitToFile(imp.file.tree, {
      module:  imp.file.module,
      logger:  this.ctx.logger,
      outputs: this.ctx.outputs,
      dump:    this.ctx.dumpModule
    })
    // Find the external module initializer
    var initName = nativeName+'_init',
        initFn   = NativeFunction.addExternalFunction(this.ctx, initName, VoidType, [])
    // And then call it so that the module gets initialized at the correct time
    this.ctx.builder.buildCall(initFn, [], '')

    var basePath = imp.file.module.getNativeName()
    if (imp.using) {
      var slots = blockCtx.slots
      // Load items from the module into the local scope
      for (var i = 0; i < imp.using.length; i++) {
        var use      = imp.using[i],
            instance = imp.file.module.getTypeOfProperty(use),
            type     = unboxInstanceType(instance),
            path     = basePath+'_'+type.getNativePrefix()+use
        // Sanity-check to make sure imp is the first time imp global has
        // been set up
        if (this.ctx.hasGlobal(path)) {
          throw new ICE('Global already exists: '+path)
        }
        var global = this.ctx.addGlobal(path, nativeTypeForType(instance)),
            value  = this.ctx.buildGlobalLoad(path)
        // Store the value in the local slot
        slots.buildSet(this.ctx, use, value)
      }
    }
  }// compileImport

  compileFunction(func: AST.Function, blockCtx) {
    var instance = func.type,
        type     = unboxInstanceType(instance, types.Function)
    if (type.parentMultiType) {
      throw new ICE('Compilation of multi-functions not yet implemented')
    } else {
      if (typeof func.name !== 'string') {
        throw new ICE('Missing name of Function statement')
      }
      var name = type.getNativePrefix()+func.name
      // Setup the native function
      var fn = new NativeFunction(name, type.args, type.ret)
      type.setNativeFunction(fn)
      // Compile the native function with our block
      this.genericCompileFunction(fn, func)
      // Set the linkage of the function to private
      LLVM.Library.LLVMSetLinkage(fn.getPtr(this.ctx), LLVM.Library.LLVMPrivateLinkage)
      // Add func to the slots
      blockCtx.slots.buildSet(this.ctx, func.name, fn.getPtr(this.ctx))
    }
  }

  compileFunctionAsExpression(func: AST.Function, blockCtx): Buffer {
    var self = this,
        fn   = getAnonymousNativeFunction(this.ctx, func)
    this.genericCompileFunction(fn, func)
    // Get the raw function as a value
    var compiledFn = fn.getPtr(this.ctx)
    return compiledFn
  }


  compileAsModuleMember(node: AST.Node, blockCtx: BlockContext, exprCtx: ExprContext) {
    switch (node.constructor) {
    case AST.Property:
      return this.compilePropertyAsModuleMember(<AST.Property>node, blockCtx, exprCtx)
    case AST.Identifier:
      return this.compileIdentifierAsModuleMember(<AST.Identifier>node, blockCtx, exprCtx)
    case AST.Call:
      return this.compileCallAsModuleMember(<AST.Call>node, blockCtx, exprCtx)
    default:
      throw new ICE('Cannot compile as module member: '+node.constructor['name'])
    }
  }

  compileProperty(prop: AST.Property, blockCtx: BlockContext, exprCtx?: ExprContext) {
    var base      = prop.base,
        parent    = prop.parent,
        property  = prop.property,
        type      = null,
        value     = null

    if (prop.base.type instanceof types.Module) {
      return this.compileAsModuleMember(prop, blockCtx, exprCtx)
    }
    if (parent === null) {
      var retCtx: any = {}
      this.compileExpression(prop.base, blockCtx, retCtx)
      type  = retCtx.type
      value = retCtx.value

    } else {
      type  = exprCtx.type
      value = exprCtx.value
    }
    assertInstanceOf(value, Buffer)
    var ret = this.compileExpression(prop.property, blockCtx, {type: type, value: value})
    if (!ret) {
      throw new ICE("Encountered a null return value")
    }
    return ret
  }
  compilePropertyAsModuleMember(prop: AST.Property, blockCtx: BlockContext, exprCtx: ExprContext) {
    var parent = null,
        path   = []
    if (parent === null) {
      var retCtx: any = {}
      this.compileAsModuleMember(prop.base, blockCtx, retCtx)
      assertInstanceOf(retCtx.path, Array)
      path = retCtx.path
    } else {
      path = exprCtx.path
    }
    return this.compileAsModuleMember(prop.property, blockCtx, {path: path})
  }

  genericCompileFunction(nativeFn: NativeFunction, node: AST.Function, preStatementsCb?) {
    var self             = this,
        block: AST.Block = node.block,
        hasThisArg       = false
    if (node instanceof AST.Init) {
      hasThisArg = true
    } else if (node instanceof AST.Function) {
      var type = unboxInstanceType(node.type, types.Function)
      hasThisArg = type.isInstanceMethod
    }
    // Predefine to be safe
    predefineTypes(this.ctx, block)

    nativeFn.defineBody(this.ctx, function (entry) {
      // Actual LLVM function that we're compiling for
      var fnPtr = nativeFn.fn
      self.compileBlock(block, fnPtr, function (blockCtx, slots) {
        var argOffset = 0
        // Setup `this` and the other function args
        if (hasThisArg) {
          argOffset = 1
          // `this` will be the first argument
          var thisValue = GetParam(nativeFn.getPtr(self.ctx), 0)
          // Store `this` in the slots
          slots.buildSet(self.ctx, 'this', thisValue)
        }
        // Handle regular arguments
        var args = node.args
        for (var i = 0; i < args.length; i++) {
          var arg      = args[i],
              argName  = arg.name,
              argValue = GetParam(nativeFn.getPtr(self.ctx), i + argOffset)
          // Store the argument value in the slot
          slots.buildSet(self.ctx, argName, argValue)
        }
        // If there was a callback to run before compiling statements, then
        // go ahead an call it
        if (preStatementsCb) {
          preStatementsCb(blockCtx, slots)
        }
      })//compileBlock

      // If it's returning Void and the last statement isn't a return then
      // go ahead an insert one for safety
      if (nativeFn.ret instanceof types.Void) {
        var lastStatement = block.statements[block.statements.length - 1]
        if (!(lastStatement instanceof AST.Return)) {
          var currentBasicBlock = LLVM.Library.LLVMGetInsertBlock(self.ctx.builder.ptr),
              hasTerminator     = isLastInstructionTerminator(currentBasicBlock)
          if (!hasTerminator) {
            self.ctx.builder.buildRetVoid()
          }
        }
      }// nativeFn.ret is types.Void
    })// nativeFn.defineBody
  }// genericCompileFunction

  compileAssignment(assg: AST.Assignment, blockCtx: BlockContext) {
    if (assg.type === 'var' || assg.type === 'let') {
      return this.compileNamedAssignment(assg, blockCtx)
    }
    if (assg.type === 'path') {
      return this.compilePathAssignment(assg, blockCtx)
    }
    throw new Error('Cannot compile assignment type: '+assg.type)
  }

  compilePathAssignment(assg: AST.Assignment, blockCtx: BlockContext) {
    // Lookup the lvalue into a receiver that we can set
    var recvPtr = this.compileAssignmentToStorable(assg, blockCtx, assg.lvalue)
    // Get the rvalue as a value to be stored in the lvalue's receiving pointer
    var rvalue = this.compileExpression(assg.rvalue, blockCtx)
    // Build the actual store into that pointer
    this.ctx.builder.buildStore(rvalue, recvPtr)
  }

  compileNamedAssignment(assg, blockCtx) {
    // Get a value pointer from the rvalue
    var rvalue = this.compileExpression(assg.rvalue, blockCtx)
    assertInstanceOf(rvalue, Buffer, 'Received non-Buffer from Node#compilerToValue')
    // Get the slot pointer
    blockCtx.slots.buildSet(this.ctx, assg.lvalue.name, rvalue)
  }

  compileAssignmentToStorable(assg, blockCtx, lvalue) {
    var ctx       = this.ctx,
        name      = lvalue.name,
        path      = lvalue.path,
        pair      = getTypeAndSlotsForName(ctx, blockCtx, name),
        slots     = pair[0],
        itemType  = pair[1],
        itemValue = null
    // If there's no path then we can just return a storable for the local
    if (!lvalue.child) {
      return slots.getStorable(name)
    } else {
      itemValue = slots.buildGet(ctx, name)
      // Get the native type and cast the pointer to it
      var nativeType = nativeTypeForType(itemType)
      itemValue = ctx.builder.buildPointerCast(itemValue, nativeType, name)
    }

    var child = lvalue.child
    while (child) {
      switch (child.constructor) {
        case AST.Identifier:
          var id = <AST.Identifier>child
          // Unbox and ensure we've got an Object we can work with
          var objType   = unboxInstanceType(itemType, types.Object),
              nativeObj = objType.getNativeObject(),
              propName  = id.name,
              propType  = id.type,
              propPtr   = nativeObj.buildStructGEPForProperty(ctx, itemValue, propName)
          // Otherwise build a dereference
          itemType  = propType
          itemValue = ctx.builder.buildLoad(propPtr, propName)
          break
        default:
          throw new ICE('Cannot handle path item type: '+child.constructor.name)
      }
      // Return storable pointer if we're the last item in the chain
      if (!child.child) { return propPtr }

      child = child.child
    }
    throw new ICE("Fall-through descending path children")
  }

  compileCall(call: AST.Call, blockCtx, exprCtx: ExprContext) {
    var self     = this,
        func     = call.parent,
        funcType = func.getInitialType(),
        value    = exprCtx.value

    if (func === null) {
      throw new ICE('Missing function for Call')
    }
    var funcType = unboxInstanceType(funcType, types.Function)

    if (funcType.isInstanceMethod) {
      var receiver = func.parent
      if (!receiver) {
        throw new ICE('Missing receiver for call')
      }
      var receiverInstance = receiver.type,
          receiverType     = unboxInstanceType(receiverInstance)
      // Check if we need to call through the instrinsic shim or can just
      // do a regular instance method call
      if (funcType.isIntrinsicShim()) {
        return this.compileIntrinsicInstanceMethodCall(call, blockCtx, exprCtx)
      } else {
        return this.compileInstanceMethodCall(call, blockCtx, exprCtx)
      }
    }

    // Pull out the function and compute the arguments
    var argValues = call.args.map(function (arg) {
          return self.compileExpression(arg, blockCtx)
        })
    // Build return call and update the context to return
    var retValue = this.ctx.builder.buildCall(value, argValues, '')
    tryUpdatingExpressionContext(exprCtx, call.type, retValue)
    return retValue
  }
  compileCallAsModuleMember(call: AST.Call, blockCtx, exprCtx: ExprContext) {
    var self           = this,
        func           = call.parent,
        fnPtr: Buffer  = null,
        args: Buffer[] = null

    if (parent === null) {
      throw new ICE('Not implemented yet')
    }
    assertInstanceOf(exprCtx.path, Array)

    var retCtx = {path: _.clone(exprCtx.path)}
    this.compileAsModuleMember(func, blockCtx, retCtx)
    var path = retCtx.path
    assertInstanceOf(path, Array)

    // Join the path and look up the function type from the box on our base
    var name = path.join('_'),
        type = <types.Function>unboxInstanceType(func.type, types.Function),
        fn   = type['getNativeFunction']()
    // If it's external (ie. C function) then we call it directly
    if (fn.external) {
      fnPtr = fn.getPtr(this.ctx)
      args  = call.args.map(function (arg) {
        return self.compileExpression(arg, blockCtx)
      })
      return self.ctx.builder.buildCall(fnPtr, args, '')
    }
    // Otherwise look up the actual function global via the name path
    var global = null
    if (this.ctx.hasGlobal(name)) {
      global = this.ctx.getGlobal(name)
    } else {
      var typeArgs = type.args.map(nativeTypeForType),
          typeRet  = nativeTypeForType(type.ret),
          fnTy     = new LLVM.FunctionType(typeRet, typeArgs, false),
          fnPtrTy  = LLVM.Types.pointerType(fnTy.ptr)
      // Add the function as a global
      global = this.ctx.addGlobal(name, fnPtrTy)
    }
    // Look up the function and call it with the arguments
    fnPtr = this.ctx.buildGlobalLoad(name)
    args = call.args.map(function (arg) {
      return arg.compileToValue(self.ctx, blockCtx)
    })
    return this.ctx.builder.buildCall(fnPtr, args, '')
  }

  compileInstanceMethodCall(call: AST.Call, blockCtx: BlockContext, exprCtx: ExprContext) {
    if(!exprCtx.parentValue) {
      throw new ICE('Missing parent value in instance method Call')
    }

    var self         = this,
        recvValue    = exprCtx.parentValue,
        recvInstance = exprCtx.parentType,
        recvType     = unboxInstanceType(recvInstance),
        func         = call.parent,
        instance     = func.getInitialType(),
        method       = instance.type

    assertInstanceOf(recvValue, Buffer)
    assertInstanceOf(method, types.Function)
    // Get the object we're going to use and compile the argument values
    var argValues = call.args.map(function (arg) {
          return self.compileExpression(arg, blockCtx)
        })
    // Get the function to call
    var methodFn = method.getNativeFunction()

    // And add the receiver object pointer and call the function
    argValues.unshift(recvValue)
    var retValue  = this.ctx.builder.buildCall(methodFn.getPtr(), argValues, '')
    exprCtx.type  = call.type
    exprCtx.value = retValue
    return retValue
  }

  compileIntrinsicInstanceMethodCall(call: AST.Call, blockCtx: BlockContext, exprCtx: ExprContext) {
    var self             = this,
        method           = <AST.Identifier>call.parent,
        receiver         = method.parent,
        receiverInstance = receiver.type,
        receiverType     = unboxInstanceType(receiverInstance)

    // Look up the shim method. The shim will get transformed into a proper call
    var shimMethodInstance = method.getInitialType(),
        shimMethod         = <types.Function>unboxInstanceType(shimMethodInstance, types.Function)
    // Look up the ultimate method via the shim
    if (!shimMethod.isIntrinsicShim()) {
      throw new ICE('Missing ultimate method for shim: '+method.name)
    }
    var ultimateMethod = shimMethod.getIntrinsicShim(),
        nativeFn       = ultimateMethod['getNativeFunction']()
          
    var argValues = call.args.map(function (arg) {
      return self.compileExpression(arg, blockCtx)
    })

    // Add the receiver to the front of the arguments
    if (!exprCtx.parentValue) {
      throw new ICE('Missing receiver in parent value')
    }
    var receiverValue = exprCtx.parentValue
    argValues.unshift(receiverValue)
    // Build the call
    var retValue = this.ctx.builder.buildCall(nativeFn.getPtr(this.ctx), argValues, '')
    tryUpdatingExpressionContext(exprCtx, call.type, retValue)
    return retValue
  }

  compileIdentifier(id: AST.Identifier, blockCtx: BlockContext, exprCtx: ExprContext) {
    var parent   = id.parent,
        newType  = null,
        newValue = null

    // First check if we're working on a module
    if (id.type instanceof types.Module) {
      return this.compileAsModuleMember(id, blockCtx, exprCtx)
    }
    if (parent === null) {
      // Look up ourselves rather than building off a parent
      var pair = getTypeAndSlotsForName(this.ctx, blockCtx, id.name)
      newValue = pair[0].buildGet(this.ctx, id.name)
      newType  = pair[1]
    } else {
      var value = exprCtx.value
      // Check the types and then build the GEP
      var objType = unboxInstanceType(exprCtx.type),
          idType  = unboxInstanceType(id.getInitialType())

      if (objType.primitive) {
        var type = objType.getTypeOfProperty(id.name, id)
        
        if (!(type instanceof types.Function)) {
          throw new ICE("Cannot compile non-Function property of primitive")
        }
        // if (type.isIntrinsicShim()) {
        //   newValue = type.getIntrinsicShim().getNativeFunction().getPtr(this.ctx)
        // } else {
        //   newValue = type.getNativeFunction().getPtr(this.ctx)
        // }
        newValue = null

      } else if (idType instanceof types.Function && idType.isInstanceMethod) {
        // Don't try to get instance method properties
        newValue = null

      } else {
        var nativeObj = objType.getNativeObject()
        // Build the pointer and load it into a value
        var ptr = nativeObj.buildStructGEPForProperty(this.ctx, value, id.name)
        newValue = this.ctx.builder.buildLoad(ptr, id.name)
      }
      newType = id.getInitialType()
    }

    if (id.child) {
      return this.compileChildOf(id, blockCtx, {
        type:        newType,
        value:       newValue,
        parentType:  (exprCtx ? exprCtx.type : null),
        parentValue: (exprCtx ? exprCtx.value : null)
      })
    } else {
      tryUpdatingExpressionContext(exprCtx, newType, newValue)
      return newValue
    }
  }

  compileChildOf(node: AST.Identifier|AST.Call, blockCtx: BlockContext, exprCtx: ExprContext) {
    if (!node.child) {
      throw new ICE('Expected child')
    }
    var child = node.child

    switch (child.constructor) {
    case AST.Identifier:
      return this.compileIdentifier(<AST.Identifier>child, blockCtx, exprCtx)
    case AST.Call:
      return this.compileCall(<AST.Call>child, blockCtx, exprCtx)
    default:
      throw new ICE('Child compilation fall-through on: '+child.constructor['name'])
    }
  }

  compileIdentifierAsModuleMember(id: AST.Identifier, blockCtx: BlockContext, exprCtx: ExprContext) {
    var path = (exprCtx.path ? exprCtx.path : []),
        type = id.type,
        name = null
    switch (type.constructor) {
      case types.Module:
        name = type.getNativeName()
        break
      case types.Instance:
        var unboxed = type.type
        assertInstanceOf(unboxed, types.Function, "Currently can only target module functions")
        name = 'F'+id.name
        break
      default:
        throw new ICE("Don't know how to handle module member of type: "+type.constructor.name)
    }
    path.push(name)
    // Update the expression context and return
    exprCtx.path = path
    return null
  }

  compileLiteral(literal: AST.Literal, blockCtx: BlockContext, exprCtx?: ExprContext) {
    var instance = literal.type,
        retType  = instance,
        retValue = null
    switch (instance.type.constructor) {
      case types.String:
        var stringValue = literal.value
        // Build a constant with our string value and return that
        retValue = this.ctx.builder.buildGlobalStringPtr(stringValue, '')
        break
      case types.Integer:
        retValue = ConstInt(Int64Type, literal.value, '')
        break
      case types.Boolean:
        var booleanValue = null
        if (literal.value === 'true') {
          booleanValue = true
        } else if (literal.value === 'false') {
          booleanValue = false
        } else {
          throw new ICE("Unexpected value when compiling boolean literal: '"+literal.value+"'")
        }
        retValue = ConstInt(Int1Type, booleanValue ? 1 : 0, '')
        break
      default:
        var name = instance.type.constructor.name
        throw new ICE('Cannot handle instance type: '+name)
    }
    tryUpdatingExpressionContext(exprCtx, retType, retValue)
    return retValue
  }

  compileClass(klass: AST.Class, blockCtx: BlockContext) {
    // Look up the computed type for this Class
    var type = klass.type

    // If it's intrinsic then make sure there are not initializers and just
    // compile the instance methods
    if (type.intrinsic) {
      if (type.initializers !== undefined) {
        throw new ICE('Intrinsic class cannot have initializers', klass)
      }
      var shimNativeObject = {
        type: type,
        defined: true,
        internalName: ObjectPrefix+klass.name
      }
      this.compileClassInstanceMethods(klass, blockCtx, shimNativeObject)
      return
    }

    // Sanity-check the initializers to make sure nothing weird is going to
    // happen when we start compiling stuff around this class
    sanityCheckInitializers(klass)

    // Then build the native object from this type
    var nativeObject = new NativeObject(type)
    type.setNativeObject(nativeObject)
    // Define the native object in the context
    nativeObject.define(this.ctx)
    // Build the initializers for the class
    this.compileClassPreinitializer(klass, blockCtx, nativeObject)
    this.compileClassInitializers(klass, blockCtx, nativeObject)
    this.compileClassInstanceMethods(klass, blockCtx, nativeObject)
  }
  compileClassPreinitializer(klass, blockCtx, nativeObject) {
    var self         = this,
        initArgs     = [new types.Instance(klass.type)],
        initRet      = blockCtx.block.scope.get('Void'),
        properties   = klass.properties,
        nativeObject = klass.type.getNativeObject()
    // Native function for the pre-initializer
    var fn = new NativeFunction(nativeObject.internalName+'_pi', initArgs, initRet)
    fn.defineBody(this.ctx, function (entry) {
      var recv = GetParam(fn.getPtr(self.ctx), 0)
      for (var i = 0; i < properties.length; i++) {
        var prop  = properties[i],
            name  = prop.lvalue.name,
            value = prop.rvalue
        if (value === false) { continue }
        // Set the value on the property of the new instance
        var ptr = nativeObject.buildStructGEPForProperty(self.ctx, recv, name)
        // Compile the value to a value
        value = self.compileExpression(value, blockCtx)
        self.ctx.builder.buildStore(value, ptr, '.'+name)
      }
      self.ctx.builder.buildRetVoid()
    })
    // Expose the native function on the type
    klass.type.nativePreinitializer = fn
  }
  compileClassInitializers(klass: AST.Class, blockCtx, nativeObject) {
    var self           = this,
        type           = klass.type,
        preinitializer = klass.type.nativePreinitializer,
        nativeObject   = type.getNativeObject(),
        initializers   = klass.initializers
    if (!preinitializer) {
      throw new TypeError('Missing preinitializer on class', klass)
    }
    // Build and compile a native function for each initializer function
    for (var i = 0; i < initializers.length; i++) {
      var init         = initializers[i],
          initType     = init.type,
          internalName = nativeObject.internalName+'_i'+i
      // Make a copy of the initializer args and prepend an argument for the
      // instance of the type being initialized (ie. `this`)
      var initArgs = _.clone(initType.args)
      initArgs.unshift(new types.Instance(type))
      // Create the native function
      var fn = new NativeFunction(internalName, initArgs, initType.ret)
      // Need to add a call to the preinitializer
      this.genericCompileFunction(fn, init, function (blockCtx, slots) {
        var ptr  = preinitializer.getPtr(),
            recv = GetParam(fn.getPtr(self.ctx), 0)
        self.ctx.builder.buildCall(ptr, [recv], '')
      })

      // Add this native function to the native object's list of initializers
      // and to the initializer function type
      nativeObject.addInitializer(fn)
      initType.setNativeFunction(fn)
    }
  }
  compileClassInstanceMethods(klass: AST.Class, blockCtx: BlockContext, nativeObject) {
    var type = klass.type
    // Iterate over our definition and find each instance method
    var statements = klass.definition.statements
    for (var i = 0; i < statements.length; i++) {
      var anyStatement: AST.Node = statements[i]
      // Skip over non-functions
      if (!(anyStatement instanceof AST.Function)) { continue }
      var stmt     = <AST.Function>anyStatement,
          instance = stmt.type
      // Mark the type as an instance method
      var type = unboxInstanceType(instance)
      if (type.isInstanceMethod !== true) {
        throw new ICE('Encountered non-instance-method in class definition')
      }
      var internalName = nativeObject.internalName+'_m'+stmt.name,
          args         = _.clone(type.args)
      // Add the instance as the first argument
      args.unshift(new types.Instance(nativeObject.type))
      // Build the actual function
      var fn = new NativeFunction(internalName, args, type.ret)
      this.genericCompileFunction(fn, stmt)
      // Save the native function
      type.setNativeFunction(fn)
    }
  }

  compileNew(node: AST.New, blockCtx: BlockContext) {
    var type         = node.constructorType,
        args         = node.args,
        nativeObject = type.getNativeObject()
    // Look up the initializer determined by the typesystem
    var initializer = node.getInitializer()
    // Figure out the correct NativeFunction to use to initialize this object
    var init: NativeFunction = initializer['getNativeFunction']()
    // Compile all of the arguments down to values
    var argValues = []
    for (var i = 0; i < args.length; i++) {
      var arg = args[i]
      argValues.push(arg.compileToValue(this.ctx, blockCtx))
    }

    // Allocate the new instance of the class through the GC
    var structType   = nativeObject.structType,
        sizeInt      = nativeObject.sizeOf(this.ctx),
        gcMallocPtr  = this.ctx.extern.GC_malloc.ptr
    // Call the GC allocator
    var objPtr = this.ctx.builder.buildCall(gcMallocPtr, [sizeInt], '')
    // Cast it to the right type (from just a plain pointer)
    objPtr = this.ctx.builder.buildPointerCast(objPtr, LLVM.Types.pointerType(structType), '')

    // Call the initializer function on the object
    var initFn = init.getPtr(this.ctx)
    argValues.unshift(objPtr)
    this.ctx.builder.buildCall(initFn, argValues, '')
    // Return the pointer to the actual object
    return objPtr
  }

  private ifCounter: number = 1

  compileIf(node: AST.If, blockCtx: BlockContext) {
    var truthyVal = compileTruthyTest(this, blockCtx, node.cond),
        blockNum  = (this.ifCounter++),
        // Get the parent function of the block
        parentFn  = blockCtx.fn.ptr
    // Set up all the blocks we'll be jumping between
    var thenBlock   = blockCtx.fn.appendBasicBlock('then'+blockNum),
        contBlock   = null,
        elseBlock   = null,
        elseIfConds = null,
        elseIfThens = null
    // If we're not the last statement then it's okay to set up a continuation
    // block for subsequent statements to go into
    if (!node.isLastStatement) {
      contBlock = blockCtx.fn.appendBasicBlock('cont'+blockNum)
    }
    // If we have an else condition then set up a block for it
    if (node.elseBlock) {
      elseBlock = blockCtx.fn.appendBasicBlock('else'+blockNum)
    }
    // Build up entries for each of the else blocks
    if (node.elseIfs.length > 0) {
      // Set up the arrays for the condition blocks and then-blocks
      var length  = node.elseIfs.length
      elseIfConds = new Array(length)
      elseIfThens = new Array(length)
      for (var i = 0; i < length; i++) {
        elseIfConds[i] = blockCtx.fn.appendBasicBlock('else'+blockNum+'_if'+i)
        elseIfThens[i] = blockCtx.fn.appendBasicBlock('else'+blockNum+'_then'+i)
      }
    }
    // If the else block is present then we'll jump to that if the else-ifs all
    // fail; otherwise we'll just go to the continuation block.
    var postElseIfsBlock = (elseBlock ? elseBlock : contBlock)
    if (!postElseIfsBlock) {
      throw new ICE('No block to jump to following else-ifs')
    }

    // We also need to figure out which block to jump to if the first
    // if-condition fails
    var afterFirstCond = null
    if (elseIfConds) {
      afterFirstCond = elseIfConds[0]
    } else if (elseBlock) {
      afterFirstCond = thenBlock
    } else if (contBlock) {
      afterFirstCond = contBlock
    } else {
      throw new ICE('No block to jump to following if condition')
    }

    // Build the branch, and then go build the blocks
    this.ctx.builder.buildCondBr(truthyVal, thenBlock, afterFirstCond)
    // Build the then-block
    this.compileConditionBlock(parentFn, node.block, thenBlock, contBlock)

    // Compile all the else-ifs
    for (var i = 0; i < node.elseIfs.length; i++) {
      var nextCond = elseIfConds[i + 1],
          cond     = elseIfConds[i],
          then     = elseIfThens[i],
          elseIf   = node.elseIfs[i]
      // If the next condition is null then we know we're at the end and will
      // just jump to the else block.
      nextCond = (nextCond ? nextCond : postElseIfsBlock)
      // Compile down the condition
      this.ctx.builder.positionAtEnd(cond)
      truthyVal = compileTruthyTest(this.ctx, blockCtx, elseIf.cond)
      this.ctx.builder.buildCondBr(truthyVal, then, nextCond)
      // Then compile down the `then`
      this.compileConditionBlock(parentFn, elseIf.block, then, nextCond)
    }

    // Build the else-block if present
    if (node.elseBlock) {
      this.compileConditionBlock(parentFn, node.elseBlock, elseBlock, contBlock)
    }

    // Position the builder at the end of the continuation block
    this.ctx.builder.positionAtEnd(contBlock)
  }
  compileConditionBlock(parentFn: Buffer, blockNode: AST.Block, blockPtr, contBlockPtr) {
    this.ctx.builder.positionAtEnd(blockPtr)
    this.compileBlock(blockNode, parentFn)
    var lastInstrTerm = isLastInstructionTerminator(blockPtr)
    if (!lastInstrTerm && contBlockPtr !== null) {
      this.ctx.builder.buildBr(contBlockPtr)
    }
  }

  compileBinary(binary: AST.Binary, blockCtx: BlockContext, exprCtx: ExprContext) {
    var lexpr = binary.lexpr,
        rexpr = binary.rexpr
    // Check (and unbox) the types
    var lexprType = lexpr.type,
        rexprType = rexpr.type
    lexprType = unboxInstanceType(lexprType)
    rexprType = unboxInstanceType(rexprType)
    // Find the binary-op NativeFunction
    var builder = BinaryOps.getBuilder(binary.op, lexprType, rexprType)
    assertInstanceOf(builder, Function)
    // Compile the two sides down to a value that we can use
    var lexprValue = this.compileExpression(lexpr, blockCtx),
        rexprValue = this.compileExpression(rexpr, blockCtx)
    // Call the builder function that we got from BinaryOps
    var retValue = builder(this.ctx, lexprValue, rexprValue),
        retType  = binary.type
    tryUpdatingExpressionContext(exprCtx, retType, retValue)
    return retValue
  }

  compileExport(node: AST.Export, blockCtx: BlockContext) {
    var self = this,
        path = [this.ctx.targetModule.getNativeName()],
        name = node.name,
        type = node.type
    // Check that we have a module to compile to
    if (!this.ctx.targetModule) {
      throw new ICE('Missing target module')
    }

    function setupGlobal (name, exportName) {
      var value = blockCtx.slots.buildGet(self.ctx, name),
          type  = TypeOf(value),
          global = LLVM.Library.LLVMAddGlobal(self.ctx.module.ptr, type, exportName)
      // Set the linkage and initializer
      LLVM.Library.LLVMSetLinkage(global, LLVM.Library.LLVMExternalLinkage)
      var initialNull = LLVM.Library.LLVMConstPointerNull(type)
      LLVM.Library.LLVMSetInitializer(global, initialNull)
      // Store the value in the global
      self.ctx.builder.buildStore(value, global, '')
      return global
    }
    var exportableTypes = [types.Function, types.String]
    if (exportableTypes.indexOf(type.constructor) === -1) {
      throw new ICE('Cannot export something of type: '+type.inspect())
    }
    var exportName = path.concat(type.getNativePrefix()+name).join('_'),
        global     = setupGlobal(name, exportName)
  }

  compileReturn(node: AST.Return, blockCtx: BlockContext) {
    if (!node.expr) {
      this.ctx.builder.buildRetVoid()
      } else {
        // Compile to a value and return that
      var value = this.compileExpression(node.expr, blockCtx)
      this.ctx.builder.buildRet(value)
    }
  }

}// LLVMCompiler

// Look up the type and Slots for a given name; begins search from the passed
// block-context. Returns a 2-tuple of Slots and Type.
function getTypeAndSlotsForName (ctx: Context, blockCtx: BlockContext, name: string, foundCb?) {
  // Keep tracking of the scope of the beginning of the chain
  var outermostScope = null
  var type = blockCtx.block.scope.get(name, function (scope, _type) {
    outermostScope = scope
  })

  // Finally look up the Slots for the outermost scope that the name belongs to
  var slots = ctx.slotsMap[outermostScope.id]
  if (!slots) {
    throw new Error("Couldn't find slots for scope #"+outermostScope.id)
  }
  return [slots, type]
}

function tryUpdatingExpressionContext (exprCtx: ExprContext, type: types.Type, value: Buffer) {
  if (!exprCtx) { return }
  exprCtx.type  = type
  exprCtx.value = value
}


function buildPointerCastIfNecessary (ctx: Context, value: Buffer, desiredType: Buffer) {
  var valueType = TypeOf(value)
  // Compare the type strings
  var vts = PrintTypeToString(valueType),
      dts = PrintTypeToString(desiredType),
      typesAreEqual = (vts === dts)
  // If the types aren't the same then we'll recast
  if (!typesAreEqual) {
    if (ctx.noCastValuePointers) {
      throw new ICE('Value type different than the one desired and no-cast-value-pointers is true: '+vts+' -> '+dts)
    }
    // Re-cast the value
    return ctx.builder.buildPointerCast(value, desiredType, '')
  }
  // Didn't need to re-cast the value
  return value
}

function bitcodeFileForSourceFile (path: string): string {
  var outFile = path.replace(/\.hb$/i, '.bc')
  if (outFile === path) {
    throw new ICE('Couldn\'t compute path for module output file')
  }
  return outFile
}

// Recursively predefine types that need definition before compilation can
// begin properly. Right now this only deals with on anonymous functions.
// TODO: Make this properly recurse.
function predefineTypes (ctx: Context, block: AST.Block) {
  block.statements.forEach(function (stmt) {
    switch (stmt.constructor) {
      case AST.Assignment:
        var assg = <AST.Assignment>stmt
        if (assg.type !== 'var' && assg.type !== 'let') {
          return
        }
        if (assg.rvalue instanceof AST.Function) {
          var rvalue             = <AST.Function>rvalue,
              rvalueInstanceType = rvalue.type,
              rvalueType         = unboxInstanceType(rvalueInstanceType)
          // If the native function hasn't been typed
          if (!rvalueType.hasNativeFunction()) {
            var fn = getAnonymousNativeFunction(ctx, rvalue)
            fn.computeType()
          }
        }
        break
    }
  })
}

var nativeFunctionCounter = 1

function getAnonymousNativeFunction (ctx: Context, node: AST.Function): NativeFunction {
  if (node.name) {
    throw new ICE('Trying to set up named function as anonymous native function')
  }
  var instance = node.type,
      type     = unboxInstanceType(instance),
      fn       = null
  // Check if the native function has already been set up
  if (type.hasNativeFunction()) {
    fn = type.getNativeFunction()
  } else {
    var prefix = (ctx.targetModule ? ctx.targetModule.getNativeName()+'_' : ''),
        name   = prefix+'A'+(nativeFunctionCounter++),
        args   = type.args,
        ret    = type.ret
    // Setup the native function
    fn = new NativeFunction(name, args, ret)
    // Save the native function on the type
    type.setNativeFunction(fn)
  }
  return fn
}

function sanityCheckInitializers (klass) {
  // Sanity-check to make sure the initializers on the type and the
  // initializers on the node match up
  var typeInitializersTypes = klass.type.initializers,
      nodeInitializersTypes = klass.initializers.map(function (i) { return i.type })
  if (typeInitializersTypes.length !== nodeInitializersTypes.length) {
    throw new ICE('Type initializers don\'t match AST node initializers')
  }
  for (var i = 0; i < typeInitializersTypes.length; i++) {
    var ti = typeInitializersTypes[i],
        ni = nodeInitializersTypes[i]
    if (ti !== ni) {
      throw new ICE('Type initializer '+i+' doesn\'t match AST node initializer')
    }
  }
}

