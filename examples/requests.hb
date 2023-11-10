import http from "net/http"
import fs from "std/fs"
// func fs.readFileSync(path, encoding = fs.Encoding.UTF8) -> Result[String, FSError]
// func fs.readFileBytesSync(path) -> Result[Buffer, FSError]

// nil and false are the only falsey values.
// nil is an alias for Optional.None.
// ? and ! operators only work on Result's.
// ?. and ?[] and ?() chaining operators only work on Optionals.

// Function declarations have four parts:
//     Function, parameters' types, return type, body
//
// The body can be either a block (`{}`) or an expression (`=> expr`), since
// blocks are also expressions one could also supply `=> { expr }` as a body
// (may disallow that just to prevent unnecessary extra options). `return`
// looks for the closest parent function block for control flow. Conversely,
// `break` and `continue` control the closest parent block.

// Function statements:
//     func nothing() -> nil {
//     func foo(url) -> String { "bar" }
//     func foo(url) { "bar" }
//     func foo(url) => "bar"

// Function expressions:
//     let foo = fn(url) -> String { "bar" }
//     let foo = fn(url) -> String => "bar"
//     let foo = fn(url) => "bar"

// try {
//     let file = fs.readFileSync("README.md")?
// } catch FSError if error.code == "ENOENT" {
//     println(`Could not read README.md: ${error}`)    
// }

async func main() {
    let file = try fs.readFileSync("README.md")? catch {
        println(`Could not read README.md: ${error}`)
        return
    }
    // Alternative using guard and an Optional:
    //     guard let file = fs.readFileSync("README.md").ok() else {
    //         println("Could not read README.md")
    //         return
    //     }
    // `guard let`/`guard var` are assignments where the right hand side of the
    // assignment must evaluate to an optional. `guard` is a statement which
    // matches the pseudo-syntax "guard EXPRESSION else EXPRESSION".
    //
    // Alternative that just panics:
    //     let file = fs.readFileSync("README.md")!
    // Alternative that's async:
    //     let file = (await fs.readFile("README.md"))!

    let requests = file
        .trim()
        .split("\n")
        .map(fn(url) => http.defaultClient().get(url))
    let responses = await requests
    // HttpClient#get() returns Future[Result[HttpResponse, HttpError]]
    // Alternative using for-await-in:
    //     for await response in requests {
    // Convert errors to nils:
    //     .map(fn(url) => http.defaultClient().get(url).then(fn(result) => result.ok()))
}

// Hummingbird is gradually typed: all values are subtypes of Any, and if it
// can't determine a more precise type it will default to Any. For example,
// a `dequeue` function can be written a number of ways:

// Func(Any) -> Any
func dequeue(array) {
    array.shift()
}

// Func[T](Array[T]) -> Option[T]
func dequeue[T](array: Array[T]) {
    array.shift()
}

// Func[A: { shift(): B }, B](A) -> B
func dequeue(array: infer) {
    array.shift()
}

// read a file in Swift
// let file = try String(contentsOfFile: "README.md")

// read a file in Node.js
// const file = fs.readFileSync('README.md', 'utf8')
