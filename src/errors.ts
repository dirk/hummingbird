var inherits = require('util').inherits

declare class Error {
  name:    string
  message: string
  stack:   string
  constructor(message?: string)
}

// Internal base error class
class BaseError extends Error {
  constructor(message: string) {
    super(message)
    this.name    = 'BaseError'
    this.message = message
    this.stack   = (new Error()).stack
  }
}

class LocativeError extends BaseError {
  origin: any

  constructor(message, origin) {
    super(message)
    this.name = 'LocativeError'
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

