#include <fstream>
#include <iostream>

#include "Parser/Parser.h"

using namespace std;

int main(int argc, char *argv[]) {
  if (argc != 2) {
    cout << "Usage: hummingbird [file]" << endl;
    exit(1);
  }

  auto filename = argv[1];

  fstream source;
  source.open(argv[1], ios_base::in);
  if (source.fail()) {
    cout << "An error has occurred whilst opening " << filename << endl;
    exit(1);
  }

  auto parser = Parser(&source);
  auto root = parser.parse();

  root->debugPrint(&cout, 0);

  return 0;
}
