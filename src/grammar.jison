

/* Lexer  ------------------------------------------------------------------ */

%lex
%%

[ \t]+                 /* Skip whitespace */
"func"                  return 'FUNC';
"return"                return 'RETURN';
"let"                   return 'LET';
"var"                   return 'VAR';
"if"                    return 'IF';
"while"                 return 'WHILE';
"for"                   return 'FOR';
[A-Za-z][A-Za-z0-9_]*   return 'WORD';
0|([1-9][0-9]*)         return 'NUMBER';
\n                      return 'NEWLINE';
<<EOF>>                 return 'EOF';
"("                     return '(';
")"                     return ')';
"["                     return '[';
"]"                     return ']';
"{"                     return '{';
"}"                     return '}';
"."                     return '.';
"+"                     return '+';
"-"                     return '-';
"*"                     return '*';
"/"                     return '/';
"="                     return '=';
";"                     return ';';

/lex

/* Grammar ----------------------------------------------------------------- */

%start root
%%

root
  : statements { return $1; }
  ;

statements
  : statements terminated_statement                     { $$ = $1.concat([$2]); }
  | terminated_statement                                { $$ = [$1]; }
  ;

terminated_statement
  : statement terminal                                  { $$ = $1; }
  ;

block
  : '{' '}'                                             { $$ = []; }
  | '{' block_statements '}'                            { $$ = $2; }
  ;

block_statements
  : statement                                           { $$ = [$1]; }
  | block_statements terminal statement                 { $$ = $1.concat([$3]); }
  ;

statement
  : declaration_statement
  | function_statement
  | return_statement
  | condition_statement
  | assignment_statement
  | expression_statement
  ;

return_statement
  : RETURN expression                                   { $$ = {'return': $2}; }
  ;

function_statement
  : FUNC identifier function_parameters block           { $$ = {name: $2, params: $3, block: $4}; }
  ;

condition_statement
  : condition_type expression block                     { $$ = {type: $1, cond: $2, block: $3}; }
  | FOR expression ';' expression ';' expression block  { $$ = {type: 'for', init: $2, cond: $4, after: $6, block: $7} }
  ;

condition_type
  : IF                                                  { $$ = 'if'; }
  | WHILE                                               { $$ = 'while'; }
  ;

declaration_statement
  : declaration_type identifier                         { $$ = {decl: $1, name: $2, val: null}; }
  | declaration_type identifier '=' expression          { $$ = {decl: $1, name: $2, val: $4}; }
  ;

declaration_type
  : LET                                                 { $$ = 'let'; }
  | VAR                                                 { $$ = 'var'; }
  ;

assignment_statement
  : expression '=' expression                           { $$ = {lhs: $1, rhs: $3}; }
  ;

expression_statement
  : expression
  ;

expression
  : multiplicative_expression
  ;

/* Highest precendence */
multiplicative_expression
  : multiplicative_expression '*' additive_expression   { $$ = {lhs: $1, rhs: $3}; }
  | multiplicative_expression '/' additive_expression   { $$ = {lhs: $1, rhs: $3}; }
  | additive_expression                                 { $$ = $1; }
  ;

additive_expression
  : postfix_expression                                  { $$ = $1; }
  | additive_expression '+' postfix_expression          { $$ = {lhs: $1, rhs: $3}; }
  | additive_expression '-' postfix_expression          { $$ = {lhs: $1, rhs: $3}; }
  ;

postfix_expression
  : primary_expression                                  { $$ = $1; }
  | postfix_expression call                             { $$ = [].concat($1, {call:     $2}); }
  | postfix_expression '[' expression ']'               { $$ = [].concat($1, {indexer:  $3}); }
  | postfix_expression '.' identifier                   { $$ = [].concat($1, {property: $3}); }
  ;

primary_expression
  : '(' expression ')'                                  { $$ = $2; }
  | FUNC function_parameters block                      { $$ = {params: $2, block: $3}; }
  | atom                                                { $$ = $1; }
  ;

call
  : '(' ')'                                             { $$ = []; }
  | '(' call_arguments ')'                              { $$ = $2; }
  ;

call_arguments
  : expression                                          { $$ = [$1]; }
  | call_arguments ',' expression                       { $$ = $1.concat([$3]); }
  ;

function_parameters
  : '(' ')'                                             { $$ = []; }
  | '(' function_parameter_list ')'                     { $$ = $1; }
  ;

function_parameter_list
  : function_parameter                                  { $$ = [$1]; }
  | function_parameter_list ',' function_paramete       { $$ = $1.concat([$3]) }
  ;

function_parameter
  : identifier
  ;

atom
  : identifier
  | number
  ;

identifier
  : WORD
  ;

number
  : NUMBER
  ;

terminal
  : NEWLINE
  | EOF
  ;
