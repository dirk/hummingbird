
var AST   = require('../ast'),
    types = require('../types'),
    Scope = require('../typesystem/scope'),
    LLVM  = require('../../../llvm2'),
    Builtins = require('./llvm/builtins')

var Int8Type    = LLVM.Types.Int8Type,
    Int32Type   = LLVM.Types.Int32Type,
    VoidType    = LLVM.Types.VoidType,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)


AST.Root.prototype.emitToFile = function (outFile) {
  var ctx = {}
  ctx.module       = new LLVM.Module('test')
  ctx.builder      = new LLVM.Builder()
  ctx.pass_manager = new LLVM.FunctionPassManager(ctx.module)
  ctx.funcs        = {}
  // Add the builtsin
  Builtins.compile(ctx, this)

  // Build the main function
  this.buildMain(ctx)

  ctx.module.dump()
  ctx.module.writeBitcodeToFile(outFile)
}

AST.Root.prototype.buildMain = function (ctx) {
  // Set up the main function
  var mainType = new LLVM.FunctionType(VoidType, [], false),
      mainFunc = ctx.module.addFunction('main', mainType)

  // Setup the entry into the function
  var entry = mainFunc.appendBasicBlock('entry')
  ctx.builder.positionAtEnd(entry)
  compileBlock(ctx, this, entry)

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
      slots       = {}
  function allocateSlot (name) {
    var slot = ctx.builder.buildAlloca(Int8PtrType, name)
    return slot
  }
  Object.keys(scopeLocals).forEach(function (name) {
    slots[name] = allocateSlot(name)
  })

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

var Int32Zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

function BlockContext (ctx, block, slots) {
  this.ctx   = ctx
  this.block = block
  this.slots = slots
}
BlockContext.prototype.setSlotValue = function (slotName, value) {
  assertInstanceOf(value, Buffer)
  var slot = this.slots[slotName],
      zero = LLVM.Library.LLVMConstInt(Int32Type, 0, true)
  // Get the element pointer
  var ptr = this.ctx.builder.buildGEP(slot, [zero], slotName)
  return this.ctx.builder.buildStore(value, ptr)
}
BlockContext.prototype.getSlotValue = function (slotName) {
  var slot = this.slots[slotName],
      ptr  = this.ctx.builder.buildGEP(slot, [Int32Zero], slotName)
  return this.ctx.builder.buildLoad(ptr, slotName)
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
  blockCtx.setSlotValue(this.lvalue.name, rvalue)
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

AST.Chain.prototype.compileToValue = function (ctx, blockCtx) {
  // Find the base and its slot
  var itemType  = blockCtx.block.scope.get(this.name),
      itemValue = blockCtx.getSlotValue(this.name)
  // Can't handle anything beyond that right now, sorry!
  if (this.tail.length > 0) {
    throw new Error("AST.Chain#compileToValue cannot handle tails yet")
  }
  return itemValue
}

AST.Chain.prototype.compile = function (ctx, blockCtx) {
  var name = this.name
  // Find the base in this scope
  var instance = blockCtx.block.scope.get(name)
  assertInstanceOf(instance, types.Instance)

  if (instance.type.intrinsic) {
    this.compileIntrinsic(ctx, blockCtx, instance)
    return
  }
  console.error('AST.Chain#compile not implemented yet')
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

