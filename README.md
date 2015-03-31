# Hummingbird

Hummingbird is a language inspired by JavaScript, ML, and Swift. It features/will feature a usable type system with ML-inspired type inference, JavaScript code generation, and a concise syntax designed for readability.

### Example

An obligatory hello world:

```go
let welcome: String = "Hello "
func sayHello () -> Boolean {
  console.log(welcome + "world")
  return true
}
sayHello()
```

For more examples see the [specification](doc/Specification.md) and [manual](doc/Manual.md).

## Specification

The Hummingbird [specification](doc/Specification.md) is designed to be both human- and machine-readable. It is organized into sections for each syntactical and semantic feature of the language.

Each feature has a `<spec name="..."></spec>` block containing the Hummingbird example source and the expected JavaScript output. These can then be parsed and a full suite of unit tests generated in `test/spec/`.

```bash
# Generating the spec tests
npm run gen-spec
# Running those tests
npm run test-spec
```

## License

Released under the Modified BSD License. See [LICENSE](LICENSE) for details.

