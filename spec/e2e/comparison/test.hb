println("integer")
println(1 == 1)
println(1 + 3 == 2 + 2)
// println(1 + 3 == 2 * 2)
println(1 == 2)

println("")

println("function")
a() -> 1
println(a == a)
println(a == () -> 1)

println("")

println("mixed")
println((() -> 1) == 1)
println("1" == 1)
