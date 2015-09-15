{// Begin preamble
  var _ = {}

  function transformArgs (args) {
    var head = args[0], tail = args[2]
    return [head].concat(tail.map(function (ti) {
      return ti[2]
    }))
  }

  // Add position information to the node then returns the node. Wrap a node
  // with this in its parse block to add position metadata.
  function pos (node) {
    // console.log(options.file+': L'+line()+' C'+column())
    if (node.setPosition) {
      node.setPosition(options.file, line(), column())
    }
    return node
  }

  // Forward declarations that will be overwritten parser-extension.js
  _.parseImport = function (name, using) { return [name, using] }
  _.parseExport = function (name) { return [name] }
  _.parseDeclaration = function (lvalue, rvalue) { return [lvalue, rvalue] }
  _.parseClass = function (name, block) { return [name, block] }
  _.parseInit = function (args, block) { return [args, block] }
  _.parseBlock = function (statements) { return statements }
  _.parseIf = function (cond, block, elseIfs, elseBlock) { return [cond, block, elseIfs, elseBlock] }
  _.parseRoot = function (statements) { return ['root'].concat(statements) }
  _.parseBinary = function (left, op, right) { return [left, op, right] }
  _.parseInteger = function (integerString) { return integerString }
  _.parseString = function (string) { return string }
  _.parseBoolean = function (boolean) { return boolean }
  _.parseLeftDeclaration = function (decl, name, type) { return [decl, name, type] }
  _.parseNew = function (name, args) { return [name, args] }
  _.parseFunction = function (name, args, returnType, whenCond, block) { return [name, args, returnType, whenCond, block] }
  _.parseFor = function (init, cond, after, block) { return [init, cond, after, block] }
  _.parseWhile = function (cond, block) { return [cond, block] }
  _.parseChain = function (name, tail) { return [name, tail] }
  _.parseAssignment = function (path, op, expr) { return [path, op, expr] }
  _.parseReturn = function (expr) { return [expr] }

  _.parseExpr = function (expr) { return ['expr', expr] }
  _.parseCall = function (args) { return ['callexpr', args] }
  _.parseIndexer = function (base, indexer) { return [base, indexer] }
  _.parseIdentifier = function (name) { return name }

  _.parseLeft = function (name, path) { return [name, path] }
  _.parseLeftIndexer = function (expr) { return expr }
  _.parseLeftProperty = function (name) { return name }

  _.parsePathExpr = function (name, path) { return ['pathexpr', name, path] }
  _.parseIndexer = function (expr) { return ['pathindexer', expr] }

  _.parseNameType = function (name) { return [name] }
  _.parseFunctionType = function (args, ret) { return [args, ret] }
  _.parseMutli = function (name, args, ret) { return [name, args, ret] }

  if (typeof require !== 'undefined') {
    for (key in _) {
      if (_.hasOwnProperty(key)) {
        delete _[key]
      }
    }
    require('./parser-extension')(_)
  }
}// End preamble


start = __ s:statements __ { return pos(_.parseRoot(s)) }

statements = terminated_statement*

terminated_statement = s:statement terminator { return s }

// Statements must be ended by a newline, semicolon, end-of-file, or a
// look-ahead right curly brace for end-of-block.
terminator = _ comment? ("\n" / ";" / eof / &"}") __

block = "{" __ s:statements __ "}" { return pos(_.parseBlock(s)) }

statement
  = modstmt
  / decl
  / ctrl
  / assg
  / multistmt
  / funcstmt
  / expr

modstmt = importstmt
        / exportstmt

importstmt = "import" whitespace "<" i:importpath ">" u:usingpath? { return pos(_.parseImport(i, u)) }
importpath = [A-Za-z0-9-_/.]+ { return text() }
usingpath  = whitespace "using" whitespace n:name e:(_ "," _ name)* {
  return [n].concat((!e) ? [] : e.map(function (a) {
    return a[3]
  }))
}
exportstmt = "export" whitespace n:name   { return pos(_.parseExport(n)) }

ctrl = ifctrl
     / whilectrl
     / forctrl
     / returnctrl

ifctrl = "if" _ c:statement __ b:block ei:(__ elseifcont)* e:(__ elsecont)? {
  ei = ei.map(function (pair) { return pair[1] })
  e  = e ? e[1] : null
  return pos(_.parseIf(c, b, ei, e))
}
// Continuations of the if control with else-ifs
elseifcont = "else" __ "if" _ c:statement __ b:block { return _.parseIf(c, b, null) }
elsecont   = "else" __ b:block { return b }

whilectrl  = "while" _ c:statement _ b:block { return pos(_.parseWhile(c, b)) }
forctrl    = "for" _ i:statement? _ ";" _ c:statement? _ ";" _ a:statement? _ b:block { return pos(_.parseFor(i, c, a, b)) }
returnctrl = "return" e:(_ e:expr)? { return pos(_.parseReturn(e ? e[1] : null)) }

decl = letvardecl
     / classdecl
     / initdecl

classdecl = "class" whitespace n:name _ b:block { return pos(_.parseClass(n, b)) }
initdecl  = "init" _ a:args _ b:block  { return pos(_.parseInit(a, b)) }

