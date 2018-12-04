#pragma once

#include <istream>
#include <vector>

#include "Lexer.h"

class PNode {};

class PRoot : PNode
{
private:
  std::vector<PNode> nodes;
};

class Parser
{
public:
  Parser(std::istream *input) : lexer(new yyFlexLexer(input)) { };

  ~Parser() {
    delete lexer;
  }

  PRoot parse();

private:
  yyFlexLexer *lexer;

  PRoot parseRoot();
};
