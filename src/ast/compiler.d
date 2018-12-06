module ast.compiler;

import ast = ast.ast;
import bb.builder;

class Compiler {
  void compile(ast.Program program) {
    auto unit = new UnitBuilder();
  }

  // Whether or not the given top-level node can be compiled to a constant
  // declaration or must be interpreted.
  bool isConstant(ast.Node node) {
    if (cast(ast.Let)node) {
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
