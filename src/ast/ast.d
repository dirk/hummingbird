module ast.ast;

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

  abstract bool eq(Node) const;

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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Assignment)anyOther) {
      return (lhs.eq(other.lhs) && rhs.eq(other.rhs));
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Block)anyOther) {
      if (nodes.length != other.nodes.length) {
        return false;
      }
      foreach (index, ownNode; nodes) {
        auto otherNode = other.nodes[index];
        if (!ownNode.eq(otherNode)) {
          return false;
        }
      }
      return true;
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Identifier)anyOther) {
      return (value == other.value);
    }
    return false;
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

  this(Node lhs, string opString, Node rhs) {
    this.lhs = lhs;
    this.rhs = rhs;

    InfixOp matchedOp;
    switch (opString) {
      case "+":
        matchedOp = InfixOp.Add;
        break;
      case "*":
        matchedOp = InfixOp.Multiply;
        break;
      default:
        throw new Error("Unrecognized infix operator: " ~ opString);
    }
    this.op = matchedOp;
  }

  override bool eq(Node anyOther) const {
    if (auto other = cast(Infix)anyOther) {
      return (
        lhs.eq(other.lhs) &&
        op == other.op &&
        rhs.eq(other.rhs)
      );
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Integer)anyOther) {
      return (value == other.value);
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Let)anyOther) {
      return (
        lhs == other.lhs &&
        rhs.eq(other.rhs) &&
        visibility == other.visibility
      );
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(PostfixCall)anyOther) {
      if (!target.eq(other.target)) {
        return false;
      }
      if (arguments.length != other.arguments.length) {
        return false;
      }
      foreach (index, ownArgument; arguments) {
        auto otherArgument = other.arguments[index];
        if (!ownArgument.eq(otherArgument)) {
          return false;
        }
      }
      return true;
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(PostfixIndex)anyOther) {
      return (
        target.eq(other.target) &&
        argument.eq(other.argument)
      );
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(PostfixProperty)anyOther) {
      return (
        target.eq(other.target) &&
        value == other.value
      );
    }
    return false;
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Program)anyOther) {
      if (nodes.length != other.nodes.length) {
        return false;
      }
      foreach (index, ownNode; nodes) {
        auto otherNode = other.nodes[index];
        if (!ownNode.eq(otherNode)) {
          return false;
        }
      }
      return true;
    }
    return false;
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

class Return : Node {
  Node rhs;

  this(Node rhs) {
    this.rhs = rhs;
  }

  override bool eq(Node anyOther) const {
    if (auto other = cast(Return)anyOther) {
      if (rhs is null) {
        return (other.rhs is null);
      }
      return rhs.eq(other.rhs);
    }
    return false;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(";
    if (rhs !is null) {
      result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
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

  override bool eq(Node anyOther) const {
    if (auto other = cast(Var)anyOther) {
      return (
        lhs == other.lhs &&
        rhs.eq(other.rhs) &&
        visibility == other.visibility
      );
    }
    return false;
  }

  override string toPrettyString(string indent = "") const {
    auto result = nameAndLocation() ~ "(" ~ to!string(visibility) ~ " " ~ lhs;
    result ~= "\n" ~ indent ~ rhs.toPrettyString(indent ~ defaultIndent);
    return result ~ ")";
  }
}
