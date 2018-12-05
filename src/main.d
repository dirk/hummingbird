import core.stdc.stdlib;
import std.stdio : File, writeln;

import parser;

void main(string[] args) {
  if (args.length != 2) {
    writeln("Usage: hummingbird [file]");
    exit(-1);
  }

  auto input = File(args[1], "r");

  auto program = parser.parse(
    "
      1 * 2 + a = 3;
      a = 4
      let b = a + 5
    ",
    true,
  );
  writeln(program.toPrettyString());
}
