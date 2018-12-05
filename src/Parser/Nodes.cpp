#include <iostream>

#include "Nodes.h"

PAssignment::PAssignment(PNode* lhs, PNode *rhs) :
  lhs(lhs),
  rhs(rhs) {
  if (lhs->isCall()) {
    // FIXME: Catch and return a parse error.
    std::cerr << "Cannot assign to result of a call" << std::endl;
    exit(1);
  }
};

PRoot::PRoot(std::vector<PNode*> nodes) : nodes(nodes) { };

PRoot::~PRoot() {
  for (auto node : nodes) {
    delete node;
  }
}

#define printIndent (*output << std::string(indent * 2, ' '))

void PArray::debugPrint(std::ostream* output, int indent) {
  printIndent << "array(" << std::endl;
  for (auto node : nodes) {
    node->debugPrint(output, indent + 1);
  }
  printIndent << ")" << std::endl;
}

void PAssignment::debugPrint(std::ostream* output, int indent) {
  printIndent << "assignment(" << std::endl;
  indent += 1;
  lhs->debugPrint(output, indent);
  printIndent << "=" << std::endl;
  rhs->debugPrint(output, indent);
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PCall::debugPrint(std::ostream* output, int indent) {
  printIndent << "call(" << std::endl;
  indent += 1;
  target->debugPrint(output, indent);
  for (auto argument : arguments) {
    argument->debugPrint(output, indent);
  }
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PIdentifier::debugPrint(std::ostream* output, int indent) {
  printIndent << "identifier(" << value << ")" << std::endl;
}

void PIndexer::debugPrint(std::ostream* output, int indent) {
  printIndent << "indexer(" << std::endl;
  indent += 1;
  receiver->debugPrint(output, indent);
  if (expression) {
    expression->debugPrint(output, indent);
  }
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PInfix::debugPrint(std::ostream* output, int indent) {
  printIndent << "infix(" << std::endl;
  indent += 1;
  lhs->debugPrint(output, indent);
  char opChar = '_';
  switch (op) {
    case PInfixOp::ADD:
      opChar = '+';
      break;
    case PInfixOp::MULTIPLY:
      opChar = '*';
      break;
  }
  printIndent << opChar << std::endl;
  rhs->debugPrint(output, indent);
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PInteger::debugPrint(std::ostream* output, int indent) {
  printIndent << "integer(" << value << ")" << std::endl;
}

void PLet::debugPrint(std::ostream* output, int indent) {
  printIndent << "let(" << std::endl;
  indent += 1;
  printIndent << lhs << std::endl;
  rhs->debugPrint(output, indent);
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PProperty::debugPrint(std::ostream* output, int indent) {
  printIndent << "property(" << std::endl;
  indent += 1;
  receiver->debugPrint(output, indent);
  printIndent << name << std::endl;
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PVar::debugPrint(std::ostream* output, int indent) {
  printIndent << "var(" << std::endl;
  indent += 1;
  printIndent << lhs << "," << std::endl;
  rhs->debugPrint(output, indent);
  indent -= 1;
  printIndent << ")" << std::endl;
}

#define DEBUG_PRINT_IF(TYPE)                        \
  auto node ## TYPE = std::get_if<TYPE>(&node);     \
  if (node ## TYPE) {                               \
    (node ## TYPE)->debugPrint(output, indent + 0); \
  }

void PNode::debugPrint(std::ostream* output, int indent) {
  // printIndent << "node(" << std::endl;
  DEBUG_PRINT_IF(PArray);
  DEBUG_PRINT_IF(PAssignment);
  DEBUG_PRINT_IF(PCall);
  DEBUG_PRINT_IF(PIdentifier);
  DEBUG_PRINT_IF(PIndexer);
  DEBUG_PRINT_IF(PInfix);
  DEBUG_PRINT_IF(PInteger);
  DEBUG_PRINT_IF(PLet);
  DEBUG_PRINT_IF(PProperty);
  DEBUG_PRINT_IF(PVar);
  // printIndent << ")" << std::endl;
}

void PRoot::debugPrint(std::ostream* output, int indent) {
  *output << "root(" << std::endl;
  for (auto node : nodes) {
    node->debugPrint(output, indent + 1);
  }
  *output << ")" << std::endl;
}
