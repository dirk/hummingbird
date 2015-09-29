/* Lexer  ------------------------------------------------------------------ */

%lex

StringEscapeSequence    \\[\'\"\\bfnrtv]
StringCharacter         ([^\"\\\n\r]+)|({StringEscapeSequence})
StringLiteral           \"{StringCharacter}\"

%%

[ \t]+                  /* Skip whitespace */
"#".*($|\n)             /* Skip commments */
"func"                  return 'FUNC';
"when"                  return 'WHEN';
"multi"                 return 'MULTI';
"new"                   return 'NEW';
"class"                 return 'CLASS';
"init"                  return 'INIT';
"return"                return 'RETURN';
"let"                   return 'LET';
"var"                   return 'VAR';
"if"                    return 'IF';
\n*"else"\n*            return 'ELSE'; /* Gobble space on either side */
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
"{"\n*                  return '{';
"}"                     return '}';
"="                     return '=';
"+="                    return '+=';
"."                     return '.';
","                     return ',';
"+"                     return '+';
"-"                     return '-';
"*"                     return '*';
"/"                     return '/';
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
  : statements terminated_statement                     { $1.push($2); $$ = $1; }
  | terminated_statement                                { $$ = [$1]; }
  ;

terminated_statement
  : statement terminal                                  { $$ = $1; }
  ;

block
  : '{' block_body '}'                                  { $$ = yy.node1('Block', @1, $2); }
  | '{' '}'                                             { $$ = yy.node1('Block', @1, []); }
  ;

block_body
  : block_statements statement                          { $1.push($2); $$ = $1; }
  | block_statements                                    { $$ = $1; }
  | statement                                           { $$ = [$1]; }
  ;

block_statements
  : block_statements terminated_statement               { $1.push($2); $$ = $1; }
  | terminated_statement                                { $$ = [$1]; }
  ;

statement
  : declaration_statement
  | class_statement
  | function_statement
  | multi_statement
  | init_statement
  | return_statement
  | condition_statement
  | assignment_statement
  | expression_statement
  ;

return_statement
  : RETURN expression
      { $$ = yy.node1('Return', @1, $2);
      }
  ;

function_statement
  : function_statement_declaration when_extension block
      { var f = $1;
        f.when = $2;
        f.block = $3;
        $$ = f;
      }
  | function_statement_declaration block
      { var f = $1;
        f.block = $2;
        $$ = f;
      }
  ;

when_extension
  : WHEN '(' expression ')'                             { $$ = $3; }
  ;

/* Name, argument types, and return type */
function_statement_declaration
  : FUNC WORD function_parameters function_return
      { var f = yy.node3('Function', @1, $3, $4, null);
        f.name = $2;
        $$ = f;
      }
  | FUNC WORD function_parameters
      { var f = yy.node3('Function', @1, $3, null, null);
        f.name = $2;
        $$ = f;
      }
  ;

multi_statement
  : MULTI WORD function_parameters function_return
      { $$ = yy.node3('Multi', @1, $2, $3, $4);
      }
  | MULTI WORD function_parameters
      { $$ = yy.node3('Multi', @1, $2, $3, null);
      }
  ;

init_statement
  : INIT function_parameters block
      { $$ = new yy.AST.Init($2, $3);
      }
  ;

function_declaration
  : function_parameters function_return block
      { $$ = yy.node3('Function', @1, $1, $2, $3);
      }
  | function_parameters block
      { $$ = yy.node3('Function', @1, $1, null, $2);
      }
  ;

function_return
  : '->' type { $$ = $2; }
  ;

condition_statement
  : if_statement
  | WHILE expression block
      { $$ = yy.node2('While', @1, $2, $3);
      }
  | FOR statement ';' statement ';' statement block
      { $$ = yy.node4('For', @1, $2, $4, $6, $7);
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
      { $$ = yy.node4('If', @1, $2, $3, null, null);
      }
  ;

else
  : ELSE block                                          { $$ = $2; }
  ;

else_if
  : ELSE if                                             { $$ = $2; }
  ;

else_if_list
  : else_if_list else_if                                { $1.push($2); $$ = $1; }
  | else_if                                             { $$ = [$1]; }
  ;

declaration_statement
  : declaration_lvalue '=' expression
      { var kind = $1.constructor.name.toLowerCase();
        $$ = yy.node4('Assignment', @1, kind, $1, $2, $3);
      }
  | declaration_lvalue
      { var kind = $1.constructor.name.toLowerCase();
        $$ = yy.node4('Assignment', @1, kind, $1, false, false);
      }
  ;

declaration_lvalue
  : declaration_type identifier ':' type
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
  : CLASS WORD block                                    { $$ = yy.node2('Class', @1, $2, $3); }
  ;

assignment_statement
  : expression assignment_op expression
      { $$ = yy.node4('Assignment', @1, 'path', $1, $2, $3);
      }
  ;

assignment_op
  : '='
  | '+='
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
  : multiplicative_expression '*' postfix_expression    { $$ = yy.binary(@1, $1, $2, $3); }
  | multiplicative_expression '/' postfix_expression    { $$ = yy.binary(@1, $1, $2, $3); }
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
  | postfix_expression_list call                        { $$ = fastConcat($1, $2); }
  | postfix_expression_list indexer                     { $$ = fastConcat($1, $2); }
  | postfix_expression_list property                    { $$ = fastConcat($1, $2); }
  ;

primary_expression
  : '(' expression ')'                                  { $$ = yy.node1('Group', @1, $2); }
  | FUNC function_declaration                           { $$ = $2; }
  | new_expression                                      { $$ = $1; }
  | atom                                                { $$ = $1; }
  ;

new_expression
  : NEW WORD '(' ')'                                    { $$ = yy.node2('New', @1, $2, []); }
  | NEW WORD '(' call_arguments ')'                     { $$ = yy.node2('New', @1, $2, $4); }
  ;

call
  : '(' ')'                                             { $$ = yy.node1('Call', @1, []); }
  | '(' call_arguments ')'                              { $$ = yy.node1('Call', @1, $2); }
  ;

call_arguments
  : call_arguments ',' expression                       { $1.push($3); $$ = $1; }
  | expression                                          { $$ = [$1]; }
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
  : function_parameter_list ',' function_parameter      { $1.push($3); $$ = $1; }
  | function_parameter                                  { $$ = [$1]; }
  ;

function_parameter
  : WORD ':' name_type                                  { $$ = {name: $1, type: $3}; }
  | WORD                                                { $$ = {name: $1, type: null}; }
  ;

type
  : name_type
  | function_type
  ;

name_type
  : WORD
      { $$ = yy.node1('NameType', @1, $1);
      }
  ;

function_type
  : function_parameters function_return
      { $$ = yy.node2('FunctionType', @1, $1, $2);
      }
  ;

atom
  : identifier
  | number_literal
  | string_literal
  | boolean_literal
  ;

identifier
  : WORD   { $$ = yy.node1('Identifier', @1, $1); }
  ;

number_literal
  : NUMBER { $$ = yy.node2('Literal', @1, parseInt($1, 10), 'Integer'); }
  ;

string_literal
  : STRING
      { var s = $1;
        s = s.slice(1, s.length - 1); // Remove the surrounding quotes
        $$ = yy.node2('Literal', @1, s, 'String');
      }
  ;

boolean_literal
  : boolean
      { $$ = yy.node2('Literal', @1, $1, 'Boolean');
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

%%

function fastConcat (left, right) {
  if (!(left instanceof Array)) {
    left = [left]
  }
  if (right instanceof Array) {
    throw new Error("Cannot fastConcat right Array")
  }
  left.push(right)
  return left
}
