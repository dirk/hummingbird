import core.stdc.stdlib;
import std.file : readText;
import std.stdio : File, writeln;

static import ast.compiler;
static import ir.compiler;
static import target.bytecode.printer;

import parser.parser : Parser;
import vm.vm : VM;

void main(string[] args) {
  if (args.length != 2) {
    writeln("Usage: hummingbird [file]");
    exit(-1);
  }

  auto source = readText!string(args[1]);

  auto program = new Parser(source).parseProgram();
  // writeln("AST");
  // writeln(program.toPrettyString());

  auto astCompiler = new ast.compiler.UnitCompiler(program);
  auto irUnit = astCompiler.compile();

  // writeln("\nINTERMEDIATE REPRESENTATION");
  // writeln(irUnit.toPrettyString());

  auto irCompiler = new ir.compiler.UnitCompiler(irUnit);
  auto bytecodeUnit = irCompiler.compile();

  writeln("\nBYTECODE");
  target.bytecode.printer.UnitPrinter.print(cast(immutable)bytecodeUnit);

  writeln("\nRUNNING...");
  auto vm = new VM();
  vm.runMain(&bytecodeUnit);
}
