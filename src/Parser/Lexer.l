%{
#include <string>

#include "Lexer.h"

#undef YY_DECL
#define YY_DECL yy::parser::symbol_type Lexer::lex()

#define yyterminate() return parser::make_EOF_(location)

#define YY_USER_ACTION location.step(); location.columns(yyleng);

using parser = yy::parser;
%}

%option c++
%option debug
%option nodefault
%option noline
%option noyywrap
%option yyclass="Lexer"
%option yylineno

newline     [\n]
non_newline [^\n]

one_whitespace [ \t\r]

integer     -?0|([1-9][0-9]*)
fractional  \.[0-9]+
identifier  [A-Za-z][A-Za-z0-9_]*

single_line_comment "//"{non_newline}*

%x MULTI_LINE_COMMENT

%%
%{
  location.step();
%}

{integer}{fractional}       { return parser::make_REAL(location); }
{integer}                   { return parser::make_INTEGER(YYText(), location); }

"let"{one_whitespace}       { return parser::make_LET(location); }
"var"{one_whitespace}       { return parser::make_VAR(location); }

"abstract"{one_whitespace}  { return parser::make_ABSTRACT(location); }
"class"{one_whitespace}     { return parser::make_CLASS(location); }
"mixin"{one_whitespace}     { return parser::make_MIXIN(location); }

{identifier}                { return parser::make_IDENTIFIER(YYText(), location); }

"{"                         { return parser::make_BRACE_LEFT(location); }
"}"                         { return parser::make_BRACE_RIGHT(location); }
"("                         { return parser::make_PAREN_LEFT(location); }
")"                         { return parser::make_PAREN_RIGHT(location); }
":"                         { return parser::make_COLON(location); }

"<"                         { return parser::make_LESS_THAN(location); }
"+"                         { return parser::make_PLUS(location); }
"*"                         { return parser::make_STAR(location); }
"="                         { return parser::make_EQUALS(location); }
"."                         { return parser::make_DOT(location); }
","                         { return parser::make_COMMA(location); }

"\""[^"]*"\""               { return parser::make_STRING(location); }

[;\n]                       { location.lines(); return parser::make_TERMINAL(location); }

"//"{non_newline}* /* Single-line comment */

"/*" BEGIN(MULTI_LINE_COMMENT);

<MULTI_LINE_COMMENT>"*/" BEGIN(INITIAL);
<MULTI_LINE_COMMENT>[^*\n\r]+
<MULTI_LINE_COMMENT>"*"

{one_whitespace}+ /* Whitespace */
