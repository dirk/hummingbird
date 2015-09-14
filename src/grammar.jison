/* Lexer  ------------------------------------------------------------------ */

%lex
%%

[ \t]+                  /* Skip whitespace */
"#".*($|\r\n|\r|\n)     /* Skip commments */
"func"                  return 'FUNC';
"class"                 return 'CLASS';
"return"                return 'RETURN';
"let"                   return 'LET';
"var"                   return 'VAR';
"if"                    return 'IF';
"else"                  return 'ELSE';
"while"                 return 'WHILE';
"for"                   return 'FOR';
"null"                  return 'NULL';
[A-Za-z][A-Za-z0-9_]*   return 'WORD';
0|([1-9][0-9]*)         return 'NUMBER';
\n                      return 'NEWLINE';
"->"                    return '->';
"=="                    return '==';
"!="                    return '!=';
"||"                    return '||';
"&&"                    return '&&';
"<"                     return '<';
">"                     return '>';
"("                     return '(';
")"                     return ')';
"["                     return '[';
"]"                     return ']';
"{"\s*                  return '{';
"}"                     return '}';
"."                     return '.';
"+"                     return '+';
"-"                     return '-';
"*"                     return '*';
"/"                     return '/';
"="                     return '=';
";"                     return ';';
":"                     return ':';
<<EOF>>                 return 'EOF';
.                       return 'ERROR';

/lex

/* Grammar ----------------------------------------------------------------- */

%start root
%%

root
  : root_body { return new yy.AST.Root($1); }
  ;

root_body
  : terminal statements { $$ = $2; }
  | statements          { $$ = $1; }
  ;

statements
  : statements terminated_statement                     { $$ = $1.concat([$2]); }
  | terminated_statement                                { $$ = [$1]; }
  ;

terminated_statement
  : statement terminal                                  { $$ = $1; }
  ;

block
  : '{' block_body '}'                                  { $$ = new yy.AST.Block($2); }
  | '{' '}'                                             { $$ = new yy.AST.Block([]); }
  ;

block_body
  : block_statements statement                          { $$ = $1.concat($2); }
  | block_statements                                    { $$ = $1; }
  | statement                                           { $$ = [$1]; }
  ;

block_statements
  : block_statements terminated_statement               { $$ = $1.concat([$2]); }
  | terminated_statement                                { $$ = [$1]; }
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
  : RETURN expression
      { $$ = new yy.AST.Return($2); }
  ;

function_statement
  : FUNC identifier function_declaration
     {  var f = $3;
        f.name = $2;
        $$ = f;
     }
  ;

function_declaration
  : function_parameters function_return block
      { var f = new yy.AST.Function($1, $2, $3);
        $$ = f;
      }
  | function_parameters block
      { var f = new yy.AST.Function($1, null, $2);
        $$ = f;
      }
  ;

function_return
  : '->' type { $$ = $2; }
  ;

condition_statement
  : if_statement
  | WHILE expression block                    
      { $$ = new yy.AST.While($2, $3);
      }
  | FOR statement ';' statement ';' statement block 
      { $$ = new yy.AST.For($2, $4, $6, $7);
      }
  ;

if_statement
  : if else_if_list else
      { var i = $1;
        i.elseIfs = $2;
        i.elseBlock = $3;
        $$ = i;
      }
  | if else_if_list
      { var i = $1;
        i.elseIfs = $2;
        $$ = i;
      }
  | if else
      { var i = $1;
        i.elseBlock = $3;
        $$ = i;
      }
  | if
      { $$ = $1;
      }
  ;

/* Fundamental if structure */
if
  : IF expression block                    
      { $$ = new yy.AST.If($2, $3, [], null);
      }
  ;

else
  : ELSE block                                          { $$ = $2; }
  ;

else_if
  : ELSE if                                             { $$ = $2; }
  ;

else_if_list
  : else_if_list else_if                                { $$ = $1.concat([$2]); }
  | else_if                                             { $$ = [$1]; }
  ;

declaration_statement
  : declaration_lvalue '=' expression
      { var kind = $1.constructor.name.toLowerCase();
        $$ = new yy.AST.Assignment(kind, $1, $2, $3);
      }
  | declaration_lvalue
      { var kind = $1.constructor.name.toLowerCase();
        $$ = new yy.AST.Assignment(kind, $1, false, null);
      }
  ;

