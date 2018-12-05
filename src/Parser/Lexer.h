#pragma once

#if ! defined(yyFlexLexerOnce)
#include <FlexLexer.h>
#endif

#include "Parser.h"

class Lexer : yyFlexLexer {
public:
  Lexer(std::istream *input) : yyFlexLexer(input) { };

  virtual yy::parser::symbol_type lex();

private:
  yy::parser::location_type location;
};
