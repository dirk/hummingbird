# Manual

This is a step-by-step overview of the Hummingbird language. It covers—with examples—the various aspects of the language.

## Basic Concepts

Hummingbird has a few core goals that guide the design and development of the language.

* **Usable type system**: The type system in Hummingbird is designed to make life easier for programmers. Typed, static compilation provides a set of assurances not available in dynamic languages.
* **Adaptable targeting**: Initially Hummingbird will target just Node.js and browser JavaScript execution environments. However, the intrinsic features of these targets should be exposed as a rich runtime API library rather than a core part of the language.
* **Concise, structured syntax**: Taking the lessons of JavaScript, CoffeeScript, and Swift to heart, Hummingbird's syntax aims to establish a pleasant balance between explicit code structure and minimizing unnecessary punctuation.


## Variables and Scope

Non-strict scoping rules can be the cause of a great deal of confusion and bugs, especially for inexperienced programmers. Hummingbird takes a fairly strict approach to variable scoping. The most notable aspect of this strictness is explicit closing of variables in closures, and also strict binding of the scope of the instance in *all* closures in a class (no more buggy usages of `this`).

```js
// Defining variables with explicit and inferred types
var a = "inferred string"
var b: String = "explicit string"

// Mandatory explicit closing of outer variables into the function.
// Attempting to use a variable in a function body without a new
// `var` definition or explicit closing will raise a
// compilation error.
var c = func () -> String with (a, b) {
  return a + " and " + b
}
```

## Functions

Hummingbird currently provides only anonymous functions. The named function syntax in JavaScript (eg. `function myFunction() {...}`) is not available.

There are three critical parts to every function declaration: parameters, return type, and closure. The latter two are optional.

```js
// Defining a function with no parameters, no return type, and no closure
var a = func () { }

// A function with parameters and return type
var b = func (c: String) -> String { return "Hello #{c}!" }

// Function with all three parts
var d = func (e: String) -> String with (b) { return b(e) }
d("world") == "Hello world!"

// Void function with closure
var f = func () with (b) { return b("world") }
f("world") == "Hello world!"

// Function with default parameters
var g = func(h: String = "world") -> String with (b) { return b(h) }
g("earth") == "Hello earth!"
g() == "Hello world!"
```

## Classes

Classes are composed of three things: properties, initializers, and methods.

### Initializers

Under the hood initializers are just special methods. Hummingbird does a little bit of work during compilation to make initialization easier for you: it sets default values for properties, resolves superclass initializers to make them available to you, and dispatches the actual initialization to the correct initializer method (the necessary heavy lifting for constructor overloading).

```js
class A {
  var b: String
  var c: Any

  init() {
    this.b = "Hello world!"
  }
  init(otherC: String) {
    // Calling other initializers from within an initializer is allowed to make
    // building higher-level initializing methods on top of lower-level ones
    // possible.
    init()
    this.c = this.b + ' ' + otherC
  }
}

class B extends A {
  var d: String

  init(this.d) {
    super.initializer("#{this.d}")
  }
  // Looking above, you can see that Hummingbird also allows you to easily
  // auto-assign properties via initializer arguments. At compile time the
  // `this.d` is translated into a hidden parameter with type `String` and an
  // assignment of that parameter to `this.d` is inserted at the top of the
  // initializer body. In effect it generates the following:
  //
  //   init(_d: String) {
  //     this.d = _d
  //     super.initializer("#{this.d}")
  //   }
}

var e = B("Hello programmer.")
e.b == "Hello world!"
e.c == "Hello world! Hello programmer"
e.d == "Hello programmer."
```

#### Use of `initializer` rather than `constructor`

We hold the view that there is a semantic distinction to be made between construction and initialization of an instance of a class.

* **Construction** deals with the actual allocation and low-level setup of an instance. In a lower-level language like C++ this would involve laying out the memory for the class to hold variable slots, pointers to the class record, and so forth. In high-level languages like JavaScript or Ruby this involves creating the basic object and the pointer to the class/prototype.

* **Initialization** involves the operations performed to setup the class after the construction of the class has occurred. These depend on the instance actually existing in memory so that program can work with that memory.

Construction happens in the language runtime—out of view from the code _you_ write—whereas the code you write does actually influence the initialization of the class instance once it has been constructed.

## Control Flow

Hummingbird supports the basic traditional control flow structures. Although a "for-in" style structure was considered, it was decided against for its propensity for introducing bugs and complexities in implementation.

```js
// Conditionals
if true {
  // Something
} else {
  // Another something
}

// Looping
for var i = 1; i <= 42; i++ {
  // Iterated something
}
while true {
  // Infinity!
}

// Exception handling
try {
  // Something that throws an error
} catch err {
  // Log our `err`!
} finally {
  // Clean up after ourselves
}
```
