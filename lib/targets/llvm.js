
var _        = require('lodash'),
    AST      = require('../ast'),
    types    = require('../types'),
    Scope    = require('../typesystem/scope'),
    LLVM     = require('../../../llvm2'),
    Builtins = require('./llvm/builtins'),
    slots    = require('./llvm/slots'),
    Errors   = require('../errors'),
    ICE      = Errors.InternalCompilerError

var NativeFunction = require('./llvm/native-function'),
    NativeObject   = require('./llvm/native-object')

// Unbox the slots module
var Slots         = slots.Slots,
    ConstantSlots = slots.ConstantSlots

var Int8Type    = LLVM.Types.Int8Type,
    Int32Type   = LLVM.Types.Int32Type,
    Int64Type   = LLVM.Types.Int64Type,
    VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)
    // EmptyFunctionType = LLVM.Library.LLVMFunctionType(VoidType, null, 0, false)

var Int32Zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

var TypeOf            = LLVM.Library.LLVMTypeOf,
    GetTypeKind       = LLVM.Library.LLVMGetTypeKind,
    DumpType          = LLVM.Library.LLVMDumpType,
    PrintTypeToString = LLVM.Library.LLVMPrintTypeToString,
    PointerTypeKind   = GetTypeKind(Int8PtrType)
    // IntegerTypeKind  = GetTypeKind(Int8Type),
    // FunctionTypeKind = GetTypeKind(EmptyFunctionType)

function Context () {}

AST.Node.prototype.compileToValue = function () {
  throw new Error('Compilation to value not yet implemented for: '+this.constructor.name)
}

AST.Root.prototype.emitToFile = function (outFile) {
  var ctx = new Context()
  ctx.module       = new LLVM.Module('test')
  ctx.builder      = new LLVM.Builder()
  ctx.pass_manager = new LLVM.FunctionPassManager(ctx.module)
  ctx.funcs        = {}
  // Slots for global values (eg. builtins)
  ctx.globalSlots  = new ConstantSlots()
  // Maps sets of Slots to their associated Scope by the scope's ID
  ctx.slotsMap     = {}
  // Add the root scope to the slots map
  var rootScope = this.scope.parent
  if (!rootScope.isRoot) { throw new Error("Couldn't find root scope") }
  ctx.slotsMap[rootScope.id] = ctx.globalSlots

  // Set up the main function
  var mainType  = new LLVM.FunctionType(VoidType, [], false),
      mainFunc  = ctx.module.addFunction('main', mainType),
      mainEntry = mainFunc.appendBasicBlock('entry')

  // Add the builtins
  Builtins.compile(ctx, mainEntry, this)
  // Compile ourselves in the main entry function
  this.buildMain(ctx, mainEntry)

  ctx.module.dump()
  ctx.module.writeBitcodeToFile(outFile)
}

AST.Root.prototype.buildMain = function (ctx, mainEntry) {
  // Setup the entry into the function
  ctx.builder.positionAtEnd(mainEntry)
  compileBlock(ctx, this)

  // var str = ctx.builder.buildGlobalStringPtr("Hello world!\n", 'greeting')
  // ctx.builder.buildCall(ctx.funcs.puts, [str], '')
  ctx.builder.buildRetVoid()
}

function compileBlock (ctx, block) {
  // Bunch of pre-conditions to make sure we got sane arguments
  if (!(block.statements instanceof Array)) {
    throw new Error('Missing statements in block')
  }
  if (!(block.scope instanceof Scope)) {
    throw new Error('Missing block\'s scope')
  }

  // Set up slots for each local
  var scopeLocals = block.scope.locals,
      slots       = new Slots()
  Object.keys(scopeLocals).forEach(function (name) {
    slots.buildAlloc(ctx, name)
  })
  // Add the slots to the map of slots in the context
  ctx.slotsMap[block.scope.id] = slots
  // Set up a new context just for this block
  var blockCtx = new BlockContext(ctx, block, slots)

  var statements = block.statements
  for (var i = 0; i < statements.length; i++) {
    var stmt = statements[i]
    stmt.compile(ctx, blockCtx)
  }
}

function assertInstanceOf (value, type, message) {
  if (!(value instanceof type)) {
    if (!message) {
      message = 'Incorrect type; expected '+type.name+', got '+value.constructor.name
    }
    throw new Error(message)
  }
}

function BlockContext (ctx, block, slots) {
  this.ctx   = ctx
  this.block = block
  this.slots = slots
}

AST.Assignment.prototype.compile = function (ctx, blockCtx) {
  if (this.type === 'var' || this.type === 'let') {
    return this.compileNamed(ctx, blockCtx)
  }
  throw new Error('Cannot compile assignment type: '+this.type)
}

