import core.stdc.stdlib;
import std.file : readText;
import std.stdio : File, writeln;

import parser = parser.parser;

void main(string[] args) {
  if (args.length != 2) {
    writeln("Usage: hummingbird [file]");
    exit(-1);
  }

  auto source = readText!string(args[1]);

  auto program = parser.parse(source, true);
  writeln(program.toPrettyString());
}
