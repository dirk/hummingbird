
// Adapting Swift/C/etc. ideas of curly braces as an explicit block construct

if condition == true {
    aFunction()
}

// Closing of variables must be explicit rather than implicit

var risky = "don't let me in your closures"

var aFunction = func (a, b) {
    return risky // This will raise a compiler error!
}

var anotherFunction = func (a, b) with illegal {
    return risky // This won't raise any errors
}



// Numeric types:
//    Float (widest numeric type)
//     /
// Integer (narrow integer type)

var typedVar: String = "this is explicit"

// Even though JS represents all numbers as floats, the compiler will follow this
// value to ensure it never exceeds 52/56 decimal points (when it would start
// loosing precision due to floatyness).
var anInteger: Integer = 1

// This will indicate to the compiler that aFloat doesn't need decimal checking
var aFloat = anInteger as Float

// To truncate a Float to an Integer
var anotherInteger = aFloat.toInteger()

// Compiler mutability checking?
let a = {b: 1}

a.b = 2 // Compiler error
a = "wrong, wrong, wrong" // Compiler error

// For now protocols will only apply to functions, not properties.
protocol MyProtocol {
    func publicFunction(a: String) -> String
    // - `private` is checked by the compiler but has no effect on generated
    //   JavaScript code.
    // - Omit the `-> ...` to imply it's a void return.
    // - `OwnClass` is a placeholder class inside a protocol and similar
    //   abstract definitions that is replaced with the class of the
    //   concrete implementation.
    private func voidPrivateFunction(b: OwnClass)
    optional func optionalPublicFunction()
}

// Conformance to MyProtocol will be checked at compile time.
class MyClass: MyProtocol {

    // One can set up multiple constructors for classes. At compile time the
    // compiler will check to ensure there can be no constructor dispatch
    // ambiguity. The generated JavaScript code will then generate the
    // necessary constructor function to take an argument list and figure
    // out which constructor to dispatch it to.
    constructor(name: String) {
        this.name = name
    }
    constructor(name: String, priv: String) {
        this.name = name
        this.privateProperty = priv
    }

    func publicFunction(a: String) -> String {
        return "Hello from #{this.name}!"
    }

    private var privateProperty: String = "default value"
    // If you provide a block to the property default, it will call that block
    // with the new instance of the class when that instance is constructed.
    var publicProperty: String = { this.privateProperty }
    var name: String

}

// Casting an object as Array will make the compiler check to make sure
// that only Array-type operations are performed on it (ie. keys may only
// be integers).
var a: Array<Any>

// Similarly this will ensure only String keys are used.
var b: Dictionary<String, Any>

// Any is the union of Object and Null, so the above would allow null values,
// the below would not allow null values.
var c: Dictionary<String, Object>

// For dealing with a value that is an Any, you need to explicity narrow
// the type.
var a: Any = ...
if a == null {
    // Handle the null value
    console.log("null")
} else {
    // This will narrow the outer `a` into this inner scope as a String type.
    var a = a as! String
    // Also we'll provide runtime type assertions to easily raise exceptions.
    assert(a isa String)
    console.log(a)
}
// Anys can also have a specific boxed type, which tells the compiler that
// they will either be null or an instance of that type.
var a: Any<String>
a = null  // Valid
a = "foo" // Valid
a = Bar() // Invalid

// Boxed anys can then use the `narrow` operator to easily narrow to the boxed type.
var b = unwrap a
// To unbox an existing Any<...> in a scope
unwrap var a
// There's also an `if narrow` shorthand to only do something if the value is
// not null; the value is unboxed in the condition block's scope.
if unwrap a {
    // `a` will be a String rather than Any<String> in this scope.
}


enum Options {
    Yes = 1
    No
}
// Compiled JavaScript would look like:
//   var Options = {Yes: 1, No: 2}
// Compiler would also check to make sure no code changed these values.

enum HTTP.StatusCodes {
    OK = 200
    NotFound = 404
}


// Passing functions to a function
someAsyncStuff((err: Any<Error>, result: Any<Object>) -> {
    if unwrap err {
        // The block only gets called if `err` isn't null. In this scope
        // `err` is an Error.
        console.log("Error doing async thing: #{err.toString()}")
        return
    }
    doSomething(unwrap result)
})

// Iteration over Arrays
let values: Array<String> = ['one', 'two']

for let value in values {
    console.log(value) // => 'one'
    value += 'yo' // Compiler error, modifying a constant
}
for var value in values {
   value += "yo"
   console.log(value) // => 'oneyo' and 'twoyo'
}

console.log(values) // ['one', 'two']


// Appling a `for-in` to any objects that aren't Arrays or subclasses of
// Array will result in a compiler error.

// To do a property iteration of an object:
let obj  = {a: 'b'}   // -> Object
let keys = obj.keys() // -> Array<Any>

for var key in keys {
    let value = obj[key] // -> Any
}


// There's also a tradition ternary `for`
for initial; condition; step {
    doSomething()
}
for var i = 1; i <= 3; i++ {
    console.log(i) // => 1, 2, 3
}

// And a `while`
var i = 1
while i <= 3 {
    console.log(i) // => 1, 2, 3
    i++
}
