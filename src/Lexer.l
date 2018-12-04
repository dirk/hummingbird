%{
#include <string>

#include "Lexer.h"

#define yyterminate() return T_EOF
%}

%option c++
%option debug
%option nodefault
%option noyywrap
%option yylineno

newline     [\n\r]
non_newline [^\n\r]

one_whitespace [ \t\n\r]

integer     -?0|([1-9][0-9]*)
fractional  \.[0-9]+
identifier  [A-Za-z][A-Za-z0-9_]*

single_line_comment "//"{non_newline}*

%x MULTI_LINE_COMMENT

%%

{integer}{fractional}       { return T_REAL; }
{integer}                   { return T_INTEGER; }

"let"{one_whitespace}       { return T_LET; }
"var"{one_whitespace}       { return T_VAR; }

"abstract"{one_whitespace}  { return T_ABSTRACT; }
"class"{one_whitespace}     { return T_CLASS; }
"mixin"{one_whitespace}     { return T_MIXIN; }

"{"                         { return T_BRACE_LEFT; }
"}"                         { return T_BRACE_RIGHT; }
"("                         { return T_PARENT_LEFT; }
")"                         { return T_PARENT_RIGHT; }
":"                         { return T_COLON; }

"<"                         { return T_LESS_THAN; }
"+"                         { return T_PLUS; }
"*"                         { return T_STAR; }
"="                         { return T_EQUALS; }

"."{identifier}             { return T_DOT_IDENTIFIER; }
{identifier}                { return T_IDENTIFIER; }

"\""[^"]*"\""               { return T_STRING; }

"//"{non_newline}* /* Single-line comment */

"/*" BEGIN(MULTI_LINE_COMMENT);

<MULTI_LINE_COMMENT>"*/" BEGIN(INITIAL);
<MULTI_LINE_COMMENT>[^*\n\r]+
<MULTI_LINE_COMMENT>"*"

{one_whitespace}+ /* Whitespace */
