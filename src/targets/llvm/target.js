var LLVM = require('./library')

function initializeTarget (ctx) {
  // Get the current native target and update the module with it
  var nativeTargetTriple = LLVM.Library.LLVMGetDefaultTargetTriple()
  LLVM.Library.LLVMSetTarget(ctx.module.ptr, nativeTargetTriple)

  // Figure out which target we're on
  var parts = nativeTargetTriple.split('-'),
      cpu   = parts[0]
  // Initialize necessary target info
  switch (cpu) {
    case 'x86_64':
      LLVM.Library.LLVMInitializeX86Target()
      LLVM.Library.LLVMInitializeX86TargetInfo()
      LLVM.Library.LLVMInitializeX86TargetMC()
      break
    default:
      throw new Error('Cannot initialize target CPU: '+cpu)
  }
  var target = LLVM.Library.LLVMGetFirstTarget()
  if (!LLVM.Library.LLVMTargetHasTargetMachine(target)) {
    throw new Error('Missing LLVM target machine')
  }
  var targetMachine = LLVM.Library.LLVMCreateTargetMachine(target, nativeTargetTriple, '', '', 0, 0, 0)
  // console.log(LLVM.Library.LLVMGetTargetMachineFeatureString(targetMachine))
  var targetData = LLVM.Library.LLVMGetTargetMachineData(targetMachine)
  // console.log(LLVM.Library.LLVMCopyStringRepOfTargetData(targetData))

  // Expose TargetData on the context
  ctx.targetData = targetData

  ctx.logger.info('Initialized target compilation for '+cpu+' ('+nativeTargetTriple+')')
}

module.exports = {
  initializeTarget: initializeTarget
}

