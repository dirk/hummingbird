
class Foo {
  var bar: Integer
  # let baz: Integer

  init () {
    this.bar = 1
  }
  init (bar: Number) {
    this.bar = bar
  }

  func zip () -> Integer {
    return this.bar
  }
}

var f = new Foo(2)
console.log(f.bar)

