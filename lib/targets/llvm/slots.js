var LLVM              = require('../../../../llvm2'),
    types             = require('../../types'),
    nativeTypeForType = require('./native-types').nativeTypeForType

var Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type),
    Int32Type   = LLVM.Types.Int32Type,
    Int32Zero   = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

// Function to check if the type has been compiled
function typeCompiled (type) {
  switch (type.constructor) {
    case types.Instance:
      var instance = type
      return typeCompiled(instance.type)
    case types.Object:
      if (type.hasNativeObject()) { return true }
      break
    case types.Function:
      if (type.hasNativeFunction()) { return true }
      break
  }
  return false
}

function Slots () {
  this.slots      = {}
  this.slotsTypes = {}
  // Queue of slots to be built once type has become available. Items
  // are 2-tuples of name (string) and type (Type)
  this.allocQueue = []
}
Slots.prototype.checkAllocQueue = function (ctx) {
  var self     = this,
      toDelete = []
  for (var i = 0; i < this.allocQueue.length; i++) {
    var pair = this.allocQueue[i],
        name = pair[0],
        type = pair[1]
    if (typeCompiled(type)) {
      toDelete.push(name)
    }
  }
  // Didn't find any slots to allocate
  if (toDelete.length === 0) { return }
  // Update the alloc queue if there were items to delete
  this.allocQueue = this.allocQueue.filter(function (pair) {
    var name = pair[0],
        idx  = toDelete.indexOf(name)
    if (idx === -1) {
      return true
    }
    var type = pair[1]
    var nativeType = nativeTypeForType(type)
    // Alloc the slot now that we have a type
    self.buildAlloc(ctx, name, nativeType)
    return false
  })
}
Slots.prototype.assertSlotNotAllocated = function (name) {
  if (this.slots[name]) {
    throw new Error('Slot already allocated: '+name)
  }
}
Slots.prototype.assertSlotAllocated = function (name) {
  if (!this.slots[name]) {
    throw new Error('Slot not allocated: '+name)
  }
}
Slots.prototype.enqueueAlloc = function (name, type) {
  this.assertSlotNotAllocated(name)
  this.allocQueue.push([name, type])
}
Slots.prototype.buildAlloc = function (ctx, name, type) {
  this.assertSlotNotAllocated(name)
  if (type === undefined) {
    type = Int8PtrType
  }
  var slot = ctx.builder.buildAlloca(type, name)
  this.slots[name]      = slot
  this.slotsTypes[name] = type
}
Slots.prototype.buildSet = function (ctx, name, value) {
  this.checkAllocQueue(ctx)
  this.assertSlotAllocated(name)
  if (!(value instanceof Buffer)) {
    throw new Error('Expected Buffer as value for setting a slot')
  }
  var slot = this.slots[name]
  // Get the element pointer
  var ptr = ctx.builder.buildGEP(slot, [Int32Zero], name)
  // console.log('storing in slot: '+name)
  return ctx.builder.buildStore(value, ptr)
}
Slots.prototype.buildGet = function (ctx, name) {
  this.assertSlotAllocated(name)
  var slot = this.slots[name],
      ptr  = ctx.builder.buildGEP(slot, [Int32Zero], name)
  return ctx.builder.buildLoad(ptr, name)
}
Slots.prototype.buildAllocAndSet = function (ctx, name, value) {
  this.buildAlloc(ctx, name)
  return this.buildSet(ctx, name, value)
}
Slots.prototype.getStorable = function (ctx, name) {
  this.assertSlotAllocated(name)
  // slot will just be a pointer to a pointer
  var slot = this.slots[name]
  return slot
}

function ConstantSlots () {
  this.slots = {}
}
ConstantSlots.prototype.buildSet = function (ctx, name, value) {
  if (this.slots[name]) {
    throw new Error('Cannot re-set constant slot: '+name)
  }
  this.slots[name] = value
}
ConstantSlots.prototype.buildGet = function (ctx, name) {
  var value = this.slots[name]
  if (!value) {
    throw new Error('Constant slot not found: '+name)
  }
  return value
}
ConstantSlots.prototype.getStorable = function (ctx, name) {
  throw new Error('Cannot get a storable for a constant slot: '+name)
}

function GlobalSlots () {
  this.slots = {}
}
GlobalSlots.prototype.buildSet = function (ctx, name, value) {
  // Create the console value as a global
  var ty     = LLVM.Library.LLVMTypeOf(value)
      global = LLVM.Library.LLVMAddGlobal(ctx.module.ptr, ty, 'G'+name)
  LLVM.Library.LLVMSetGlobalConstant(global, false)
  var initialNull = LLVM.Library.LLVMConstPointerNull(ty)
  LLVM.Library.LLVMSetInitializer(global, initialNull)
  ctx.builder.buildStore(value, global, '')
  // Save the global pointer
  this.slots[name] = global
}
GlobalSlots.prototype.buildGet = function (ctx, name) {
  if (!this.slots[name]) { throw new Error('Global not found: '+name) }
  var global = this.slots[name],
      ptr    = ctx.builder.buildGEP(global, [Int32Zero], name)
  return ctx.builder.buildLoad(ptr, name) 
}

module.exports = {
  Slots: Slots,
  ConstantSlots: ConstantSlots,
  GlobalSlots:   GlobalSlots
}

