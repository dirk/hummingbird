#include "Driver.h"

#include "Lexer.h"
#include "Parser.h"

Driver::Driver() { }

PRoot* Driver::parse(std::istream* input) {
  auto lexer = Lexer(input);
  auto parser = yy::parser(this, &lexer);
  parser.parse();
  return root;
}

void Driver::setRoot(PRoot* root) {
  this->root = root;
}
