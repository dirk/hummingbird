/* Lexer  ------------------------------------------------------------------ */

%lex

StringEscapeSequence    \\[\'\"\\bfnrtv]
StringCharacter         ([^\"\\\n\r]+)|({StringEscapeSequence})
StringLiteral           \"{StringCharacter}\"

%%

[ \t]+                  /* Skip whitespace */
"#".*($|\r\n|\r|\n)     /* Skip commments */
"func"                  return 'FUNC';
"new"                   return 'NEW';
"class"                 return 'CLASS';
"init"                  return 'INIT';
"return"                return 'RETURN';
"let"                   return 'LET';
"var"                   return 'VAR';
"if"                    return 'IF';
\s*"else"\s*            return 'ELSE'; /* Gobble space on either side */
"while"                 return 'WHILE';
"for"                   return 'FOR';
"null"                  return 'NULL';
"true"                  return 'TRUE';
"false"                 return 'FALSE';
[A-Za-z][A-Za-z0-9_]*   return 'WORD';
0|([1-9][0-9]*)         return 'NUMBER';
{StringLiteral}         return 'STRING';
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
","                     return ',';
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
  : '{' block_body '}'                                  { $$ = yy.node('Block', @1, $2); }
  | '{' '}'                                             { $$ = yy.node('Block', @1, []); }
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
  | class_statement
  | function_statement
  | init_statement
  | return_statement
  | condition_statement
  | assignment_statement
  | expression_statement
  ;

return_statement
  : RETURN expression
      { $$ = yy.node('Return', @1, $2); }
  ;

function_statement
  : FUNC WORD function_declaration
     { var f = $3;
       f.name = $2;
       $$ = f;
     }
  ;

init_statement
  : INIT function_parameters block
      { $$ = new yy.AST.Init($2, $3);
      }
  ;

function_declaration
  : function_parameters function_return block
      { var f = yy.node('Function', @1, $1, $2, $3);
        $$ = f;
      }
  | function_parameters block
      { var f = yy.node('Function', @1, $1, null, $2);
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
      { $$ = yy.node('For', @1, $2, $4, $6, $7);
      }
  ;

if_statement
  : if else_if_list else
      { $$ = yy.extendIf($1, $2, $3);
      }
  | if else_if_list
      { $$ = yy.extendIf($1, $2);
      }
  | if else
      { $$ = yy.extendIf($1, null, $2);
      }
  | if
      { $$ = $1;
      }
  ;

/* Fundamental if structure */
if
  : IF expression block
      { $$ = new yy.AST.If($2, $3, null, null);
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
        $$ = yy.node('Assignment', @1, kind, $1, $2, $3);
      }
  | declaration_lvalue
      { var kind = $1.constructor.name.toLowerCase();
        $$ = yy.node('Assignment', @1, kind, $1, false, null);
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

class_statement
  : CLASS WORD block                                    { $$ = yy.node('Class', @1, $2, $3); }
  ;

assignment_statement
  : expression '=' expression
      { $$ = yy.node('Assignment', @1, 'path', $1, '=', $3);
      }
  ;

expression_statement
  : expression
  ;

expression
  : logical_expression
  ;

logical_expression
  : logical_expression '||' comparitive_expression      { $$ = yy.binary(@1, $1, $2, $3); }
  | logical_expression '&&' comparitive_expression      { $$ = yy.binary(@1, $1, $2, $3); }
  | comparitive_expression                              { $$ = $1; }
  ;

comparitive_expression
  : comparitive_expression '<' equality_expression      { $$ = yy.binary(@1, $1, $2, $3); }
  | comparitive_expression '>' equality_expression      { $$ = yy.binary(@1, $1, $2, $3); }
  | equality_expression                                 { $$ = $1; }
  ;

equality_expression
  : equality_expression '==' additive_expression        { $$ = yy.binary(@1, $1, $2, $3); }
  | equality_expression '!=' additive_expression        { $$ = yy.binary(@1, $1, $2, $3); }
  | additive_expression                                 { $$ = $1; }
  ;

additive_expression
  : additive_expression '+' multiplicative_expression   { $$ = yy.binary(@1, $1, $2, $3); }
  | additive_expression '-' multiplicative_expression   { $$ = yy.binary(@1, $1, $2, $3); }
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
  | new_expression                                      { $$ = $1; }
  | atom                                                { $$ = $1; }
  ;

new_expression
  : NEW WORD '(' ')'                                    { $$ = yy.node('New', @1, $2, []); }
  | NEW WORD '(' call_arguments ')'                     { $$ = yy.node('New', @1, $2, $4); }
  ;

call
  : '(' ')'                                             { $$ = yy.node('Call', @1, []); }
  | '(' call_arguments ')'                              { $$ = yy.node('Call', @1, $2); }
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
  | number_literal
  | string_literal
  | boolean_literal
  ;

identifier
  : WORD   { $$ = yy.node('Identifier', @1, $1); }
  ;

number_literal
  : NUMBER { $$ = yy.node('Literal', @1, parseInt($1, 10), 'Integer'); }
  ;

string_literal
  : STRING
      { var s = $1;
        s = s.slice(1, s.length - 1); // Remove the surrounding quotes
        $$ = new yy.AST.Literal(s, 'String');
      }
  ;

boolean_literal
  : boolean
      { $$ = yy.node('Literal', @1, $1, 'Boolean');
      }
  ;

boolean
  : TRUE
  | FALSE
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
