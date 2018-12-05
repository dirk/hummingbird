%require "3.2.2"
%skeleton "lalr1.cc"
%debug
%no-lines

%code requires {
  #include "Nodes.h"

  class Driver;
  class Lexer;
}

%defines
%define api.token.constructor
%define api.value.type variant
%define parse.assert
%define parse.error verbose
%locations

%parse-param { Driver* driver }
%parse-param { Lexer* lexer }

%code {
  #include "Driver.h"
  #include "Lexer.h"

  #undef yylex
  #define yylex lexer->lex
}

%token EOF_ 0 "end of file"
%token ABSTRACT
%token CLASS
%token COLON
%token COMMA
%token BRACE_LEFT
%token BRACE_RIGHT
%token DOT
%token EQUALS
%token <std::string> IDENTIFIER
%token <long long int> INTEGER
%token LET
%token LESS_THAN
%token MIXIN
%token PAREN_LEFT
%token PAREN_RIGHT
%token PLUS
%token REAL
%token SQUARE_LEFT
%token SQUARE_RIGHT
%token STAR
%token STRING
%token TERMINAL
%token UNRECOGNIZED
%token VAR

%left EQUALS
%left PLUS
%left STAR

%type <PNode*> array
%type <PNode*> assignment
%type <PNode*> atom
%type <PNode*> postfix
%type <PNode*> postfix_indexer
%type <PNode*> postfix_indexer_expression
%type <PNode*> expression
%type <PNode*> identifier
%type <PNode*> infix
%type <PNode*> let
%type <PNode*> literal
%type <PNode*> statement
%type <PNode*> var

%type <std::vector<PNode*>> array_arguments
%type <std::vector<PNode*>> call_arguments
%type <std::vector<PNode*>> postfix_call
%type <std::vector<PNode*>> statements

%type <std::string> postfix_property

%start program

%code {
  std::vector<PNode*> push_front(std::vector<PNode*> vector, PNode* node) {
    vector.insert(vector.begin(), node);
    return vector;
  }
}

%%

program: statements { driver->setRoot(new PRoot($1)); };

statements: statement statements { $$ = push_front($2, $1); };
statements: statement { $$ = {$1}; };

statement:
    expression terminals { $$ = $1; }
  | let terminals        { $$ = $1; }
  | var terminals        { $$ = $1; };

let: LET IDENTIFIER EQUALS expression { $$ = new PNode(PLet($2, $4)); };
var: VAR IDENTIFIER EQUALS expression { $$ = new PNode(PVar($2, $4)); };

expression:
    array
  | infix;

array:
    SQUARE_LEFT array_arguments SQUARE_RIGHT       { $$ = new PNode(PArray($2)); }
  | SQUARE_LEFT array_arguments COMMA SQUARE_RIGHT { $$ = new PNode(PArray($2)); };

array_arguments:
    expression COMMA array_arguments { $$ = push_front($3, $1); }
  | expression { $$ = {$1}; }
  | { $$ = {}; };

infix: assignment infix_op infix;
infix: assignment;

infix_op: PLUS | STAR;

assignment: postfix EQUALS expression { $$ = new PNode(PAssignment($1, $3)); };
assignment: postfix;

postfix: postfix postfix_call { $$ = new PNode(PCall($1, $2)); };
postfix: postfix postfix_property { $$ = new PNode(PProperty($1, $2)); };
postfix: postfix postfix_indexer { $$ = new PNode(PIndexer($1, $2)); };
postfix: atom;

postfix_call: PAREN_LEFT call_arguments PAREN_RIGHT { $$ = $2; };
postfix_property: DOT IDENTIFIER { $$ = $2; };
postfix_indexer: SQUARE_LEFT postfix_indexer_expression SQUARE_RIGHT { $$ = $2; };
postfix_indexer_expression: expression;
postfix_indexer_expression: { $$ = nullptr; };

call_arguments: expression COMMA call_arguments { $$ = push_front($3, $1); };
call_arguments: expression { $$ = {$1}; };
call_arguments: { $$ = {}; };

atom: identifier | literal;

identifier: IDENTIFIER { $$ = new PNode(PIdentifier($1)); };

literal: INTEGER { $$ = new PNode(PInteger($1)); };

terminals: terminals TERMINAL;
terminals: TERMINAL;

%%

void yy::parser::error(const location_type& location, const std::string &err_message) {
  std::cerr << "Error: " << err_message << " at " << location << std::endl;
}