// Declaration via let or var keywords
letvardecl = lvalue:leftdecl rvalue:(_ "=" _ expr)? { return pos(_.parseDeclaration(lvalue, rvalue ? rvalue[3] : false)) }
leftdecl = k:("let" / "var") whitespace n:name t:(":" whitespace type)? { return pos(_.parseLeftDeclaration(k, n, t ? t[2] : null)) }

assg = path:leftpath _ op:assgop _ e:expr { return pos(_.parseAssignment(path, op, e)) }
assgop = "="
       / "+="

// Path assignment of existing variables and their indexes/properties
leftpath     = n:identifier path:(leftproperty)* { return _.parseLeft(n, path) }
leftindexer  = "[" _ e:expr _ "]" { return pos(_.parseLeftIndexer(e)) }
leftproperty = "." i:identifier { return pos(_.parseLeftProperty(i)) }

multistmt = "multi" whitespace n:name _ a:args _ r:ret? { return pos(_.parseMulti(n, a, r)) }

expr
  = e:binary_expression                                 { return _.parseExpr(e) }

// Binary expressions have highest precedence
binary_expression
  = le:unary_expression _ op:binaryop _ re:binary_expression
      { return pos(_.parseBinary(le, op, re))
      }
  / unary_expression

unary_expression = postfix_expression

// unary_expression = "!" path:postfix_expression       { return path }
//                  / postfix_expression

postfix_expression
  = e:primary_expression p:postfix*                     { return _.parsePathExpr(e, p) }

primary_expression
  = "(" e:expr ")"                                      { return e }
  / funcexpr
  / newexpr
  / literal
  / identifier

postfix
  = c:call
  / property
  / indexer

call
  = "(" _ args:call_arguments? _ ")"                    { return pos(_.parseCall(args || [])) }

call_arguments
  = head:expr _ tail:("," _ expr _)*
      { return [head].concat(tail.map(function (t) {
          return t[2]
        }))
      }

property
  = "." i:identifier                                    { return i }

indexer
  = "[" _ e:expr _ "]"                                  { return pos(_.parseIndexer(e)) }

identifier
  = n:name                                              { return pos(_.parseIdentifier(n)) }

// chainexpr = n:name t:(indexer / property / call)* { return pos(_.parseChain(n, t)) }

newexpr = "new" whitespace n:name _ "(" _ a:(expr _ ("," _ expr _)* )? _ ")" { return pos(_.parseNew(n, a ? transformArgs(a) : [])) }

funcstmt = "func" whitespace n:name _ a:args _ r:ret? _ w:when? _ b:block { return pos(_.parseFunction(n, a, r, w, b)) }
funcexpr = "func" _ a:args _ r:ret? _ b:block { return pos(_.parseFunction(null, a, r, null, b)) }
args     = "(" _ list:arglist? _ ")" { return (list ? list : []) }
arglist  = ( h:arg _ t:("," _ arg _)* ) { return [h].concat(t.map(function (ti) { return ti[2] })) }
arg      = n:argname _ t:(":" _ type)? _ d:("=" _ literal)? { return {name: n, type: (t ? t[2] : null), def: (d ? d[2] : null)} }
argname  = "_" { return text() }
         / name
ret      = "->" _ t:type { return t }
when     = "when" _ "(" _ e:expr _ ")" { return e }

// Building blocks

name = [A-Za-z] [A-Za-z0-9_]* { return text() }
type = nametype / functype

nametype = [A-Z] [A-Za-z0-9_]* { return _.parseNameType(text()) }
functype = "(" _ args:argtypelist? _ ")" _ "->" _ ret:type { return _.parseFunctionType(args, ret) }
argtypelist = ( h:type _ t:("," _ type _)* ) { return [h].concat(t.map(function (ti) { return ti[2] })) }

// Literals

literal
  = i:integer { return _.parseInteger(i) }
  / s:string  { return s }
  / b:boolean { return b }

integer = "0"
        / ("-"? [1-9] [0-9]*) { return text() }

string = '"' c:stringchar* '"' { return _.parseString(c.join('')) }
// Adapted from: https://github.com/pegjs/pegjs/blob/master/examples/javascript.pegjs
stringchar = unescapedchar
           / "\\" sequence:(
                 '"'
               / "\\"
               / "/"
               / "b" { return "\b"; }
               / "f" { return "\f"; }
               / "n" { return "\n"; }
               / "r" { return "\r"; }
               / "t" { return "\t"; }
               / "u" digits:$(HEXDIGIT HEXDIGIT HEXDIGIT HEXDIGIT) {
                   return String.fromCharCode(parseInt(digits, 16));
                 }
             ) { return sequence }

unescapedchar = [\x20-\x21\x23-\x5B\x5D-\u10FFFF]

boolean = b:("true" / "false") { return _.parseBoolean(b) }

// See RFC 4234, Appendix B (http://tools.ietf.org/html/rfc4627)
HEXDIGIT = [0-9a-f]i

binaryop
  = "+"
  / "+"
  / "-"
  / "*"
  / "/"
  / "%"
  / "=="
  / "||"
  / "<"
  / ">"

__ = (comment / "\n" / whitespace)*
_  = whitespace*

// A comment is a pound sign, followed by anything but a newline,
// followed by a non-consumed newline.
comment = "#" (!"\n" .)* &"\n"

whitespace = " " / "\t"

eof = !.
