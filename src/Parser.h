#pragma once

#include <istream>
#include <string>
#include <variant>
#include <vector>

#include "Lexer.h"

class PVar {
public:
  PVar() { };

  ~PVar() {
    if (identifier) {
      delete identifier;
    }
  }

  std::string* identifier = nullptr;
};

typedef struct {} PUnknown;

class PNode {
public:
  PNode(PVar var) : node(var) { };
  PNode() : node((PUnknown){}) { };

  bool isUnknown() {
    return std::holds_alternative<PUnknown>(node);
  }

private:
  std::variant<
    PVar,
    PUnknown
  > node;
};

class PRoot {
public:
  std::vector<PNode> nodes;
};

class Parser {
public:
  Parser(std::istream* input) : lexer(new yyFlexLexer(input)) { };

  ~Parser() {
    delete lexer;
  }

  PRoot* parse();

private:
  yyFlexLexer* lexer;

  PRoot* parseRoot();
  PNode parseStatement(token_t token);
  PVar parseVarStatement(token_t token);

  token_t expect(token_t token);

  void fatalNodeError(PNode node);
  void fatalTokenError(token_t t);
};
