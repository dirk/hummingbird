import std.algorithm : canFind, map, startsWith;
import std.array : array;
import std.conv : to;
import std.stdio : writeln;
import std.string : stripLeft;

import pegged.peg : ParseTree;

import ast;
import grammar : Grammar;

Program parse(string input, bool debugPrint = false) {
  auto tree = Grammar(input);
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
  tree.name = tree.name.stripLeft("hummingbird.");
  return tree;
}

string[] keepTreeNames = [
  "Let",
  "Program",
  "Statement",
  "Var",
];

ParseTree simplifyTree(ref ParseTree tree) {
  foreach (ref child; tree.children) {
    child = simplifyTree(child);
  }

  if (tree.children.length != 1) {
    return tree;
  } else if (keepTreeNames.canFind(tree.name)) {
    return tree;
  } else {
    // linear tree
    return tree.children[0];
  }
}

Node visitTree(ref ParseTree tree) {
  switch (tree.name) {
    case "Assignment":
      return visitAssignment(tree);
    case "Statement":
      return visitStatement(tree);
    case "Identifier":
      return visitIdentifier(tree);
    case "InfixAdd":
      return visitInfix(tree, InfixOp.add);
    case "InfixMultiply":
      return visitInfix(tree, InfixOp.multiply);
    case "Integer":
      return visitInteger(tree);
    case "Let":
      return visitLet(tree);
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
  assert(tree.children.length == 2);
  assert(tree.children[0].name == "Identifier");
  assert(tree.children[0].matches.length == 1);
  auto lhs = tree.children[0].matches[0];
  auto rhs = visitTree(tree.children[1]);
  return new Let(lhs, rhs);
}

Program visitProgram(ref ParseTree tree) {
  auto nodes = tree.children.map!(visitTree).array();
  return new Program(nodes);
}

Node visitStatement(ref ParseTree tree) {
  assert(tree.children.length == 2);
  assert(tree.children[1].name == "Terminal");
  return visitTree(tree.children[0]);
}
