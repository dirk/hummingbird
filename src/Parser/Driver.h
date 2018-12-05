#pragma once

#include "Nodes.h"

class Driver {
public:
  Driver();

  PRoot* parse(std::istream* input);

  void setRoot(PRoot* root);

private:
  PRoot *root;
};
