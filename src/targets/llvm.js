
var _            = require('lodash'),
    AST          = require('../ast'),
    types        = require('../types'),
    scope        = require('../typesystem/scope'),
    Scope        = scope.Scope,
    ClosingScope = scope.ClosingScope,
    LLVM         = require('../../../llvm2'),
    Builtins     = require('./llvm/builtins'),
    slots        = require('./llvm/slots'),
    Errors       = require('../errors'),
    ICE          = Errors.InternalCompilerError,
    // Target information and setup
    target       = require('./llvm/target'),
    util         = require('./llvm/util'),
    BinaryOps    = require('./llvm/binary-ops')

var isLastInstructionTerminator = util.isLastInstructionTerminator,
    compileTruthyTest           = util.compileTruthyTest,
    assertInstanceOf            = util.assertInstanceOf

var NativeFunction    = require('./llvm/native-function'),
    NativeObject      = require('./llvm/native-object'),
    nativeTypeForType = require('./llvm/native-types').nativeTypeForType

// Unbox the slots module
var Slots         = slots.Slots,
    ConstantSlots = slots.ConstantSlots,
    GlobalSlots   = slots.GlobalSlots

var Int8Type    = LLVM.Types.Int8Type,
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
    PointerTypeKind   = GetTypeKind(Int8PtrType)

/*
  Compilation tags:
    Interfaces: (non-value)
      M = module
      T = type (class)
      m = method
    Values:
      A = anonymous function value
      F = named function value
      G = global value (constant)

  Example layout of some compilation tags:
    For the `concat` function in `std.core.types.string`:
      Mstd_mcore_mtypes_mstring_fconcat
    For the `log` method of an instance of the `BuiltinConsole` class type:
      TBuiltinConsole_mlog
*/

function Context () {
  // Externally linked functions
  this.extern = {}
  // Globals
  this.globals = {}
}
Context.prototype.addGlobal = function (name, type) {
  assertInstanceOf(type, Buffer)
  var global = LLVM.Library.LLVMAddGlobal(this.module.ptr, type, name)
  this.globals[name] = global
  return global
}
Context.prototype.getGlobal = function (name) {
  if (!this.hasGlobal(name)) { throw new Error('Global definition not found: '+name) }
  return this.globals[name]
}
Context.prototype.hasGlobal = function (name) {
  return (this.globals[name] ? true : false)
}
Context.prototype.buildGlobalLoad = function (name) {
  var global = this.globals[name],
      // ptr = this.builder.buildGEP(global, [Int32Zero], name),
      val    = this.builder.buildLoad(global, name)
  return val
}


function BlockContext (ctx, parentFn, block, slots) {
  this.ctx   = ctx
  this.fn    = parentFn
  this.block = block
  this.slots = slots
}

function BasicLogger () {
  this.info = console.log
}

AST.Node.prototype.compileToValue = function () {
  throw new Error('Compilation to value not yet implemented for: '+this.constructor.name)
}

AST.Root.prototype.emitToFile = function (opts) {
  if (opts.module) {
    assertInstanceOf(opts.module, types.Module)
  }
  var ctx = new Context()
  ctx.isMain       = (opts.module ? true : false)
  ctx.targetModule = (opts.module ? opts.module : null)
  ctx.module       = new LLVM.Module(ctx.targetModule ? ctx.targetModule.getNativeName() : 'main')
  ctx.builder      = new LLVM.Builder()
  ctx.pass_manager = new LLVM.FunctionPassManager(ctx.module)
  ctx.funcs        = {}
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
  var outFile = bitcodeFileForSourceFile(this.file.path)
  ctx.outputs.push(outFile)

  var mainType = new LLVM.FunctionType(VoidType, [], false),
      mainFunc = null
  if (ctx.isMain) {
    var initName = ctx.targetModule.getNativeName()+'_init'
    mainFunc = ctx.module.addFunction(initName, mainType)
  } else {
    // Set up the main function
    mainFunc = ctx.module.addFunction('main', mainType)
    // Also setup information about our compilation target
    target.initializeTarget(ctx)
  }
  var mainEntry = mainFunc.appendBasicBlock('entry')

  // Add the root scope to the slots map
  var rootScope = this.scope.parent
  if (!rootScope.isRoot) { throw new Error("Couldn't find root scope") }
  ctx.slotsMap[rootScope.id] = ctx.globalSlots

  // Add the builtins
  Builtins.compile(ctx, mainEntry, this)
  BinaryOps.initialize(ctx)

  // Setup the entry into the function
  ctx.builder.positionAtEnd(mainEntry)
  if (!opts.module) {
    // Initialize the GC
    ctx.builder.buildCall(ctx.extern.GC_init, [], '')
  }

  // Compile ourselves in the main entry function
  this.buildMain(ctx, mainFunc, mainEntry)

  // ctx.module.dump()

  // Verify the module to make sure that nothing's amiss before we hand it
  // of to the bitcode compiler
  var errPtr = LLVM.Library.newCStringPtr()
  LLVM.Library.LLVMVerifyModule(ctx.module.ptr, LLVM.Library.LLVMAbortProcessAction, errPtr)

  ctx.module.writeBitcodeToFile(outFile)
  ctx.logger.info('Wrote bitcode to file: '+outFile)
}

