#pragma once

#if ! defined(yyFlexLexerOnce)
#include <FlexLexer.h>
#endif

typedef int token_t;

enum Token {
  T_EOF = 0,
  T_ABSTRACT,
  T_CLASS,
  T_COLON,
  T_COMMA,
  T_BRACE_LEFT,
  T_BRACE_RIGHT,
  T_DOT,
  T_EQUALS,
  T_IDENTIFIER,
  T_INTEGER,
  T_LET,
  T_LESS_THAN,
  T_MIXIN,
  T_PAREN_LEFT,
  T_PAREN_RIGHT,
  T_PLUS,
  T_REAL,
  T_STAR,
  T_STRING,
  T_VAR,
  T_UNRECOGNIZED,
};
