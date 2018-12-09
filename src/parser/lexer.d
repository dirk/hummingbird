module parser.lexer;

import std.algorithm : canFind;
import std.conv : to;
import std.format : format;
import std.typecons : Tuple;

struct Position {
  int index;
  int line;
  int column;
}

class StringStream {
  string input;
  int index = 0;

  int line = 1;
  int column = 1;

  this(string input) {
    this.input = input;
  }

  this(StringStream copyable) {
    input = copyable.input;
    index = copyable.index;
    line = copyable.line;
    column = copyable.column;
  }

  char read() {
    auto character = safePeek(index);
    index += 1;
    if (character == '\n') {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
    return character;
  }

  void readUntil(char target) {
    readUntil([target]);
  }

  void readUntil(char[] targets ...) {
    while (!targets.canFind(peek())) {
      read();
    }
  }

  char peek() {
    return safePeek(index);
  }

  bool match(string target) {
    // Out of bounds check.
    if ((index + target.length) > input.length)
      return false;

    // Check if they actually match.
    if (input[index..(target.length + index)] != target)
      return false;

    // Consume if matched.
    foreach (_character; target) {
      read();
    }

    return true;
  }

  Position getPosition() {
    return Position(index, line, column);
  }

  // Bounds-checked peeking; will return '\0' if target is beyond the end of
  // the input.
  private char safePeek(int target) {
    return (target < input.length) ? input[target] : '\0';
  }
}

string[] KEYWORDS = [
  "let",
  "var",
];

enum TokenType {
  BINARY_OP,
  BRACE_LEFT,
  BRACE_RIGHT,
  COMMA,
  DOT,
  IDENTIFIER,
  INTEGER,
  KEYWORD,
  EOF,
  EQUALS_OP,
  ERROR,
  PARENTHESES_LEFT,
  PARENTHESES_RIGHT,
  TERMINAL,
}

immutable TokenType[char] characterMap;

static this() {
  characterMap = [
    '{'  : TokenType.BRACE_LEFT,
    '}'  : TokenType.BRACE_RIGHT,
    ','  : TokenType.COMMA,
    '.'  : TokenType.DOT,
    '\0' : TokenType.EOF,
    '='  : TokenType.EQUALS_OP,
    '('  : TokenType.PARENTHESES_LEFT,
    ')'  : TokenType.PARENTHESES_RIGHT,
  ];
}

struct Token {
  TokenType type;
  union {
    string stringValue;
    long integerValue;
  }
  Position begin;

  this(TokenType type, Position begin) {
    assert(
      type == TokenType.BRACE_LEFT ||
      type == TokenType.BRACE_RIGHT ||
      type == TokenType.COMMA ||
      type == TokenType.DOT ||
      type == TokenType.EOF ||
      type == TokenType.EQUALS_OP ||
      type == TokenType.PARENTHESES_LEFT ||
      type == TokenType.PARENTHESES_RIGHT
    );
    this.type = type;
    this.begin = begin;
  }

  this(TokenType type, string value, Position begin) {
    assert(stringValueType(type), "Not a string value type: " ~ to!string(type) ~ " (value = \"" ~ value ~ "\")");
    this.type = type;
    this.stringValue = value;
    this.begin = begin;
  }

  this(TokenType type, long value, Position begin) {
    assert(integerValueType(type));
    this.type = type;
    this.integerValue = value;
    this.begin = begin;
  }

  bool opEquals(const Token other) const {
    if (other.type != type) {
      return false;
    }
    if (stringValueType(type)) {
      return other.stringValue == stringValue;
    } else if (integerValueType(type)) {
      return other.integerValue == integerValue;
    }
    return true;
  }

  string toString() const {
    auto result = to!string(type);
    if (stringValueType(type)) {
      auto inspected = (stringValue == "\n") ? "\\n" : stringValue;
      result ~= "(" ~ inspected ~ ")";
    } else if (integerValueType(type)) {
      result ~= "(" ~ to!string(integerValue) ~ ")";
    }
    return result;
  }

  bool stringValueType(TokenType type) const {
    return (
      type == TokenType.BINARY_OP ||
      type == TokenType.IDENTIFIER ||
      type == TokenType.KEYWORD ||
      type == TokenType.TERMINAL
    );
  }

  bool integerValueType(TokenType type) const {
    return (
      type == TokenType.INTEGER
    );
  }

  bool newline() const {
    return (
      type == TokenType.TERMINAL &&
      stringValue == "\n"
    );
  }
}

class TokenStream {
  StringStream input;

  bool peeking;
  Token nextToken;

  this(StringStream input) {
    this.input = input;
  }

  this(TokenStream copyable) {
    input = new StringStream(copyable.input);
    peeking = copyable.peeking;
    nextToken = copyable.nextToken;
  }

  void backtrack(TokenStream destination) {
    input = destination.input;
    peeking = destination.peeking;
    nextToken = destination.nextToken;
  }

  Token peek() {
    if (!peeking) {
      peeking = true;
      nextToken = lex();
    }
    return nextToken;
  }

  Token read() {
    auto token = peek();
    peeking = false;
    return token;
  }

  bool peekEof() {
    return peek().type == TokenType.EOF;
  }

  Token lex() {
    consumeSpaceAndComments();

    char character;
    Position begin;

    while (true) {
      character = input.peek();
      begin = getPosition();

      if (identifierHead(character))
        return lexIdentifier(begin);

      if (numericHead(character))
        return lexNumeric(begin);

      if (character == '+' || character == '*') {
        // Not checking for '-' because it's already handled by `lexNumeric`.
        return Token(TokenType.BINARY_OP, to!string(input.read()), begin);
      }

      if (character == '\n') {
        input.read();
        consumeMoreNewlineTerminals();
        return Token(TokenType.TERMINAL, "\n", begin);
      }

      if (character == ';') {
        input.read();
        return Token(TokenType.TERMINAL, ";", begin);
      }

      auto mapped = (character in characterMap);
      if (mapped !is null) {
        input.read();
        return Token(*mapped, begin);
      }

      throw new Error("Unrecognized character: " ~ character);
    }
  }

