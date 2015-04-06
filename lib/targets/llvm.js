
var AST   = require('../ast'),
    types = require('../types'),
    Scope = require('../typesystem/scope'),
    LLVM  = require('../../../llvm2'),
    Builtins = require('./llvm/builtins')

var Int8Type    = LLVM.Types.Int8Type,
    Int32Type   = LLVM.Types.Int32Type,
    VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

var Int32Zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

function Slots () {
  this.slots = {}
}
Slots.prototype.checkAllocated = function (name) {
  var slot = this.slots[name]
  if (!slot) {
    throw new Error('Slot not allocated: '+name)
  }
}
Slots.prototype.buildAlloc = function (ctx, name) {
  if (this.slots[name]) {
    throw new Error('Slot already allocated: '+name)
  }
  var slot = ctx.builder.buildAlloca(Int8PtrType, name)
  this.slots[name] = slot
}
Slots.prototype.buildSet = function (ctx, name, value) {
  assertInstanceOf(value, Buffer)
  var slot = this.slots[name],
      zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)
  // Get the element pointer
  var ptr = ctx.builder.buildGEP(slot, [zero], name)
  return ctx.builder.buildStore(value, ptr)
}
Slots.prototype.buildGet = function (ctx, name) {
  this.checkAllocated(name)
  var slot = this.slots[name],
      ptr  = ctx.builder.buildGEP(slot, [Int32Zero], name)
  return ctx.builder.buildLoad(ptr, name)
}
Slots.prototype.buildAllocAndSet = function (ctx, name, value) {
  this.buildAlloc(ctx, name)
  return this.buildSet(ctx, name, value)
}

function ConstantSlots () {
  this.slots = {}
}
ConstantSlots.prototype.buildGet = function (ctx, name) {
  var value = this.slots[name]
  if (!value) {
    throw new Error('Constant slot not found: '+name)
  }
  return value
}

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
    this.compileNamed(ctx, blockCtx)
  }
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

AST.Chain.prototype.compileToValue = function (ctx, blockCtx) {
  // Keep tracking of the scope of the beginning of the chain
  var outermostScope = null
  var itemType = blockCtx.block.scope.get(this.name, function (scope, type) {
    outermostScope = scope
  })

  var scope     = blockCtx.block.scope,
      scopePath = []
  while (scope !== outermostScope) {
    if (!scope) {
      throw new Error('Ran out of scopes!')
    }
    scopePath.unshift(scope)
    scope = scope.parent
  }
  // Outermost scope will now be the first in `scopePath`; innermost scope
  // will be the last in `scopePath`.

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


AST.Chain.prototype.compileIntrinsic = function (ctx, blockCtx, baseInstance) {
  var base = baseInstance.type,
      self = this
  var lastIndex = this.tail.length - 1
  for (var i = 0; i < this.tail.length; i++) {
    var item = this.tail[i]
    switch (item.constructor) {
      case AST.Property:
        base = base.getTypeOfProperty(item.name)
        continue
      case AST.Call:
        if (i !== lastIndex) {
          throw new Error('AST.Chain#compileIntrinsic: Can only handle call as last index')
        }
        assertInstanceOf(base, types.Function)
        var nativeFunction = base.getNativeFunction()
        var args = item.args.map(function (arg) {
          return arg.compileToValue(ctx, blockCtx)
        })
        var fn = nativeFunction.fn
        return ctx.builder.buildCall(fn, args, '')
        continue
      default:
        throw new Error('Cannot handle item type: '+item.constructor.name)
    }
  }
  throw new Error('AST.Chain#compileIntrinsic: Unreachable')
}

