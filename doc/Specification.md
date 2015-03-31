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

