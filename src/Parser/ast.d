import std.algorithm.searching : findSkip;
import std.conv : to;
import std.typecons : Tuple;

import peg = pegged.peg;

enum Visibility {
  Public,
  Private,
}

alias Position = Tuple!(size_t, "line", size_t, "column");

struct Location {
  string source;
  size_t begin, end;

  static const missing = Location("", -1, -1);

  this(string source, size_t begin, size_t end) {
    this.source = source;
    this.begin = begin;
    this.end = end;
  }

  this(peg.ParseTree tree) {
    source = tree.input;
    begin = tree.begin;
    end = tree.end;
  }

  auto position() const {
    auto resolved = peg.position(source[0..begin]);
    return Position(resolved.line, resolved.col);
  }

  bool present() const {
    return source != "" && begin != -1 && end != -1;
  }
}

string defaultIndent = "  ";

class Node {
  Location location = Location.missing;

  abstract string toPrettyString(string indent = "") const;

  string nameAndLocation() const {
    string name = to!string(this.classinfo.name);
    name.findSkip(".");
    auto result = name;
    if (location.present()) {
      auto position = location.position();
      result ~= "{" ~ to!string(position.line) ~ "," ~ to!string(position.column) ~ "}";
    }
    return result;
  }
}

class Assignment : Node {
  Node lhs;
  Node rhs;

  this(Node lhs, Node rhs) {
    this.lhs = lhs;
    this.rhs = rhs;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(";
    result ~= "\n" ~ indent ~ lhs.toPrettyString(indent ~ defaultIndent);
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class Block : Node {
  Node[] nodes;

  this(Node[] nodes) {
    this.nodes = nodes;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(";
    if (this.nodes.length > 0) {
      foreach (node; nodes) {
        result ~= "\n" ~ indent ~ node.toPrettyString(indent ~ defaultIndent);
      }
    }
    return result ~ ")";
  }
}

class Identifier : Node {
  string value;

  this(string value) {
    this.value = value;
  }

  override string toPrettyString(string indent = "") const {
    return nameAndLocation() ~ "(" ~ value ~ ")";
  }
}

enum InfixOp {
  Add,
  Multiply,
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
    auto result = nameAndLocation() ~ "(" ~ to!string(op);
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
    return nameAndLocation() ~ "(" ~ to!string(value) ~ ")";
  }
}

class Let : Node {
  string lhs;
  Node rhs;
  Visibility visibility;

  this(string lhs, Node rhs, Visibility visibility) {
    this.lhs = lhs;
    this.rhs = rhs;
    this.visibility = visibility;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(" ~ to!string(visibility) ~ " " ~ lhs;
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
    auto result = nameAndLocation() ~ "([";
    foreach (ref argument; arguments) {
      result ~= "\n" ~ indent ~ argument.toPrettyString(indent ~ defaultIndent);
    }
    result ~= "]\n" ~ indent ~ target.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class PostfixIndex : Node {
  Node target;
  Node argument;

  this(Node target, Node argument) {
    this.target = target;
    this.argument = argument;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(";
    result ~= "\n" ~ indent ~ argument.toPrettyString(indent ~ defaultIndent);
    result ~= "\n" ~ indent ~ target.toPrettyString(indent ~ defaultIndent);
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
    auto result = nameAndLocation() ~ "(";
    result ~= "\n" ~ indent ~ value;
    result ~= "\n" ~ indent ~ target.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}

class Program : Node {
  Node[] nodes;

  this(Node[] nodes) {
    this.nodes = nodes;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(";
    if (this.nodes.length > 0) {
      indent ~= defaultIndent;
      foreach (node; nodes) {
        result ~= "\n" ~ indent ~ node.toPrettyString(indent ~ defaultIndent);
      }
    }
    return result ~ ")";
  }
}

class Var : Node {
  string lhs;
  Node rhs;
  Visibility visibility;

  this(string lhs, Node rhs, Visibility visibility) {
    this.lhs = lhs;
    this.rhs = rhs;
    this.visibility = visibility;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(" ~ to!string(visibility) ~ " " ~ lhs;
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}
