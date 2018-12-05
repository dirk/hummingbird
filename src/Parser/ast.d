import std.conv : to;

string defaultIndent = "  ";

class Node {
  abstract string toPrettyString(string indent = "") const;
}

class Assignment : Node {
  Node lhs;
  Node rhs;

  this(Node lhs, Node rhs) {
    this.lhs = lhs;
    this.rhs = rhs;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "Assignment(";
    result ~= "\n" ~ indent ~ lhs.toPrettyString(indent ~ defaultIndent);
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class Identifier : Node {
  string value;

  this(string value) {
    this.value = value;
  }

  override string toPrettyString(string indent = "") const {
    return "Identifier(" ~ value ~ ")";
  }
}

enum InfixOp {
  add,
  multiply,
}

class Infix : Node {
  Node lhs;
  InfixOp op;
  Node rhs;

  this(Node lhs, InfixOp op, Node rhs) {
    this.lhs = lhs;
    this.op = op;
    this.rhs = rhs;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "Infix(" ~ to!string(op);
    result ~= "\n" ~ indent ~ lhs.toPrettyString(indent ~ defaultIndent);
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class Integer : Node {
  long value;

  this(long value) {
    this.value = value;
  }

  override string toPrettyString(string indent = "") const {
    return "Integer(" ~ to!string(value) ~ ")";
  }
}

class Let : Node {
  string lhs;
  Node rhs;

  this(string lhs, Node rhs) {
    this.lhs = lhs;
    this.rhs = rhs;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "Let(" ~ lhs;
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class PostfixCall : Node {
  Node target;
  Node[] arguments;

  this(Node target, Node[] arguments) {
    this.target = target;
    this.arguments = arguments;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "PostfixCall([";
    foreach (ref argument; arguments) {
      result ~= "\n" ~ indent ~ argument.toPrettyString(indent ~ defaultIndent);
    }
    result ~= "]\n" ~ indent ~ target.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class PostfixProperty : Node {
  Node target;
  string value;

  this(Node target, string value) {
    this.target = target;
    this.value = value;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "PostfixProperty(";
    result ~= "\n" ~ indent ~ target.toPrettyString(indent ~ defaultIndent);
    result ~= "\n" ~ indent ~ value;
    return result ~ ")";
  }
}

class Program : Node {
  Node[] nodes;

  this(Node[] nodes) {
    this.nodes = nodes;
  }

  override string toPrettyString(string indent = "") const {
    auto result = "Program(";
    if (this.nodes.length > 0) {
      indent ~= defaultIndent;
      foreach (node; nodes) {
        result ~= "\n" ~ indent ~ node.toPrettyString(indent ~ defaultIndent);
      }
    }
    return result ~ ")";
  }
}
