module parser.parser;

import std.algorithm : canFind, map, startsWith;
import std.array : array;
import std.conv : to;
import std.range.primitives : popFront;
import std.stdio : writeln;
import std.string : stripLeft;

import pegged.peg : ParseTree;

import ast.ast;
import grammar : grammar = Grammar;

Program parse(string input, bool debugPrint = false) {
  auto tree = grammar(input);
  auto renamedTree = renameTree(tree);
  auto simplifiedTree = simplifyTree(renamedTree);
  if (debugPrint) {
    writeln(simplifiedTree);
  }
  return visitProgram(simplifiedTree);
}

ParseTree renameTree(ref ParseTree tree) {
  foreach (ref child; tree.children) {
    renameTree(child);
  }
  tree.name = tree.name.stripLeft("Grammar.");
  return tree;
}

enum {
  TAbstractModifier = "AbstractModifier",
  TClassModifiers = "ClassModifiers",
  TVisibilityModifier = "VisibilityModifier",
}

string[] keepTreeNames = [
  "Block",
  "CallArgs",
  "Let",
  "PostfixCall",
  "PostfixIndex",
  "PostfixList",
  "PostfixProperty",
  "Program",
  "Statement",
  "Var",
];

string[] discardChildrenTreeNames = [
  TAbstractModifier,
  TClassModifiers,
  TVisibilityModifier,
];

ParseTree simplifyTree(ref ParseTree tree) {
  foreach (ref child; tree.children) {
    child = simplifyTree(child);
  }

  if (tree.children.length != 1) {
    return tree;
  } else if (discardChildrenTreeNames.canFind(tree.name)) {
    tree.children = [];
    return tree;
  } else if (keepTreeNames.canFind(tree.name)) {
    return tree;
  } else {
    return tree.children[0];
  }
}

Node visitTree(ref ParseTree tree) {
  auto node = visitTreeImpl(tree);
  // Statements include their terminal in their position, so don't overwrite
  // their inner node with a position that includes the terminal.
  if (tree.name != "Statement") {
    node.location = Location(tree);
  }
  return node;
}

Node visitTreeImpl(ref ParseTree tree) {
  switch (tree.name) {
    case "Assignment":
      return visitAssignment(tree);
    case "Block":
      return visitBlock(tree);
    case "Statement":
      return visitStatement(tree);
    case "Identifier":
      return visitIdentifier(tree);
    case "InfixAdd":
      return visitInfix(tree, InfixOp.Add);
    case "InfixMultiply":
      return visitInfix(tree, InfixOp.Multiply);
    case "Integer":
      return visitInteger(tree);
    case "Let":
      return visitLet(tree);
    case "Postfix":
      return visitPostfix(tree);
    case "Var":
      return visitVar(tree);
    default:
      throw new Error("Unknown tree name " ~ tree.name);
  }
}

Assignment visitAssignment(ref ParseTree tree) {
  assert(tree.children.length == 2);
  Node lhs = visitTree(tree.children[0]);
  Node rhs = visitTree(tree.children[1]);
  return new Assignment(lhs, rhs);
}

Block visitBlock(ref ParseTree tree) {
  Node[] statements;
  foreach (ref child; tree.children) {
    statements ~= visitStatement(child);
  }
  return new Block(statements);
}

Identifier visitIdentifier(ref ParseTree tree) {
  assert(tree.matches.length == 1);
  string match = tree.matches[0];
  return new Identifier(match);
}

Infix visitInfix(ref ParseTree tree, InfixOp op) {
  assert(tree.children.length == 2);
  auto lhs = visitTree(tree.children[0]);
  auto rhs = visitTree(tree.children[1]);
  return new Infix(lhs, op, rhs);
}

Integer visitInteger(ref ParseTree tree) {
  assert(tree.matches.length == 1);
  string match = tree.matches[0];
  auto value = to!long(match);
  return new Integer(value);
}

Let visitLet(ref ParseTree tree) {
  auto children = tree.children.dup();
  auto visibility = shiftVisibility(children);
  assert(children.length == 2);
  auto lhs = identifierTreeToString(children[0]);
  auto rhs = visitTree(children[1]);
  return new Let(lhs, rhs, visibility);
}

Node visitPostfix(ref ParseTree tree) {
  assert(tree.children.length == 2);
  assert(tree.children[1].name == "PostfixList");
  auto node = visitTree(tree.children[0]);
  auto postfixList = tree.children[1];
  // Convert list of postfix operations into a tree.
  foreach (postfixTree; postfixList.children) {
    switch (postfixTree.name) {
      case "PostfixCall":
        node = visitPostfixCall(postfixTree, node);
        break;
      case "PostfixProperty":
        node = visitPostfixProperty(postfixTree, node);
        break;
      case "PostfixIndex":
        node = visitPostfixIndex(postfixTree, node);
        break;
      default:
        throw new Error("Unrecognized postfix tree name: " ~ postfixTree.name);
    }
  }
  return node;
}

PostfixIndex visitPostfixIndex(ref ParseTree tree, Node target) {
  assert(tree.children.length == 1);
  auto argument = visitTree(tree.children[0]);
  return new PostfixIndex(target, argument);
}

PostfixCall visitPostfixCall(ref ParseTree tree, Node target) {
  assert(tree.children.length == 0 || tree.children.length == 1);
  Node[] arguments;
  if (tree.children.length == 1) {
    auto argumentsTree = tree.children[0];
    assert(argumentsTree.name == "CallArgs");
    foreach (ref argumentTree; argumentsTree.children) {
      arguments ~= visitTree(argumentTree);
    }
  }
  return new PostfixCall(target, arguments);
}

PostfixProperty visitPostfixProperty(ref ParseTree tree, Node target) {
  assert(tree.children.length == 1);
  auto value = identifierTreeToString(tree.children[0]);
  return new PostfixProperty(target, value);
}

Program visitProgram(ref ParseTree tree) {
  auto nodes = tree.children.map!(visitTree).array();
  return new Program(nodes);
}

Node visitStatement(ref ParseTree tree) {
  assert(tree.children.length == 1 || tree.children.length == 2);
  if (tree.children.length == 2) assert(tree.children[1].name == "Terminal");
  return visitTree(tree.children[0]);
}

Var visitVar(ref ParseTree tree) {
  auto children = tree.children.dup();
  auto visibility = shiftVisibility(children);
  assert(children.length == 2);
  auto lhs = identifierTreeToString(children[0]);
  auto rhs = visitTree(children[1]);
  return new Var(lhs, rhs, visibility);
}

Visibility shiftVisibility(ref ParseTree[] children) {
  auto child = children.shiftChildIf(TVisibilityModifier);
  if (child) {
    assert(child.matches.length == 1);
    auto match = child.matches[0];
    switch (match) {
      case "public":
        return Visibility.Public;
      case "private":
        return Visibility.Private;
      default:
        throw new Error("Unrecognized visibility: " ~ match);
    }
  }
  return Visibility.Public;
}

ParseTree* shiftChild(ref ParseTree[] children) {
  auto child = &children[0];
  children.popFront();
  return child;
}

ParseTree* shiftChildIf(ref ParseTree[] children, string name) {
  if (children[0].name == name) {
    return shiftChild(children);
  }
  return null;
}

string identifierTreeToString(ref ParseTree tree) {
  assert(tree.name == "Identifier");
  assert(tree.matches.length == 1);
  return tree.matches[0];
}
