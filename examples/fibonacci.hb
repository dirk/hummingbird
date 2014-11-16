// Given n number in the fibonacci sequence, calculate it
let fibonacciRecursive = func (n: Integer) -> Integer {
  
  // If the number is 1 or 0, we've hit the bottom of the trough
  if n == 1 || n == 0 {
    return n
  }
  
  // Recurse down
  return fibonacciRecursive(n - 1) + fibonacciRecursive(n - 2)
}

// The only syntax available to create a function
let fibonacciIterative = func (n: Integer) -> Integer {

  // Explicity typed variables
  var current: Int = 0
  var next: Int = 1
  var future: Int = 1

  // Looping
  for var i = 0; i < n; i += 1 {
    current = next
    next    = future
    future  = current + next
  }

  return current

}
