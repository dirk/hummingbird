var inherits = require('util').inherits

function InternalCompilerError (message, origin) {
  Error.apply(this)
  this.name    = 'InternalCompilerError'
  this.message = message
  this.origin  = (origin ? origin : null)
}
inherits(InternalCompilerError, Error)

module.exports = {
  InternalCompilerError: InternalCompilerError
}

