
var AST      = require('../ast'),
    types    = require('../types'),
    Scope    = require('../typesystem/scope'),
    LLVM     = require('../../../llvm2'),
    Builtins = require('./llvm/builtins'),
    slots    = require('./llvm/slots')

// Unbox the slots module
var Slots         = slots.Slots,
    ConstantSlots = slots.ConstantSlots

var Int8Type    = LLVM.Types.Int8Type,
    Int32Type   = LLVM.Types.Int32Type,
    VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var Int32Zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

AST.Root.prototype.emitToFile = function (outFile) {
  var ctx = {}
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
  compileBlock(ctx, this, mainEntry)

  // var str = ctx.builder.buildGlobalStringPtr("Hello world!\n", 'greeting')
  // ctx.builder.buildCall(ctx.funcs.puts, [str], '')
  ctx.builder.buildRetVoid()
}

function compileBlock (ctx, block, entry) {
  // Bunch of pre-conditions to make sure we got sane arguments
  if (!(block.statements instanceof Array)) {
    throw new Error('Missing statements in block')
  }
  if (!(block.scope instanceof Scope)) {
    throw new Error('Missing block\'s scope')
  }
  if (!entry) {
    throw new Error('Missing entry point for block')
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

AST.Chain.prototype.compileToValue = function (ctx, blockCtx) {
  // Keep tracking of the scope of the beginning of the chain
  var outermostScope = null
  var itemType = blockCtx.block.scope.get(this.name, function (scope, type) {
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

  // Look up the Slots for the scope that the head of our chain belongs to
  var slots = ctx.slotsMap[outermostScope.id]
  if (!slots) {
    throw new Error("Couldn't find slots for scope #"+outermostScope.id)
  }
  var itemValue = slots.buildGet(ctx, this.name)

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
      itemType = type.getTypeOfProperty(propertyName)
      itemValue = ctx.builder.buildLoad(ptr, propertyName)
      break
    case AST.Call:
      assertInstanceOf(itemType, types.Function)
      // Compile all the args into values
      var argValues = item.args.map(function (arg) {
        return arg.compileToValue(ctx, blockCtx)
      })
      itemType  = new types.Instance(itemType.ret)
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

