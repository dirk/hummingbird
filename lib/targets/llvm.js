
var _        = require('lodash'),
    AST      = require('../ast'),
    types    = require('../types'),
    Scope    = require('../typesystem/scope'),
    LLVM     = require('../../../llvm2'),
    Builtins = require('./llvm/builtins'),
    slots    = require('./llvm/slots'),
    Errors   = require('../errors'),
    ICE      = Errors.InternalCompilerError

var NativeFunction    = require('./llvm/native-function'),
    NativeObject      = require('./llvm/native-object'),
    nativeTypeForType = require('./llvm/native-types').nativeTypeForType

// Unbox the slots module
var Slots         = slots.Slots,
    ConstantSlots = slots.ConstantSlots

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
  this.buildMain(ctx, mainFunc, mainEntry)

  // ctx.module.dump()
  ctx.module.writeBitcodeToFile(outFile)
}

AST.Root.prototype.buildMain = function (ctx, mainFunc, mainEntry) {
  // Setup the entry into the function
  ctx.builder.positionAtEnd(mainEntry)
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
  var scopeLocals = block.scope.locals,
      slots       = new Slots()
  Object.keys(scopeLocals).forEach(function (name) {
    /*
    var slotType = Int8PtrType
    var isConstant = block.scope.localHasFlag(name, Scope.Flags.Constant)
    if (isConstant) {
      var localType = block.scope.getLocal(name)
      try {
        slotType = nativeTypeForType(localType)
      } catch (err) {
        console.log('Enqueueing slot allocation for: '+name+' (has type: '+localType.inspect()+')')
        if (/^Native object not found for type/.test(err.message)) {
          slots.enqueueAlloc(name, localType)
          return
        }
        throw err
      }
    }// if(isConstant)
    */
    var slotType  = null,
        localType = block.scope.getLocal(name)
    try {
      slotType = nativeTypeForType(localType)
      console.log('Added slot allocation for: '+name+' (has type: '+localType.inspect()+')')
    } catch (err) {
      console.info('Enqueueing slot allocation for: '+name+' (has type: '+localType.inspect()+')')
      if (/^Native (object|function) not found for type/.test(err.message)) {
        slots.enqueueAlloc(name, localType)
        return
      }
      throw err
    }
    slots.buildAlloc(ctx, name, slotType)
  })
  // Add the slots to the map of slots in the context
  ctx.slotsMap[block.scope.id] = slots
  // Set up a new context just for this block
  var blockCtx = new BlockContext(ctx, parentFn, block, slots)
  // If a callback to run before statements are compiled is provided then
  // call that callback with the new block-content and the slots.
  if (preStatementsCb) {
    preStatementsCb(blockCtx, slots)
  }
  // Compile all the statements
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

function BlockContext (ctx, parentFn, block, slots) {
  this.ctx   = ctx
  this.fn    = parentFn
  this.block = block
  this.slots = slots
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
    case AST.Property:
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
      break
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

AST.Literal.prototype.compileToValue = function (ctx, blockCtx) {
  var instance = this.type
  switch (instance.type.constructor) {
    case types.String:
      var stringValue = this.value
      // Build a constant with our string value and return that
      var val = ctx.builder.buildGlobalStringPtr(stringValue, '')
      return val
    case types.Number:
      throw new ICE('Number support not yet implemented')
      return LLVM.Library.LLVMConstInt(Int64Type, this.value, '')
    default:
      var name = instance.type.constructor.name
      throw new ICE('Cannot handle instance type: '+name)
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
  var returnValue = ctx.builder.buildCall(nativeFunction.fn.ptr, args, ''),
      returnType  = new types.Instance(instanceMethod.ret)
  return [returnType, returnValue]
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
      var structPointerType = LLVM.Types.pointerType(nativeObject.structType),
          itemValueType     = TypeOf(itemValue)
      // Recast if necessary
      if (!nativeTypesEqual(itemValueType, structPointerType)) {
        itemValue = ctx.builder.buildPointerCast(itemValue, structPointerType, '')
      }
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
      // If the types aren't the same then we'll recast
      if (!nativeTypesEqual(itemValueType, funcType)) {
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

function nativeTypesEqual (a, b) {
  var aTypeString = PrintTypeToString(a),
      bTypeString = PrintTypeToString(b)
  return (a === b)
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

function genericCompileFunction (ctx, nativeFn, node) {
  var hasThisArg = (node instanceof AST.Init),
      block      = node.block
  nativeFn.defineBody(ctx, function (entry) {
    // Actual LLVM function that we're compiling for
    var fnPtr = nativeFn.fn
    compileBlock(ctx, block, fnPtr, function (blockCtx, slots) {
      var argOffset = 0
      // Setup `this` and the other function args
      if (hasThisArg) {
        argOffset = 1
        // `this` will be the first argument
        var thisValue = GetParam(nativeFn.fn.ptr, 0)
        // Turn it into an unadorned pointer
        thisValue = ctx.builder.buildPointerCast(thisValue, Int8PtrType, 'this')
        // Store `this` in the slots
        slots.buildSet(ctx, 'this', thisValue)
      }
      // Handle regular arguments
      var args = node.args
      for (var i = 0; i < args.length; i++) {
        var arg      = args[i],
            argName  = arg.name,
            argValue = GetParam(nativeFn.fn.ptr, i + argOffset)
        // Unadorn and set the slot
        argValue = ctx.builder.buildPointerCast(argValue, Int8PtrType, argName)
        slots.buildSet(ctx, argName, argValue)
      }
    })
    // If it's returning Void and the last statement isn't a return then
    // go ahead an insert one for safety
    if (nativeFn.ret instanceof types.Void) {
      var lastStatement = block.statements[block.statements.length - 1]
      if (!(lastStatement instanceof AST.Return)) {
        ctx.builder.buildRetVoid()
      }
    }// nativeFn.ret is types.Void
  })// nativeFn.defineBody
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
  genericCompileFunction(ctx, fn, this)
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
        internalName = nativeObject.internalName+'_I'+i
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
    var internalName = nativeObject.internalName+'_M'+stmt.name,
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
  // TODO: Heap-allocate and GC tag this instead of stack-allocating!
  var objPtr = ctx.builder.buildAlloca(nativeObject.structType, nativeObject.type.name)
  // Call the initializer function on the object
  var initFn = init.fn.ptr
  argValues.unshift(objPtr)
  ctx.builder.buildCall(initFn, argValues, '')
  // Then cast it to a simple pointer and return
  return ctx.builder.buildPointerCast(objPtr, Int8PtrType, '')
}

function compileTruthyTest (ctx, blockCtx, expr) {
  var value    = expr.compileToValue(ctx, blockCtx),
      instance = expr.type
  // Can only truthy-test instances
  assertInstanceOf(instance, types.Instance)
  type = instance.type
  switch (type.constructor) {
  case types.String:
    var nullStringPtr = LLVM.Library.LLVMConstNull(Int8PtrType)
    // Compare the string pointer to the NULL pointer
    return ctx.builder.buildICmp(LLVM.Library.LLVMIntNE, value, nullStringPtr, '')
  default:
    throw new ICE('Cannot compile to truthy-testable value: '+type.constructor.name)
  }
}

AST.If.prototype.compile = function (ctx, blockCtx) {
  var truthyVal = compileTruthyTest(ctx, blockCtx, this.cond)
  // Set up all the blocks we'll be jumping between
  var thenBlock = blockCtx.fn.appendBasicBlock('then'),
      contBlock = blockCtx.fn.appendBasicBlock('cont'),
      elseBlock = contBlock
  // If we have an else condition then also set up a block for it
  if (this.elseBlock) {
    elseBlock = blockCtx.fn.appendBasicBlock('else')
  }
  if (this.elseIfs.length > 0) {
    throw new ICE('Else-if conditions not implemented yet')
  }
  var parentFn = blockCtx.fn.ptr
  // Build the branch, and then go build the blocks
  ctx.builder.buildCondBr(truthyVal, thenBlock, elseBlock)

  // Build the then-block
  ctx.builder.positionAtEnd(thenBlock)
  compileBlock(ctx, this.block, parentFn)
  ctx.builder.buildBr(contBlock)

  // Build the else-block if present
  if (this.elseBlock) {
    ctx.builder.positionAtEnd(elseBlock)
    compileBlock(ctx, this.elseBlock, parentFn)
    ctx.builder.buildBr(contBlock)
  }
  
  // Position the builder at the end of the continuation block
  ctx.builder.positionAtEnd(contBlock)
}

