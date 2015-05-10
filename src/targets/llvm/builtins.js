var LLVM           = require('./library'),
    types          = require('../../types'),
    NativeFunction = require('./native-function'),
    NativeObject   = require('./native-object')

var VoidType    = LLVM.Types.VoidType,
    Int32Type   = LLVM.Types.Int32Type,
    Int8PtrType = LLVM.Types.pointerType(LLVM.Types.Int8Type)

// Global external linkages
var putsType = new LLVM.FunctionType(Int32Type, [Int8PtrType], false)

// Sets up external "linkages"
// ctx:  Compilation context object
// root: AST.Root node
function compile (ctx, mainEntry, root) {
  var topLevelScope = root.scope,
      rootScope     = topLevelScope.parent

  // Functions required for main runtime -------------------------------------

  if (ctx.isMain === true) {
    // External linkages for GC
    ctx.extern.GC_init   = NativeFunction.addExternalFunction(ctx, 'GC_init', VoidType, [])
  }

  // General garbage collection functions -----------------------------------

  var sizeTType = Int32Type
  ctx.extern.GC_malloc = NativeFunction.addExternalFunction(ctx, 'GC_malloc', Int8PtrType, [sizeTType])


  // Builtin functions -------------------------------------------------------

  // Setup the console object
  var consoleInstance = rootScope.getLocal('console'),
      consoleType     = consoleInstance.type,
      logType         = consoleType.getTypeOfProperty('log')

  // Create the NativeFunction for logging strings and define it as
  // externally (computes type and builds external function pointer).
  var log = new NativeFunction('TBuiltinConsole_mlog', [rootScope.getLocal('String')], rootScope.getLocal('Void'))
  log.defineExternal(ctx)
  logType.setNativeFunction(log)
  /*
  log.defineBody(ctx, function (entry) {
    var builder = ctx.builder
    // Get the string parameter and fetch the `puts` C library function
    var str  = LLVM.Library.LLVMGetParam(this.fn.ptr, 0),
        puts = ctx.extern.puts
    builder.buildCall(puts, [str], '')
    builder.buildRetVoid()
  })
  // Reposition the builder now that we're done
  ctx.builder.positionAtEnd(mainEntry)
  */

  var consoleObject = new NativeObject(consoleInstance.type)
  // Update the console type to point to its generated struct type
  consoleInstance.type.setNativeObject(consoleObject)
  // Must define the object before we can instantiate
  consoleObject.define(ctx)
  
  /*
  var consoleValue = consoleObject.build(ctx, 'console'),
      logFnPtr     = consoleObject.buildStructGEPForProperty(ctx, consoleValue, 'log')
  // Store the native log function in the console object instance
  ctx.builder.buildStore(log.fn.ptr, logFnPtr)
  */

  // Finally we'll expose the console object in the global slots
  ctx.globalSlots.buildDefine(ctx, 'console', LLVM.Types.pointerType(consoleObject.structType))
  // ctx.globalSlots.buildSet(ctx, 'console', consoleValue)

  var typesModule   = rootScope.getLocal('std').getChild('core').getChild('types'),
      stringModule  = typesModule.getChild('string'),
      integerModule = typesModule.getChild('integer'),
      StringType    = rootScope.getLocal('String'),
      IntegerType   = rootScope.getLocal('Integer')
  
  var uppercase = new NativeFunction('Mstd_Mcore_Mtypes_Mstring_Fuppercase', [StringType], StringType)
  uppercase.defineExternal(ctx)
  stringModule.getTypeOfProperty('uppercase').setNativeFunction(uppercase)

  var lowercase = new NativeFunction('Mstd_Mcore_Mtypes_Mstring_Flowercase', [StringType], StringType)
  lowercase.defineExternal(ctx)
  stringModule.getTypeOfProperty('lowercase').setNativeFunction(lowercase)

  var integerToString = new NativeFunction('Mstd_Mcore_Mtypes_Minteger_FtoString', [IntegerType], StringType)
  integerToString.defineExternal(ctx)
  integerModule.getTypeOfProperty('toString').setNativeFunction(integerToString)
}

module.exports = {
  compile: compile
}

