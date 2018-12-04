#include "Parser.h"

PRoot* Parser::parse() {
  return parseRoot();
}

PVar Parser::parseVarStatement(int token) {
  auto var = PVar();
  expect(T_IDENTIFIER);
  expect(T_EQUALS);
  return var;
}

PNode Parser::parseStatement(int token) {
  switch (token) {
    case T_VAR:
      return PNode(parseVarStatement(token));
  }
  fatalTokenError(token);
  return PNode();
}

PRoot* Parser::parseRoot() {
  auto root = new PRoot();

  while (true) {
    auto token = lexer->yylex();
    switch (token) {
      case T_EOF:
        goto eof;
      default:
        auto node = parseStatement(token);
        if (node.isUnknown()) {
          fatalNodeError(node);
        }
        root->nodes.push_back(node);
        break;
    }
  }
  eof:

  return root;
}

token_t Parser::expect(token_t expected) {
  auto token = lexer->yylex();
  if (token != expected) {
    std::cerr
      << "Unexpected token: expected " << expected
      << ", got " << token
      << std::endl;
    exit(1);
  }
  return token;
}

void Parser::fatalNodeError(PNode node) {
  std::cerr << "Unmatched node" << std::endl;
  exit(1);
}

void Parser::fatalTokenError(token_t t) {
  std::cerr
    << "Unrecognized token " << t
    << " at line " << lexer->lineno()
    << std::endl;
  exit(1);
}