AST.Assignment.prototype.compileNamed = function (ctx, blockCtx) {
  // Get a value pointer from the rvalue
  var rvalue = this.rvalue.compileToValue(ctx, blockCtx)
  assertInstanceOf(rvalue, Buffer, 'Received non-Buffer from Node#compilerToValue')
  // Get the slot pointer
  blockCtx.slots.buildSet(ctx, this.lvalue.name, rvalue)
}

AST.Literal.prototype.compileToValue = function (ctx, blockCtx) {
  var instance = this.type
  switch (instance.type.constructor) {
    case types.String:
      var stringValue = this.value
      // Build a constant with our string value and return that
      var val = ctx.builder.buildGlobalStringPtr(stringValue, '')
      return val
    case types.Number:
      return LLVM.Library.LLVMConstInt(Int64Type, this.value, '')
    default:
      var name = instance.type.constructor.name
      throw new Error('AST.Literal#compileToValue: Cannot handle instance type: '+name)
  }
}

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

// Look up the type and Slots for a given name; beings search from the passed
// block-context. Returns a 2-tuple of Slots and Type.
function getTypeAndSlotsForName (ctx, blockCtx, name, foundCb) {
  // Keep tracking of the scope of the beginning of the chain
  var outermostScope = null
  var type = blockCtx.block.scope.get(name, function (scope, _type) {
    outermostScope = scope
  })

  // Outermost scope will be the first in `scopePath`; innermost scope
  // will be the last in `scopePath`.
  var scopePath = computeScopePath(blockCtx, outermostScope)

  // Sanity-check the `scopePath` to make sure it's outermost scope is the
  // same as the one we got from the Scope class' scope resolver.
  if (scopePath[0] !== outermostScope) {
    throw new Error('Invalid scope path: first scope does not match outermost scope')
  }

  // Check this scope path to make sure we didn't bust any closures
  checkScopePathForClosureOverreach(scopePath)

  // Finally look up the Slots for the outermost scope that the name belongs to
  var slots = ctx.slotsMap[outermostScope.id]
  if (!slots) {
    throw new Error("Couldn't find slots for scope #"+outermostScope.id)
  }
  return [slots, type]
}

AST.Chain.prototype.compileToValue = function (ctx, blockCtx) {
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
          propertyName = item.name
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
      // Get the string representations of the types; sadly this is the
      // most straightforward way to compare them right now
      var itemValueTypeString = LLVM.Library.LLVMPrintTypeToString(itemValueType),
          funcTypeString      = LLVM.Library.LLVMPrintTypeToString(funcType)
      // If the types aren't the same then we'll recast
      if (itemValueTypeString !== funcTypeString) {
        // Re-cast the value to be the proper type
        itemValue = ctx.builder.buildPointerCast(itemValue, funcPtrType, '')
      }
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

function genericCompileFunction (ctx, fn, block) {
  fn.defineBody(ctx, function (entry) {
    compileBlock(ctx, block)
    // If it's returning Void and the last statement isn't a return then
    // go ahead an insert one for safety
    if (fn.ret instanceof types.Void) {
      var lastStatement = block.statements[block.statements.length - 1]
      if (!(lastStatement instanceof AST.Return)) {
        ctx.builder.buildRetVoid()
      }
    }// fn.ret is types.Void
  })// fn.defineBody
}

var nativeFunctionCounter = 1

AST.Function.prototype.compileToValue = function (ctx, block) {
  var self     = this,
      name     = "A"+(nativeFunctionCounter++),
      instance = this.type,
      type     = instance.type
  // Unbox the instance
  var args = type.args,
      ret  = type.ret
  // Setup the native function
  var fn = new NativeFunction(name, args, ret)
  genericCompileFunction(ctx, fn, this.block)
  // Save the native function on the type
  type.setNativeFunction(fn)
  // Get the raw function as a value
  var compiledFn = fn.fn.ptr
  // Convert it to a pointer
  var fnAddr = LLVM.Library.LLVMConstBitCast(compiledFn, Int8PtrType)
  return fnAddr
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

AST.Class.prototype.compile = function (ctx, block) {
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
  this.compileInitializers(ctx, block, nativeObject)
}
AST.Class.prototype.compileInitializers = function (ctx, block, nativeObject) {
  var type         = this.type,
      initializers = this.initializers
  // Build and compile a native function for each initializer function
  for (var i = 0; i < initializers.length; i++) {
    var init         = initializers[0],
        initType     = init.type,
        internalName = nativeObject.internalName+'_I'+i
    // Create the native function
    var fn = new NativeFunction(internalName, initType.args, initType.ret)
    genericCompileFunction(ctx, fn, init.block)
    // TODO: Generalize compilation of function blocks
    // TODO: Handle missing return statement at end of function when it's
    //       a Void function
  }
}

