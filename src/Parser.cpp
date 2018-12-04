#include "Parser.h"

PRoot Parser::parse() {
  return parseRoot();
}

PRoot Parser::parseRoot() {
  auto root = PRoot();

  while (true) {
    auto token = lexer->yylex();
    if (token == 0) {
      break;
    }

    std::cout << token << std::endl;
  }

  return root;
}