  private Position getPosition() {
    return input.getPosition();
  }

  private void consumeMoreNewlineTerminals() {
    while (true) {
      consumeSpaceAndComments();
      if (input.peek() == '\n') {
        input.read();
        continue;
      } else {
        break;
      }
    }
  }

  private void consumeSpaceAndComments() {
    char character;
    while (true) {
      character = input.peek();

      // Space
      if (character == ' ' || character == '\t') {
        input.read();
      }
      // Single-line comment
      else if (input.match("//")) {
        input.readUntil('\n');
      }
      // Multi-line comment
      else if (input.match("/*")) {
        while (true) {
          if (input.match("*/")) {
            break;
          } else {
            input.read();
          }
        }
      }
      // Not a comment
      else {
        break;
      }
    }
  }

  Token lexIdentifier(Position begin) {
    string identifier = "" ~ input.read();
    while (true) {
      char character = input.peek();
      if (identifierTail(character)) {
        input.read();
        identifier ~= character;
      } else {
        break;
      }
    }
    if (KEYWORDS.canFind(identifier)) {
      return Token(TokenType.KEYWORD, identifier, begin);
    }
    return Token(TokenType.IDENTIFIER, identifier, begin);
  }

  Token lexNumeric(Position begin) {
    string number = "" ~ input.read();

    // If it's just a minus with no digit after it then return the minus as a
    // binary operator.
    if (number == "-" && !digit(input.peek())) {
      return Token(TokenType.BINARY_OP, number, begin);
    }

    while (true) {
      auto const character = input.peek();
      if (digit(character)) {
        number ~= input.read();
      } else {
        break;
      }
    }

    return Token(TokenType.INTEGER, to!long(number), begin);
  }

  // The first character of identifiers must be alphabetical.
  bool identifierHead(char c) {
    return alphabetical(c);
  }

  bool identifierTail(char c) {
    return alphabetical(c);
  }

  bool numericHead(char c) {
    return digit(c) || (c == '-');
  }

  bool alphabetical(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
  }

  bool digit(char c) {
    return (c >= '0' && c <= '9');
  }
}

unittest {
  Token testRead(string input, Token expected) {
    auto lexer = new TokenStream(new StringStream(input));
    auto actual = lexer.read();
    if (expected != actual) {
      assert(false,
        "Incorrect lex of:\n" ~
        input ~ "\n" ~
        "Expected:\n" ~
        format!"%s"(expected) ~
        "\nActual:\n" ~
        format!"%s"(actual)
      );
    }
    // Check that it fully consumsed the input.
    auto const eof = lexer.read();
    assert(eof.type == TokenType.EOF);
    return actual;
  }

  Token[] testReads(string input, Token[] expected ...) {
    auto lexer = new TokenStream(new StringStream(input));
    Token[] actual;
    while (true) {
      auto result = lexer.read();
      actual ~= result;
      if (result.type == TokenType.EOF) {
        break;
      }
    }
    assert(expected == actual,
      "Incomplete lex of:\n" ~
      input ~ "\n" ~
      format!"Expected:\n%s\n"(expected) ~
      format!"Actual:\n%s\n"(actual)
    );
    return actual;
  }

  testRead("foo", Token(TokenType.IDENTIFIER, "foo"));
  
  testRead("var", Token(TokenType.KEYWORD, "var"));
  testReads("var foo = 1",
    Token(TokenType.KEYWORD, "var"),
    Token(TokenType.IDENTIFIER, "foo"),
    Token(TokenType.EQUALS_OP),
    Token(TokenType.INTEGER, 1),
    Token(TokenType.EOF),
  );

  testRead("1", Token(TokenType.INTEGER, 1));
  testRead("-1", Token(TokenType.INTEGER, -1));
  testReads("- 1",
    Token(TokenType.BINARY_OP, "-"),
    Token(TokenType.INTEGER, 1),
    Token(TokenType.EOF),
  );
  testReads("1+2",
    Token(TokenType.INTEGER, 1),
    Token(TokenType.BINARY_OP, "+"),
    Token(TokenType.INTEGER, 2),
    Token(TokenType.EOF),
  );

  testReads("foo /* Comment */ bar",
    Token(TokenType.IDENTIFIER, "foo"),
    Token(TokenType.IDENTIFIER, "bar"),
    Token(TokenType.EOF),
  );

  testReads("foo // Comment \n bar",
    Token(TokenType.IDENTIFIER, "foo"),
    Token(TokenType.TERMINAL, "\n"),
    Token(TokenType.IDENTIFIER, "bar"),
    Token(TokenType.EOF),
  );

  testReads(
    "foo
      // Comment about the call
      // Another comment about the call
      bar()",
    Token(TokenType.IDENTIFIER, "foo"),
    Token(TokenType.TERMINAL, "\n"),
    Token(TokenType.IDENTIFIER, "bar"),
    Token(TokenType.PARENTHESES_LEFT),
    Token(TokenType.PARENTHESES_RIGHT),
    Token(TokenType.EOF),
  );

  testReads(
    "{ 1; }",
    Token(TokenType.BRACE_LEFT),
    Token(TokenType.INTEGER, 1),
    Token(TokenType.TERMINAL, ";"),
    Token(TokenType.BRACE_RIGHT),
    Token(TokenType.EOF),
  );
}
