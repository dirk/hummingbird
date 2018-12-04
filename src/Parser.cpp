#include "Parser.h"

PRoot* Parser::parse() {
  return parseRoot();
}

PNode* Parser::parseStatement(int token) {
  switch (token) {
    case T_VAR:
      return new PNode(parseVar(token));
    default:
      return parseExpression(token);
  }
}

PNode* Parser::parseExpression(int token) {
  switch (token) {
    case T_INTEGER:
      long long int value = std::stoll(lexer->YYText());
      return new PNode(PIntegerLiteral(value));
  }
  fatalTokenError(token);
  return new PNode();
}

PVar Parser::parseVar(int token) {
  expect(T_IDENTIFIER);
  auto lhs = std::string(lexer->YYText());
  expect(T_EQUALS);
  auto rhs = parseExpression(lexer->yylex());
  return PVar(lhs, rhs);
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
        if (node->isUnknown()) {
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
      << " at line " << lexer->lineno()
      << std::endl;
    exit(1);
  }
  return token;
}

void Parser::fatalNodeError(PNode* node) {
  std::cerr
    << "Unmatched node"
    << " at line " << lexer->lineno()
    << std::endl;
  exit(1);
}

void Parser::fatalTokenError(token_t t) {
  std::cerr
    << "Unrecognized token " << t
    << " at line " << lexer->lineno()
    << std::endl;
  exit(1);
}

#define printIndent (*output << std::string(indent, ' '))

void PIntegerLiteral::debugPrint(std::ostream* output, int indent) {
  printIndent << "integerLiteral(" << value << ")" << std::endl;
}

void PVar::debugPrint(std::ostream* output, int indent) {
  printIndent << "var(" << std::endl;
  indent += 1;
  printIndent << lhs << "," << std::endl;
  rhs->debugPrint(output, indent);
  indent -= 1;
  printIndent << ")" << std::endl;
}

void PNode::debugPrint(std::ostream* output, int indent) {
  printIndent << "node(" << std::endl;
  auto integerLiteral = std::get_if<PIntegerLiteral>(&node);
  if (integerLiteral) {
    integerLiteral->debugPrint(output, indent + 1);
  }
  auto var = std::get_if<PVar>(&node);
  if (var) {
    var->debugPrint(output, indent + 1);
  }
  printIndent << ")" << std::endl;
}

void PRoot::debugPrint(std::ostream* output, int indent) {
  *output << "root(" << std::endl;
  for (auto node : nodes) {
    node->debugPrint(output, indent + 1);
  }
  *output << ")" << std::endl;
}
