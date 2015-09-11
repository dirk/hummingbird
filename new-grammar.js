var util       = require('util'),
    prettyjson = require('prettyjson')

var identity = "$$ = $1;"

var grammar = {
  lex: {
    rules: [
      ["[ \\t]+"              , "/* Skip whitespace */"],
      ["func"                 , "return 'FUNC';"],
      ["return"               , "return 'RETURN';"],
      ["let"                  , "return 'LET';"],
      ["var"                  , "return 'VAR';"],
      ["[A-Za-z][A-Za-z0-9_]*", "return 'WORD';"],
      ["0|([1-9][0-9]*)"      , "return 'NUMBER';"],
      ["$"                    , "return 'EOF';"],
      ["\\n"                  , "return 'NEWLINE';"],
      ["\\("                  , "return '(';"],
      ["\\)"                  , "return ')';"],
      ["\\["                  , "return '[';"],
      ["\\]"                  , "return ']';"],
      ["\\{"                  , "return '{';"],
      ["\\}"                  , "return '}';"],
      ["\\."                  , "return '.';"],
      ["\\+"                  , "return '+';"],
      ["-"                    , "return '-';"],
      ["\\/"                  , "return '/';"],
      ["\\*"                  , "return '*';"],
      [","                    , "return ',';"],
      ["="                    , "return '=';"],
    ]// rules
  },// lex

  start: 'root',

  bnf: {
    root:       [["statements"                                      , "return $1;"]],

    statements: [["statements terminated_statement"                 , "$$ = $1.concat([$2]);"],
                 ["terminated_statement"                            , "$$ = [$1];"]],

    terminated_statement:
                [["statement terminal"                              , identity]],

    block:      [["{ }"                                             , "$$ = [];"],
                 ["{ block_statements }"                            , "$$ = $2;"]],
    
    block_statements:
                [["statement"                                       , "$$ = [$1];"],
                 ["block_statements terminal statement"             , "$$ = $1.concat([$3]);"]],

    statement:  [["declaration_statement"                           , identity],
                 ["function_statement"                              , identity],
                 ["return_statement"                                , identity],
                 ["assignment_statement"                            , identity],
                 ["expression_statement"                            , identity]],

    return_statement:
                [["RETURN expression"                               , "$$ = {'return': $2};"]],

    function_statement:
                [["FUNC identifier function_parameters block"       , "$$ = {name: $2, params: $3, block: $4};"]],

    declaration_statement:
                [["declaration_type identifier"                     , "$$ = {decl: $1, name: $2, val: null};"],
                 ["declaration_type identifier = expression"        , "$$ = {decl: $1, name: $2, val: $4};"]],

    declaration_type:
                [["LET"                                             , "$$ = 'let';"],
                 ["VAR"                                             , "$$ = 'var';"]],

    assignment_statement:
                [["expression = expression"                         , "$$ = {lhs: $1, rhs: $3};"]],

    expression_statement:
                [["expression"                                      , identity]],

    expression: [["multiplicative_expression"                       , identity]],

    multiplicative_expression: // Highest precendence
                [["multiplicative_expression * additive_expression" , "$$ = {lhs: $1, rhs: $3};"],
                 ["multiplicative_expression / additive_expression" , "$$ = {lhs: $1, rhs: $3};"],
                 ["additive_expression"                             , identity]],

    additive_expression:
                [["postfix_expression"                              , identity],
                 ["additive_expression + postfix_expression"        , "$$ = {lhs: $1, rhs: $3};"],
                 ["additive_expression - postfix_expression"        , "$$ = {lhs: $1, rhs: $3};"]],

    postfix_expression:
                [["primary_expression"                                , identity],
                 ["postfix_expression call"                         , "$$ = [].concat($1, {call:     $2});"],
                 ["postfix_expression [ expression ]"               , "$$ = [].concat($1, {indexer:  $3});"],
                 ["postfix_expression . identifier"                 , "$$ = [].concat($1, {property: $3});"]],

    primary_expression:
                [["( expression )"                                  , "$$ = $2;"],
                 ["FUNC function_parameters block"                  , "$$ = {params: $2, block: $3};"],
                 ["atom"                                            , identity]],

    call:       [["( )"                                             , "$$ = [];"],
                 ["( call_arguments )"                              , "$$ = $2;"]],

    call_arguments:
                [["expression"                                      , "$$ = [$1];"],
                 ["call_arguments , expression"                     , "$$ = $1.concat([$3]);"]],

    function_parameters:
                [["( )"                                             , "$$ = [];"],
                 ["( function_parameter_list )"                     , "$$ = $1;"]],

    function_parameter_list:
                [["function_parameter"                              , "$$ = [$1];"],
                 ["function_parameter_list , function_parameter"    , "$$ = $1.concat([$3])"]],

    function_parameter:
                [["identifier"                                      , identity]],

    indexer:    [["[ expression ]"      , "$$ = $2;"]],
    property:   [[". identifier"        , "$$ = $2;"]],

    atom:       [["identifier"          , identity],
                 ["number"              , identity]],

    identifier: [["WORD"                , identity]],
    number:     [["NUMBER"              , identity]],

    terminal:   [["NEWLINE"             , identity],
                 ["EOF"                 , identity]],
  }// bnf
}

var jison = require('jison')

var parser = new jison.Parser(grammar)

function inspect(object) {
  // console.log(util.inspect(object, {depth: 10}))
  console.log(prettyjson.render(object, {}))
}

var lines = [
  "a(b.c)[0].d ",
  "func e(f) { return f * 2 }",
  "g = func () {}",
  "let h = 1",
]

inspect(parser.parse(lines.join("\n")))
