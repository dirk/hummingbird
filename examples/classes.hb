
class Foo {
  var bar: String = "0"
  let baz: String = "2"

  init () {
    this.bar = "1"
  }
  init (bar: String) {
    this.bar = bar
  }

  func zip () -> String {
    return this.bar
  }
}

var f = new Foo("3")
console.log(f.bar)