declaration_lvalue
  : declaration_type identifier ':' name_type
      { var Con = $1;
        $$ = new Con($2.name, $4);
      }
  | declaration_type identifier
      { var Con = $1;
        $$ = new Con($2.name, null);
      }
  ;

declaration_type
  : LET                                                 { $$ = yy.AST.Let; }
  | VAR                                                 { $$ = yy.AST.Var; }
  ;

assignment_statement
  : expression '=' expression
      { $$ = new yy.AST.Assignment('path', $1, '=', $3); }
  ;

expression_statement
  : expression
  ;

expression
  : logical_expression
  ;

logical_expression
  : logical_expression '||' comparitive_expression      { $$ = yy.binary($1, $2, $3); }
  | logical_expression '&&' comparitive_expression      { $$ = yy.binary($1, $2, $3); }
  | comparitive_expression                              { $$ = $1; }
  ;

comparitive_expression
  : comparitive_expression '<' equality_expression      { $$ = yy.binary($1, $2, $3); }
  | comparitive_expression '>' equality_expression      { $$ = yy.binary($1, $2, $3); }
  | equality_expression                                 { $$ = $1; }
  ;

equality_expression
  : equality_expression '==' additive_expression        { $$ = yy.binary($1, $2, $3); }
  | equality_expression '!=' additive_expression        { $$ = yy.binary($1, $2, $3); }
  | additive_expression                                 { $$ = $1; }
  ;

additive_expression
  : additive_expression '+' multiplicative_expression   { $$ = yy.binary($1, $2, $3); }
  | additive_expression '-' multiplicative_expression   { $$ = yy.binary($1, $2, $3); }
  | multiplicative_expression                           { $$ = $1; }
  ;

/* Highest binary precendence */
multiplicative_expression
  : multiplicative_expression '*' postfix_expression    { $$ = yy.binary($1, $2, $3); }
  | multiplicative_expression '/' postfix_expression    { $$ = yy.binary($1, $2, $3); }
  | postfix_expression                                  { $$ = $1; }
  ;

postfix_expression
  : postfix_expression_list
      { var x = $1;
        if (x instanceof Array) {
          // console.log(x)
          $$ = yy.AST.constructPath(x[0], x.slice(1, x.length));
        } else {
          $$ = x;
        }
      }
  ;

postfix_expression_list
  : primary_expression                                  { $$ = $1; }
  | postfix_expression_list call                        { $$ = [].concat($1, $2); }
  | postfix_expression_list indexer                     { $$ = [].concat($1, $2); }
  | postfix_expression_list property                    { $$ = [].concat($1, $2); }
  ;

primary_expression
  : '(' expression ')'                                  { $$ = $2; }
  | FUNC function_declaration                           { $$ = $2; }
  | atom                                                { $$ = $1; }
  ;

call
  : '(' ')'                                             { $$ = new yy.AST.Call([]); }
  | '(' call_arguments ')'                              { $$ = new yy.AST.Call($2); }
  ;

call_arguments
  : expression                                          { $$ = [$1]; }
  | call_arguments ',' expression                       { $$ = $1.concat([$3]); }
  ;

indexer
  : '[' expression ']'                                  { $$ = new yy.AST.Indexer($2); }
  ;

property
  : '.' identifier                                      { $$ = $2; }
  ;

function_parameters
  : '(' ')'                                             { $$ = []; }
  | '(' function_parameter_list ')'                     { $$ = $2; }
  ;

function_parameter_list
  : function_parameter                                  { $$ = [$1]; }
  | function_parameter_list ',' function_parameter      { $$ = $1.concat([$3]) }
  ;

function_parameter
  : WORD ':' name_type                                  { $$ = {name: $1, type: $3}; }
  ;

type
  : name_type
  ;

name_type
  : WORD   { $$ = new yy.AST.NameType($1); }
  ;

atom
  : identifier
  | number
  ;

identifier
  : WORD   { $$ = new yy.AST.Identifier($1); }
  ;

number
  : NUMBER { $$ = new yy.AST.Literal(parseInt($1, 10), 'Integer'); }
  ;

terminal
  : terminal terminal_token
  | terminal_token
  ;

terminal_token
  : NEWLINE
  | EOF
  | ';'
  ;
