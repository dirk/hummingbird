# Specification

Semi-formal specification of the syntax and semantics of the Hummingbird language.

## Variables

There are two types of variable definitions: `var` and `let`. The former declares a **mutable** variable in the scope. The latter declares an **immutable** variable in the scope.

Immutability is checked at only the reference level. (It ensures that the variable will always refer to the same thing; however mutating properties of that thing can still occur!) Furthermore, this checking happens only at compile time.

<spec name="variables">
```hb
var a: Integer = 1
let b: Integer = 2
```
```js
var a = 1;
var b = 2;
```
</spec>

## Functions

Hummingbird provides first-class, anonymous functions.

<spec name="functions">
```hb
var a = func (x: String) -> String { }
```
```js
var a = function (x) { };
```
</spec>

### Inferred return types

In many cases Hummingbird can infer the return type of a function based off of its arguments and body.

<spec name="inferred-returns">
```hb
var a = func (x: Integer) { return x }
```
```js
var a = function (x) {
  return x;
};
```
</spec>

### Dynamic dispatch

Run-time dynamic dispatching is provided through the `multi` statement.

<spec name="multi">
```hb
multi a (b: Integer, c: Integer) -> Integer
func a (b, c) when (b == 0 || c == 0) { return 1 }
func a (b, c) { return b * c }
```
```js
function a (b, c) {
  switch (false) {
  case !(b == 0 || c == 0):
    return a_1(b, c);
  default:
    return a_2(b, c);
  }
  function a_1 (b, c) {
    return 1;
  }
  function a_2 (b, c) {
    return b * c;
  }
}
```
</spec>

# Control flow

The expected suite of control flow statements, such as if, for, and while, are provided.

## While

<spec name="while">
```hb
var a: Integer = 1
while a < 10 {
  a = a + 1
}
# a will equal 10 here
```
```js
var a = 1;
while (a < 10) {
  a = a + 1;
}
```
</spec>

