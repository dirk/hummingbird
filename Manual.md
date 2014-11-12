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
var c = func () -> String with a, b {
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

  initializer() {
    this.b = "Hello world!"
  }
  initializer(otherC: String) {
    // Calling other initializers from within an initializer is allowed to make
    // building higher-level initializing methods on top of lower-level ones
    // possible.
    initializer()
    this.c = this.b + ' ' + otherC
  }
}

class B extends A {
  var d: String

  initializer(this.d) {
    super.initializer("#{this.d}")
  }
  // Looking above, you can see that Hummingbird also allows you to easily
  // auto-assign properties via initializer arguments. At compile time the
  // `this.d` is translated into a hidden parameter with type `String` and an
  // assignment of that parameter to `this.d` is inserted at the top of the
  // initializer body. In effect it generates the following:
  //
  //   initializer(_d: String) {
  //     this.d = _d
  //     super.initializer("#{this.d}")
  //   }
}

var e = B("Hello programmer.")
e.b == "Hello world!"
e.c == "Hello world! Hello programmer"
e.d == "Hello programmer."
```
