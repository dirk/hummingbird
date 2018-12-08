module parser.new_parser;

import std.algorithm.mutation : remove;
import std.array : replaceInPlace;
import std.format : format;
import std.stdio : writeln;
import std.variant : Algebraic;

import ast.ast;
import parser.lexer;

class Parser {
  TokenStream input;

  this(string input) {
    this.input = new TokenStream(new StringStream(input));
  }

  this(TokenStream input) {
    this.input = input;
  }

  Program parseProgram() {
    Node[] statements;
    while (!input.peekEof()) {
      statements ~= parseStatement(true);
    }
    return new Program(statements);
  }

  Node parseStatement(bool allowEofAsTerminal = false) {
    Node node;

    auto next = input.peek();
    if (next.type == TokenType.KEYWORD) {
      if (next.stringValue == "var") {
        node = parseVar();
        goto terminal;
      }
    }
    node = parseExpression();

  terminal:
    auto token = input.read();
    auto const isTerminal = (token.type == TokenType.TERMINAL);
    auto const isEof = (token.type == TokenType.EOF);
    if (!isTerminal && (allowEofAsTerminal && !isEof)) {
      throwUnexpected(token);
    }
    return node;
  }

  Node parseVar() {
    input.read(); // `var` keyword

    auto lhs = input.read();
    if (lhs.type != TokenType.IDENTIFIER) throwUnexpected(lhs);

    Node rhs;
    if (input.peek().type == TokenType.EQUALS_OP) {
      input.read(); // `=` operator
      rhs = parseExpression();
    }

    return new Var(lhs.stringValue, rhs, Visibility.Public);
  }

  Node parseExpression() {
    return parseInfix();
  }

  Node parseInfix() {
    alias Op = string;
    alias Subnode = Algebraic!(Node, Op);

    void reduceNodes(ref Subnode[] nodes, string reduceOp) {
      for (int index = 0; index < nodes.length; index++) {
        auto const node = nodes[index];
        auto op = node.peek!(Op);
        if (op !is null && *op == reduceOp) {
          auto lhs = nodes[index - 1].get!(Node);
          auto rhs = nodes[index + 1].get!(Node);
          Node newNode = new Infix(lhs, *op, rhs);
          nodes.replaceInPlace(index - 1, index + 2, [Subnode(newNode)]);
          // If we matched then we need to go backwards a step since we've
          // replaced the nodes on either side of this index.
          index--;
        }
      }
    }

    Subnode[] nodes = [Subnode(parseAssignment())];
    while (input.peek().type == TokenType.BINARY_OP) {
      nodes ~= Subnode(input.read().stringValue);
      nodes ~= Subnode(parseAssignment());
    }
    if (nodes.length == 1) {
      // Don't bother reducing if there weren't any operations.
      goto end;
    }
    // The order in which we do reductions defines the associativity, with the
    // earlier reductions having higher associativity than later ones.
    reduceNodes(nodes, "*");
    reduceNodes(nodes, "+");
    assert(nodes.length == 1, "Failed to reduce infix nodes");
  end:
    return nodes[0].get!(Node);
  }

  Node parseAssignment() {
    auto lhs = parseAtom();
    if (input.peek().type == TokenType.EQUALS_OP) {
      input.read();
      auto rhs = parseExpression();
      return new Assignment(lhs, rhs);
    }
    return lhs;
  }

  Node parseAtom() {
    auto token = input.read();

    if (token.type == TokenType.IDENTIFIER) {
      return new Identifier(token.stringValue);
    } else if (token.type == TokenType.INTEGER) {
      return new Integer(token.integerValue);
    }
    throwUnexpected(token);
    return null;
  }

  void throwUnexpected(Token token) {
    throw new Error("Unexpected token: " ~ format!"%s"(token));
  }
}

unittest {
  Program testParse(string input, Node[] args ...) {
    auto expected = new Program(args);
    auto actual = new Parser(input).parseProgram();
    if (!actual.eq(expected)) {
      assert(false,
        "Incorrect parse of:\n" ~
        input ~ "\n" ~
        "Expected:\n" ~
        expected.toPrettyString() ~
        "\nActual:\n" ~
        actual.toPrettyString()
      );
    }
    return actual;
  }

  testParse("var a = 1",
    new Var("a", new Integer(1), Visibility.Public),
  );

  testParse("a = 1",
    new Assignment(new Identifier("a"), new Integer(1)),
  );

  testParse("foo",
    new Identifier("foo"),
  );
  testParse("foo \n bar",
    new Identifier("foo"),
    new Identifier("bar"),
  );

  // Test basic infix.
  testParse("1 + 2 + 3",
    new Infix(
      new Infix(
        new Integer(1),
        "+",
        new Integer(2),
      ),
      "+",
      new Integer(3),
    ),
  );
  // Test associativity.
  testParse("1 * 2 + 3 * 4",
    new Infix(
      new Infix(
        new Integer(1),
        InfixOp.Multiply,
        new Integer(2),
      ),
      InfixOp.Add,
      new Infix(
        new Integer(3),
        InfixOp.Multiply,
        new Integer(4),
      ),
    ),
  );
}
