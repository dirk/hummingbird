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

class PAssignment {
public:
  PAssignment(PNode* lhs, PNode *rhs) :
    lhs(lhs),
    rhs(rhs) { };

  void debugPrint(std::ostream* output, int indent);

  PNode* lhs;
  PNode* rhs;
};

class PCall {
public:
  PCall(PNode* target, std::vector<PNode*> arguments) :
    target(target),
    arguments(arguments) { };

  void debugPrint(std::ostream* output, int indent);

  PNode* target;
  std::vector<PNode*> arguments;
};

class PIdentifier {
public:
  PIdentifier(std::string value) : value(std::string(value)) { };

  void debugPrint(std::ostream* output, int indent);

  std::string value;
};

enum struct PInfixOp {
  ADD,
  MULTIPLY,
};

class PInfix {
public:
  PInfix(PNode* lhs, PInfixOp op, PNode* rhs) :
    lhs(lhs),
    op(op),
    rhs(rhs) { };

  void debugPrint(std::ostream* output, int indent);

  PNode* lhs;
  PInfixOp op;
  PNode* rhs;
};

class PIntegerLiteral {
public:
  PIntegerLiteral(long long int value) : value(value) { };

  void debugPrint(std::ostream* output, int indent);

  long long int value;
};

class PProperty {
public:
  PProperty(PNode* receiver, std::string name) :
    receiver(receiver),
    name(name) { };

  void debugPrint(std::ostream* output, int indent);

  PNode* receiver;
  std::string name;
};

typedef struct {} PUnknown;

class PNode {
public:
  PNode(PAssignment assignment) : node(assignment) { };
  PNode(PCall call) : node(call) { };
  PNode(PIdentifier identifier) : node(identifier) { };
  PNode(PInfix infix) : node(infix) { };
  PNode(PIntegerLiteral integerLiteral) : node(integerLiteral) { };
  PNode(PLet let) : node(let) { };
  PNode(PProperty property) : node(property) { };
  PNode(PVar var) : node(var) { };
  PNode() : node((PUnknown){}) { };

  bool isUnknown() {
    return std::holds_alternative<PUnknown>(node);
  }

  void debugPrint(std::ostream* output, int indent);

private:
  std::variant<
    PAssignment,
    PCall,
    PIdentifier,
    PInfix,
    PIntegerLiteral,
    PLet,
    PProperty,
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
  Parser(std::istream* input);

  ~Parser() {
    delete lexer;
  }

  PRoot* parse();

private:
  yyFlexLexer* lexer;

  // Variables for managing look-ahead;
  bool peeking = false;
  token_t currentToken;
  const char* currentText;
  token_t peekedToken;
  const char* peekedText;

  // Actual interface (it is look-ahead-aware).
  token_t next();
  std::string text();
  token_t peek();

  /// Advance the lexer and assert that the token it returns is what
  /// is expected.
  token_t expect(token_t token);

  PRoot* parseRoot();
  PNode* parseStatement(token_t token);
  PNode* parseExpression(token_t token);
  PNode* parseAddition(token_t token);
  PNode* parseMultiplication(token_t token);
  PNode* parseAssignment(token_t token);
  /// Parses chains off the end:
  ///   - Properties (eg. `foo.bar`)
  ///   - Calls (`foo()`)
  PNode* parseChain(token_t token);
  PNode* parseIdentifier(token_t token);
  PNode* parseLiteral(token_t token);
  PNode* parseLet(token_t token);
  PNode* parseVar(token_t token);

  /// Called *within* a call to parse the arguments.
  std::vector<PNode*> parseCallArguments();

  void fatalNodeError(PNode* node);
  void fatalTokenError(token_t t);
};
