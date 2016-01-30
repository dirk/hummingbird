[![Build Status][travis-image]][travis-url]
[![Coverage Status][coveralls-image]][coveralls-url]

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

For more examples see the [specification](doc/specification.md) and [manual](doc/manual.md).

## Getting started

The quickest way to get started is to clone the repository and use that directly. This language is actively being built out, so many features you would expect may be missing.

```bash
git clone git@github.com:dirk/hummingbird.git
cd hummingbird
# Install the dependencies
npm install
# We're actively transitioning to TypeScript, so right now you'll need to
# call the `compile` or `watch` tasks to compile the sources to JavaScript.
# nb. We use a custom invocation script for Jake that exposes the V8
#     garbage collector.
./jake # Default task, calls ts:compile
# Run the command-line tool with no arguments to see the options
bin/hb
# To see the parsed and type-checked AST of a file
bin/hb inspect examples/simple.js
# To compile and run a file
bin/hb run examples/simple.js
```

### Contributing

To contribute just [fork][fork] the repository, commit your changes on a branch on your fork, and [create a pull request][pull]!

If you're planning to introduce significant changes/features, then we highly suggest creating an issue with the "Proposal" label ahead-of-time so that everyone can contribute to a discussion before starting to commit development time. We really don't want to have to needlessly turn down pull requests!

[fork]: https://github.com/dirk/hummingbird/fork
[pull]: https://github.com/dirk/hummingbird/compare

## Specification

The Hummingbird [specification](doc/specification.md) is designed to be both human- and machine-readable. It is organized into sections for each syntactical and semantic feature of the language.

Each feature has a `<spec name="..."></spec>` block containing the Hummingbird example source and the expected JavaScript output. These can then be parsed and a full suite of unit tests generated in `test/spec/`.

```bash
# Generating the spec tests
npm run gen-spec
# Running those tests
npm run test-spec
```

## Native compilation (via LLVM)

The LLVM-based native compiler is still in its pre-alpha stages and should be considered very unstable. Getting started with it requires a few more steps on top of the basic setup:

```bash
# The BDW garbage collector is required for building and running, so
# make sure you have it installed. On a Mac with Homebrew you will
# want to do:
brew install bdw-gc
# Call the Jake default task if you haven't already to build the
# standard library "extension" objects
./jake
# Then call the native compiler with a source file
bin/hbn examples/simple.hb
# And run the compiled binary
./a.out
```

## License

Released under the Modified BSD License. See [LICENSE](LICENSE) for details.

[travis-image]: https://travis-ci.org/dirk/hummingbird.svg
[travis-url]: https://travis-ci.org/dirk/hummingbird
[coveralls-image]: https://coveralls.io/repos/github/dirk/hummingbird/badge.svg?branch=master
[coveralls-url]: https://coveralls.io/r/dirk/hummingbird
