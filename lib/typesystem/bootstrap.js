var types = require('../types')

module.exports = function (TypeSystem) {

  TypeSystem.prototype.bootstrap = function () {
    // The root object is the base object from which all objects descend
    var rootObject = new types.Object('fake')
    rootObject.supertype = null
    rootObject.isRoot    = true
    // Setup the basic intrinsic types (Any, Object, Number, etc.)
    this.bootstrapIntrinsicTypes(rootObject)
    // Set up our built-ins
    this.bootstrapConsole(rootObject)
    // Expose rootObject to later functions
    this.rootObject = rootObject
  }// bootstrap()

  TypeSystem.prototype.bootstrapIntrinsicTypes = function (rootObject) {
    var Number = new types.Number(rootObject)
    this.root.setLocal('Any',     new types.Any())
    this.root.setLocal('Object',  new types.Object(rootObject))
    this.root.setLocal('String',  new types.String(rootObject))
    this.root.setLocal('Number',  Number)
    this.root.setLocal('Boolean', new types.Boolean(rootObject))
    // Alias Integer to Number
    this.root.setLocal('Integer', Number)
  }

  TypeSystem.prototype.bootstrapConsole = function (rootObject) {
    var consoleType = new types.Object(this.root.getLocal('Object'))
    consoleType.intrinsic = true
    consoleType.name      = 'BuiltinConsole'
    // Create the `log(...)` function and add it to the console's type
    var consoleLogFunction = new types.Function(rootObject, [this.root.getLocal('Any')])
    consoleType.properties['log'] = consoleLogFunction 
    // Create a faux instance of the console and add it to the root scope
    var consoleInstance = new types.Instance(consoleType)
    this.root.setLocal('console', consoleInstance)
  }

}// module.exports

