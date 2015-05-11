
multi computeCombination (n: Integer, r: Integer) -> Integer
func computeCombination (_, r) -> Integer when (r == 0) { return 1 }
func computeCombination (n: Integer, r: Integer) -> Integer { return n + r }

class A {
  init () { }

  multi b() -> Integer
  func b() { return 1 }
}

# func computeCombination (n, _) { ... }
# func computeCombination (n, r) when (r == 0) { ... }
# func computeCombination (n, r) { ... }

# multi print (Any) -> PrintStatus
# func print(d: Document) { ... }
# func print(p: Photo) { ... }

