var inherits = require('util').inherits

declare class Error {
  name:    string
  message: string
}

class LocativeError extends Error {
  origin: any

  constructor(message, origin) {
    super()
    this.message = message
    this.origin = (origin ? origin : null)
  }
}

export class InternalCompilerError extends LocativeError {
  constructor(message, origin) {
    super(message, origin)
    this.name = 'InternalCompilerError'
  }
}

export class TypeError extends LocativeError {
  constructor(message, origin) {
    super(message, origin)
    this.name = 'TypeError'
  }
}

