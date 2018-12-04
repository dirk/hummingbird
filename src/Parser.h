#pragma once

#include <istream>
#include <string>
#include <variant>
#include <vector>

#include "Lexer.h"

// Forward declaration because node types will contain nodes.
class PNode;

class PLet {
public:
  PLet(std::string lhs, PNode* rhs) : lhs(lhs), rhs(rhs) { };

  void debugPrint(std::ostream* output, int indent);

  std::string lhs;
  PNode* rhs;
};

class PVar {
public:
  PVar(std::string lhs, PNode* rhs) : lhs(lhs), rhs(rhs) { };

  void debugPrint(std::ostream* output, int indent);

  std::string lhs;
  PNode* rhs;
};

class PIntegerLiteral {
public:
  PIntegerLiteral(long long int value) : value(value) { };

  void debugPrint(std::ostream* output, int indent);

  long long int value;
};

typedef struct {} PUnknown;

class PNode {
public:
  PNode(PIntegerLiteral integerLiteral) : node(integerLiteral) { };
  PNode(PLet let) : node(let) { };
  PNode(PVar var) : node(var) { };
  PNode() : node((PUnknown){}) { };

  bool isUnknown() {
    return std::holds_alternative<PUnknown>(node);
  }

  void debugPrint(std::ostream* output, int indent);

private:
  std::variant<
    PIntegerLiteral,
    PLet,
    PVar,
    PUnknown
  > node;
};

class PRoot {
public:
  void debugPrint(std::ostream* output, int indent);

  std::vector<PNode*> nodes;
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
  PNode* parseStatement(token_t token);
  PNode* parseExpression(token_t token);
  PLet parseLet(token_t token);
  PVar parseVar(token_t token);

  token_t expect(token_t token);

  void fatalNodeError(PNode* node);
  void fatalTokenError(token_t t);
};
