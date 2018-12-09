module parser.parser;

import std.algorithm.mutation : remove;
import std.algorithm.searching : canFind;
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
      statements ~= parseStatements(TokenType.EOF);
    }
    return new Program(statements);
  }

  // Parses a sequence of statements: will only halt when it encounters the
  // `terminator`. Note that it will leave that on the input, so the caller
  // needs to do its own `input.read()`.
  Node[] parseStatements(TokenType terminator) {
    Node[] nodes;
    while (true) {
      consumeTerminals();
      if (input.peek().type == terminator) break;

      nodes ~= parseStatement(terminator);

      consumeTerminals();
      if (input.peek().type == terminator) break;
    }
    return nodes;
  }

  // `terminator` is a pseudo-terminal that can act like a terminal. However it
  // will peeked for, not read, from the input.
  Node parseStatement(TokenType terminator) {
    Node node;

    auto const next = input.peek();
    if (next.type == TokenType.KEYWORD) {
      if (next.stringValue == "let" || next.stringValue == "var") {
        node = parseLetAndVar();
        goto terminal;
      } else if (next.stringValue == "return") {
        node = parseReturn(terminator);
        goto terminal;
      }
    }
    node = parseExpression();

  terminal:
    // Treat the statement as fully parsed if we encounter the terminator.
    if (input.peek().type == terminator) {
      return node;
    }
    auto token = input.read();
    if (token.type != TokenType.TERMINAL) {
      throwUnexpected(token);
    }
    return node;
  }

  Node parseLetAndVar() {
    auto keyword = input.read(); // `let` or `var` keyword

    auto lhs = input.read();
    if (lhs.type != TokenType.IDENTIFIER) throwUnexpected(lhs);

    Node rhs;
    if (input.peek().type == TokenType.EQUALS_OP) {
      input.read(); // `=` operator
      rhs = parseExpression();
    }

    Node node;
    if (keyword.stringValue == "let") {
      node = new Let(lhs.stringValue, rhs, Visibility.Public);
    } else if (keyword.stringValue == "var") {
      node = new Var(lhs.stringValue, rhs, Visibility.Public);
    } else {
      throw new Error("Unrecognized keyword: " ~ keyword.stringValue);
    }
    node.location = Location(keyword);
    return node;
  }

  Node parseReturn(TokenType terminator) {
    auto keyword = input.read(); // `return` keyword

    Node rhs = null;
    auto next = input.peek();
    if (next.type != TokenType.TERMINAL && next.type != terminator) {
      rhs = parseExpression();
    }
    auto node = new Return(rhs);
    node.location = Location(keyword);
    return node;
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

    Subnode[] nodes = [Subnode(parseBlock())];
    while (input.peek().type == TokenType.BINARY_OP) {
      nodes ~= Subnode(input.read().stringValue);
      nodes ~= Subnode(parseBlock());
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

  Node parseBlock() {
    if (input.peek().type != TokenType.BRACE_LEFT) {
      return parseParentheses();
    }
    input.read(); // Opening brace
    auto nodes = parseStatements(TokenType.BRACE_RIGHT);
    input.read(); // Closing brace
    return new Block(nodes);
  }

  Node parseParentheses() {
    if (input.peek().type != TokenType.PARENTHESES_LEFT) {
      return parseAssignment();
    }
    input.read(); // Opening parentheses
    auto node = parseExpression();
    // Should be closing parentheses.
    auto token = input.read();
    if (token.type != TokenType.PARENTHESES_RIGHT) throwUnexpected(token);
    return node;
  }

  Node parseAssignment() {
    auto lhs = parsePostfix();
    if (input.peek().type == TokenType.EQUALS_OP) {
      input.read();
      auto rhs = parseExpression();
      return new Assignment(lhs, rhs);
    }
    return lhs;
  }

  Node parsePostfix() {
    auto target = parseAtom();
    Node newTarget;
    while (true) {
      newTarget = tryParsePostfixProperty(target);
      if (newTarget !is null) {
        target = newTarget;
        continue;
      }
      if (input.peek().type == TokenType.PARENTHESES_LEFT) {
        target = parsePostfixProperty(target);
      }
      break;
    }
    return target;
  }

  Node parsePostfixProperty(Node target) {
    input.read(); // Left parentheses
    Node[] arguments;
    if (input.peek().type == TokenType.PARENTHESES_RIGHT) {
      input.read();
      goto end;
    }
    while (true) {
      auto argument = parseExpression();
      arguments ~= argument;
      auto next = input.peek();
      if (next.type == TokenType.COMMA) {
        input.read(); // Comma
        // Allow a trailing comma before the closing parentheses.
        if (input.peek().type == TokenType.PARENTHESES_RIGHT) goto end;
        continue;
      } else if (next.type == TokenType.PARENTHESES_RIGHT) {
        goto end;
      } else {
        throwUnexpected(next);
      }
    }
  end:
    input.read(); // Right parentheses
    return new PostfixCall(target, arguments);
  }

  Node tryParsePostfixProperty(Node target) {
    bool needsBacktrack = false;
    auto savepoint = new TokenStream(input);
    Token identifier;

    // Lookahead to for subsequent-line properties, eg:
    //   myCoolVariable
    //     .myCoolProperty
    if (input.peek().newline()) {
      input.read(); // Newline
      needsBacktrack = true;
    }

    if (input.peek().type != TokenType.DOT) goto backtrack;
    input.read(); // Dot
    needsBacktrack = true;

    if (input.peek().type != TokenType.IDENTIFIER) goto backtrack;
    identifier = input.read();
    return new PostfixProperty(target, identifier.stringValue);

  backtrack:
    if (needsBacktrack) input.backtrack(savepoint);
    return null;
  }

  Node parseAtom() {
    auto token = input.read();
    Node node;
    if (token.type == TokenType.IDENTIFIER) {
      node = new Identifier(token.stringValue);
    } else if (token.type == TokenType.INTEGER) {
      node = new Integer(token.integerValue);
    } else {
      throwUnexpected(token);
    }
    node.location = Location(token);
    return node;
  }

  void consumeTerminals() {
    while (input.peek().type == TokenType.TERMINAL) {
      input.read();
    }
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
  testParse("let a = 1",
    new Let("a", new Integer(1), Visibility.Public),
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

  // Test grouping.
  testParse("1 * (2 + 3)",
    new Infix(
      new Integer(1),
      InfixOp.Multiply,
      new Infix(
        new Integer(2),
        InfixOp.Add,
        new Integer(3),
      ),
    ),
  );

  testParse("a.b",
    new PostfixProperty(new Identifier("a"), "b"),
  );
  // Test mulit-line properties.
  testParse("a\n  .b",
    new PostfixProperty(new Identifier("a"), "b"),
  );

  // Test calls.
  testParse("a(1)",
    new PostfixCall(
      new Identifier("a"),
      [ new Integer(1) ],
    ),
  );
  testParse("a(1,)",
    new PostfixCall(
      new Identifier("a"),
      [ new Integer(1) ],
    ),
  );
  testParse("a(1,2)",
    new PostfixCall(
      new Identifier("a"),
      [ new Integer(1), new Integer(2) ],
    ),
  );
  testParse("a(1,2,)",
    new PostfixCall(
      new Identifier("a"),
      [ new Integer(1), new Integer(2) ],
    ),
  );
  testParse("a\n .b()",
    new PostfixCall(
      new PostfixProperty(new Identifier("a"), "b"),
      [],
    ),
  );

  // Test blocks.
  testParse(
    "{}",
    new Block([]),
  );
  testParse(
    "{ 1 }",
    new Block([ new Integer(1) ]),
  );
  testParse(
    "{ 1; }",
    new Block([ new Integer(1) ]),
  );
  testParse(
    "{ 1; 2 }",
    new Block([
      new Integer(1),
      new Integer(2),
    ]),
  );
  testParse(
    "{ 1; 2; }",
    new Block([
      new Integer(1),
      new Integer(2),
    ]),
  );
  testParse(
    "
    {
      1
      2;
    }",
    new Block([
      new Integer(1),
      new Integer(2),
    ]),
  );

  // Test handling of multiple newlines (terminals).
  testParse("
    1

    2

    ",
    new Integer(1),
    new Integer(2),
  );

  // Test spamming terminals.
  testParse("1;// Comment\n;;",
    new Integer(1),
  );
}
