var test = "outer"
retrieve() -> test
println(retrieve())

{
  var test = "inner"
  retrieve() -> test
  println(retrieve()) // Should print "inner"
}

println(retrieve()) // Should print "outer"
