var inherits = require('util').inherits

function TypeError (message, origin) {
  Error.apply(this)
  this.name = 'TypeError'
  this.message = message
  this.origin = (origin !== undefined) ? origin : null
}
inherits(TypeError, Error)

module.exports = TypeError
