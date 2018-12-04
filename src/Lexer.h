#pragma once

#if !defined(yyFlexLexerOnce)
#include <FlexLexer.h>
#endif

enum Token {
  T_ABSTRACT = 1,
  T_CLASS,
  T_COLON,
  T_BRACE_LEFT,
  T_BRACE_RIGHT,
  T_DOT_IDENTIFIER,
  T_EQUALS,
  T_IDENTIFIER,
  T_INTEGER,
  T_LET,
  T_LESS_THAN,
  T_MIXIN,
  T_PARENT_LEFT,
  T_PARENT_RIGHT,
  T_PLUS,
  T_REAL,
  T_STRING,
  T_VAR,
  T_UNRECOGNIZED,
};
