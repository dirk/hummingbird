{// Begin preamble
  var p = {}

  function transformArgs (args) {
    var head = args[0], tail = args[2]
    return [head].concat(tail.map(function (ti) {
      return ti[2]
    }))
  }

  // Forward declarations that will be overwritten parser-extension.js
  p.parseDeclaration = function (lvalue, rvalue) { return [lvalue, rvalue] }
  p.parseBlock = function (statements) { return statements }
  p.parseIf = function (cond, block) { return [cond, block] }
  p.parseRoot = function (statements) { return statements }
  p.parseBinary = function (left, op, right) { return [left, op, right] }
  p.parseInteger = function (integerString) { return integerString }
  p.parseLeftDeclaration = function (decl, name, type) { return [decl, name, type] }
  p.parseFunction = function (args, returnType, block) { return [args, returnType, block] }
  p.parseFor = function (init, cond, after, block) { return [init, cond, after, block] }
  p.parseWhile = function (cond, block) { return [cond, block] }
  p.parseIf = function (cond, block) { return [cond, block] }
  p.parseChain = function (name, tail) { return [name, tail] }
  p.parseAssignment = function (path, op, expr) { return [path, op, expr] }
  p.parseReturn = function (expr) { return [expr] }
  p.parseCall = function (expr) { return [expr] }
  p.parsePath = function (name) { return [name] }

  if (typeof require !== 'undefined') {
    require('./parser-extension')(p)
  }
}// End preamble


start = __ s:statements __ { return p.parseRoot(s) }

statements = statement*

// Statements must be ended by a newline, semicolon, end-of-file, or a
// look-ahead right curly brace for end-of-block.
terminator = _ comment? ("\n" / ";" / eof / &"}") __


block = "{" __ s:statements __ "}" { return p.parseBlock(s) }

statement = s:innerstmt terminator { return s }

innerstmt = decl
          / ctrl
          / assg
          / expr

ctrl = ifctrl
     / whilectrl
     / forctrl
     / returnctrl

ifctrl     = "if" _ c:innerstmt _ b:block { return p.parseIf(c, b) }
whilectrl  = "while" _ c:innerstmt _ b:block { return p.parseWhile(c, b) }
forctrl    = "for" _ i:innerstmt? _ ";" _ c:innerstmt? _ ";" _ a:innerstmt? _ b:block { return p.parseFor(i, c, a, b) }
returnctrl = "return" e:(_ e:expr)? { return p.parseReturn(e ? e[1] : null) }

// Declaration via let or var keywords
decl = lvalue:leftdecl rvalue:(_ "=" _ expr)? { return p.parseDeclaration(lvalue, rvalue ? rvalue[3] : false) }
leftdecl = k:("let" / "var") whitespace n:name t:(":" whitespace type)? { return p.parseLeftDeclaration(k, n, t ? t[2] : null) }

assg = path:path _ op:assgop _ e:expr { return p.parseAssignment(path, op, e) }
assgop = "="
       / "+="

// Path assignment of existing variables and their indexes/properties
path = n:name (indexer / property)* { return p.parsePath(n) }
indexer = "[" _ expr _ "]"
property = "." name

expr = binaryexpr

// Binary expressions have highest precedence
binaryexpr = le:unaryexpr _ op:binaryop _ re:binaryexpr { return p.parseBinary(le, op, re) }
           / unaryexpr


unaryexpr = "!" e:groupexpr { return e }
          / groupexpr

groupexpr = "(" e:expr ")" { return e }
          / basicexpr

basicexpr = funcexpr
          / literalexpr
          / chainexpr

chainexpr = n:name t:(indexer / property / call)* { return p.parseChain(n, t) }
call = "(" _ args:(expr _ ("," _ expr _)* )? _ ")" { return p.parseCall(args ? transformArgs(args) : []) }

literalexpr = i:integer { return p.parseInteger(i) }

funcexpr = "func" _ a:args _ rt:ret? _ b:block { return p.parseFunction(a ? a : [], rt, b) }
args     = "(" _ list:arglist? _ ")" { return list }
arglist  = ( h:arg _ t:("," _ arg _)* ) { return [h].concat(t.map(function (ti) { return ti[2] })) }
arg      = n:name _ t:(":" _ type)? { return {name: n, type: (t ? t[2] : null)} }
ret      = "->" whitespace t:type { return t }

// Building blocks

name = [A-Za-z] [A-Za-z0-9_]* { return text() }
type = [A-Z] [A-Za-z0-9_]* { return text() }

integer = "0"
        / ("-"? [1-9] [0-9]*) { return text() }

binaryop  = "+"
          / "+"
          / "-"
          / "*"
          / "/"
          / "%"
          / "=="
          / "||"
          / "<"

__ = (comment / "\n" / whitespace)*

// A comment is a pound sign, followed by anything but a newline,
// followed by a non-consumed newline.
comment = "#" [^\n]* &"\n"

whitespace = " " / "\t"
_ = whitespace*


eof = !.
