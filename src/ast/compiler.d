module ast.compiler;

import std.algorithm : map;
import std.array : array, split;
import std.conv : to;
import std.stdio : writeln;

import ast = ast.ast;
import ir.builder;

bool isLast(ast.Node[] nodes, ast.Node node) {
  return (nodes[$-1] == node);
}

class UnitCompiler {
  ast.Program program;

  this(ast.Program program) {
    this.program = program;
  }

  UnitBuilder compile() {
    auto unit = new UnitBuilder();
    foreach(node; program.nodes) {
      compileNode(node, unit.mainFunction);
    }
    // Naively ensure the main function ends with a return.
    unit.mainFunction.current.buildReturnNull();
    return unit;
  }

  Value compileNode(ast.Node node, FunctionBuilder func) {
    // Roll-your-own dynamic dispatch!
    string buildDynamicDispatches(string[] typeNames ...) {
      string result = "";
      foreach (typeName; typeNames) {
        string demodularizedTypeName = typeName.split(".")[$-1];
        result ~= "
          if (auto lhs = cast(" ~ typeName ~ ")node) {
            return compile" ~ demodularizedTypeName ~ "(lhs, func);
          }
        ";
      }
      return result;
    }
    mixin(buildDynamicDispatches(
      "ast.Assignment",
      "ast.Identifier",
      "ast.Integer",
      "ast.PostfixCall",
      "ast.Var",
    ));
    if (auto lhs = cast(ast.Block)node) {
      return compileAnonymousBlock(lhs, func);
    }
    throw new Error("Not implemented for: " ~ to!string(node.classinfo.name));
  }

  // Compile a block that doesn't appear as part of a function, clsas, etc.
  Value compileAnonymousBlock(ast.Block node, FunctionBuilder func) {
    // Don't even bother branching for an anonymous block and just return null.
    if (node.nodes.length == 0) {
      return func.nullValue();
    }

    BasicBlockBuilder enteringFrom = func.current;
    BasicBlockBuilder block = func.newBlock();
    // Make the block we just left branch into us.
    enteringFrom.buildBranch(block);

    Value implicitReturn;
    foreach (index, childNode; node.nodes) {
      auto result = compileNode(childNode, func);
      if ((index + 1) == node.nodes.length) {
        implicitReturn = result;
      }
    }

    BasicBlockBuilder leavingTo = func.newBlock();
    block.buildBranch(leavingTo);
    return implicitReturn;
  }

  Value compileAssignment(ast.Assignment node, FunctionBuilder func) {
    auto head = node.lhs;
    Value delegate(Value rval) compileAssigner;

    if (auto identifier = cast(ast.Identifier)head) {
      auto local = identifier.value;
      if (func.haveLocal(local)) {
        auto index = func.getLocal(local);
        compileAssigner = (Value rval) {
          func.current.buildSetLocal(index, rval);
          return rval;
        };
      } else {
        compileAssigner = (Value rval) {
          func.current.buildSetLocalLexical(local, rval);
          return rval;
        };
      }
    } else {
      // TODO: Implement more kinds of assignment.
      throw new Error("Cannot compile assignment head node: " ~ to!string(head));
    }

    auto rval = compileNode(node.rhs, func);
    return compileAssigner(rval);
  }

  Value compileIdentifier(ast.Identifier identifier, FunctionBuilder func) {
    auto local = identifier.value;
    if (func.haveLocal(local)) {
      auto index = func.getLocal(local);
      return func.current.buildGetLocal(index);
    } else {
      throw new Error("Cannot compile non-local identifier: " ~ local);
    }
    assert(0);
  }

  Value compileInteger(ast.Integer node, FunctionBuilder func) {
    return func.current.buildMakeInteger(node.value);
  }

  Value compilePostfixCall(ast.PostfixCall node, FunctionBuilder func) {
    auto target = compileNode(node.target, func);
    Value[] arguments;
    foreach (argumentNode; node.arguments) {
      arguments ~= compileNode(argumentNode, func);
    }
    return func.current.buildCall(target, arguments);
  }

  Value compileVar(ast.Var node, FunctionBuilder func) {
    auto index = func.getOrAddLocal(node.lhs);
    if (node.rhs) {
      auto rval = compileNode(node.rhs, func);
      func.current.buildSetLocal(index, rval);
    }
    return func.nullValue();
  }

  // Whether or not the given top-level node can be compiled to a constant
  // declaration or must be interpreted.
  bool isConstant(ast.Node node) {
    if (cast(ast.Integer)node) {
      return true;
    }
    return false;
  }
}

unittest {
  auto compiler = new Compiler();
  auto identifier = new ast.Identifier("foo");

  auto let = new ast.Let("bar", identifier, ast.Visibility.Public);
  assert(compiler.isConstant(let) == true);

  auto assignment = new ast.Assignment(identifier, new ast.Integer(1));
  assert(compiler.isConstant(assignment) == false);
}
