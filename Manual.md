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
