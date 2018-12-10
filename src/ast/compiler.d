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

  FunctionBuilder currentFunction;

  this(ast.Program program) {
    this.program = program;
  }

  UnitBuilder compile() {
    auto unit = new UnitBuilder();
    currentFunction = unit.mainFunction;
    foreach(node; program.nodes) {
      compileNode(node);
      assert(currentFunction is unit.mainFunction, "Didn't end up back in main function");
    }
    // Naively ensure the main function ends with a return.
    unit.mainFunction.current.buildReturnNull();
    return unit;
  }

  Value compileNode(ast.Node node) {
    // Roll-your-own dynamic dispatch!
    string buildDynamicDispatches(string[] typeNames ...) {
      string result = "";
      foreach (typeName; typeNames) {
        string demodularizedTypeName = typeName.split(".")[$-1];
        result ~= "
          if (auto lhs = cast(" ~ typeName ~ ")node) {
            return compile" ~ demodularizedTypeName ~ "(lhs);
          }
        ";
      }
      return result;
    }
    mixin(buildDynamicDispatches(
      "ast.Assignment",
      "ast.Function",
      "ast.Identifier",
      "ast.Integer",
      "ast.PostfixCall",
      "ast.Return",
      "ast.Var",
    ));
    if (auto lhs = cast(ast.Block)node) {
      return compileAnonymousBlock(lhs);
    }
    throw new Error("Not implemented for: " ~ to!string(node.classinfo.name));
  }

  // Compile a block that doesn't appear as part of a function, clsas, etc.
  Value compileAnonymousBlock(ast.Block node) {
    // Don't even bother branching for an anonymous block and just return null.
    if (node.nodes.length == 0) {
      return currentFunction.nullValue();
    }

    BasicBlockBuilder enteringFrom = currentBlock;
    BasicBlockBuilder block = currentFunction.newBlock();
    // Make the block we just left branch into us.
    enteringFrom.buildBranch(block);

    Value implicitReturn;
    foreach (index, childNode; node.nodes) {
      auto result = compileNode(childNode);
      if ((index + 1) == node.nodes.length) {
        implicitReturn = result;
      }
    }

    BasicBlockBuilder leavingTo = currentFunction.newBlock();
    block.buildBranch(leavingTo);
    return implicitReturn;
  }

  Value compileAssignment(ast.Assignment node) {
    auto head = node.lhs;
    Value delegate(Value rval) compileAssigner;

    if (auto identifier = cast(ast.Identifier)head) {
      auto local = identifier.value;
      if (currentFunction.haveLocal(local)) {
        auto index = currentFunction.getLocal(local);
        compileAssigner = (Value rval) {
          currentBlock.buildSetLocal(index, rval);
          return rval;
        };
      } else {
        compileAssigner = (Value rval) {
          currentBlock.buildSetLocalLexical(local, rval);
          return rval;
        };
      }
    } else {
      // TODO: Implement more kinds of assignment.
      throw new Error("Cannot compile assignment head node: " ~ to!string(head));
    }

    auto rval = compileNode(node.rhs);
    return compileAssigner(rval);
  }

  Value compileFunction(ast.Function funcNode) {
    auto outerFunction = currentFunction;
    currentFunction = outerFunction.parent.newFunction(funcNode.name);
    foreach(node; funcNode.block.nodes) {
      compileNode(node);
    }
    // Naively ensure the function at least returns null.
    currentBlock.buildReturnNull();
    currentFunction = outerFunction;
    return currentFunction.nullValue();
  }

  Value compileIdentifier(ast.Identifier identifier) {
    auto local = identifier.value;
    if (currentFunction.haveLocal(local)) {
      auto index = currentFunction.getLocal(local);
      return currentBlock.buildGetLocal(index);
    } else {
      return currentBlock.buildGetLocalLexical(local);
    }
  }

  Value compileInteger(ast.Integer node) {
    return currentBlock.buildMakeInteger(node.value);
  }

  Value compilePostfixCall(ast.PostfixCall node) {
    auto target = compileNode(node.target);
    Value[] arguments;
    foreach (argumentNode; node.arguments) {
      arguments ~= compileNode(argumentNode);
    }
    return currentBlock.buildCall(target, arguments);
  }

  Value compileReturn(ast.Return ret) {
    if (ret.rhs is null) {
      currentBlock.buildReturnNull();
    } else {
      auto rval = compileNode(ret.rhs);
      currentBlock.buildReturn(rval);
    }
    return currentFunction.nullValue();
  }

  Value compileVar(ast.Var node) {
    auto index = currentFunction.getOrAddLocal(node.lhs);
    if (node.rhs) {
      auto rval = compileNode(node.rhs);
      currentBlock.buildSetLocal(index, rval);
    }
    return currentFunction.nullValue();
  }

  // Whether or not the given top-level node can be compiled to a constant
  // declaration or must be interpreted.
  bool isConstant(ast.Node node) {
    if (cast(ast.Integer)node) {
      return true;
    }
    return false;
  }

  @property BasicBlockBuilder currentBlock() {
    return currentFunction.current;
  }
}

unittest {
  auto compiler = new UnitCompiler(new ast.Program([]));
  auto identifier = new ast.Identifier("foo");

  auto let = new ast.Integer(1);
  assert(compiler.isConstant(let) == true);

  auto assignment = new ast.Assignment(identifier, new ast.Integer(1));
  assert(compiler.isConstant(assignment) == false);
}
