var inherits = require('util').inherits

function LocativeError (message, origin) {
  Error.apply(this)
  this.message = message
  this.origin  = (origin ? origin : null)
}
inherits(LocativeError, Error)

function InternalCompilerError (message, origin) {
  LocativeError.call(this, message, origin)
  this.name = 'InternalCompilerError'
}
inherits(InternalCompilerError, LocativeError)

function TypeError (message, origin) {
  LocativeError.call(this, message, origin)
  this.name = 'TypeError'
}
inherits(TypeError, LocativeError)

module.exports = {
  InternalCompilerError: InternalCompilerError,
  TypeError:             TypeError
}

