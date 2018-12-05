#pragma once

#include <istream>
#include <string>
#include <variant>
#include <vector>

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
  PAssignment(PNode* lhs, PNode *rhs);

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

class PIndexer {
public:
  PIndexer(PNode* receiver, PNode* expression) : receiver(receiver), expression(expression) { };

  void debugPrint(std::ostream* output, int indent);

  PNode* receiver;
  PNode* expression;
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

class PInteger {
public:
  PInteger(long long int value) : value(value) { };

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
  PNode(PIndexer indexer) : node(indexer) { };
  PNode(PInfix infix) : node(infix) { };
  PNode(PInteger integerLiteral) : node(integerLiteral) { };
  PNode(PLet let) : node(let) { };
  PNode(PProperty property) : node(property) { };
  PNode(PVar var) : node(var) { };
  PNode() : node((PUnknown){}) { };

  bool isUnknown() { return std::holds_alternative<PUnknown>(node); }
  bool isCall() { return std::holds_alternative<PCall>(node); }

  void debugPrint(std::ostream* output, int indent);

private:
  std::variant<
    PAssignment,
    PCall,
    PIndexer,
    PIdentifier,
    PInfix,
    PInteger,
    PLet,
    PProperty,
    PVar,
    PUnknown
  > node;
};

class PRoot {
public:
  PRoot(std::vector<PNode*> nodes);

  ~PRoot();

  void debugPrint(std::ostream* output, int indent);

  std::vector<PNode*> nodes;
};
