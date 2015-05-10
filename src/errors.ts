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
  }// constructor
  ensureStack() {
    if (this.stack === undefined) {
      var stack = (new Error()).stack
      if (stack) {
        // Split it into lines and slice off the first description line
        var lines = stack.trim().split("\n").slice(1)
        while (lines.length > 0 && /\/src\/errors\.js/.test(lines[0])) {
          lines.shift()
        }
        lines.unshift(this.toString())
        this.stack = lines.join("\n")
      }
    }// this.stack === undefined
  }// ensureStack()
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
  constructor(message, origin?) {
    super(message, origin)
    this.name = 'InternalCompilerError'
    super.ensureStack()
  }
}

export class TypeError extends LocativeError {
  constructor(message, origin?) {
    super(message, origin)
    this.name = 'TypeError'
    super.ensureStack()
  }
}

