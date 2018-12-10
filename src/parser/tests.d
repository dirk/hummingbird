module parser.tests;

import ast.ast;
import parser.parser : Parser;

Program testParse(string input, Program expected) {
  auto program = new Parser(input).parseProgram();
  if (!program.eq(expected)) {
    assert(false,
      "Incorrect parse of:\n" ~
      input ~ "\n" ~
      "Expected:\n" ~
      expected.toPrettyString() ~
      "\nActual:\n" ~
      program.toPrettyString()
    );
  }
  return program;
}

Program testParse(string input, Node[] args ...) {
  auto expected = new Program(args);
  return testParse(input, expected);
}

unittest {
  testParse("var a = 1",
    new Var("a", new Integer(1), Visibility.Public),
  );

  // Test associativity.
  testParse("var a = 1 * 2 + 3 * 4",
    new Var(
      "a",
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
      Visibility.Public,
    ),
  );

  // Test terminals.
  testParse("\n123\n",
    new Program([
      new Integer(123),
    ]),
  );

  // Test comments.
  testParse(
    "
    // This is valid
    12 // So is this
    var ab /* And this */ = 34
    ",
    new Program([
      new Integer(12),
      new Var("ab", new Integer(34), Visibility.Public),
    ]),
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

  // Test functions.
  testParse(
    "func foo() { 123 }",
    new Function(
      "foo",
      new Block([
        new Integer(123),
      ]),
    ),
  );
}