AST.Root.prototype.buildMain = function (ctx, mainFunc, mainEntry) {
  // Compile ourselves now that setup is done
  compileBlock(ctx, this, mainFunc)

  // var str = ctx.builder.buildGlobalStringPtr("Hello world!\n", 'greeting')
  // ctx.builder.buildCall(ctx.funcs.puts, [str], '')
  ctx.builder.buildRetVoid()
}

function compileBlock (ctx, block, parentFn, preStatementsCb) {
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
    var stmt = statements[i]
    stmt.compile(ctx, blockCtx)
  }//for
}//compileBlock

// Look up the type and Slots for a given name; begins search from the passed
// block-context. Returns a 2-tuple of Slots and Type.
function getTypeAndSlotsForName (ctx, blockCtx, name, foundCb) {
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

AST.Assignment.prototype.compile = function (ctx, blockCtx) {
  if (this.type === 'var' || this.type === 'let') {
    return this.compileNamed(ctx, blockCtx)
  }
  if (this.type === 'path') {
    return this.compilePath(ctx, blockCtx)
  }
  throw new Error('Cannot compile assignment type: '+this.type)
}

AST.Assignment.prototype.compileToStorable = function (ctx, blockCtx, lvalue) {
  var name      = lvalue.name,
      path      = lvalue.path,
      pair      = getTypeAndSlotsForName(ctx, blockCtx, name),
      slots     = pair[0],
      itemType  = pair[1],
      itemValue = null
  // If there's no path then we can just return a storable for the local
  if (path.length === 0) {
    return slots.getStorable(name)
  } else {
    itemValue = slots.buildGet(ctx, name)
    // Get the native type and cast the pointer to it
    var nativeType = nativeTypeForType(itemType)
    itemValue = ctx.builder.buildPointerCast(itemValue, nativeType, name)
  }
  for (var i = 0; i < path.length; i++) {
    var isLast = false
    if (i === (path.length - 1)) {
      isLast = true
    }
    var item = path[i]
    switch (item.constructor) {
      case AST.Identifier:
        // Unbox and ensure we've got an Object we can work with
        assertInstanceOf(itemType, types.Instance)
        var objType = itemType.type
        assertInstanceOf(objType, types.Object)
        var nativeObj = objType.getNativeObject(),
            propName  = item.name,
            propType  = item.type,
            propPtr   = nativeObj.buildStructGEPForProperty(ctx, itemValue, propName)
        // Return storable pointer if we're the last item in the chain
        if (isLast) { return propPtr }
        // Otherwise build a dereference
        itemType  = propType
        itemValue = ctx.builder.buildLoad(propPtr, propName)
        break
   /* case AST.Property:
        assertInstanceOf(itemType, types.Instance)
        // Make sure it's pointing to an Object
        var objectType = itemType.type
        assertInstanceOf(objectType, types.Object)
        var nativeObject = objectType.getNativeObject()
        // Now get the property
        var propName = item.name,
            propPtr  = nativeObject.buildStructGEPForProperty(ctx, itemValue, propName)
        // If it's the end of the path then we return the pointer since it's storable
        if (isLast) {
          return propPtr
        }
        itemType = new types.Instance(itemType.getTypeOfPropertyName(propName))
        itemValue = ctx.builder.buildLoad(propPtr, propName)
        break */
      default:
        throw new ICE('Cannot handle path item type: '+item.constructor.name)
    }
  }
}

AST.Assignment.prototype.compilePath = function (ctx, blockCtx) {
  // Lookup the lvalue into a receiver that we can set
  var recvPtr = this.compileToStorable(ctx, blockCtx, this.lvalue)
  // Get the rvalue as a value to be stored in the lvalue's receiving pointer
  var rvalue = this.rvalue.compileToValue(ctx, blockCtx)
  // Build the actual store into that pointer
  ctx.builder.buildStore(rvalue, recvPtr)
}

AST.Assignment.prototype.compileNamed = function (ctx, blockCtx) {
  // Get a value pointer from the rvalue
  var rvalue = this.rvalue.compileToValue(ctx, blockCtx)
  assertInstanceOf(rvalue, Buffer, 'Received non-Buffer from Node#compilerToValue')
  // Get the slot pointer
  blockCtx.slots.buildSet(ctx, this.lvalue.name, rvalue)
}

AST.Property.prototype.compile = function (ctx, blockCtx, exprCtx) {
  this.compileToValue(ctx, blockCtx, exprCtx)
}

AST.Property.prototype.compileToValue = function (ctx, blockCtx, exprCtx) {
  var base      = this.base,
      parent    = this.parent,
      property  = this.property,
      type      = null,
      value     = null

  if (this.base.type instanceof types.Module) {
    return this.compileAsModuleMember(ctx, blockCtx, exprCtx)
  }
  if (parent === null) {
    var retCtx = {}
    this.base.compile(ctx, blockCtx, retCtx)
    type  = retCtx.type
    value = retCtx.value

  } else {
    type  = exprCtx.type
    value = exprCtx.value
  }
  assertInstanceOf(value, Buffer)
  var ret = this.property.compileToValue(ctx, blockCtx, {type: type, value: value})
  if (!ret) {
    throw new ICE("Encountered a null return value")
  }
  return ret
}
AST.Property.prototype.compileAsModuleMember = function (ctx, blockCtx, exprCtx) {
  var parent = null,
      path   = []
  if (parent === null) {
    var retCtx = {}
    this.base.compileAsModuleMember(ctx, blockCtx, retCtx)
    assertInstanceOf(retCtx.path, Array)
    path = retCtx.path
  } else {
    path = exprCtx.path
  }
  return this.property.compileAsModuleMember(ctx, blockCtx, {path: path})
}

AST.Call.prototype.compileInstanceMethodCall = function (ctx, blockCtx, exprCtx) {
  var recvValue    = exprCtx.value,
      recvInstance = exprCtx.type,
      instance     = this.base.type,
      method       = instance.type
  assertInstanceOf(recvValue, Buffer)
  assertInstanceOf(recvInstance.type, types.Object)
  assertInstanceOf(method, types.Function)
  // Get the object we're going to use and compile the argument values
  var recvObj   = recvInstance.type.getNativeObject(),
      argValues = this.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
  // Get the function to call
  var methodFn = method.getNativeFunction()
  // And add the receiver object pointer and call the function
  argValues.unshift(recvValue)
  var retValue  = ctx.builder.buildCall(methodFn.getPtr(), argValues, '')
  exprCtx.type  = this.type
  exprCtx.value = retValue
  return retValue
}

AST.Call.prototype.compileIntrinsicInstanceMethodCall = function (ctx, blockCtx, exprCtx) {
  var receiverInstance = this.parent.type,
      receiverType     = receiverInstance.type
  
  // Look up the shim method. The shim will get transformed into a proper call
  var shimMethodInstance = this.base.type,
      shimMethod         = shimMethodInstance.type
  // Look up the ultimate method via the shim
  if (shimMethod.shimFor == null) {
    throw new ICE('Missing ultimate method for shim: '+this.base.name)
  }
  var method   = shimMethod.shimFor,
      nativeFn = method.getNativeFunction(),
      argValues = this.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
  // Add the receiver to the front of the arguments
  var receiverValue = exprCtx.value
  argValues.unshift(receiverValue)
  // Build the call
  var retValue = ctx.builder.buildCall(nativeFn.getPtr(), argValues, '')
  tryUpdatingExpressionContext(exprCtx, this.type, retValue)
  return retValue
}

function unboxInstanceType (instance, expectedType) {
  assertInstanceOf(instance, types.Instance)
  var type = instance.type
  if (expectedType !== undefined) {
    assertInstanceOf(type, expectedType)
  }
  return type
}

AST.Call.prototype.compile = function (ctx, blockCtx, exprCtx) {
  this.compileToValue(ctx, blockCtx, exprCtx)
}
AST.Call.prototype.compileToValue = function (ctx, blockCtx, exprCtx) {
  var parent = this.parent,
      type   = null,
      value  = null

  // First we need to check for it being an instance method
  while (parent !== null) {
    var methodType = this.base.type
    if (!(methodType instanceof types.Instance)) { break }
    // Unbox the instance and check if it's an instace method
    methodType = methodType.type
    if (!methodType.isInstanceMethod) { break }
    var receiverInstance = parent.type,
        receiverType     = receiverInstance.type
    if (receiverType.intrinsic === true) {
      // Need to do a little bit of special handling for intrinsics
      return this.compileIntrinsicInstanceMethodCall(ctx, blockCtx, exprCtx)
    } else {
      // If it was an instance method then we'll go directly to that
      // compilation path
      return this.compileInstanceMethodCall(ctx, blockCtx, exprCtx)
    }
  }

  if (parent === null) {
    // Make sure we have a real Identifier to start off with
    assertInstanceOf(this.base, AST.Identifier)
    var retCtx = {}
    this.base.compile(ctx, blockCtx, retCtx)
    type  = retCtx.type
    value = retCtx.value

  } else {
    baseType  = exprCtx.type
    baseValue = exprCtx.value
    var retCtx = {type: baseType, value: baseValue}
    this.base.compile(ctx, blockCtx, retCtx)
    type  = retCtx.type
    value = retCtx.value
  }
  var funcType  = unboxInstanceType(type, types.Function),
      argValues = this.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
  // Build return call and update the context to return
  var retValue = ctx.builder.buildCall(value, argValues, '')
  tryUpdatingExpressionContext(exprCtx, this.type, retValue)
  return retValue
}
AST.Call.prototype.compileAsModuleMember = function (ctx, blockCtx, exprCtx) {
  var parent = this.parent
  if (parent === null) {
    throw new ICE('Not implemented yet')
  }
  assertInstanceOf(exprCtx.path, Array)
  var retCtx = {path: _.clone(exprCtx.path)}
  this.base.compileAsModuleMember(ctx, blockCtx, retCtx)
  var path = retCtx.path
  assertInstanceOf(path, Array)
  // Join the path and look up the function type from the box on our base
  var name = path.join('_')
  assertInstanceOf(this.base.type, types.Instance)
  assertInstanceOf(this.base.type.type, types.Function)
  var type = this.base.type.type
  // Look up the actual function global via the name path
  var global = null
  if (ctx.hasGlobal(name)) {
    global = ctx.getGlobal(name)
  } else {
    var args    = type.args.map(nativeTypeForType),
        ret     = nativeTypeForType(type.ret),
        fnTy    = new LLVM.FunctionType(ret, args, false),
        fnPtrTy = LLVM.Types.pointerType(fnTy.ptr)
    // Add the function as a global
    global = ctx.addGlobal(name, fnPtrTy)
  }
  // Look up the function and call it with the arguments
  var fn = ctx.buildGlobalLoad(name),
      args = this.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
  return ctx.builder.buildCall(fn, args, '')
}

function tryUpdatingExpressionContext (exprCtx, type, value) {
  if (!exprCtx) { return }
  exprCtx.type  = type
  exprCtx.value = value
}

AST.Identifier.prototype.compile = function (ctx, blockCtx, exprCtx) {
  this.compileToValue(ctx, blockCtx, exprCtx)
}
AST.Identifier.prototype.compileToValue = function (ctx, blockCtx, exprCtx) {
  var parent   = this.parent,
      newType  = null,
      newValue = null

  // First check if we're working on a module
  if (this.type instanceof types.Module) {
    return this.compileAsModuleMember(ctx, blockCtx, exprCtx)
  }
  if (parent === null) {
    // Look up ourselves rather than building off a parent
    var pair = getTypeAndSlotsForName(ctx, blockCtx, this.name)
    newValue = pair[0].buildGet(ctx, this.name)
    newType  = pair[1]
  } else {
    var type  = exprCtx.type,
        value = exprCtx.value
    // Check the types and then build the GEP
    assertInstanceOf(type, types.Instance)
    var objType   = type.type,
        nativeObj = objType.getNativeObject()

    // Build the pointer and load it into a value
    var ptr  = nativeObj.buildStructGEPForProperty(ctx, value, this.name)
    newType  = this.type
    newValue = ctx.builder.buildLoad(ptr, this.name)
  }
  tryUpdatingExpressionContext(exprCtx, newType, newValue)
  return newValue
}
AST.Identifier.prototype.compileAsModuleMember = function (ctx, blockCtx, exprCtx) {
  var path = (exprCtx.path ? exprCtx.path : []),
      type = this.type,
      name = null
  switch (type.constructor) {
    case types.Module:
      name = type.getNativeName()
      break
    case types.Instance:
      var unboxed = type.type
      assertInstanceOf(unboxed, types.Function, "Currently can only target module functions")
      name = 'F'+this.name
      break
    default:
      throw new ICE("Don't know how to handle module member of type: "+type.constructor.name)
  }
  path.push(name)
  // Update the expression context and return
  exprCtx.path = path
  return null
}


AST.Literal.prototype.compileToValue = function (ctx, blockCtx) {
  var instance = this.type
  switch (instance.type.constructor) {
    case types.String:
      var stringValue = this.value
      // Build a constant with our string value and return that
      return ctx.builder.buildGlobalStringPtr(stringValue, '')
    case types.Integer:
      return LLVM.Library.LLVMConstInt(Int64Type, this.value, '')
    default:
      var name = instance.type.constructor.name
      throw new ICE('Cannot handle instance type: '+name)
  }
}

/*
// FIXME: This can probably be completely removed now that ClosingScope is
//        integrated into the type-system and preventing closure overreach
//        at that earlier stage of compilation.
function checkScopePathForClosureOverreach (scopePath) {
  // Figure out the first (outermost) and last (innermost) indices of the closures
  var firstClosingIdx = -1,
      lastClosingIdx  = -1
  for (var i = 0; i < scopePath.length; i++) {
    var s = scopePath[i]
    if (s.isClosing) {
      // If the first closing index has been set then we know we're the first
      // one found
      if (firstClosingIdx === -1) { firstClosingIdx = i }
      // Push back the last closing index if we found another one
      if (i > lastClosingIdx) { lastClosingIdx = i }
    }
  }
  // If the closing indices don't match then we know we've busted through closures
  if (firstClosingIdx !== lastClosingIdx) {
    throw new Error('Too many closures')
  }
  // If the first closing index isn't the outermost one then we've busted
  // beyond a closure
  if (firstClosingIdx > 0) {
    throw new Error('Accessing variable beyond closure')
  }
}

function computeScopePath (blockCtx, outermostScope) {
  var scope = blockCtx.block.scope,
      path  = []
  while (true) {
    path.unshift(scope)
    // Stop building the scope path if we encountered the outermost scope
    if (scope === outermostScope) {
      break
    }
    // Otherwise go to the parent scope
    scope = scope.parent
    if (!scope) {
      throw new Error('Ran out of scopes!')
    }
  }
  return path
}
*/

AST.Chain.prototype.compileInstanceMethodCall = function (ctx, blockCtx, receiver, receiverType, instanceMethod, callNode) {
  assertInstanceOf(receiver, Buffer)
  assertInstanceOf(instanceMethod, types.Function)
  assertInstanceOf(callNode, AST.Call)
  // Compile all the arguments to values
  var args = callNode.args.map(function (arg) {
    return arg.compileToValue(ctx, blockCtx)
  })
  // Unbox the receiver instance
  receiverType = receiverType.type
  // Then get the native type as a pointer
  var nativeReceiver  = receiverType.getNativeObject(),
      receiverPtrType = LLVM.Types.pointerType(nativeReceiver.structType)
  // Cast the plain pointer to the correct type
  receiver = ctx.builder.buildPointerCast(receiver, receiverPtrType, '')
  args.unshift(receiver)
  var nativeFunction = instanceMethod.getNativeFunction()
  // Build the actual call to the instance method
  var returnValue = ctx.builder.buildCall(nativeFunction.getPtr(), args, ''),
      returnType  = new types.Instance(instanceMethod.ret)
  return [returnType, returnValue]
}

function buildPointerCastIfNecessary (ctx, value, desiredType) {
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

function bitcodeFileForSourceFile (path) {
  var outFile = path.replace(/\.hb$/i, '.bc')
  if (outFile === path) {
    throw new ICE('Couldn\'t compute path for module output file')
  }
  return outFile
}

AST.Import.prototype.compile = function (ctx, blockCtx) {
  var moduleRoot = this.file.tree,
      nativeName = this.file.module.getNativeName(),
      outFile    = bitcodeFileForSourceFile(this.file.path)
  moduleRoot.emitToFile({
    module: this.file.module,
    logger: ctx.logger,
    outputs: ctx.outputs
  })
  // Find the external module initializer
  var initName = nativeName+'_init',
      initFn   = NativeFunction.addExternalFunction(ctx, initName, VoidType, [])
  // And then call it so that the module gets initialized at the correct time
  ctx.builder.buildCall(initFn, [], '')
  
  var basePath = this.file.module.getNativeName()
  if (this.using) {
    var slots = blockCtx.slots
    // Load items from the module into the local scope
    for (var i = 0; i < this.using.length; i++) {
      var use      = this.using[i],
          instance = this.file.module.getTypeOfProperty(use),
          type     = unboxInstanceType(instance),
          path     = basePath+'_'+type.getNativePrefix()+use
      // Sanity-check to make sure this is the first time this global has
      // been set up
      if (ctx.hasGlobal(path)) {
        throw new ICE('Global already exists: '+path)
      }
      var global   = ctx.addGlobal(path, nativeTypeForType(instance)),
          value    = ctx.buildGlobalLoad(path)
      // Store the value in the local slot
      slots.buildSet(ctx, use, value)
    }
  }
}

AST.Export.prototype.compile = function (ctx, blockCtx) {
  if (!ctx.targetModule) {
    throw new ICE('Missing target module')
  }
  var path = [ctx.targetModule.getNativeName()],
      name = this.name,
      type = this.type
  switch (type.constructor) {
    case types.Function:
      path.push('F'+name)
      var exportName = path.join('_')
      // Create a global value with that name
      var value    = blockCtx.slots.buildGet(ctx, this.name),
          nativeFn = type.getNativeFunction(),
          nativeTy = TypeOf(nativeFn.getPtr()),
          global   = LLVM.Library.LLVMAddGlobal(ctx.module.ptr, nativeTy, exportName)
      // Set it to externally link and initialize it to null
      LLVM.Library.LLVMSetLinkage(global, LLVM.Library.LLVMExternalLinkage)
      var initialNull = LLVM.Library.LLVMConstPointerNull(nativeTy)
      LLVM.Library.LLVMSetInitializer(global, initialNull)
      // Set the native function pointer in the global so it will be exposed
      ctx.builder.buildStore(value, global, '')
      break

    default:
      throw new ICE('Cannot export something of type: '+type.inspect())
  }
}

AST.Chain.prototype.compileModulePathToValue = function (ctx, blockCtx) {
  // TODO: Check for modules imported into this context
  var type = this.headType,
      path = ['M'+type.name]
  for (var i = 0; i < this.tail.length; i++) {
    var item = this.tail[i]
    switch (item.constructor) {
    case AST.Property:
      var propertyName = item.name,
          propertyType = type.getTypeOfProperty(propertyName)
      if (propertyType instanceof types.Function) {
        // TODO: Handle calls
        path.push('F'+propertyName)
      }
      type = propertyType
      break
    case AST.Call:
      var isLastItem = (i === (this.tail.length - 1))
      if (!isLastItem) {
        throw new ICE('Can only handle calls as last item of module path')
      }
      assertInstanceOf(type, types.Function)
      var global = null,
          name   = path.join('_')
      // Save the module global so that we don't recreate it every time
      if (ctx.hasGlobal(name)) {
        global = ctx.getGlobal(name)
      } else {
        // Build the path to the external function and assemble its type
        var args = type.args.map(nativeTypeForType),
            ret  = nativeTypeForType(type.ret)
        // Get the external function
        var fnType = new LLVM.FunctionType(ret, args, false),
            fnPtrType = LLVM.Types.pointerType(fnType.ptr)
        // Add the global to the module
        global = ctx.addGlobal(name, fnPtrType)
      }
      var fn = ctx.buildGlobalLoad(name)
      // Compile all the args into values
      var argValues = item.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
      return ctx.builder.buildCall(fn, argValues, '')
    }
  }
  throw new ICE('Unreachable point in compiling a module path')
}

AST.Chain.prototype.compileToValue = function (ctx, blockCtx) {
  var headType = this.headType
  if (headType instanceof types.Module) {
    return this.compileModulePathToValue(ctx, blockCtx)
  }
  var slots, itemValue, itemType;
  // Look up the Slots for the scope that the head of our chain belongs to
  var pair  = getTypeAndSlotsForName(ctx, blockCtx, this.name),
  slots     = pair[0]
  itemType  = pair[1]
  itemValue = slots.buildGet(ctx, this.name)
  // Handle the tail
  for (var i = 0; i < this.tail.length; i++) {
    var item = this.tail[i]
    switch (item.constructor) {
    case AST.Property:
      assertInstanceOf(itemType, types.Instance)
      // Look up the type of the instance and get the native struct for it
      var type         = itemType.type,
          nativeObject = type.getNativeObject(),
          propertyName = item.name,
          propertyType = type.getTypeOfProperty(propertyName)
      // Check if it's an instance method
      if ((propertyType instanceof types.Function) && propertyType.isInstanceMethod) {
        var nextItem = this.tail[i + 1]
        if (!(nextItem instanceof AST.Call)) {
          throw new ICE('Instance method is missing its Call node')
        }
        var pair = this.compileInstanceMethodCall(ctx, blockCtx, itemValue, itemType, propertyType, nextItem)
        itemType  = pair[0]
        itemValue = pair[1]
        // Advance the index beyond the subsequent AST.Call
        i += 1;
        // Skip over the rest of this block
        break
      }
      // Cast the item value to the structure type so we can use it properly
      var structPointerType = LLVM.Types.pointerType(nativeObject.structType)
      // Recast if necessary
      itemValue = buildPointerCastIfNecessary(ctx, itemValue, structPointerType)
      // Get the pointer to the property value
      var ptr = nativeObject.buildStructGEPForProperty(ctx, itemValue, propertyName)
      // Update the type and value
      itemType  = new types.Instance(type.getTypeOfProperty(propertyName))
      itemValue = ctx.builder.buildLoad(ptr, propertyName)
      break
    case AST.Call:
      assertInstanceOf(itemType, types.Instance)
      // Unbox the instance and check its type
      var type = itemType.type
      assertInstanceOf(type, types.Function)
      // Compile all the args into values
      var argValues = item.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
      // Check that we have a function pointer
      var itemValueType     = TypeOf(itemValue),
          itemValueTypeKind = GetTypeKind(itemValueType)
      if (itemValueTypeKind !== PointerTypeKind) {
        throw new Error('Encountered non-pointer type for function')
      }
      // Build a pointer type for the function
      var funcType    = type.nativeFunction.type.ptr,
          funcPtrType = LLVM.Types.pointerType(funcType)
      itemValue = buildPointerCastIfNecessary(ctx, itemValue, funcPtrType)
      // Update the type to what it's going to return and then build the
      // actual function call
      itemType  = new types.Instance(type.ret)
      itemValue = ctx.builder.buildCall(itemValue, argValues, '')
      break
    default:
      throw new Error("Cannot handle item type: "+item.constructor.name)
    }
  }
  return itemValue
}

AST.Chain.prototype.compile = function (ctx, blockCtx) {
  this.compileToValue(ctx, blockCtx)
}

AST.Return.prototype.compile = function (ctx, blockCtx) {
  if (!this.expr) {
    ctx.builder.buildRetVoid()
    } else {
      // Compile to a value and return that
    var value = this.expr.compileToValue(ctx, blockCtx)
    ctx.builder.buildRet(value)
  }
}

// Recursively predefine types that need definition before compilation can
// begin properly. Right now this only deals with on anonymous functions.
// TODO: Make this properly recurse.
function predefineTypes (ctx, block) {
  block.statements.forEach(function (stmt) {
    switch (stmt.constructor) {
      case AST.Assignment:
        if (stmt.type !== 'var' && stmt.type !== 'let') {
          return
        }
        if (stmt.rvalue instanceof AST.Function) {
          var rvalueInstanceType = stmt.rvalue.type,
              rvalueType         = rvalueInstanceType.type
          // If the native function hasn't been typed
          if (!rvalueType.hasNativeFunction()) {
            var fn = stmt.rvalue.getAnonymousNativeFunction(ctx)
            fn.computeType()
          }
        }
        break
    }
  })
}

function genericCompileFunction (ctx, nativeFn, node) {
  var hasThisArg = (node instanceof AST.Init),
      block      = node.block
  // Predefine to be safe
  predefineTypes(ctx, block)

  nativeFn.defineBody(ctx, function (entry) {
    // Actual LLVM function that we're compiling for
    var fnPtr = nativeFn.fn
    compileBlock(ctx, block, fnPtr, function (blockCtx, slots) {
      var argOffset = 0
      // Setup `this` and the other function args
      if (hasThisArg) {
        argOffset = 1
        // `this` will be the first argument
        var thisValue = GetParam(nativeFn.getPtr(), 0)
        // Store `this` in the slots
        slots.buildSet(ctx, 'this', thisValue)
      }
      // Handle regular arguments
      var args = node.args
      for (var i = 0; i < args.length; i++) {
        var arg      = args[i],
            argName  = arg.name,
            argValue = GetParam(nativeFn.getPtr(), i + argOffset)
        // Store the argument value in the slot
        slots.buildSet(ctx, argName, argValue)
      }
    })

    // If it's returning Void and the last statement isn't a return then
    // go ahead an insert one for safety
    if (nativeFn.ret instanceof types.Void) {
      var lastStatement = block.statements[block.statements.length - 1]
      if (!(lastStatement instanceof AST.Return)) {
        var currentBasicBlock = LLVM.Library.LLVMGetInsertBlock(ctx.builder.ptr),
            hasTerminator     = isLastInstructionTerminator(currentBasicBlock)
        if (!hasTerminator) {
          ctx.builder.buildRetVoid()
        }
      }
    }// nativeFn.ret is types.Void

    /*
    // Check for empty basic blocks and append unreachable to them
    var bb = LLVM.Library.LLVMGetFirstBasicBlock(fnPtr.ptr)
    while (true) {
      var lastInstr = LLVM.Library.LLVMGetLastInstruction(bb)
      // If the last instruction is null then we know it's empty
      if (lastInstr.isNull()) {
        ctx.builder.positionAtEnd(bb)
        ctx.builder.buildUnreachable()
      }
      // Get the next basic block
      bb = LLVM.Library.LLVMGetNextBasicBlock(bb)
      if (bb.isNull()) { break }
    }
    */
  })// nativeFn.defineBody
}

var nativeFunctionCounter = 1

AST.Function.prototype.getAnonymousNativeFunction = function (ctx) {
  if (this.name) {
    throw new ICE('Trying to set up named function as anonymous native function')
  }
  assertInstanceOf(this.type, types.Instance)

  var instance = this.type,
      type     = instance.type,
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

AST.Function.prototype.compile = function(ctx, blockCtx) {
  var instance = this.type,
      type     = unboxInstanceType(instance, types.Function)
  if (type.parentMultiType) {
    throw new ICE('Compilation of multi-functions not yet implemented')
  } else {
    if (typeof this.name !== 'string') {
      throw new ICE('Missing name of Function statement')
    }
    var name = type.getNativePrefix()+this.name
    // Setup the native function
    var fn = new NativeFunction(name, type.args, type.ret)
    type.setNativeFunction(fn)
    // Compile the native function with our block
    genericCompileFunction(ctx, fn, this)
    // Set the linkage of the function to private
    LLVM.Library.LLVMSetLinkage(fn.getPtr(), LLVM.Library.LLVMPrivateLinkage)
    // Add this to the slots
    blockCtx.slots.buildSet(ctx, this.name, fn.getPtr())
  }
}

AST.Function.prototype.compileToValue = function (ctx, blockCtx) {
  var self = this,
      fn   = this.getAnonymousNativeFunction(ctx)
  genericCompileFunction(ctx, fn, this)
  // Get the raw function as a value
  var compiledFn = fn.getPtr()
  return compiledFn
}

AST.Class.prototype.sanityCheckInitializers = function () {
  // Sanity-check to make sure the initializers on the type and the
  // initializers on the node match up
  var typeInitializersTypes = this.type.initializers,
      nodeInitializersTypes = this.initializers.map(function (i) { return i.type })
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

AST.Class.prototype.compile = function (ctx, blockCtx) {
  // Sanity-check the initializers to make sure nothing weird is going to
  // happen when we start compiling stuff around this class
  this.sanityCheckInitializers()

  // Look up the computed type for this Class
  var type = this.type
  // Then build the native object from this type
  var nativeObject = new NativeObject(type)
  type.setNativeObject(nativeObject)
  // Define the native object in the context
  nativeObject.define(ctx)
  // Build the initializers for the class
  this.compileInitializers(ctx, blockCtx, nativeObject)
  this.compileInstanceMethods(ctx, blockCtx, nativeObject)
}
AST.Class.prototype.compileInitializers = function (ctx, blockCtx, nativeObject) {
  var type         = this.type,
      nativeObject = type.getNativeObject(),
      initializers = this.initializers
  // Build and compile a native function for each initializer function
  for (var i = 0; i < initializers.length; i++) {
    var init         = initializers[0],
        initType     = init.type,
        internalName = nativeObject.internalName+'_i'+i
    // Make a copy of the initializer args and prepend an argument for the
    // instance of the type being initialized (ie. `this`)
    var initArgs = _.clone(initType.args)
    initArgs.unshift(new types.Instance(type))
    // Create the native function
    var fn = new NativeFunction(internalName, initArgs, initType.ret)
    genericCompileFunction(ctx, fn, init)

    // Add this native function to the native object's list of initializers
    nativeObject.addInitializer(fn)
  }
}
AST.Class.prototype.compileInstanceMethods = function (ctx, blockCtx, nativeObject) {
  var type = this.type
  // Iterate over our definition and find each instance method
  var statements = this.definition.statements
  for (var i = 0; i < statements.length; i++) {
    var stmt = statements[i]
    // Skip over non-functions
    if (!(stmt instanceof AST.Function)) { continue }
    var instance = stmt.type
    assertInstanceOf(instance, types.Instance)
    // Mark the type as an instance method
    var type = instance.type
    if (type.isInstanceMethod !== true) {
      throw new ICE('Encountered non-instance-method in class definition')
    }
    var internalName = nativeObject.internalName+'_m'+stmt.name,
        args         = _.clone(type.args)
    // Add the instance as the first argument
    args.unshift(new types.Instance(nativeObject.type))
    // Build the actual function
    var fn = new NativeFunction(internalName, args, type.ret)
    genericCompileFunction(ctx, fn, stmt)
    // Save the native function
    type.setNativeFunction(fn)
  }
}

AST.New.prototype.compile = function (ctx, blockCtx) {
  this.compileToValue(ctx, blockCtx)
}

// TODO: Maybe move this into `AST.New.prototype`?
function findInitializer (nativeObject, argsNodes) {
  var type         = nativeObject.type,
      initializers = nativeObject.initializers
  for (var i = 0; i < initializers.length; i++) {
    var init = initializers[i]
    // Slice off the first argument since it will be an instance of the type
    var initArgs = init.args.slice(1)
    // Skip if lengths aren't the same
    if (initArgs.length !== argsNodes.length) {
      continue
    }
    // Then actually check the types of the arguments
    var argsMatch = true
    for (var x = 0; x < initArgs.length; x++) {
      var ia = initArgs[x],
          an = argsNodes[x]
      console.log(ia)
      console.log(an)
      throw new ICE('Argument comparison not implemented yet')
    }
    // If all the arguments match then we've found our NativeFunction intializer
    if (argsMatch) {
      return init
    }
  }
  throw new ICE('Initializer not found')
}

AST.New.prototype.compileToValue = function (ctx, blockCtx) {
  var type         = this.constructorType,
      args         = this.args,
      nativeObject = type.getNativeObject()
  // Figure out the correct NativeFunction to use to initialize this object
  var init = findInitializer(nativeObject, args)
  // Compile all of the arguments down to values
  var argValues = []
  for (var i = 0; i < args.length; i++) {
    var arg = args[i]
    argValues.push(arg.compileToValue(ctx, blockCtx))
  }

  // Allocate the new instance of the class through the GC
  var structType   = nativeObject.structType,
      sizeInt      = nativeObject.sizeOf(ctx),
      gcMallocPtr  = ctx.extern.GC_malloc.ptr
  // Call the GC allocator
  var objPtr = ctx.builder.buildCall(gcMallocPtr, [sizeInt], '')
  // Cast it to the right type (from just a plain pointer)
  objPtr = ctx.builder.buildPointerCast(objPtr, LLVM.Types.pointerType(structType), '')
  // var objPtr = ctx.builder.buildAlloca(nativeObject.structType, nativeObject.type.name)

  // Call the initializer function on the object
  var initFn = init.getPtr()
  argValues.unshift(objPtr)
  ctx.builder.buildCall(initFn, argValues, '')
  // Return the pointer to the actual object
  return objPtr
}

var ifCounter = 1

AST.If.prototype.compile = function (ctx, blockCtx) {
  var truthyVal = compileTruthyTest(ctx, blockCtx, this.cond),
      blockNum  = (ifCounter++),
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
  if (!this.isLastStatement) {
    contBlock = blockCtx.fn.appendBasicBlock('cont'+blockNum)
  }
  // If we have an else condition then set up a block for it
  if (this.elseBlock) {
    elseBlock = blockCtx.fn.appendBasicBlock('else'+blockNum)
  }
  // Build up entries for each of the else blocks
  if (this.elseIfs.length > 0) {
    // Set up the arrays for the condition blocks and then-blocks
    var length  = this.elseIfs.length
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
  ctx.builder.buildCondBr(truthyVal, thenBlock, afterFirstCond)
  // Build the then-block
  this.compileConditionBlock(ctx, parentFn, this.block, thenBlock, contBlock)

  // Compile all the else-ifs
  for (var i = 0; i < this.elseIfs.length; i++) {
    var nextCond = elseIfConds[i + 1],
        cond     = elseIfConds[i],
        then     = elseIfThens[i],
        elseIf   = this.elseIfs[i]
    // If the next condition is null then we know we're at the end and will
    // just jump to the else block.
    nextCond = (nextCond ? nextCond : postElseIfsBlock)
    // Compile down the condition
    ctx.builder.positionAtEnd(cond)
    truthyVal = compileTruthyTest(ctx, blockCtx, elseIf.cond)
    ctx.builder.buildCondBr(truthyVal, then, nextCond)
    // Then compile down the `then`
    this.compileConditionBlock(ctx, parentFn, elseIf.block, then, nextCond)
  }

  // Build the else-block if present
  if (this.elseBlock) {
    this.compileConditionBlock(ctx, parentFn, this.elseBlock, elseBlock, contBlock)
  }

  // Position the builder at the end of the continuation block
  ctx.builder.positionAtEnd(contBlock)
}
AST.If.prototype.compileConditionBlock = function (ctx, parentFn, blockNode, blockPtr, contBlockPtr) {
  ctx.builder.positionAtEnd(blockPtr)
  compileBlock(ctx, blockNode, parentFn)
  var lastInstrTerm = isLastInstructionTerminator(blockPtr)
  if (!lastInstrTerm && contBlockPtr !== null) {
    ctx.builder.buildBr(contBlockPtr)
  }
}

AST.Binary.prototype.compile = function (ctx, blockCtx, exprCtx) {
  this.compileToValue(ctx, blockCtx, exprCtx)
}

AST.Binary.prototype.compileToValue = function (ctx, blockCtx, exprCtx) {
  var lexpr = this.lexpr,
      rexpr = this.rexpr
  // Check (and unbox) the types
  var lexprType = lexpr.type,
      rexprType = rexpr.type
  assertInstanceOf(lexprType, types.Instance)
  assertInstanceOf(rexprType, types.Instance)
  lexprType = lexprType.type
  rexprType = rexprType.type
  // Find the binary-op NativeFunction
  var builder = BinaryOps.getBuilder(this.op, lexprType, rexprType)
  assertInstanceOf(builder, Function)
  // Compile the two sides down to a value that we can use
  var lexprValue = lexpr.compileToValue(ctx, blockCtx),
      rexprValue = rexpr.compileToValue(ctx, blockCtx)
  // Call the builder function that we got from BinaryOps
  var retValue = builder(ctx, lexprValue, rexprValue),
      retType  = this.type
  tryUpdatingExpressionContext(exprCtx, retType, retValue)
  return retValue
}

