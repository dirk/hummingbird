[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/dirk/hummingbird.svg
[travis-url]: https://travis-ci.org/dirk/hummingbird

# Hummingbird

This is an experimental language and virtual machine.

For the previous JavaScript implementation which compiled to JavaScript or binary-via-LLVM check out the [`legacy-2015`](https://github.com/dirk/hummingbird/tree/legacy-2015) branch.

## Notes

A few environment variables can be used to control debug printing:

- **DEBUG_ALL**: Print all stages of compilation
- **DEBUG_AST**: Print parsed AST
- **DEBUG_IR**: Print intermediate representation (IR)
- **DEBUG_BYTECODE**: Print bytecode

## License

Released under the Modified BSD License. See [LICENSE](LICENSE) for details.
