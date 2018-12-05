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
  tree.name = tree.name.stripLeft("Grammar.");
  return tree;
}

string[] keepTreeNames = [
  "Block",
  "CallArgs",
  "Let",
  "PostfixCall",
  "PostfixProperty",
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
    return tree.children[0];
  }
}

Node visitTree(ref ParseTree tree) {
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
      return visitInfix(tree, InfixOp.add);
    case "InfixMultiply":
      return visitInfix(tree, InfixOp.multiply);
    case "Integer":
      return visitInteger(tree);
    case "Let":
      return visitLet(tree);
    case "PostfixCall":
      return visitPostfixCall(tree);
    case "PostfixProperty":
      return visitPostfixProperty(tree);
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
  assert(tree.children.length == 0 || tree.children.length == 1);
  Node[] statements;
  if (tree.children.length == 1) {
    foreach (ref child; tree.children) {
      statements ~= visitTree(child);
    }
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
  assert(tree.children.length == 2);
  auto lhs = identifierTreeToString(tree.children[0]);
  auto rhs = visitTree(tree.children[1]);
  return new Let(lhs, rhs);
}

PostfixProperty visitPostfixProperty(ref ParseTree tree) {
  assert(tree.children.length == 2);
  auto target = visitTree(tree.children[0]);
  auto value = identifierTreeToString(tree.children[1]);
  return new PostfixProperty(target, value);
}

PostfixCall visitPostfixCall(ref ParseTree tree) {
  assert(tree.children.length == 1 || tree.children.length == 2);
  auto target = visitTree(tree.children[0]);
  Node[] arguments;
  if (tree.children.length == 2) {
    auto argumentsTree = tree.children[1];
    assert(argumentsTree.name == "CallArgs");
    foreach (ref argumentTree; argumentsTree.children) {
      arguments ~= visitTree(argumentTree);
    }
  }
  return new PostfixCall(target, arguments);
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

Var visitVar(ref ParseTree tree) {
  assert(tree.children.length == 2);
  auto lhs = identifierTreeToString(tree.children[0]);
  auto rhs = visitTree(tree.children[1]);
  return new Var(lhs, rhs);
}

string identifierTreeToString(ref ParseTree tree) {
  assert(tree.name == "Identifier");
  assert(tree.matches.length == 1);
  return tree.matches[0];
}
