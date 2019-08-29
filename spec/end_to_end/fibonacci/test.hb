var first = 1
var second = 1
var index = 2

while index < 50 {
  var new = first + second
  first = second
  second = new
  index = index + 1
}

println(second)
