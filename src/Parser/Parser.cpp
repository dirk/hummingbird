#include "Parser.h"

PRoot* Parser::parse() {
  return parseRoot();
}

Parser::Parser(std::istream* input) :
  lexer(new yyFlexLexer(input)) { };

// This is to implement single-depth peeking capability, as that's all we
// need in our grammar.

token_t Parser::next() {
  if (peeking) {
    currentToken = peekedToken;
    currentText = peekedText;
    peeking = false;
  } else {
    currentToken = lexer->yylex();
  }
  return currentToken;
}

std::string Parser::text() {
  if (peeking) {
    return std::string(currentText);
  } else {
    return std::string(lexer->YYText());
  }
}

token_t Parser::peek() {
  if (!peeking) {
    peeking = true;
    peekedToken = lexer->yylex();
    peekedText = lexer->YYText();
  }
  return peekedToken;
}

// We want every parsing function to be inlined.
#define IPN inline PNode*

IPN Parser::parseStatement(int token) {
  switch (token) {
    case T_LET:
      return parseLet(token);
    case T_VAR:
      return parseVar(token);
    default:
      return parseExpression(token);
  }
}

IPN Parser::parseExpression(int token) {
  return parseAddition(token);
}

IPN Parser::parseAddition(int token) {
  auto lhs = parseMultiplication(token);
  auto nextToken = peek();
  if (nextToken == T_PLUS) {
    expect(T_PLUS);
    auto rhs = parseAddition(next());
    return new PNode(PInfix(lhs, PInfixOp::ADD, rhs));
  }
  return lhs;
}

IPN Parser::parseMultiplication(int token) {
  auto lhs = parseAssignment(token);
  auto nextToken = peek();
  if (nextToken == T_STAR) {
    expect(T_STAR);
    auto rhs = parseMultiplication(next());
    return new PNode(PInfix(lhs, PInfixOp::MULTIPLY, rhs));
  }
  return lhs;
}

IPN Parser::parseAssignment(int token) {
  auto lhs = parseChain(token);
  auto nextToken = peek();
  if (nextToken == T_EQUALS) {
    expect(T_EQUALS);
    // Assignment is greedy and will eat as much of the expression as it can.
    auto rhs = parseExpression(next());
    return new PNode(PAssignment(lhs, rhs));
  }
  return lhs;
}

IPN Parser::parseChain(token_t token) {
  auto node = parseLiteral(token);
  while (true) {
    auto nextToken = peek();
    if (nextToken == T_DOT) {
      expect(T_DOT);
      expect(T_IDENTIFIER);
      auto name = text();
      node = new PNode(PProperty(node, name));
    } else if (nextToken == T_PAREN_LEFT) {
      expect(T_PAREN_LEFT);
      auto arguments = parseCallArguments();
      expect(T_PAREN_RIGHT);
      node = new PNode(PCall(node, arguments));
    } else {
      break;
    }
  }
  return node;
}

IPN Parser::parseLiteral(int token) {
  switch (token) {
    case T_INTEGER:
      long long int value = std::stoll(text());
      return new PNode(PIntegerLiteral(value));
  }
  return parseIdentifier(token);
}

IPN Parser::parseIdentifier(int token) {
  if (token == T_IDENTIFIER) {
    return new PNode(PIdentifier(text()));
  }
  fatalTokenError(token);
  return new PNode();
}

IPN Parser::parseLet(int token) {
  expect(T_IDENTIFIER);
  auto lhs = text();
  expect(T_EQUALS);
  auto rhs = parseExpression(next());
  return new PNode(PLet(lhs, rhs));
}

IPN Parser::parseVar(int token) {
  expect(T_IDENTIFIER);
  auto lhs = text();
  expect(T_EQUALS);
  auto rhs = parseExpression(next());
  return new PNode(PVar(lhs, rhs));
}

std::vector<PNode*> Parser::parseCallArguments() {
  std::vector<PNode*> arguments;
  while (true) {
    auto nextToken = peek();
    if (nextToken == T_PAREN_RIGHT) {
      break;
    }
    if (arguments.size() > 0) {
      expect(T_COMMA);
    }
    auto node = parseExpression(next());
    arguments.push_back(node);
  }
  return arguments;
}

PRoot* Parser::parseRoot() {
  auto root = new PRoot();

  while (true) {
    auto token = next();
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
  auto token = next();
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

#define printIndent (*output << std::string(indent * 2, ' '))

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

void PIntegerLiteral::debugPrint(std::ostream* output, int indent) {
  printIndent << "integerLiteral(" << value << ")" << std::endl;
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
  DEBUG_PRINT_IF(PAssignment);
  DEBUG_PRINT_IF(PCall);
  DEBUG_PRINT_IF(PIdentifier);
  DEBUG_PRINT_IF(PInfix);
  DEBUG_PRINT_IF(PIntegerLiteral);
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

PRoot::~PRoot() {
  for (auto node : nodes) {
    delete node;
  }
}
