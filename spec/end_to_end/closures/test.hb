var static = "static"
export { static }
closes_static() -> static
println(closes_static())

(() -> {
  var local = "local"
  var closes_local_anonymous = () -> local
  closes_local_named() -> {
    local
  }
  println("anonymous")
  println(closes_local_anonymous())
  println("named")
  println(closes_local_named())
})()

var nested = "nested"
closes_nested1() -> {
  closes_nested2() -> nested
  closes_nested2()
}
println(closes_nested1())
