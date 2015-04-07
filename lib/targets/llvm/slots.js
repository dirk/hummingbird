var LLVM = require('../../../../llvm2')

var Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type),
    Int32Type   = LLVM.Types.Int32Type,
    Int32Zero   = LLVM.Library.LLVMConstInt(Int32Type, 0, true)

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
  if (!(value instanceof Buffer)) {
    throw new Error('Expected Buffer as value for setting a slot')
  }
  var slot = this.slots[name]
  // Get the element pointer
  var ptr = ctx.builder.buildGEP(slot, [Int32Zero], name)
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
Slots.prototype.getStorable = function (ctx, name) {
  this.checkAllocated(name)
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

module.exports = {
  Slots: Slots,
  ConstantSlots: ConstantSlots
}

