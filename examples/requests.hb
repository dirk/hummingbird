import http from "net/http"
import fs from "std/fs"
// func fs.readFileSync(path, encoding = fs.Encoding.UTF8) -> Result[String, FSError]
// func fs.readFileBytesSync(path) -> Result[Buffer, FSError]

// nil and false are the only falsey values.
// nil is an alias for Optional.None.
// ? operator only works on Results.
// ?. and ?[] and ?() chaining operators only work on Optionals.

// Function declarations have four parts:
//     Function, parameters' types, return type, body
//
// The body can be either a block (`{}`) or an expression (`=> expr`), since
// blocks are also expressions one could also supply `=> { expr }` as a body,
// but `return` is not allowed in the latter case since it's within a regular
// block rather than a function block. `return` looks for the closest parent
// function block for control flow. (Conversely, `break` and `continue` control
// the closest parent block).

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
    // Alternative that just panics:
    //     let file = fs.readFileSync("README.md")!
    // Alternative that's async:
    //     let file = (await fs.readFile("README.md"))!

    let requests = file
        .trim()
        .split("\n")
        .map(fn(url) => http.defaultClient().get(url))
    let responses = await requests
    // Alternative using for-await-in:
    //     for await response in requests {
    // Convert errors to nils:
    //     map(async fn(url) => http.defaultClient().get(url).ok())
}

// read a file in Swift
// let file = try String(contentsOfFile: "README.md")

// read a file in Node.js
// const file = fs.readFileSync('README.md', 'utf8')
