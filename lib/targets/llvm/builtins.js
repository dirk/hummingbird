var LLVM = require('../../../../llvm2')

var types          = require('../../types'),
    NativeFunction = require('./native-function'),
    NativeObject   = require('./native-object')

var VoidType    = LLVM.Types.VoidType,
    Int32Type   = LLVM.Types.Int32Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

// Global external linkages
var putsType = new LLVM.FunctionType(Int32Type, [Int8PtrType], false)

// ctx:  Compilation context object
// root: AST.Root node
function compile (ctx, mainEntry, root) {
  // Setup our external "linkages"
  var extern = {
    puts: ctx.module.addFunction('puts', putsType)
  }

  var topLevelScope = root.scope,
      rootScope     = topLevelScope.parent
  // Setup the console object
  var consoleInstance = rootScope.getLocal('console'),
      consoleType     = consoleInstance.type,
      logType         = consoleType.getTypeOfProperty('log')

  // Create the NativeFunction for logging strings
  var log = new NativeFunction('TBuiltinConsole_Mlog', [rootScope.getLocal('String')], rootScope.getLocal('Void'))
  log.defineBody(ctx, function (entry) {
    var builder = ctx.builder
    // Get the string parameter and fetch the `puts` C library function
    var str  = LLVM.Library.LLVMGetParam(this.fn.ptr, 0),
        puts = extern.puts
    builder.buildCall(puts, [str], '')
    builder.buildRetVoid()
  })
  logType.setNativeFunction(log)
  // Reposition the builder now that we're done
  ctx.builder.positionAtEnd(mainEntry)

  var consoleObject = new NativeObject(consoleInstance.type)
  // Update the console type to point to its generated struct type
  consoleInstance.type.setNativeObject(consoleObject)
  // Must define the object before we can instantiate
  consoleObject.define(ctx)
  var consoleValue = consoleObject.build(ctx, 'console'),
      logFnPtr     = consoleObject.buildStructGEPForProperty(ctx, consoleValue, 'log')
  // Store the native log function in the console object instance
  ctx.builder.buildStore(log.fn.ptr, logFnPtr)

  // Finally we'll expose the console object in the global slots
  ctx.globalSlots.buildSet(ctx, 'console', consoleValue)

  // Define external linkages for GC
  var sizeTType    = Int32Type,
      gcMallocType = new LLVM.FunctionType(Int8PtrType, sizeTType, false)
  // Add the `GC_malloc` function
  extern.GC_malloc = ctx.module.addFunction('GC_malloc', gcMallocType)
}

module.exports = {
  compile: compile
}

